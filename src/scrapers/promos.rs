//! Bank and payment-method promos ("30% los martes con Naranja X") of Carrefour and Coto.
//!
//! Both chains publish them nationwide, with no region field: promos for other provinces are
//! dropped by their text. Installment-only promos (no percentage) are dropped too. DAR only
//! publishes them as images.

use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, Utc};
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::Http;
use crate::model::{BankPromo, Chain};
use crate::site::fold;

const CARREFOUR_GRAPHQL: &str = "https://www.carrefour.com.ar/_v/segment/graphql/v1?workspace=master";
/// Where Carrefour lists its promos with their legal terms.
pub const CARREFOUR_PAGE: &str = "https://www.carrefour.com.ar/descuentos-bancarios";
const COTO_PROMOS: &str = "https://www.coto.com.ar/rest/model/atg/actors/cProfileActor/getPromocionesMulticanal?enviroment=ag";
/// Where Coto lists its promos and links their legal terms.
pub const COTO_PAGE: &str = "https://www.coto.com.ar/descuentos";

const STORE_ONLY_PREFIX: &str = "Exclusivo en sucursales.";

const CARREFOUR_FIELDS: [&str; 20] = [
    "id", "title", "sub_title", "discount_percentage", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday",
    "sunday", "hyper", "market", "express", "maxi", "ecommerce", "legal", "active_to", "idBank", "idCard",
];
const WEEKDAYS: [&str; 7] = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
/// Carrefour store formats, as the promo flags them, and the name shown for each.
const CARREFOUR_FORMATS: [(&str, &str); 4] = [("hyper", "Hiper"), ("market", "Market"), ("express", "Express"), ("maxi", "Maxi")];

static PERCENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{1,2})\s*%").expect("valid regex"));
/// A list of provinces or cities the promo is limited to ("PROVINCIAS DE: SANTA FE, ...",
/// "Válido para tiendas de AMBA, ..."). Matched on folded (lowercase, no accents) text.
static REGION_LIMIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"provincias? de\b|valido (para|en) (las )?tiendas de").expect("valid regex"));
/// Coto promos only for Buenos Aires city residents. Matched on folded text.
static CABA_ONLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"portena|portenos|\bcaba\b|ciudad autonoma").expect("valid regex"));

/// Reads the chain's promos. `names` turns the chain's bank, card or logo ids into names people
/// recognize (`promo_names` in `config/stores.toml`).
pub async fn scrape(chain: Chain, names: &BTreeMap<String, String>, http: &Http) -> Result<Option<Vec<BankPromo>>> {
    let promos = match chain {
        Chain::Carrefour => carrefour(names, http).await?,
        Chain::Coto => coto(names, http).await?,
        Chain::Dar => return Ok(None),
    };
    Ok(Some(promos))
}

// ---------- Carrefour: Master Data entity "BP" through the site's public GraphQL ----------

#[derive(Debug, Deserialize)]
struct GraphqlResponse {
    data: GraphqlData,
}

#[derive(Debug, Deserialize)]
struct GraphqlData {
    #[serde(default)]
    documents: Vec<Document>,
}

#[derive(Debug, Deserialize)]
struct Document {
    #[serde(default)]
    fields: Vec<Field>,
}

#[derive(Debug, Deserialize)]
struct Field {
    key: String,
    value: Option<String>,
}

/// One Master Data document as a map, with the "null" strings the API sends left out.
fn document_fields(document: Document) -> HashMap<String, String> {
    document
        .fields
        .into_iter()
        .filter_map(|f| f.value.filter(|v| !v.trim().is_empty() && v != "null").map(|v| (f.key, v)))
        .collect()
}

fn documents_query(acronym: &str, fields: &[&str], filter: Option<&str>) -> Value {
    let fields = fields.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(",");
    let filter = filter.map(|w| format!(", where: \"{w}\", sort: \"order ASC\"")).unwrap_or_default();
    json!({ "query": format!("{{ documents(acronym: \"{acronym}\", schema: \"mdv1\", fields: [{fields}]{filter}, pageSize: 999) {{ fields {{ key value }} }} }}") })
}

