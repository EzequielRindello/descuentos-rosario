//! Coto: the legacy Endeca JSON endpoint (no API key needed), filtered to one store's availability.
//!
//! Offers are grouped in the `product.tipoOferta` dimension ("Hasta 30% DTO!!", "2X1", ...).
//! The dimension is multi-select, so all offer types are combined into a single `N-aZbZc` query.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::Value;

use crate::config::CotoConfig;
use crate::http::Http;
use crate::model::{Chain, Offer, Source, pesos_to_cents};

const BASE: &str = "https://www.coto.com.ar";
const CATEGORY_PATH: &str = "/sitios/cdigi/categoria";
const OFFER_DIMENSION: &str = "product.tipoOferta";
/// Safety stop in case pagination misbehaves.
const MAX_PAGES: u32 = 200;

pub async fn scrape(config: &CotoConfig, http: &Http) -> Result<Vec<Offer>> {
    let filter = store_filter(&config.store_id);
    let offer_types = discover_offer_types(http, &filter).await?;
    if offer_types.is_empty() {
        bail!("no offer types found in the {OFFER_DIMENSION} dimension");
    }
    tracing::info!("coto: {} offer types", offer_types.len());

    let url = format!("{BASE}{CATEGORY_PATH}/_/N-{}", offer_types.join("Z"));
    let page_size = config.page_size.to_string();
    let mut offers: BTreeMap<String, Offer> = BTreeMap::new();
    let mut offset: u64 = 0;
    let mut excluded = 0usize;
    for page in 1..=MAX_PAGES {
        let offset_str = offset.to_string();
        let request = http.get(&url).query(&[
            ("Nrpp", page_size.as_str()),
            ("No", offset_str.as_str()),
            ("Nr", filter.as_str()),
            ("format", "json"),
        ]);
        let body: Value = http.json(request).await?;
        let results = find_results(&body).context("no result list in Coto response")?;
        let total = results.get("totalNumRecs").and_then(Value::as_u64).unwrap_or(0);
        let records = results.get("records").and_then(Value::as_array).cloned().unwrap_or_default();
        if records.is_empty() {
            break;
        }
        offset += records.len() as u64;
        for record in &records {
            if is_excluded(record, &config.exclude_categories) {
                excluded += 1;
                continue;
            }
            if let Some(offer) = parse_record(record, &config.store) {
                offers.entry(offer.id.clone()).or_insert(offer);
            }
        }
        tracing::info!("coto page {page}: {offset}/{total} records, {} offers so far", offers.len());
        if offset >= total {
            break;
        }
    }
    tracing::info!("coto: {excluded} records skipped as non-grocery categories");
    Ok(offers.into_values().collect())
}

/// True when any of the product's ancestor categories is in the exclusion list.
fn is_excluded(record: &Value, exclude: &[String]) -> bool {
    if exclude.is_empty() {
        return false;
    }
    record
        .pointer("/records/0/attributes/allAncestors.displayName")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|ancestor| exclude.iter().any(|e| e == ancestor))
}

/// Endeca record filter: products available in the given store (1004 = available).
fn store_filter(store_id: &str) -> String {
    format!("AND(product.sDisp_{store_id}:1004,product.language:español,OR(product.siteId:CotoDigital))")
}

/// Reads the offer-type menu (following its "show more" link) and returns the dimension value ids.
async fn discover_offer_types(http: &Http, filter: &str) -> Result<Vec<String>> {
    let root: Value = http
        .json(http.get(&format!("{BASE}{CATEGORY_PATH}")).query(&[("Nr", filter), ("format", "json")]))
        .await?;
    let mut ids = Vec::new();
    let Some(menu) = find_offer_menu(&root) else { return Ok(ids) };
    collect_ids(menu, &mut ids);

    let more_state = menu.pointer("/moreLink/navigationState").and_then(Value::as_str).map(str::to_string);
    if let Some(state) = more_state {
        let more: Value = http
            .json(http.get(&format!("{BASE}{CATEGORY_PATH}{state}")).query(&[("format", "json")]))
            .await?;
        if let Some(full_menu) = find_offer_menu(&more) {
            collect_ids(full_menu, &mut ids);
        }
    }
    Ok(ids)
}

