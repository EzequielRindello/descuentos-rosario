//! Static site generator: reads `data/` and writes a ready-to-deploy `dist/`.
//!
//! Every page carries its offers in the HTML (no client-side rendering) so search engines and
//! AI crawlers can read them. JavaScript only adds search, saved offers and the install prompt.

use std::cmp::Ordering;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::DateTime;
use minijinja::{Environment, Value};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::Config;
use crate::model::{Chain, Offer};
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
    url: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChainView {
    id: &'static str,
    name: &'static str,
    store: String,
    count: usize,
    count_label: String,
    has_list_prices: bool,
    /// The last scrape of this chain failed: the site shows no offers instead of stale ones.
    failed: bool,
    top: Vec<OfferView>,
}

struct ChainData {
    chain: Chain,
    store: String,
    offers: Vec<Offer>,
    failed: bool,
}

pub fn build(data_dir: &Path, out_dir: &Path, site: &SiteConfig, stores: &Config) -> Result<()> {
    let report = read_report(data_dir);
    let updated = updated_label(report.as_ref());
    let chains = load_chains(data_dir, stores, report.as_ref())?;
    // Machine-readable run time: pages carry it so app.js can tell stale (cached) pages apart.
    let updated_iso = report.as_ref().map(|r| r.generated_at.clone()).unwrap_or_default();
    let mut env = environment()?;
    env.add_global("updated_iso", updated_iso.clone());

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
            count: c.offers.len(),
            count_label: group_thousands(c.offers.len() as u64),
            has_list_prices: c.offers.iter().any(|o| o.list_price.is_some()),
            failed: c.failed,
            top: c.offers.iter().take(site.home_per_chain).map(view).collect(),
        })
        .collect();
    let total: usize = chains.iter().map(|c| c.offers.len()).sum();
    let available: Vec<&str> = chains.iter().filter(|c| !c.failed).map(|c| c.chain.name()).collect();
    let mut sitemap = vec![format!("{}/", site.url), format!("{}/datos/", site.url)];

    // Home.
    let home_offers: Vec<&Offer> = chains.iter().flat_map(|c| c.offers.iter().take(site.home_per_chain)).collect();
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
    write_data_files(out_dir, &chains)?;
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

    render(&env, "404.html", &out_dir.join("404.html"), &context(site, &updated, &chain_views, json!({
        "title": format!("Página no encontrada | {}", site.name),
        "description": site.description,
        "canonical": format!("{}/", site.url),
    })))?;

    write_static_assets(out_dir, &chain_views)?;
    write_seo_files(out_dir, site, &chain_views, &sitemap, &updated)?;
    tracing::info!("site: {} pages, {total} offers -> {}", sitemap.len(), out_dir.display());
    Ok(())
}

fn environment() -> Result<Environment<'static>> {
    let mut env = Environment::new();
    env.add_template("base.html", include_str!("../templates/base.html"))?;
    env.add_template("offers.html", include_str!("../templates/offers.html"))?;
    env.add_template("failed.html", include_str!("../templates/failed.html"))?;
    env.add_template("index.html", include_str!("../templates/index.html"))?;
    env.add_template("chain.html", include_str!("../templates/chain.html"))?;
    env.add_template("datos.html", include_str!("../templates/datos.html"))?;
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
        let store = match chain {
            Chain::Carrefour => stores.carrefour.store.clone(),
            Chain::Coto => stores.coto.store.clone(),
            Chain::Dar => stores.dar.store.clone(),
        };
        chains.push(ChainData { chain, store, offers, failed });
    }
    Ok(chains)
}

/// Biggest discount first; offers without a known discount after, by name.
fn rank(a: &Offer, b: &Offer) -> Ordering {
    b.discount_pct.unwrap_or(0).cmp(&a.discount_pct.unwrap_or(0)).then_with(|| a.name.cmp(&b.name))
}

fn view(offer: &Offer) -> OfferView {
    OfferView {
        key: format!("{}:{}", offer.chain.id(), offer.id),
        name: offer.name.clone(),
        chain_name: offer.chain.name(),
        price: format_ars(offer.price),
        list_price: offer.list_price.map(format_ars),
        discount_pct: offer.discount_pct,
        promo: offer.promo.clone(),
        url: offer.url.clone(),
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

fn write_data_files(out_dir: &Path, chains: &[ChainData]) -> Result<()> {
    let dir = out_dir.join("datos");
    fs::create_dir_all(&dir)?;
    let mut search_rows = Vec::new();
    let mut csv = csv::Writer::from_path(dir.join("hoy.csv"))?;
    csv.write_record(["cadena", "sucursal", "id", "ean", "nombre", "precio_centavos", "precio_anterior_centavos", "descuento_pct", "promo", "url"])?;
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
            ])?;
            // Compact rows for the in-browser search: [key, chain, name, price, list, pct, promo, url].
            search_rows.push(json!([
                format!("{}:{}", offer.chain.id(), offer.id),
                offer.chain.name(),
                offer.name,
                offer.price,
                offer.list_price,
                offer.discount_pct,
                offer.promo,
                offer.url,
            ]));
        }
    }
    csv.flush()?;
    fs::write(dir.join("buscar.json"), serde_json::to_string(&search_rows)?)?;
    Ok(())
}

fn write_static_assets(out_dir: &Path, chains: &[ChainView]) -> Result<()> {
    fs::write(out_dir.join("app.js"), include_str!("../web/app.js"))?;
    fs::write(out_dir.join("manifest.webmanifest"), include_str!("../web/manifest.webmanifest"))?;
    fs::write(out_dir.join("_headers"), include_str!("../web/_headers"))?;
    let icons = out_dir.join("icons");
    fs::create_dir_all(&icons)?;
    fs::write(icons.join("icon-192.png"), include_bytes!("../web/icons/icon-192.png"))?;
    fs::write(icons.join("icon-512.png"), include_bytes!("../web/icons/icon-512.png"))?;
    fs::write(icons.join("maskable-512.png"), include_bytes!("../web/icons/maskable-512.png"))?;
    fs::write(icons.join("favicon.svg"), include_str!("../web/icons/favicon.svg"))?;

    // Offline: the shell, the search index and the first page of each chain.
    let mut precache = vec!["/".to_string(), "/app.js".into(), "/manifest.webmanifest".into(), "/datos/buscar.json".into(), "/icons/icon-192.png".into()];
    precache.extend(chains.iter().map(|c| format!("/{}/", c.id)));
    let version = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let sw = include_str!("../web/sw.js").replace("__VERSION__", &version).replace("__PRECACHE__", &serde_json::to_string(&precache)?);
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
        "# {}\n\n> {}\n\nSitio independiente y de código abierto. Se actualiza todos los días por la mañana (hora de Argentina). Última actualización: {updated}. \
         Los precios son los de la web de cada cadena para Rosario, Santa Fe, Argentina, y pueden diferir del precio en góndola.\n\n## Ofertas por cadena\n\n",
        site.name, site.description
    );
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
    fn joins_chain_names() {
        assert_eq!(join_names(&[]), "");
        assert_eq!(join_names(&["Coto"]), "Coto");
        assert_eq!(join_names(&["Carrefour", "DAR"]), "Carrefour y DAR");
        assert_eq!(join_names(&["Carrefour", "Coto", "DAR"]), "Carrefour, Coto y DAR");
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
}