async fn carrefour_documents(http: &Http, query: Value) -> Result<Vec<HashMap<String, String>>> {
    let response: GraphqlResponse = http.json(http.post(CARREFOUR_GRAPHQL).json(&query)).await?;
    Ok(response.data.documents.into_iter().map(document_fields).collect())
}

async fn carrefour(names: &BTreeMap<String, String>, http: &Http) -> Result<Vec<BankPromo>> {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S.000Z");
    let filter = format!("active=true AND ((active_from < {now}) AND (active_to > {now}))");
    let promos = carrefour_documents(http, documents_query("BP", &CARREFOUR_FIELDS, Some(&filter))).await.context("reading Carrefour promos")?;
    let banks = carrefour_documents(http, documents_query("FB", &["id", "name"], None)).await.context("reading Carrefour banks")?;
    let cards = carrefour_documents(http, documents_query("FC", &["id", "name"], None)).await.context("reading Carrefour cards")?;
    let total = promos.len();
    let parsed = parse_carrefour(promos, &id_names(banks), &id_names(cards), names);
    tracing::info!("carrefour: {} bank promos ({} left out: installments or other provinces)", parsed.len(), total - parsed.len());
    Ok(parsed)
}

fn id_names(documents: Vec<HashMap<String, String>>) -> HashMap<String, String> {
    documents.into_iter().filter_map(|mut d| Some((d.remove("id")?, d.remove("name")?))).collect()
}

fn parse_carrefour(
    promos: Vec<HashMap<String, String>>,
    banks: &HashMap<String, String>,
    cards: &HashMap<String, String>,
    names: &BTreeMap<String, String>,
) -> Vec<BankPromo> {
    let mut parsed = Vec::new();
    for promo in promos {
        let get = |key: &str| promo.get(key).map(|v| v.trim().to_string());
        let flag = |key: &str| promo.get(key).is_some_and(|v| v == "true");
        let (Some(id), Some(title)) = (get("id"), get("title")) else { continue };
        let Some(pct) = get("discount_percentage").and_then(|p| p.parse::<f64>().ok()).filter(|p| *p >= 1.0 && *p < 100.0) else {
            continue; // installments
        };
        let conditions = get("sub_title");
        let legal = get("legal");
        let text = fold(&[title.as_str(), conditions.as_deref().unwrap_or(""), legal.as_deref().unwrap_or("")].join(" "));
        if !applies_in_santa_fe(&text) {
            continue;
        }
        let payment = [(get("idBank"), banks), (get("idCard"), cards)]
            .into_iter()
            .find_map(|(id, table)| table.get(&id?).cloned())
            .map(|raw| display_name(&raw, names));
        parsed.push(BankPromo {
            chain: Chain::Carrefour,
            id,
            discount_pct: pct as u8,
            title,
            payment,
            conditions,
            days: WEEKDAYS.iter().zip(1u8..).filter(|(day, _)| flag(day)).map(|(_, n)| n).collect(),
            online: flag("ecommerce"),
            in_store: CARREFOUR_FORMATS.iter().any(|(key, _)| flag(key)),
            formats: CARREFOUR_FORMATS.iter().filter(|(key, _)| flag(key)).map(|(_, name)| name.to_string()).collect(),
            valid_until: get("active_to").and_then(|t| last_valid_day(&t)),
            legal,
            url: CARREFOUR_PAGE.to_string(),
        });
    }
    parsed
}

/// Carrefour promos limited to a list of provinces only count when Santa Fe (or Rosario) is in it.
fn applies_in_santa_fe(folded: &str) -> bool {
    !REGION_LIMIT.is_match(folded) || folded.contains("santa fe") || folded.contains("rosario")
}

