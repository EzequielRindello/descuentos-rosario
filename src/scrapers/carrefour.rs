//! Carrefour: VTEX Intelligent Search, with the Rosario region so prices are local.

use std::collections::BTreeMap;

use anyhow::Result;
use base64::Engine;
use serde::Deserialize;

use crate::config::CarrefourConfig;
use crate::http::Http;
use crate::model::{Chain, Offer, Source, pesos_to_cents};

const BASE: &str = "https://www.carrefour.com.ar";
const PAGE_SIZE: u32 = 50;
/// The API returns nothing past ~2,500 results per query, so each category stops there.
const MAX_RESULTS: u32 = 2_500;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    products: Vec<Product>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Product {
    product_id: String,
    product_name: String,
    #[serde(default)]
    link_text: String,
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    #[serde(default)]
    ean: Option<String>,
    #[serde(default)]
    images: Vec<Image>,
    #[serde(default)]
    sellers: Vec<Seller>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Image {
    image_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Seller {
    commertial_offer: CommercialOffer,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CommercialOffer {
    #[serde(default)]
    price: f64,
    #[serde(default)]
    list_price: f64,
    #[serde(default)]
    available_quantity: f64,
}

pub fn region_id(seller: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(format!("SW#{seller}"))
}

pub async fn scrape(config: &CarrefourConfig, http: &Http) -> Result<Vec<Offer>> {
    let region = region_id(&config.seller);
    // Keyed by product id: a product can show up in more than one category.
    let mut offers: BTreeMap<String, Offer> = BTreeMap::new();

    for category in &config.categories {
        let url = format!("{BASE}/api/io/_v/api/intelligent-search/product_search/category-1/{category}");
        let mut page = 1;
        loop {
            let page_str = page.to_string();
            let count_str = PAGE_SIZE.to_string();
            let request = http.get(&url).query(&[
                ("query", ""),
                ("count", count_str.as_str()),
                ("page", page_str.as_str()),
                ("sort", "discount:desc"),
                ("hideUnavailableItems", "true"),
                ("regionId", region.as_str()),
            ]);
            let response: SearchResponse = http.json(request).await?;
            if response.products.is_empty() {
                break;
            }
            let (page_offers, best_discount) = parse_products(response, &config.store);
            let found = page_offers.len();
            for offer in page_offers {
                offers.entry(offer.id.clone()).or_insert(offer);
            }
            tracing::info!("carrefour {category} page {page}: {found} offers (best {best_discount}%)");
            // Sorted by discount, so once a page has no real discount the rest won't either.
            if best_discount < config.min_discount_pct || page * PAGE_SIZE >= MAX_RESULTS {
                break;
            }
            page += 1;
        }
    }
    Ok(offers.into_values().collect())
}

/// Returns the offers on one page and the best discount seen on it.
fn parse_products(response: SearchResponse, store: &str) -> (Vec<Offer>, u8) {
    let mut offers = Vec::new();
    let mut best_discount = 0u8;
    for product in response.products {
        let Some(item) = product.items.first() else { continue };
        let Some(seller) = item.sellers.first() else { continue };
        let offer = &seller.commertial_offer;
        let (Some(price), Some(list_price)) = (pesos_to_cents(offer.price), pesos_to_cents(offer.list_price)) else {
            continue;
        };
        if offer.available_quantity <= 0.0 || list_price <= price {
            continue;
        }
        let discount = 100 - (price * 100 / list_price) as u8;
        best_discount = best_discount.max(discount);
        offers.push(Offer {
            chain: Chain::Carrefour,
            store: store.to_string(),
            id: product.product_id,
            ean: item.ean.clone().filter(|e| !e.is_empty()),
            name: product.product_name.trim().to_string(),
            price,
            list_price: Some(list_price),
            discount_pct: None,
            promo: None,
            url: (!product.link_text.is_empty()).then(|| format!("{BASE}/{}/p", product.link_text)),
            image: item.images.first().map(|i| i.image_url.clone()),
            source: Source::Web,
        });
    }
    (offers, best_discount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_matches_known_value() {
        assert_eq!(region_id("carrefourar0018"), "U1cjY2FycmVmb3VyYXIwMDE4");
    }

    #[test]
    fn parses_fixture() {
        let body = include_str!("../../tests/fixtures/carrefour_search.json");
        let response: SearchResponse = serde_json::from_str(body).unwrap();
        let (offers, best) = parse_products(response, "test");
        assert!(!offers.is_empty(), "fixture should contain discounted products");
        assert!(best > 0);
        for offer in &offers {
            assert!(offer.list_price.unwrap() > offer.price);
            assert!(offer.url.as_deref().unwrap().starts_with("https://www.carrefour.com.ar/"));
        }
    }
}
