//! Static site generator: reads `data/` and writes a ready-to-deploy `dist/`.
//!
//! Every page carries its offers in the HTML (no client-side rendering) so search engines and
//! AI crawlers can read them. JavaScript only adds search, saved offers and the install prompt.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use chrono::DateTime;
use minijinja::{Environment, Value};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::{Category, Config};
use crate::history::{self, Signal};
use crate::model::{BankPromo, Chain, Offer};
use crate::output::RunReport;

#[derive(Debug, Deserialize, Serialize)]
pub struct SiteConfig {
    pub url: String,
    pub name: String,
    pub description: String,
    pub per_page: usize,
    pub home_per_chain: usize,
    /// How to reach the maintainers (shown in FAQ, terms and privacy). Hidden when empty.
    #[serde(default)]
    pub contact: String,
    /// Public source repository URL (linked in the footer). Hidden when empty.
    #[serde(default)]
    pub repo: String,
    /// Worker that receives the push subscriptions (`cron/`). Empty: no alerts button.
    #[serde(default)]
    pub push_url: String,
    /// Public VAPID key (base64url) the browser subscribes with. Empty: no alerts button.
    #[serde(default)]
    pub vapid_public_key: String,
}

impl SiteConfig {
    pub fn load(path: &Path) -> Result<SiteConfig> {
        let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut config: SiteConfig = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        config.url = config.url.trim_end_matches('/').to_string();
        Ok(config)
    }
}