fn collect_ids(menu: &Value, ids: &mut Vec<String>) {
    let pattern = Regex::new(r"N-([0-9a-z]+)").expect("valid regex");
    for refinement in menu.get("refinements").and_then(Value::as_array).into_iter().flatten() {
        let state = refinement.get("navigationState").and_then(Value::as_str).unwrap_or_default();
        if let Some(captures) = pattern.captures(state) {
            for id in captures[1].split('Z') {
                if !id.is_empty() && !ids.iter().any(|known| known == id) {
                    ids.push(id.to_string());
                }
            }
        }
    }
}

fn find_offer_menu(node: &Value) -> Option<&Value> {
    find_node(node, &|n| {
        n.get("dimensionName").and_then(Value::as_str) == Some(OFFER_DIMENSION) && n.get("refinements").is_some()
    })
}

/// The main result list is the node with `totalNumRecs` and the most records.
fn find_results(node: &Value) -> Option<&Value> {
    let mut best: Option<&Value> = None;
    visit(node, &mut |n| {
        if n.get("totalNumRecs").is_some() {
            let len = n.get("records").and_then(Value::as_array).map_or(0, Vec::len);
            let best_len = best.and_then(|b| b.get("records")).and_then(Value::as_array).map_or(0, Vec::len);
            if best.is_none() || len > best_len {
                best = Some(n);
            }
        }
    });
    best
}

fn find_node<'a>(node: &'a Value, predicate: &dyn Fn(&Value) -> bool) -> Option<&'a Value> {
    if predicate(node) {
        return Some(node);
    }
    match node {
        Value::Object(map) => map.values().find_map(|v| find_node(v, predicate)),
        Value::Array(items) => items.iter().find_map(|v| find_node(v, predicate)),
        _ => None,
    }
}

fn visit<'a>(node: &'a Value, f: &mut impl FnMut(&'a Value)) {
    f(node);
    match node {
        Value::Object(map) => map.values().for_each(|v| visit(v, f)),
        Value::Array(items) => items.iter().for_each(|v| visit(v, f)),
        _ => {}
    }
}

/// Endeca attributes are arrays of strings; this returns the first one.
fn attr<'a>(attributes: &'a Value, key: &str) -> Option<&'a str> {
    attributes.get(key)?.as_array()?.first()?.as_str()
}

/// Parses "2631.85", "$1860.00" or "$2631.85c/u" (dot as decimal separator) into pesos.
fn parse_plain_amount(text: &str) -> Option<f64> {
    let cleaned: String = text.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    cleaned.trim_matches('.').parse().ok()
}

