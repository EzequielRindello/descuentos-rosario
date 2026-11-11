//! DAR: classic ASP site. The offers section is opened with a form POST and then paginated,
//! all inside one session cookie.
//!
//! The session only works when it starts at `index.asp` (which redirects to `Login.asp` and
//! serves the home page). Starting directly at `Login.asp` gives a session that bounces back.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use regex::Regex;
use reqwest::header::REFERER;
use scraper::{Html, Selector};

use crate::config::DarConfig;
use crate::http::Http;
use crate::model::{Chain, Offer, Source, parse_ars};

const BASE: &str = "https://www.darentucasa.com.ar";
/// Safety stop in case the page count cannot be read.
const MAX_PAGES: u32 = 60;

pub async fn scrape(config: &DarConfig, http: &Http) -> Result<Vec<Offer>> {
    // 1. Start the session the way a browser does.
    http.text(http.get(&format!("{BASE}/index.asp"))).await?;

    // 2. Open the offers section (same form the "OFERTAS" menu submits).
    let form = [("TM", "CM"), ("1n", "undefined"), ("2n", "undefined"), ("3n", "undefined"), ("4n", "undefined"), ("Crit", ""), ("Dat", "")];
    http.text(http.post(&format!("{BASE}/carrito.asp")).header(REFERER, format!("{BASE}/Login.asp")).form(&form)).await?;

    // 3. First page of products.
    let referer = format!("{BASE}/carrito.asp");
    let first = http.text(http.get(&format!("{BASE}/Productos.asp?Of=S")).header(REFERER, &referer)).await?;
    let first_page = parse_page(&first, &config.store);
    if first_page.offers.is_empty() {
        bail!("DAR returned no products on the first offers page (session not accepted?)");
    }

    let mut offers: BTreeMap<String, Offer> = BTreeMap::new();
    let total_pages = first_page.total_pages.unwrap_or(1).min(MAX_PAGES);
    tracing::info!("dar page 1/{total_pages}: {} offers", first_page.offers.len());
    for offer in first_page.offers {
        offers.entry(offer.id.clone()).or_insert(offer);
    }

    // 4. Remaining pages.
    for page in 2..=total_pages {
        let url = format!("{BASE}/productos.asp?page={page}&N1=&N2=&N3=&N4=");
        let body = http.text(http.get(&url).header(REFERER, &referer)).await?;
        let parsed = parse_page(&body, &config.store);
        tracing::info!("dar page {page}/{total_pages}: {} offers", parsed.offers.len());
        for offer in parsed.offers {
            offers.entry(offer.id.clone()).or_insert(offer);
        }
    }
    Ok(offers.into_values().collect())
}

struct Page {
    offers: Vec<Offer>,
    total_pages: Option<u32>,
}

fn parse_page(html: &str, store: &str) -> Page {
    let document = Html::parse_document(html);
    let product = Selector::parse("li.cuadProd").expect("valid selector");
    let name = Selector::parse(".desc").expect("valid selector");
    let price = Selector::parse(".precio").expect("valid selector");
    let buy = Selector::parse(".AgregaArt").expect("valid selector");
    let code_pattern = Regex::new(r"PCompra\('(\d+)'\)").expect("valid regex");
    let pages_pattern = Regex::new(r"\d+\s+de\s+(\d+)").expect("valid regex");

    let offers = document
        .select(&product)
        .filter_map(|item| {
            let name = item.select(&name).next()?.text().collect::<String>().trim().to_string();
            let price_text = item.select(&price).next()?.text().collect::<String>();
            let price = parse_ars(&price_text)?;
            let onclick = item.select(&buy).next()?.value().attr("onclick")?;
            let code = code_pattern.captures(onclick)?[1].to_string();
            Some(Offer {
                chain: Chain::Dar,
                store: store.to_string(),
                image: Some(format!("{BASE}/Fotos/Articulos/{code}.jpg")),
                id: code,
                ean: None,
                name,
                price,
                list_price: None,
                discount_pct: None,
                promo: Some("Oferta".to_string()),
                url: None,
                source: Source::Web,
            })
        })
        .collect();

    let text = document.root_element().text().collect::<String>();
    let total_pages = pages_pattern.captures(&text).and_then(|c| c[1].parse().ok());
    Page { offers, total_pages }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixture() {
        let page = parse_page(include_str!("../../tests/fixtures/dar_offers.html"), "test");
        assert_eq!(page.offers.len(), 3);
        assert!(page.total_pages.unwrap() > 1);
        let first = &page.offers[0];
        assert_eq!(first.id.len(), 13);
        assert!(first.price > 0);
        assert!(!first.name.is_empty());
    }
}