/// One offer as the templates and the search index see it.
#[derive(Debug, Serialize)]
struct OfferView {
    key: String,
    name: String,
    chain_name: &'static str,
    price: String,
    list_price: Option<String>,
    discount_pct: Option<u8>,
    promo: Option<String>,
    /// "Llevando N" promos: the price is per unit when taking this many, not the shelf price.
    units: Option<u8>,
    url: Option<String>,
    /// Thumbnail URL on the chain's image server (Carrefour and Coto only).
    image: Option<String>,
    /// What the price history says (`history::Signal`): kind ("good", "warn") and text.
    history: Option<(&'static str, String)>,
}

#[derive(Debug, Serialize)]
struct ChainView {
    id: &'static str,
    name: &'static str,
    store: String,
    /// How to see the same price on the chain's site (from `config/stores.toml`).
    hint: String,
    count: usize,
    count_label: String,
    has_list_prices: bool,
    /// Some offer has a site category (never DAR): the chain page shows the "Categoría" filter.
    has_categories: bool,
    /// The last scrape of this chain failed: the site shows no offers instead of stale ones.
    failed: bool,
    top: Vec<OfferView>,
}

/// One bank promo as the `/bancos/` page shows it.
#[derive(Debug, Serialize)]
struct PromoView {
    chain_name: &'static str,
    discount_pct: u8,
    title: String,
    payment: Option<String>,
    conditions: Option<String>,
    /// "Online", "En sucursal (Hiper, Market)", "Online y en sucursal".
    #[serde(rename = "where")]
    where_label: String,
    /// dd/mm/yyyy.
    valid_until: Option<String>,
    legal: Option<String>,
    url: String,
}

/// One day of the week on the `/bancos/` page (id 0: promos with no day given).
#[derive(Debug, Serialize)]
struct PromoDay {
    id: u8,
    slug: &'static str,
    name: &'static str,
    promos: Vec<PromoView>,
}

const PROMO_DAYS: [(u8, &str, &str); 8] = [
    (1, "lunes", "Lunes"),
    (2, "martes", "Martes"),
    (3, "miercoles", "Miércoles"),
    (4, "jueves", "Jueves"),
    (5, "viernes", "Viernes"),
    (6, "sabado", "Sábado"),
    (7, "domingo", "Domingo"),
    (0, "sin-dia", "Sin día indicado"),
];

struct ChainData {
    chain: Chain,
    store: String,
    hint: String,
    offers: Vec<Offer>,
    failed: bool,
}

pub fn build(data_dir: &Path, out_dir: &Path, site: &SiteConfig, stores: &Config) -> Result<()> {
    let report = read_report(data_dir);
    let updated = updated_label(report.as_ref());
    let chains = load_chains(data_dir, stores, report.as_ref())?;
    // Machine-readable run time: pages carry it so app.js can tell stale (cached) pages apart.
    let updated_iso = report.as_ref().map(|r| r.generated_at.clone()).unwrap_or_default();
    let categories = category_views(&chains, &stores.categories);
    let today = chrono::Utc::now().with_timezone(&chrono::FixedOffset::west_opt(3 * 3600).expect("valid offset")).format("%Y-%m-%d").to_string();
    let promos = load_promos(data_dir, &today)?;
    // Compared with the days before this run (its own date, not the build's).
    let run_day = report.as_ref().and_then(|r| DateTime::parse_from_rfc3339(&r.generated_at).ok()).map_or(today.clone(), |d| d.format("%Y-%m-%d").to_string());
    let signals = history_signals(&chains, &history::load(data_dir, &run_day));
    let view = |offer: &Offer| view(offer, &signals);
    let mut env = environment()?;
    // The "Bancos" tab only shows up when there are promos to show.
    env.add_global("has_promos", !promos.is_empty());
    env.add_global("updated_iso", updated_iso.clone());
    // The "Categoría" filter, on the home page and every chain page that has categories.
    env.add_global("categories", Value::from(minijinja::value::Serde(&categories)));

    if out_dir.exists() {
        fs::remove_dir_all(out_dir).with_context(|| format!("cleaning {}", out_dir.display()))?;
    }
    fs::create_dir_all(out_dir)?;

    let chain_views: Vec<ChainView> = chains
        .iter()
        .map(|c| ChainView {
            id: c.chain.id(),
            name: c.chain.name(),
            store: c.store.clone(),
            hint: c.hint.clone(),
            count: c.offers.len(),
            count_label: group_thousands(c.offers.len() as u64),
            has_list_prices: c.offers.iter().any(|o| o.list_price.is_some()),
            has_categories: c.offers.iter().any(|o| o.category.is_some()),
            failed: c.failed,
            top: highlights(&c.offers, site.home_per_chain).into_iter().map(view).collect(),
        })
        .collect();
    let total: usize = chains.iter().map(|c| c.offers.len()).sum();
    let available: Vec<&str> = chains.iter().filter(|c| !c.failed).map(|c| c.chain.name()).collect();
    let mut sitemap = vec![format!("{}/", site.url), format!("{}/datos/", site.url)];

    // Home.
    let home_offers: Vec<&Offer> = chains.iter().flat_map(|c| highlights(&c.offers, site.home_per_chain)).collect();
    let home_ld = json!([
        { "@context": "https://schema.org", "@type": "WebSite", "name": site.name, "url": format!("{}/", site.url),
          "inLanguage": "es-AR", "description": site.description },
        item_list(&home_offers, "Mejores ofertas del día en Rosario"),
    ]);
    render(&env, "index.html", &out_dir.join("index.html"), &context(site, &updated, &chain_views, json!({
        "title": format!("{} | Ofertas de supermercados en Rosario hoy", site.name),
        "description": site.description,
        "canonical": format!("{}/", site.url),
        "json_ld": json_ld(&home_ld),
        "total_label": group_thousands(total as u64),
        "available_label": join_names(&available),
        "quick": quick_counts(&chains, &categories),
    })))?;

    // One paginated section per chain.
    for (data, chain_view) in chains.iter().zip(&chain_views) {
        let base = format!("/{}/", data.chain.id());
        let pages = data.offers.len().div_ceil(site.per_page).max(1);
        for page in 1..=pages {
            let start = (page - 1) * site.per_page;
            let slice: Vec<&Offer> = data.offers.iter().skip(start).take(site.per_page).collect();
            let path = page_path(&base, page);
            let url = format!("{}{}", site.url, path);
            sitemap.push(url.clone());
            let page_suffix = if page > 1 { format!(" (página {page})") } else { String::new() };
            let title = format!("Ofertas de {} en Rosario hoy{} | {}", data.chain.name(), page_suffix, site.name);
            let description = if data.failed {
                format!("Hoy no pudimos leer las ofertas de {} en Rosario ({}). Actualizado el {}.", data.chain.name(), data.store, updated)
            } else {
                format!(
                    "{} ofertas de {} en Rosario ({}), actualizadas el {}. Precio de oferta, precio anterior y descuento.",
                    chain_view.count_label,
                    data.chain.name(),
                    data.store,
                    updated
                )
            };
            let ld = item_list(&slice, &format!("Ofertas de {} en Rosario", data.chain.name()));
            let prev_url = (page > 1).then(|| format!("{}{}", site.url, page_path(&base, page - 1)));
            let next_url = (page < pages).then(|| format!("{}{}", site.url, page_path(&base, page + 1)));
            let out = out_dir.join(path.trim_start_matches('/')).join("index.html");
            render(&env, "chain.html", &out, &context(site, &updated, &chain_views, json!({
                "title": title,
                "description": description,
                "canonical": url,
                "json_ld": json_ld(&ld),
                "chain": chain_view,
                "chain_base": base,
                "offers": slice.iter().map(|o| view(o)).collect::<Vec<_>>(),
                "start": start + 1,
                "page": page,
                "pages": pages,
                "prev_url": prev_url,
                "next_url": next_url,
            })))?;
        }
    }

    // Open data page and files.
    render(&env, "datos.html", &out_dir.join("datos/index.html"), &context(site, &updated, &chain_views, json!({
        "title": format!("Datos abiertos | {}", site.name),
        "description": "Descargá las ofertas del día de los supermercados de Rosario en JSON y CSV. Libres para usar.",
        "canonical": format!("{}/datos/", site.url),
    })))?;
    write_data_files(out_dir, &chains, &signals)?;
    fs::write(out_dir.join("datos/bancos.json"), serde_json::to_string(&promos)?)?;

    // Bank promos, by day of the week.
    if !promos.is_empty() {
        let url = format!("{}/bancos/", site.url);
        sitemap.push(url.clone());
        let promo_chains: Vec<&str> = Chain::ALL.into_iter().filter(|c| promos.iter().any(|p| p.chain == *c)).map(Chain::name).collect();
        render(&env, "bancos.html", &out_dir.join("bancos/index.html"), &context(site, &updated, &chain_views, json!({
            "title": format!("Descuentos por banco en supermercados de Rosario | {}", site.name),
            "description": format!("Qué descuento hay cada día con bancos, tarjetas y billeteras en {} de Rosario. Actualizado el {updated}.", join_names(&promo_chains)),
            "canonical": url,
            "promo_chains": join_names(&promo_chains),
            "promo_count": promos.len(),
            "promo_days": promo_days(&promos),
        })))?;
    }
    // Tiny file the service worker never caches: app.js compares it with the page's own date.
    fs::write(out_dir.join("datos/actualizado.json"), serde_json::to_string(&json!({ "generated_at": updated_iso }))?)?;

    // Info pages: FAQ, terms, privacy.
    let info_pages = [
        ("preguntas.html", "preguntas-frecuentes", "Preguntas frecuentes", "De dónde salen los precios, cada cuánto se actualiza, cómo se calcula el descuento y más."),
        ("terminos.html", "terminos", "Términos de uso y aviso legal", "Términos de uso de Ofertas Rosario: sitio informativo, sin relación con las cadenas, precios informativos."),
        ("privacidad.html", "privacidad", "Privacidad", "Ofertas Rosario no pide registro, no usa cookies ni analítica y no recolecta datos personales."),
    ];
    for (template, slug, title, description) in info_pages {
        let url = format!("{}/{slug}/", site.url);
        sitemap.push(url.clone());
        render(&env, template, &out_dir.join(slug).join("index.html"), &context(site, &updated, &chain_views, json!({
            "title": format!("{title} | {}", site.name),
            "description": description,
            "canonical": url,
        })))?;
    }

    // No canonical: the 404 is served for any unknown URL and is not the home page (it used to
    // point at "/", which also marked "Inicio" as the current tab).
    render(&env, "404.html", &out_dir.join("404.html"), &context(site, &updated, &chain_views, json!({
        "title": format!("Página no encontrada | {}", site.name),
        "description": site.description,
        "canonical": "",
        "noindex": true,
    })))?;

    write_static_assets(out_dir, &chain_views, &site.push_url)?;
    write_seo_files(out_dir, site, &chain_views, &sitemap, &updated)?;
    tracing::info!("site: {} pages, {total} offers -> {}", sitemap.len(), out_dir.display());
    Ok(())
}

fn environment() -> Result<Environment<'static>> {
    let mut env = Environment::new();
    env.add_template("base.html", include_str!("../templates/base.html"))?;
    env.add_template("icons.html", include_str!("../templates/icons.html"))?;
    env.add_template("search_form.html", include_str!("../templates/search_form.html"))?;
    env.add_template("offers.html", include_str!("../templates/offers.html"))?;
    env.add_template("failed.html", include_str!("../templates/failed.html"))?;
    env.add_template("index.html", include_str!("../templates/index.html"))?;
    env.add_template("chain.html", include_str!("../templates/chain.html"))?;
    env.add_template("datos.html", include_str!("../templates/datos.html"))?;
    env.add_template("bancos.html", include_str!("../templates/bancos.html"))?;
    env.add_template("404.html", include_str!("../templates/404.html"))?;
    env.add_template("preguntas.html", include_str!("../templates/preguntas.html"))?;
    env.add_template("terminos.html", include_str!("../templates/terminos.html"))?;
    env.add_template("privacidad.html", include_str!("../templates/privacidad.html"))?;
    Ok(env)
}

/// Common context plus page-specific values.
fn context(site: &SiteConfig, updated: &str, chains: &[ChainView], page: serde_json::Value) -> Value {
    let mut ctx = json!({ "site": site, "updated": updated, "chains": chains });
    if let (Some(base), serde_json::Value::Object(extra)) = (ctx.as_object_mut(), page) {
        base.extend(extra);
    }
    Value::from(minijinja::value::Serde(ctx))
}

fn render(env: &Environment, template: &str, out: &Path, ctx: &Value) -> Result<()> {
    let html = env.get_template(template)?.render(ctx).with_context(|| format!("rendering {template}"))?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, html).with_context(|| format!("writing {}", out.display()))
}