/// `active_to` is the first instant the promo is over (midnight UTC): the last valid day is the
/// Argentine date one second before.
fn last_valid_day(active_to: &str) -> Option<String> {
    let end = DateTime::parse_from_rfc3339(active_to).ok()?;
    let argentina = FixedOffset::west_opt(3 * 3600)?;
    Some((end - chrono::Duration::seconds(1)).with_timezone(&argentina).format("%Y-%m-%d").to_string())
}

// ---------- Coto: the public endpoint behind coto.com.ar/descuentos ----------

#[derive(Debug, Deserialize)]
struct CotoResponse {
    result: CotoResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CotoResult {
    #[serde(default)]
    promociones_digitales: Vec<CotoPromo>,
    #[serde(default)]
    promociones_sucursales_fisicas: Vec<CotoPromo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CotoPromo {
    id: String,
    #[serde(default)]
    texto_descuento: Option<String>,
    #[serde(default)]
    descripcion: Option<String>,
    #[serde(default)]
    observacion: Option<String>,
    #[serde(default)]
    icono: Option<String>,
    #[serde(default)]
    dias: Option<Vec<CotoDay>>,
}

#[derive(Debug, Deserialize)]
struct CotoDay {
    /// 1 = Sunday ... 7 = Saturday.
    id: u8,
}

async fn coto(names: &BTreeMap<String, String>, http: &Http) -> Result<Vec<BankPromo>> {
    let response: CotoResponse = http.json(http.get(COTO_PROMOS)).await.context("reading Coto promos")?;
    let total = response.result.promociones_digitales.len() + response.result.promociones_sucursales_fisicas.len();
    let parsed = parse_coto(response.result, names);
    tracing::info!("coto: {} bank promos from {total} (installments, Buenos Aires city only and repeats left out)", parsed.len());
    Ok(parsed)
}

fn parse_coto(result: CotoResult, names: &BTreeMap<String, String>) -> Vec<BankPromo> {
    // The same promo often shows up online and in stores, and sometimes twice in one list: it is
    // kept once, with both channels.
    let mut kept: Vec<BankPromo> = Vec::new();
    let channels = [(true, result.promociones_digitales), (false, result.promociones_sucursales_fisicas)];
    for (online, promos) in channels {
        for promo in promos {
            let clean = |v: &Option<String>| v.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
            let (Some(discount), Some(title)) = (clean(&promo.texto_descuento), clean(&promo.descripcion)) else { continue };
            // "Exclusivo en sucursales." repeats what in_store already says on the page.
            let title = title.strip_prefix(STORE_ONLY_PREFIX).map(str::trim_start).filter(|t| !t.is_empty()).map_or(title.clone(), str::to_string);
            let Some(pct) = PERCENT.captures(&discount).and_then(|c| c[1].parse::<u8>().ok()).filter(|p| *p >= 1) else {
                continue; // installments
            };
            let conditions = clean(&promo.observacion);
            let icon = clean(&promo.icono);
            let text = fold(&[title.as_str(), conditions.as_deref().unwrap_or(""), icon.as_deref().unwrap_or("")].join(" "));
            if CABA_ONLY.is_match(&text) {
                continue;
            }
            let mut days: Vec<u8> = promo.dias.unwrap_or_default().iter().map(|d| if d.id == 1 { 7 } else { d.id - 1 }).filter(|d| (1..=7).contains(d)).collect();
            days.sort_unstable();
            days.dedup();
            let payment = icon.map(|i| display_name(&i, names));
            if let Some(same) = kept.iter_mut().find(|p| p.discount_pct == pct && p.days == days && p.payment == payment && p.title == title && p.conditions == conditions) {
                same.online |= online;
                same.in_store |= !online;
                continue;
            }
            kept.push(BankPromo {
                chain: Chain::Coto,
                id: promo.id,
                discount_pct: pct,
                title,
                payment,
                conditions,
                days,
                online,
                in_store: !online,
                formats: Vec::new(),
                valid_until: None,
                legal: None,
                url: COTO_PAGE.to_string(),
            });
        }
    }
    kept
}

/// Name people recognize for a bank, card or logo id: `promo_names` if it is there, else the id
/// cleaned up ("logo_naranjax2.png" -> "Naranjax", "Banco_Formosa" -> "Banco Formosa").
fn display_name(raw: &str, names: &BTreeMap<String, String>) -> String {
    if let Some(name) = names.get(raw) {
        return name.clone();
    }
    let base = raw.rsplit_once('.').filter(|(_, ext)| ext.len() <= 4).map_or(raw, |(stem, _)| stem);
    let base = base.strip_prefix("logo_").unwrap_or(base);
    let base = base.trim_end_matches(|c: char| c.is_ascii_digit() || c == '_');
    let mut words = base.split('_').filter(|w| !w.is_empty()).map(|w| {
        let mut chars = w.chars();
        chars.next().map(|first| first.to_uppercase().chain(chars).collect::<String>()).unwrap_or_default()
    });
    let first = words.next().unwrap_or_else(|| raw.to_string());
    words.fold(first, |acc, w| format!("{acc} {w}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_documents(text: &str) -> Vec<HashMap<String, String>> {
        let response: GraphqlResponse = serde_json::from_str(text).unwrap();
        response.data.documents.into_iter().map(document_fields).collect()
    }

    fn repo_names() -> BTreeMap<String, String> {
        let config = crate::config::Config::load(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/config/stores.toml"))).unwrap();
        config.promo_names
    }

    fn carrefour_fixture() -> Vec<BankPromo> {
        let promos = fixture_documents(include_str!("../../tests/fixtures/carrefour_promos.json"));
        let banks = id_names(fixture_documents(include_str!("../../tests/fixtures/carrefour_banks.json")));
        let cards = id_names(fixture_documents(include_str!("../../tests/fixtures/carrefour_cards.json")));
        parse_carrefour(promos, &banks, &cards, &repo_names())
    }

    #[test]
    fn carrefour_keeps_percent_promos_valid_in_rosario() {
        let promos = carrefour_fixture();
        let titles: Vec<&str> = promos.iter().map(|p| p.title.as_str()).collect();
        // Installments, Buenos Aires province only, and public employees of other provinces: out.
        assert!(!titles.iter().any(|t| t.contains("Cuotas")), "{titles:?}");
        assert!(!titles.contains(&"10% de descuento en tu compra ¡Sin Tope!"), "{titles:?}");
        assert!(!titles.contains(&"Si sos empleada/o público todos los miércoles"), "{titles:?}");
        // The Thursday one lists Santa Fe: in. The company address in CABA is not a region limit.
        assert!(titles.contains(&"Si sos empleado/a público todos los jueves"), "{titles:?}");
        assert!(titles.contains(&"20% de descuento en un pago con Cuenta Digital de Carrefour Banco"), "{titles:?}");
        assert_eq!(promos.len(), 4, "{titles:?}");
    }

    #[test]
    fn carrefour_promo_fields() {
        let promos = carrefour_fixture();
        let digital = promos.iter().find(|p| p.title.starts_with("20% de descuento en un pago con Cuenta Digital")).unwrap();
        assert_eq!(digital.discount_pct, 20);
        assert_eq!(digital.days, vec![4]);
        assert!(digital.online && !digital.in_store);
        assert!(digital.formats.is_empty());
        assert_eq!(digital.payment.as_deref(), Some("Cuenta Digital Carrefour Banco"));
        assert!(digital.conditions.as_deref().unwrap().contains("Tope de devolución $10.000"));
        // Ends 2026-12-01T00:00 UTC: the last day is November 30 in Argentina.
        assert_eq!(digital.valid_until.as_deref(), Some("2026-11-30"));
        assert!(digital.legal.as_deref().unwrap().starts_with("DESCUENTO EXCLUSIVO"));

        let patagonia = promos.iter().find(|p| p.title.starts_with("15% de Ahorro")).unwrap();
        assert_eq!(patagonia.payment.as_deref(), Some("Patagonia"));
        assert!(patagonia.in_store && !patagonia.online);
        assert_eq!(patagonia.formats, ["Hiper", "Market", "Express", "Maxi"]);

        let modo = promos.iter().find(|p| p.title.starts_with("Ahorra con MODO")).unwrap();
        assert_eq!(modo.payment, None);
        assert_eq!(modo.days, vec![3]);
    }

    #[test]
    fn coto_keeps_percent_promos_once_with_both_channels() {
        let result: CotoResponse = serde_json::from_str(include_str!("../../tests/fixtures/coto_promos.json")).unwrap();
        let promos = parse_coto(result.result, &repo_names());
        let payments: Vec<&str> = promos.iter().filter_map(|p| p.payment.as_deref()).collect();
        // Installments (Macro) and "Ciudadanía porteña" are out.
        assert!(!payments.contains(&"Macro"), "{payments:?}");
        assert!(!promos.iter().any(|p| fold(&p.title).contains("portena")), "{payments:?}");

        let naranja = promos.iter().filter(|p| p.payment.as_deref() == Some("Naranja X")).collect::<Vec<_>>();
        assert_eq!(naranja.len(), 1, "the same Naranja X promo is online and in stores");
        assert!(naranja[0].online && naranja[0].in_store);
        assert_eq!(naranja[0].discount_pct, 30);
        assert_eq!(naranja[0].days, vec![2]);

        let mercadopago = promos.iter().find(|p| p.payment.as_deref() == Some("Mercado Pago")).unwrap();
        assert!(mercadopago.in_store && !mercadopago.online);
        assert_eq!(mercadopago.title, "Utilizando todos los medios de pago dentro de la app", "no repeated store-only prefix");
        assert_eq!(mercadopago.days, vec![6, 7], "Saturday and Sunday (Coto counts from Sunday = 1)");

        let anses = promos.iter().find(|p| p.payment.as_deref() == Some("ANSES")).unwrap();
        assert!(anses.days.is_empty(), "Coto gives no day for this one");
        let extra = promos.iter().find(|p| p.title.starts_with("PLAN SUELDO") && p.payment.as_deref() == Some("Credicoop")).unwrap();
        assert_eq!(extra.discount_pct, 10, "+10% reads as 10");
        assert!(promos.iter().all(|p| p.url == COTO_PAGE && p.legal.is_none()));
    }

    #[test]
    fn region_limits_need_santa_fe() {
        assert!(applies_in_santa_fe("valido en todo el pais. beruti 2915, caba"));
        assert!(!applies_in_santa_fe("valido en las provincias de chaco, cordoba y mendoza"));
        assert!(applies_in_santa_fe("valido en las provincias de: santa fe, formosa"));
        assert!(!applies_in_santa_fe("valido para tiendas de amba, mendoza y cordoba"));
    }

    #[test]
    fn unknown_ids_get_a_readable_name() {
        let names = BTreeMap::new();
        assert_eq!(display_name("logo_naranjax2.png", &names), "Naranjax");
        assert_eq!(display_name("logo_columbia_1.png", &names), "Columbia");
        assert_eq!(display_name("Banco_Formosa", &names), "Banco Formosa");
        assert_eq!(display_name("Personal_pay.png", &names), "Personal Pay");
    }

    #[test]
    fn every_coto_logo_in_the_fixture_has_a_name() {
        let result: CotoResponse = serde_json::from_str(include_str!("../../tests/fixtures/coto_promos.json")).unwrap();
        let names = repo_names();
        for promo in result.result.promociones_digitales.iter().chain(&result.result.promociones_sucursales_fisicas) {
            let icon = promo.icono.as_deref().unwrap();
            assert!(names.contains_key(icon), "{icon} missing from promo_names");
        }
    }
}