fn parse_record(record: &Value, store: &str) -> Option<Offer> {
    let attributes = record.pointer("/records/0/attributes")?;
    let id = attr(attributes, "product.repositoryId")?.to_string();
    let name = attr(attributes, "product.displayName")?.trim().to_string();

    let price_info: Value = serde_json::from_str(attr(attributes, "sku.dtoPrice")?).ok()?;
    let list_price = price_info.get("precioLista").and_then(Value::as_f64).and_then(pesos_to_cents)?;

    let discounts: Vec<Value> = serde_json::from_str(attr(attributes, "product.dtoDescuentos").unwrap_or("[]")).ok()?;
    let (price, promo, regular) = discounts.iter().find_map(|d| {
        let amount = d
            .get("precioDesc")
            .and_then(Value::as_str)
            .and_then(parse_plain_amount)
            .or_else(|| d.get("precioDescuento").and_then(Value::as_str).and_then(parse_plain_amount))?;
        let cents = pesos_to_cents(amount)?;
        let text = d.get("textoDescuento").and_then(Value::as_str).unwrap_or_default().trim();
        let condition = d.get("textoLlevando").and_then(Value::as_str).unwrap_or_default().trim();
        let promo = match (text.is_empty(), condition.is_empty()) {
            (true, _) => None,
            (false, true) => Some(text.to_string()),
            (false, false) => Some(format!("{text} ({condition})")),
        };
        // "Precio Contado: $1599" is the regular price Coto shows next to the offer. It matches
        // `precioLista` when that one is set, and is the only source for "Otras Ofertas" (mostly
        // fresh produce), where `precioLista` equals the offer price.
        let regular = d
            .get("textoPrecioRegular")
            .and_then(Value::as_str)
            .and_then(parse_plain_amount)
            .and_then(pesos_to_cents);
        Some((cents, promo, regular))
    })?;
    let list_price = Some(list_price).filter(|&list| list > price).or(regular.filter(|&r| r > price));

    let url = record
        .pointer("/detailsAction/recordState")
        .and_then(Value::as_str)
        .map(|state| format!("{BASE}/sitios/cdigi/productos{}", state.trim_end_matches("?format=json")));

    Some(Offer {
        chain: Chain::Coto,
        store: store.to_string(),
        id,
        ean: attr(attributes, "product.eanPrincipal").map(str::to_string).filter(|e| !e.is_empty()),
        name,
        price,
        list_price,
        discount_pct: None,
        promo,
        url,
        image: attr(attributes, "product.mediumImage.url").map(str::to_string),
        source: Source::Web,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_amounts() {
        assert_eq!(parse_plain_amount("2631.85"), Some(2631.85));
        assert_eq!(parse_plain_amount("$2631.85c/u"), Some(2631.85));
        assert_eq!(parse_plain_amount("$1860.00"), Some(1860.0));
    }

    #[test]
    fn uses_regular_price_text_when_list_price_equals_offer() {
        // Trimmed "Naranja Jugo Xkg" record from 11/11/2026: the site shows $899 with "Precio regular: $2.499".
        let discounts = r#"[{"textoPrecioRegular":"Precio Contado: $2499","textoDescuento":"","precioDesc":"899.00"}]"#;
        let record = serde_json::json!({"records": [{"attributes": {
            "product.repositoryId": ["prod00061005"],
            "product.displayName": ["Naranja Jugo Xkg"],
            "sku.dtoPrice": [r#"{"precioLista":899.0}"#],
            "product.dtoDescuentos": [discounts],
        }}]});
        let offer = parse_record(&record, "test").unwrap();
        assert_eq!(offer.price, 89_900);
        assert_eq!(offer.list_price, Some(249_900));

        let empty = discounts.replace("$2499", "");
        let record = serde_json::json!({"records": [{"attributes": {
            "product.repositoryId": ["prod1"],
            "product.displayName": ["Ensalada"],
            "sku.dtoPrice": [r#"{"precioLista":899.0}"#],
            "product.dtoDescuentos": [empty],
        }}]});
        assert_eq!(parse_record(&record, "test").unwrap().list_price, None);
    }

    #[test]
    fn excludes_non_grocery_categories() {
        let exclude = vec!["Textil".to_string(), "Hogar".to_string()];
        let shoe = serde_json::json!({"records": [{"attributes": {"allAncestors.displayName": ["CotoDigital", "Textil", "Calzado"]}}]});
        let milk = serde_json::json!({"records": [{"attributes": {"allAncestors.displayName": ["CotoDigital", "Frescos", "Leche"]}}]});
        assert!(is_excluded(&shoe, &exclude));
        assert!(!is_excluded(&milk, &exclude));
        assert!(!is_excluded(&shoe, &[]));
    }

    #[test]
    fn finds_offer_types_in_root_fixture() {
        let root: Value = serde_json::from_str(include_str!("../../tests/fixtures/coto_root.json")).unwrap();
        let menu = find_offer_menu(&root).expect("offer menu");
        let mut ids = Vec::new();
        collect_ids(menu, &mut ids);
        assert!(ids.len() >= 5, "expected several offer types, got {ids:?}");
        assert!(menu.pointer("/moreLink/navigationState").is_some());
    }

    #[test]
    fn parses_offer_fixture() {
        let body: Value = serde_json::from_str(include_str!("../../tests/fixtures/coto_offers.json")).unwrap();
        let results = find_results(&body).expect("results");
        assert!(results.get("totalNumRecs").and_then(Value::as_u64).unwrap() > 0);
        let records = results["records"].as_array().unwrap();
        let offers: Vec<Offer> = records.iter().filter_map(|r| parse_record(r, "test")).collect();
        assert!(!offers.is_empty());
        for offer in &offers {
            assert!(offer.price > 0);
            assert!(offer.promo.is_some());
            if let Some(list) = offer.list_price {
                assert!(list > offer.price);
            }
        }
    }
}