fn page_path(base: &str, page: usize) -> String {
    if page == 1 { base.to_string() } else { format!("{base}pagina/{page}/") }
}

/// Bank promos of every chain that has a promos file, minus the ones already over (a chain whose
/// promos could not be read keeps an old file). Biggest discount first.
fn load_promos(data_dir: &Path, today: &str) -> Result<Vec<BankPromo>> {
    let mut promos = Vec::new();
    for chain in Chain::ALL {
        let path = data_dir.join("promos").join(format!("{}.json", chain.id()));
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let chain_promos: Vec<BankPromo> = serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        promos.extend(chain_promos.into_iter().filter(|p| p.valid_until.as_deref().is_none_or(|until| until >= today) && p.url.starts_with("https://")));
    }
    promos.sort_by(|a, b| b.discount_pct.cmp(&a.discount_pct).then_with(|| a.chain.cmp(&b.chain)).then_with(|| a.title.cmp(&b.title)));
    Ok(promos)
}

/// Groups the promos by day: one that runs several days shows up in each of them.
fn promo_days(promos: &[BankPromo]) -> Vec<PromoDay> {
    PROMO_DAYS
        .iter()
        .map(|&(id, slug, name)| PromoDay {
            id,
            slug,
            name,
            promos: promos.iter().filter(|p| if id == 0 { p.days.is_empty() } else { p.days.contains(&id) }).map(promo_view).collect(),
        })
        .filter(|day| !day.promos.is_empty())
        .collect()
}

fn promo_view(promo: &BankPromo) -> PromoView {
    let formats = if promo.formats.is_empty() { String::new() } else { format!(" ({})", promo.formats.join(", ")) };
    let where_label = match (promo.online, promo.in_store) {
        (true, true) => format!("Online y en sucursal{formats}"),
        (true, false) => "Solo online".to_string(),
        (false, true) => format!("Solo en sucursal{formats}"),
        (false, false) => "Ver dónde aplica en el legal".to_string(),
    };
    PromoView {
        chain_name: promo.chain.name(),
        discount_pct: promo.discount_pct,
        title: promo.title.clone(),
        payment: promo.payment.clone(),
        conditions: promo.conditions.clone(),
        where_label,
        valid_until: promo.valid_until.as_deref().and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()).map(|d| d.format("%d/%m/%Y").to_string()),
        legal: promo.legal.clone(),
        url: promo.url.clone(),
    }
}

fn read_report(data_dir: &Path) -> Option<RunReport> {
    let text = fs::read_to_string(data_dir.join("run.json")).ok()?;
    serde_json::from_str(&text).ok()
}

/// Run date as shown to people, e.g. "11/11/2026 06:03". Falls back to "hoy".
fn updated_label(report: Option<&RunReport>) -> String {
    report
        .and_then(|report| DateTime::parse_from_rfc3339(&report.generated_at).ok())
        .map(|date| date.format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|| "hoy".to_string())
}

/// "Carrefour", "Carrefour y DAR", "Carrefour, Coto y DAR".
fn join_names(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => only.to_string(),
        [rest @ .., last] => format!("{} y {last}", rest.join(", ")),
    }
}

/// Loads each chain's data file. A chain whose last scrape failed (per `run.json`) gets no
/// offers: `data/` keeps yesterday's file, but the site only shows today's offers.
fn load_chains(data_dir: &Path, stores: &Config, report: Option<&RunReport>) -> Result<Vec<ChainData>> {
    let mut chains = Vec::new();
    for chain in Chain::ALL {
        let failed = report.and_then(|r| r.chains.get(chain.id())).is_some_and(|c| !c.ok);
        let path = data_dir.join("offers").join(format!("{}.json", chain.id()));
        if !failed && !path.exists() {
            tracing::warn!("{} has no data file, skipping", chain.id());
            continue;
        }
        let mut offers: Vec<Offer> = if failed {
            tracing::warn!("{}: last scrape failed, publishing it with no offers", chain.id());
            Vec::new()
        } else {
            let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?
        };
        // Defense in depth: only https links reach the pages, never javascript: or data: URLs.
        for offer in &mut offers {
            if offer.url.as_deref().is_some_and(|url| !url.starts_with("https://")) {
                offer.url = None;
            }
        }
        offers.sort_by(rank);
        let (store, hint) = match chain {
            Chain::Carrefour => (stores.carrefour.store.clone(), stores.carrefour.hint.clone()),
            Chain::Coto => (stores.coto.store.clone(), stores.coto.hint.clone()),
            Chain::Dar => (stores.dar.store.clone(), stores.dar.hint.clone()),
        };
        chains.push(ChainData { chain, store, hint, offers, failed });
    }
    Ok(chains)
}

/// Biggest discount first; offers without a known discount after, by name.
fn rank(a: &Offer, b: &Offer) -> Ordering {
    b.discount_pct.unwrap_or(0).cmp(&a.discount_pct.unwrap_or(0)).then_with(|| a.name.cmp(&b.name))
}

/// The home page's "best offers" of a chain: in rank order, but one per product kind (first word
/// of the name), so five sizes of the same dog collar or every flavor of a jelly don't fill the
/// list. For a chain without discounts (DAR) this also spreads the list over different products.
fn highlights(offers: &[Offer], n: usize) -> Vec<&Offer> {
    let mut seen = std::collections::HashSet::new();
    offers
        .iter()
        .filter(|offer| seen.insert(fold(offer.name.split(|c: char| !c.is_alphanumeric()).find(|w| !w.is_empty()).unwrap_or(""))))
        .take(n)
        .collect()
}

/// Offer key -> what its price history says, for today's offers that have something to say.
fn history_signals(chains: &[ChainData], history: &history::History) -> HashMap<String, Signal> {
    chains
        .iter()
        .flat_map(|c| &c.offers)
        .filter_map(|offer| Some((format!("{}:{}", offer.chain.id(), offer.id), history::signal(offer, history.past(offer))?)))
        .collect()
}

fn view(offer: &Offer, signals: &HashMap<String, Signal>) -> OfferView {
    let key = format!("{}:{}", offer.chain.id(), offer.id);
    OfferView {
        history: signals.get(&key).map(|s| (s.kind(), s.label())),
        key: format!("{}:{}", offer.chain.id(), offer.id),
        name: offer.name.clone(),
        chain_name: offer.chain.name(),
        price: format_ars(offer.price),
        list_price: offer.list_price.map(format_ars),
        discount_pct: offer.discount_pct,
        promo: promo_chip(offer.promo.as_deref()),
        units: promo_units(offer.promo.as_deref()),
        url: offer.url.clone(),
        image: thumbnail(offer),
    }
}

static TAKING: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)\s*\(?\s*llevando\s*(\d+)\s*\)?").expect("valid regex"));

/// "6x4 (Llevando 6)" -> 6: the offer price is per unit when taking that many (Coto's multi-buy
/// promos). Same rule as `promoUnits` in app.js.
fn promo_units(promo: Option<&str>) -> Option<u8> {
    let units: u8 = TAKING.captures(promo?)?.get(1)?.as_str().parse().ok()?;
    (units > 1).then_some(units)
}

/// Promo text worth showing next to the discount badge. Hidden when it repeats what the page
/// already says: a bare "35%Dto" (the badge shows the discount) or DAR's generic "Oferta" (every
/// DAR offer has it). "(Llevando N)" is dropped: the price line already says "c/u llevando N".
/// The full text stays in the data files. Same rule as `promoChip` in app.js.
fn promo_chip(promo: Option<&str>) -> Option<String> {
    static BARE_PCT: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)^\d+\s*%\s*dto\.?$").expect("valid regex"));
    let promo = TAKING.replace_all(promo?, "");
    let promo = promo.trim();
    (!promo.is_empty() && !promo.eq_ignore_ascii_case("oferta") && !BARE_PCT.is_match(promo)).then(|| promo.to_string())
}

/// Kind of promotion, from the chain's promo text and the discount. Same rule as `promoKind` in
/// app.js, so the shortcut counts on the home page match what the search filter finds.
fn promo_kind(offer: &Offer) -> &'static str {
    static NXM: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)\d\s*x\s*\d").expect("valid regex"));
    static SECOND: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)2da").expect("valid regex"));
    static SEVERAL: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)llevando").expect("valid regex"));
    let promo = offer.promo.as_deref().unwrap_or("");
    if NXM.is_match(promo) {
        "nxm"
    } else if SECOND.is_match(promo) {
        "2da"
    } else if SEVERAL.is_match(promo) {
        "varias"
    } else if offer.discount_pct.unwrap_or(0) > 0 {
        "rebaja"
    } else {
        ""
    }
}

/// How many of today's offers each home shortcut finds ("50% o más", "2x1 y 3x2"...). Zero means
/// the shortcut is hidden. Failed chains have no offers, same as in the search index.
fn quick_counts(chains: &[ChainData], categories: &[CategoryView]) -> serde_json::Value {
    let offers = || chains.iter().flat_map(|c| c.offers.iter());
    let label = |n: usize| (n > 0).then(|| group_thousands(n as u64));
    json!({
        "min50": label(offers().filter(|o| o.discount_pct.unwrap_or(0) >= 50).count()),
        "min40": label(offers().filter(|o| o.discount_pct.unwrap_or(0) >= 40).count()),
        "nxm": label(offers().filter(|o| promo_kind(o) == "nxm").count()),
        "second": label(offers().filter(|o| promo_kind(o) == "2da").count()),
        "categories": categories,
    })
}

/// A site category with offers today, for the home shortcuts and the "Categoría" filter.
#[derive(Debug, Serialize)]
struct CategoryView {
    id: String,
    name: String,
    count: String,
}

/// The configured categories that have offers today, in config order. Empty when the data has no
/// categories yet (written before they existed): then neither the shortcuts nor the filter show.
fn category_views(chains: &[ChainData], categories: &[Category]) -> Vec<CategoryView> {
    categories
        .iter()
        .filter_map(|category| {
            let n = chains.iter().flat_map(|c| &c.offers).filter(|o| o.category.as_deref() == Some(category.id.as_str())).count();
            (n > 0).then(|| CategoryView { id: category.id.clone(), name: category.name.clone(), count: group_thousands(n as u64) })
        })
        .collect()
}

/// Lowercase without accents, like `normalize` in app.js (enough for Spanish product names).
pub(crate) fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// Image hosts the pages may load from. Must match `img-src` in `web/_headers`.
const CARREFOUR_IMAGES: &str = "https://carrefourar.vtexassets.com/";
const COTO_IMAGES: &str = "https://static.cotodigital3.com.ar/";

/// Small product photo shown next to an offer, loaded by the visitor's browser from the chain's
/// image server. Carrefour gets a 160 px version (VTEX resizes on the fly: `/ids/<id>-160-160/`);
/// Coto's "medium" photo is already small. DAR is left out: it serves photos from its own small
/// server and we don't want to load it with every visit.
fn thumbnail(offer: &Offer) -> Option<String> {
    let url = offer.image.as_deref()?;
    match offer.chain {
        Chain::Carrefour if url.starts_with(CARREFOUR_IMAGES) => {
            static IDS: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(/arquivos/ids/\d+)/").expect("valid regex"));
            Some(IDS.replace(url, "${1}-160-160/").into_owned())
        }
        Chain::Coto if url.starts_with(COTO_IMAGES) => Some(url.to_string()),
        _ => None,
    }
}

/// 13400 -> "13.400".
fn group_thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut grouped = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(digit);
    }
    grouped
}

/// "$1.234,56", or "$1.234" for whole pesos.
pub fn format_ars(cents: u64) -> String {
    let grouped = group_thousands(cents / 100);
    match cents % 100 {
        0 => format!("${grouped}"),
        rest => format!("${grouped},{rest:02}"),
    }
}

fn price_decimal(cents: u64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// schema.org ItemList of Product + Offer for the offers shown on a page.
fn item_list(offers: &[&Offer], name: &str) -> serde_json::Value {
    let items: Vec<serde_json::Value> = offers
        .iter()
        .enumerate()
        .map(|(i, offer)| {
            let mut product = json!({
                "@type": "Product",
                "name": offer.name,
                "offers": {
                    "@type": "Offer",
                    "price": price_decimal(offer.price),
                    "priceCurrency": "ARS",
                    "availability": "https://schema.org/InStock",
                    "seller": { "@type": "Organization", "name": offer.chain.name() },
                },
            });
            if let Some(ean) = offer.ean.as_deref().filter(|e| e.len() == 13 && e.chars().all(|c| c.is_ascii_digit())) {
                product["gtin13"] = json!(ean);
            }
            if let Some(image) = &offer.image {
                product["image"] = json!(image);
            }
            if let Some(url) = &offer.url {
                product["offers"]["url"] = json!(url);
            }
            json!({ "@type": "ListItem", "position": i + 1, "item": product })
        })
        .collect();
    json!({ "@context": "https://schema.org", "@type": "ItemList", "name": name, "numberOfItems": items.len(), "itemListElement": items })
}

/// Serializes JSON-LD safely for a <script> tag.
fn json_ld(value: &serde_json::Value) -> String {
    value.to_string().replace("</", "<\\/")
}

fn write_data_files(out_dir: &Path, chains: &[ChainData], signals: &HashMap<String, Signal>) -> Result<()> {
    let dir = out_dir.join("datos");
    fs::create_dir_all(&dir)?;
    let mut search_rows = Vec::new();
    let mut csv = csv::Writer::from_path(dir.join("hoy.csv"))?;
    // New columns go at the end, so scripts that read columns by position keep working.
    csv.write_record(["cadena", "sucursal", "id", "ean", "nombre", "precio_centavos", "precio_anterior_centavos", "descuento_pct", "promo", "url", "categoria"])?;
    for data in chains {
        fs::write(dir.join(format!("{}.json", data.chain.id())), serde_json::to_string(&data.offers)?)?;
        for offer in &data.offers {
            let opt = |v: Option<String>| v.unwrap_or_default();
            csv.write_record([
                offer.chain.id().to_string(),
                offer.store.clone(),
                offer.id.clone(),
                opt(offer.ean.clone()),
                offer.name.clone(),
                offer.price.to_string(),
                opt(offer.list_price.map(|p| p.to_string())),
                opt(offer.discount_pct.map(|p| p.to_string())),
                opt(offer.promo.clone()),
                opt(offer.url.clone()),
                opt(offer.category.clone()),
            ])?;
            // Compact rows for the in-browser search: [key, chain, name, price, list, pct, promo, url, image, category, history].
            // history is [kind, text] or null (see `history::Signal`).
            let key = format!("{}:{}", offer.chain.id(), offer.id);
            let history = signals.get(&key).map(|s| json!([s.kind(), s.label()]));
            search_rows.push(json!([
                key,
                offer.chain.name(),
                offer.name,
                offer.price,
                offer.list_price,
                offer.discount_pct,
                offer.promo,
                offer.url,
                thumbnail(offer),
                offer.category,
                history,
            ]));
        }
    }
    csv.flush()?;
    fs::write(dir.join("buscar.json"), serde_json::to_string(&search_rows)?)?;
    Ok(())
}

fn write_static_assets(out_dir: &Path, chains: &[ChainView], push_url: &str) -> Result<()> {
    fs::write(out_dir.join("app.js"), include_str!("../web/app.js"))?;
    fs::write(out_dir.join("styles.css"), include_str!("../web/styles.css"))?;
    fs::write(out_dir.join("theme.js"), include_str!("../web/theme.js"))?;
    // Inter (SIL OFL 1.1), Latin subset, variable weight; the license travels with the font.
    let fonts = out_dir.join("fonts");
    fs::create_dir_all(&fonts)?;
    fs::write(fonts.join("inter-latin-wght.woff2"), include_bytes!("../web/fonts/inter-latin-wght.woff2"))?;
    fs::write(fonts.join("OFL.txt"), include_str!("../web/fonts/OFL.txt"))?;
    fs::write(out_dir.join("manifest.webmanifest"), include_str!("../web/manifest.webmanifest"))?;
    // The page and the service worker talk to the alerts Worker: its origin joins connect-src.
    let push_origin = reqwest::Url::parse(push_url).ok().map(|u| format!(" {}", u.origin().ascii_serialization())).unwrap_or_default();
    fs::write(out_dir.join("_headers"), include_str!("../web/_headers").replace("connect-src 'self'", &format!("connect-src 'self'{push_origin}")))?;
    let icons = out_dir.join("icons");
    fs::create_dir_all(&icons)?;
    fs::write(icons.join("icon-192.png"), include_bytes!("../web/icons/icon-192.png"))?;
    fs::write(icons.join("icon-512.png"), include_bytes!("../web/icons/icon-512.png"))?;
    fs::write(icons.join("maskable-512.png"), include_bytes!("../web/icons/maskable-512.png"))?;
    fs::write(icons.join("favicon.svg"), include_str!("../web/icons/favicon.svg"))?;

    // Offline: the shell and the first page of each chain. The search index (~100 KB compressed)
    // is not precached: it is cached when first used, and app.js warms it up in the installed app.
    let mut precache = vec![
        "/".to_string(),
        "/app.js".into(),
        "/styles.css".into(),
        "/theme.js".into(),
        "/fonts/inter-latin-wght.woff2".into(),
        "/manifest.webmanifest".into(),
        "/icons/icon-192.png".into(),
        "/icons/favicon.svg".into(),
    ];
    precache.extend(chains.iter().map(|c| format!("/{}/", c.id)));
    let version = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let sw = include_str!("../web/sw.js")
        .replace("__VERSION__", &version)
        .replace("__PRECACHE__", &serde_json::to_string(&precache)?)
        .replace("__PUSH_URL__", push_url.trim_end_matches('/'));
    fs::write(out_dir.join("sw.js"), sw)?;
    Ok(())
}

fn write_seo_files(out_dir: &Path, site: &SiteConfig, chains: &[ChainView], sitemap: &[String], updated: &str) -> Result<()> {
    // Search engines and AI assistants are explicitly welcome.
    let robots = format!(
        "User-agent: *\nAllow: /\n\n\
         User-agent: GPTBot\nAllow: /\n\n\
         User-agent: OAI-SearchBot\nAllow: /\n\n\
         User-agent: ChatGPT-User\nAllow: /\n\n\
         User-agent: ClaudeBot\nAllow: /\n\n\
         User-agent: Claude-SearchBot\nAllow: /\n\n\
         User-agent: Claude-User\nAllow: /\n\n\
         User-agent: PerplexityBot\nAllow: /\n\n\
         User-agent: Google-Extended\nAllow: /\n\n\
         User-agent: Applebot-Extended\nAllow: /\n\n\
         Sitemap: {}/sitemap.xml\n",
        site.url
    );
    fs::write(out_dir.join("robots.txt"), robots)?;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for url in sitemap {
        xml.push_str(&format!("  <url><loc>{url}</loc><lastmod>{today}</lastmod><changefreq>daily</changefreq></url>\n"));
    }
    xml.push_str("</urlset>\n");
    fs::write(out_dir.join("sitemap.xml"), xml)?;

    // llms.txt: a plain summary for AI assistants (https://llmstxt.org).
    let mut llms = format!(
        "# {}\n\n> {}\n\nSitio independiente y de código abierto, sin relación con las cadenas. Se actualiza en general una vez por día. Última actualización: {updated}. \
         Los precios son los de la web de cada cadena para Rosario, Santa Fe, Argentina, y pueden diferir del precio en góndola.\n\n## Ofertas por cadena\n\n",
        site.name, site.description
    );
    if sitemap.iter().any(|url| url.ends_with("/bancos/")) {
        llms.push_str(&format!("- [Descuentos por banco en Rosario]({}/bancos/): qué descuento hay cada día con bancos, tarjetas y billeteras.\n", site.url));
    }
    for c in chains {
        let count = if c.failed { "hoy sin datos (no se pudo leer la web de la cadena)".to_string() } else { format!("{} ofertas hoy", c.count_label) };
        llms.push_str(&format!("- [Ofertas de {} en Rosario]({}/{}/): {count}. {}.\n", c.name, site.url, c.id, c.store));
    }
    llms.push_str(&format!(
        "\n## Datos abiertos\n\n- [Todas las ofertas del día en CSV]({url}/datos/hoy.csv)\n",
        url = site.url
    ));
    for c in chains {
        llms.push_str(&format!("- [Ofertas de {} en JSON]({}/datos/{}.json): precios en centavos de peso argentino.\n", c.name, site.url, c.id));
    }
    if sitemap.iter().any(|url| url.ends_with("/bancos/")) {
        llms.push_str(&format!("- [Descuentos por banco en JSON]({}/datos/bancos.json): promos con bancos, tarjetas y billeteras, por día.\n", site.url));
    }
    llms.push_str(&format!("- [Descripción de los campos]({}/datos/)\n", site.url));
    fs::write(out_dir.join("llms.txt"), llms)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_pesos_argentine_style() {
        assert_eq!(format_ars(186_000), "$1.860");
        assert_eq!(format_ars(283_765), "$2.837,65");
        assert_eq!(format_ars(47_500), "$475");
        assert_eq!(format_ars(123_456_789), "$1.234.567,89");
        assert_eq!(format_ars(5), "$0,05");
    }

    #[test]
    fn json_ld_cannot_close_the_script_tag() {
        assert_eq!(json_ld(&json!({"name": "</script>"})), r#"{"name":"<\/script>"}"#);
    }

    #[test]
    fn templates_parse() {
        environment().unwrap();
    }

    #[test]
    fn thumbnails_only_for_carrefour_and_coto() {
        let offer = |chain: Chain, image: &str| Offer {
            chain,
            store: "x".into(),
            id: "1".into(),
            ean: None,
            name: "x".into(),
            price: 100,
            list_price: None,
            discount_pct: None,
            promo: None,
            url: None,
            image: Some(image.into()),
            source: crate::model::Source::Web,
            category: None,
        };
        assert_eq!(
            thumbnail(&offer(Chain::Carrefour, "https://carrefourar.vtexassets.com/arquivos/ids/171526/foto.jpg?v=1")).as_deref(),
            Some("https://carrefourar.vtexassets.com/arquivos/ids/171526-160-160/foto.jpg?v=1")
        );
        let coto = "https://static.cotodigital3.com.ar/sitios/fotos/medium/00000100/00000157.jpg";
        assert_eq!(thumbnail(&offer(Chain::Coto, coto)).as_deref(), Some(coto));
        assert_eq!(thumbnail(&offer(Chain::Dar, "https://www.darentucasa.com.ar/Fotos/Articulos/1.jpg")), None);
        // Hosts outside the CSP never reach the pages.
        assert_eq!(thumbnail(&offer(Chain::Coto, "https://otro.example/foto.jpg")), None);
    }

    #[test]
    fn promo_chip_hides_repeated_text() {
        assert_eq!(promo_chip(Some("35%Dto")), None);
        assert_eq!(promo_chip(Some("Oferta")), None);
        assert_eq!(promo_chip(None), None);
        // "(Llevando N)" moves to the price line ("c/u llevando N").
        assert_eq!(promo_chip(Some("35%Dto (Llevando 2)")), None);
        assert_eq!(promo_chip(Some("2x1 (Llevando 2)")).as_deref(), Some("2x1"));
        assert_eq!(promo_chip(Some("70% 2da (Llevando 2)")).as_deref(), Some("70% 2da"));
    }


    #[test]
    fn multi_buy_price_is_per_unit() {
        assert_eq!(promo_units(Some("6x4 (Llevando 6)")), Some(6));
        assert_eq!(promo_units(Some("70% 2da (Llevando 2)")), Some(2));
        assert_eq!(promo_units(Some("Llevando 1")), None);
        assert_eq!(promo_units(Some("35%Dto")), None);
        assert_eq!(promo_units(None), None);
    }

    #[test]
    fn highlights_skip_variants_of_the_same_product() {
        let offer = |name: &str| Offer {
            chain: Chain::Dar,
            store: "x".into(),
            id: name.into(),
            ean: None,
            name: name.into(),
            price: 100,
            list_price: None,
            discount_pct: None,
            promo: None,
            url: None,
            image: None,
            source: crate::model::Source::Web,
            category: None,
        };
        let offers = ["Gelatina Anana", "Gelatina Cereza", "ACONDIC. SEDAL", "ACONDIC.PLUSBELLE", "Acondic. Dove", "Yerba"].map(offer);
        let names: Vec<&str> = highlights(&offers, 8).iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, ["Gelatina Anana", "ACONDIC. SEDAL", "Yerba"]);
        assert_eq!(highlights(&offers, 2).len(), 2);
    }

    #[test]
    fn promo_kind_matches_the_search_filter() {
        let offer = |promo: Option<&str>, discount_pct: Option<u8>| Offer {
            chain: Chain::Coto,
            store: "x".into(),
            id: "1".into(),
            ean: None,
            name: "x".into(),
            price: 100,
            list_price: None,
            discount_pct,
            promo: promo.map(Into::into),
            url: None,
            image: None,
            source: crate::model::Source::Web,
            category: None,
        };
        assert_eq!(promo_kind(&offer(Some("2x1 (Llevando 2)"), Some(50))), "nxm");
        assert_eq!(promo_kind(&offer(Some("3 X 2"), None)), "nxm");
        assert_eq!(promo_kind(&offer(Some("70% 2da (Llevando 2)"), None)), "2da");
        assert_eq!(promo_kind(&offer(Some("35%Dto (Llevando 2)"), Some(17))), "varias");
        assert_eq!(promo_kind(&offer(Some("35%Dto"), Some(35))), "rebaja");
        assert_eq!(promo_kind(&offer(None, None)), "");
    }

    #[test]
    fn joins_chain_names() {
        assert_eq!(join_names(&[]), "");
        assert_eq!(join_names(&["Coto"]), "Coto");
        assert_eq!(join_names(&["Carrefour", "DAR"]), "Carrefour y DAR");
        assert_eq!(join_names(&["Carrefour", "Coto", "DAR"]), "Carrefour, Coto y DAR");
    }

    #[test]
    fn pager_links_stay_on_the_same_site() {
        let root = std::env::temp_dir().join(format!("ofertas-rosario-pager-{}", std::process::id()));
        let (data, out) = (root.join("data"), root.join("dist"));
        fs::create_dir_all(data.join("offers")).unwrap();
        let offers: Vec<String> = (1..=3)
            .map(|i| {
                format!(
                    r#"{{"chain":"carrefour","store":"x","id":"{i}","ean":null,"name":"Producto {i}","price":100,"list_price":200,"discount_pct":50,"promo":null,"url":null,"image":null,"source":"web"}}"#
                )
            })
            .collect();
        fs::write(data.join("offers/carrefour.json"), format!("[{}]", offers.join(","))).unwrap();
        fs::write(
            data.join("run.json"),
            r#"{"generated_at":"2026-11-12T07:05:00-03:00","chains":{"carrefour":{"ok":true,"offers":3,"seconds":1,"error":null}}}"#,
        )
        .unwrap();

        let stores = Config::load(Path::new("config/stores.toml")).unwrap();
        let mut site = SiteConfig::load(Path::new("config/site.toml")).unwrap();
        site.per_page = 1;
        build(&data, &out, &site, &stores).unwrap();

        // minijinja's HTML escaping writes "/" as "&#x2f;" inside attributes; browsers read both the same.
        let page2 = fs::read_to_string(out.join("carrefour/pagina/2/index.html")).unwrap().replace("&#x2f;", "/");
        let pager = &page2[page2.find(r#"<nav class="pager""#).unwrap()..];
        let pager = &pager[..pager.find("</nav>").unwrap()];
        assert!(pager.contains(r#"href="/carrefour/" rel="prev""#), "{pager}");
        assert!(pager.contains(r#"href="/carrefour/pagina/3/" rel="next""#), "{pager}");
        // The canonical (absolute) URLs belong in <link rel>, never in the visible links.
        assert!(!pager.contains(&site.url), "{pager}");
        assert!(page2.contains(&format!(r#"<link rel="next" href="{}/carrefour/pagina/3/">"#, site.url)));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn failed_chain_is_published_without_stale_offers() {
        let root = std::env::temp_dir().join(format!("ofertas-rosario-test-{}", std::process::id()));
        let (data, out) = (root.join("data"), root.join("dist"));
        fs::create_dir_all(data.join("offers")).unwrap();
        let offer = |chain: &str, name: &str| {
            format!(
                r#"[{{"chain":"{chain}","store":"x","id":"1","ean":null,"name":"{name}","price":100,"list_price":200,"discount_pct":50,"promo":null,"url":null,"image":null,"source":"web"}}]"#
            )
        };
        fs::write(data.join("offers/carrefour.json"), offer("carrefour", "Yerba de hoy")).unwrap();
        fs::write(data.join("offers/coto.json"), offer("coto", "Fideos de ayer")).unwrap();
        fs::write(
            data.join("run.json"),
            r#"{"generated_at":"2026-11-12T07:05:00-03:00","chains":{
                "carrefour":{"ok":true,"offers":1,"seconds":1,"error":null},
                "coto":{"ok":false,"offers":0,"seconds":1,"error":"timeout"}}}"#,
        )
        .unwrap();

        let stores = Config::load(Path::new("config/stores.toml")).unwrap();
        let site = SiteConfig::load(Path::new("config/site.toml")).unwrap();
        build(&data, &out, &site, &stores).unwrap();

        let home = fs::read_to_string(out.join("index.html")).unwrap();
        let coto = fs::read_to_string(out.join("coto/index.html")).unwrap();
        let search = fs::read_to_string(out.join("datos/buscar.json")).unwrap();
        let csv = fs::read_to_string(out.join("datos/hoy.csv")).unwrap();
        assert!(home.contains("Yerba de hoy"));
        assert!(home.contains("Hoy no pudimos leer las ofertas de Coto"));
        assert!(coto.contains("Hoy no pudimos leer las ofertas de Coto"));
        for text in [&home, &coto, &search, &csv] {
            assert!(!text.contains("Fideos de ayer"));
        }
        assert_eq!(fs::read_to_string(out.join("datos/coto.json")).unwrap(), "[]");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn categories_become_shortcuts_and_a_filter() {
        let root = std::env::temp_dir().join(format!("ofertas-rosario-cat-test-{}", std::process::id()));
        let (data, out) = (root.join("data"), root.join("dist"));
        fs::create_dir_all(data.join("offers")).unwrap();
        let offer = |chain: &str, id: &str, name: &str, category: Option<&str>| {
            let category = category.map(|c| format!(r#","category":"{c}""#)).unwrap_or_default();
            format!(
                r#"{{"chain":"{chain}","store":"x","id":"{id}","ean":null,"name":"{name}","price":100,"list_price":200,"discount_pct":50,"promo":null,"url":null,"image":null,"source":"web"{category}}}"#
            )
        };
        let carrefour = [offer("carrefour", "1", "Vacio x kg", Some("carnes")), offer("carrefour", "2", "Alimento gato carne", Some("mascotas"))];
        fs::write(data.join("offers/carrefour.json"), format!("[{}]", carrefour.join(","))).unwrap();
        fs::write(data.join("offers/coto.json"), format!("[{}]", offer("coto", "3", "Higado X KG", Some("carnes")))).unwrap();
        fs::write(data.join("offers/dar.json"), format!("[{}]", offer("dar", "4", "YERBA", None))).unwrap();

        let stores = Config::load(Path::new("config/stores.toml")).unwrap();
        let site = SiteConfig::load(Path::new("config/site.toml")).unwrap();
        build(&data, &out, &site, &stores).unwrap();

        let home = fs::read_to_string(out.join("index.html")).unwrap().replace("&#x2f;", "/");
        // Only categories with offers today, with their count.
        assert!(home.contains(r#"<a href="/?categoria=carnes#buscar" data-quick>Carnes y pescados <span class="quick-words-count">2<"#));
        assert!(home.contains("/?categoria=mascotas#buscar"));
        assert!(!home.contains("/?categoria=limpieza#buscar"));
        assert!(home.contains(r#"<option value="carnes">Carnes y pescados</option>"#));
        // DAR has no categories: no filter on its page; Coto's page has it.
        assert!(!fs::read_to_string(out.join("dar/index.html")).unwrap().contains(r#"id="f-cat""#));
        assert!(fs::read_to_string(out.join("coto/index.html")).unwrap().contains(r#"id="f-cat""#));
        // The search index and the open data carry the category.
        let search: Vec<serde_json::Value> = serde_json::from_str(&fs::read_to_string(out.join("datos/buscar.json")).unwrap()).unwrap();
        let category_of = |key: &str| search.iter().find(|row| row[0] == key).map(|row| row[9].clone()).unwrap();
        assert_eq!(category_of("coto:3"), "carnes");
        assert_eq!(category_of("dar:4"), serde_json::Value::Null);
        let csv = fs::read_to_string(out.join("datos/hoy.csv")).unwrap();
        assert!(csv.lines().next().unwrap().ends_with(",url,categoria"));
        assert!(csv.lines().any(|line| line.starts_with("coto,") && line.ends_with(",carnes")));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn alerts_appear_only_when_configured() {
        let root = std::env::temp_dir().join(format!("ofertas-rosario-push-test-{}", std::process::id()));
        let (data, out) = (root.join("data"), root.join("dist"));
        fs::create_dir_all(data.join("offers")).unwrap();
        fs::write(data.join("offers/coto.json"), "[]").unwrap();
        let stores = Config::load(Path::new("config/stores.toml")).unwrap();
        let mut site = SiteConfig::load(Path::new("config/site.toml")).unwrap();

        site.push_url = String::new();
        site.vapid_public_key = String::new();
        build(&data, &out, &site, &stores).unwrap();
        assert!(!fs::read_to_string(out.join("index.html")).unwrap().contains(r#"id="avisos""#));
        assert!(fs::read_to_string(out.join("_headers")).unwrap().contains("connect-src 'self';"));
        assert!(!fs::read_to_string(out.join("privacidad/index.html")).unwrap().contains("Avisos de ofertas"));

        site.push_url = "https://avisos.example.workers.dev/".into();
        site.vapid_public_key = "BPUBLICA".into();
        build(&data, &out, &site, &stores).unwrap();
        let home = fs::read_to_string(out.join("index.html")).unwrap().replace("&#x2f;", "/");
        assert!(home.contains(r#"data-push-url="https://avisos.example.workers.dev/" data-vapid-key="BPUBLICA""#));
        assert!(fs::read_to_string(out.join("_headers")).unwrap().contains("connect-src 'self' https://avisos.example.workers.dev;"));
        assert!(fs::read_to_string(out.join("sw.js")).unwrap().contains("const PUSH_URL = 'https://avisos.example.workers.dev';"));
        assert!(fs::read_to_string(out.join("privacidad/index.html")).unwrap().contains("Avisos de ofertas"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn data_without_categories_shows_no_category_ui() {
        let root = std::env::temp_dir().join(format!("ofertas-rosario-nocat-test-{}", std::process::id()));
        let (data, out) = (root.join("data"), root.join("dist"));
        fs::create_dir_all(data.join("offers")).unwrap();
        // Written before categories existed: no "category" key at all.
        let old = r#"[{"chain":"coto","store":"x","id":"1","ean":null,"name":"Fideos","price":100,"list_price":200,"discount_pct":50,"promo":null,"url":null,"image":null,"source":"web"}]"#;
        fs::write(data.join("offers/coto.json"), old).unwrap();

        let stores = Config::load(Path::new("config/stores.toml")).unwrap();
        let site = SiteConfig::load(Path::new("config/site.toml")).unwrap();
        build(&data, &out, &site, &stores).unwrap();

        let home = fs::read_to_string(out.join("index.html")).unwrap();
        assert!(!home.contains("Por categoría"));
        assert!(!home.contains(r#"id="f-cat""#));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn bank_promos_page_groups_by_day_and_hides_expired() {
        let root = std::env::temp_dir().join(format!("ofertas-rosario-promos-test-{}", std::process::id()));
        let (data, out) = (root.join("data"), root.join("dist"));
        fs::create_dir_all(data.join("offers")).unwrap();
        fs::create_dir_all(data.join("promos")).unwrap();
        fs::write(data.join("offers/coto.json"), "[]").unwrap();
        let stores = Config::load(Path::new("config/stores.toml")).unwrap();
        let site = SiteConfig::load(Path::new("config/site.toml")).unwrap();

        // No promos files: no page, no tab.
        build(&data, &out, &site, &stores).unwrap();
        assert!(!out.join("bancos/index.html").exists());
        assert!(!fs::read_to_string(out.join("index.html")).unwrap().contains(r#"href="/bancos/""#));
        // Without the page, the terms keep saying bank discounts are not included.
        assert!(fs::read_to_string(out.join("terminos/index.html")).unwrap().contains("No incluimos descuentos por banco"));

        let promo = |id: &str, pct: u8, title: &str, days: &str, until: &str| {
            format!(
                r#"{{"chain":"coto","id":"{id}","discount_pct":{pct},"title":"{title}","payment":"Naranja X","conditions":"Tope $12.000","days":{days},"online":true,"in_store":true,"formats":[],"valid_until":{until},"legal":null,"url":"https://www.coto.com.ar/descuentos"}}"#
            )
        };
        let promos = [
            promo("1", 30, "Martes y jueves", "[2,4]", "null"),
            promo("2", 10, "Sin dia", "[]", "null"),
            promo("3", 50, "Ya vencida", "[2]", r#""2020-01-01""#),
        ];
        fs::write(data.join("promos/coto.json"), format!("[{}]", promos.join(","))).unwrap();
        build(&data, &out, &site, &stores).unwrap();

        let page = fs::read_to_string(out.join("bancos/index.html")).unwrap();
        assert_eq!(page.matches("<h3 class=\"card-name promo-title\">Martes y jueves</h3>").count(), 2, "one card per day");
        assert!(page.contains(r#"<section id="martes""#) && page.contains(r#"<section id="jueves""#));
        assert!(!page.contains(r#"<section id="lunes""#), "days with no promos are left out");
        assert!(page.contains(r#"<section id="sin-dia""#));
        assert!(page.contains("Online y en sucursal"));
        assert!(!page.contains("Ya vencida"));
        assert!(fs::read_to_string(out.join("index.html")).unwrap().contains(r#"href="/bancos/""#));
        let open: Vec<serde_json::Value> = serde_json::from_str(&fs::read_to_string(out.join("datos/bancos.json")).unwrap()).unwrap();
        assert_eq!(open.len(), 2);
        assert!(fs::read_to_string(out.join("sitemap.xml")).unwrap().contains("/bancos/</loc>"));
        // Legal coverage: terms, privacy and FAQ talk about the bank promos.
        let terms = fs::read_to_string(out.join("terminos/index.html")).unwrap();
        assert!(terms.contains(r#"<p id="bancos">"#) && !terms.contains("No incluimos descuentos por banco"));
        assert!(fs::read_to_string(out.join("privacidad/index.html")).unwrap().contains("<h2>Descuentos por banco</h2>"));
        assert!(fs::read_to_string(out.join("preguntas-frecuentes/index.html")).unwrap().contains("¿De dónde salen los descuentos por banco?"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn price_history_comes_from_the_daily_commits() {
        let root = std::env::temp_dir().join(format!("ofertas-rosario-history-test-{}", std::process::id()));
        let (data, out) = (root.join("data"), root.join("dist"));
        fs::create_dir_all(data.join("offers")).unwrap();
        let git = |args: &[&str], date: &str| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["-c", "user.name=bot", "-c", "user.email=bot@example.com", "-c", "commit.gpgsign=false"])
                .args(args)
                .env("GIT_AUTHOR_DATE", date)
                .env("GIT_COMMITTER_DATE", date)
                .status();
            status.is_ok_and(|s| s.success())
        };
        if !git(&["init", "-q"], "2026-11-01T12:00:00+00:00") {
            fs::remove_dir_all(&root).unwrap();
            return; // no git on this machine
        }
        let offers = |yerba: (u64, u64), fideos: u64| {
            format!(
                r#"[{{"chain":"carrefour","store":"x","id":"1","ean":null,"name":"Yerba","price":{},"list_price":{},"discount_pct":10,"promo":null,"url":null,"image":null,"source":"web"}},
{{"chain":"carrefour","store":"x","id":"2","ean":null,"name":"Fideos","price":{fideos},"list_price":2000,"discount_pct":10,"promo":null,"url":null,"image":null,"source":"web"}}]"#,
                yerba.0, yerba.1
            )
        };
        // Three earlier days (the bot commits at UTC times) with the usual prices.
        for day in ["2026-11-10", "2026-11-11", "2026-11-12"] {
            fs::write(data.join("offers/carrefour.json"), offers((900, 1000), 1800)).unwrap();
            assert!(git(&["add", "-A"], &format!("{day}T14:00:00+00:00")));
            assert!(git(&["commit", "-q", "--allow-empty", "-m", "data"], &format!("{day}T14:00:00+00:00")));
        }
        // Today: same yerba price with a raised "before" price, fideos at their lowest.
        fs::write(data.join("offers/carrefour.json"), offers((900, 1500), 1500)).unwrap();
        fs::write(data.join("run.json"), r#"{"generated_at":"2026-11-13T12:05:00-03:00","chains":{"carrefour":{"ok":true,"offers":2,"seconds":1,"error":null}}}"#).unwrap();

        let stores = Config::load(Path::new("config/stores.toml")).unwrap();
        let site = SiteConfig::load(Path::new("config/site.toml")).unwrap();
        build(&data, &out, &site, &stores).unwrap();

        let page = fs::read_to_string(out.join("carrefour/index.html")).unwrap();
        assert!(page.contains(r#"<p class="history-note history-warn">El precio anterior subió hace poco</p>"#), "{page}");
        assert!(page.contains(r#"<p class="history-note history-good">El más bajo en 30 días</p>"#));
        let search: Vec<serde_json::Value> = serde_json::from_str(&fs::read_to_string(out.join("datos/buscar.json")).unwrap()).unwrap();
        let history_of = |key: &str| search.iter().find(|row| row[0] == key).map(|row| row[10].clone()).unwrap();
        assert_eq!(history_of("carrefour:1"), json!(["warn", "El precio anterior subió hace poco"]));
        assert_eq!(history_of("carrefour:2"), json!(["good", "El más bajo en 30 días"]));
        fs::remove_dir_all(&root).unwrap();
    }
}
