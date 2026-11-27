//! Price history from the repo itself: the bot commits `data/offers/<chain>.json` every day, so
//! git already holds one snapshot per day ("git scraping"). The build reads the last few weeks
//! of those snapshots to tell a real price drop from a raised "before" price.
//!
//! Only offers are recorded (a product that is not on offer has no price in the data), so these
//! are comparisons against the prices we saw, not against every price the product had.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::process::Command;

use chrono::{DateTime, FixedOffset};
use serde::Deserialize;

use crate::model::{Chain, Offer};

/// How far back the comparison looks.
pub const WINDOW_DAYS: i64 = 30;
/// Earlier days a product needs to have been seen on before any comparison is shown.
pub const MIN_DAYS: usize = 3;
/// "Before" price this much above the lowest "before" price seen (and the price did not go up
/// with it, which would just be inflation): it was raised.
const INFLATED_PCT: u64 = 115;
/// Price this much below its usual (median) price, for offers with no "before" price.
const DROP_MIN_PCT: u8 = 5;
/// Bigger drops than this are almost always a change of unit or pack size ("x kg" to "x 100 g"),
/// not a price cut: no comparison is shown.
const MAX_DROP_PCT: u64 = 60;

/// Price of one product on one earlier day.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PricePoint {
    pub price: u64,
    pub list_price: Option<u64>,
}

/// Earlier prices per offer key (`<chain>:<id>`), one point per day seen, oldest first.
#[derive(Debug, Default)]
pub struct History {
    by_key: HashMap<String, Vec<PricePoint>>,
}

#[derive(Debug, Deserialize)]
struct StoredOffer {
    id: String,
    price: u64,
    #[serde(default)]
    list_price: Option<u64>,
}

impl History {
    pub fn past(&self, offer: &Offer) -> &[PricePoint] {
        self.by_key.get(&format!("{}:{}", offer.chain.id(), offer.id)).map_or(&[], Vec::as_slice)
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

}

/// Rebuilds what the data said on each day of the window before `today` (yyyy-mm-dd, Argentina).
/// The bot only commits when the data changed, so a day with no commit keeps the previous one.
/// Without git or outside a repo (a copy of `data/`, the tests) the history is just empty.
pub fn load(data_dir: &Path, today: &str) -> History {
    let mut history = History::default();
    let Ok(today_date) = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d") else { return history };
    let first_day = today_date - chrono::Duration::days(WINDOW_DAYS);
    // Argentina has no daylight saving time: always UTC-3.
    let argentina = FixedOffset::west_opt(3 * 3600).expect("valid offset");
    let since = format!("{}T00:00:00-03:00", first_day.format("%Y-%m-%d"));
    let mut days_read = 0usize;
    for chain in Chain::ALL {
        let path = format!("offers/{}.json", chain.id());
        let Some(log) = git(data_dir, &["log", &format!("--since={since}"), "--format=%H %cI", "--", &path]) else {
            tracing::info!("no git history for {path}: price comparisons are off");
            return History::default();
        };
        // The data as it was when the window starts: the last commit before it.
        let base = git(data_dir, &["log", "-1", &format!("--before={since}"), "--format=%H", "--", &path])
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty());
        // Newest first: the first commit of each Argentine day is that day's final data.
        let mut per_day: BTreeMap<chrono::NaiveDate, String> = BTreeMap::new();
        for line in log.lines() {
            let Some((hash, date)) = line.split_once(' ') else { continue };
            let Ok(date) = DateTime::parse_from_rfc3339(date) else { continue };
            per_day.entry(date.with_timezone(&argentina).date_naive()).or_insert_with(|| hash.to_string());
        }
        let mut files: HashMap<String, Vec<StoredOffer>> = HashMap::new();
        let mut current = base;
        let mut day = first_day;
        while day < today_date {
            if let Some(hash) = per_day.get(&day) {
                current = Some(hash.clone());
            }
            if let Some(hash) = &current {
                let offers = files.entry(hash.clone()).or_insert_with(|| {
                    git(data_dir, &["show", &format!("{hash}:./{path}")])
                        .and_then(|text| serde_json::from_str(&text).ok())
                        .unwrap_or_default()
                });
                if !offers.is_empty() {
                    days_read += 1;
                }
                for offer in offers.iter() {
                    history
                        .by_key
                        .entry(format!("{}:{}", chain.id(), offer.id))
                        .or_default()
                        .push(PricePoint { price: offer.price, list_price: offer.list_price });
                }
            }
            day += chrono::Duration::days(1);
        }
    }
    tracing::info!("price history: {days_read} chain-days, {} products", history.by_key.len());
    history
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// What the history says about today's price. At most one per offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// The "before" price is well above the lowest one we saw, and the price is not lower than
    /// before: the discount comes from raising the "before" price.
    RaisedBefore,
    /// No "before" price published (DAR): this much below the usual price we saw.
    Drop(u8),
    /// Lower than every earlier price we saw in the window.
    Lowest,
}

impl Signal {
    /// Short code for the search index (app.js styles by it).
    pub fn kind(self) -> &'static str {
        match self {
            Signal::RaisedBefore => "warn",
            Signal::Drop(_) | Signal::Lowest => "good",
        }
    }

    /// Text shown on the offer card. Same wording in the pages and in app.js (it comes from here).
    pub fn label(self) -> String {
        match self {
            Signal::RaisedBefore => "El precio anterior subió hace poco".to_string(),
            Signal::Drop(pct) => format!("{pct}% menos que en los últimos {WINDOW_DAYS} días"),
            Signal::Lowest => format!("El más bajo en {WINDOW_DAYS} días"),
        }
    }
}

pub fn signal(offer: &Offer, past: &[PricePoint]) -> Option<Signal> {
    if past.len() < MIN_DAYS {
        return None;
    }
    let lowest_price = past.iter().map(|p| p.price).min()?;
    if offer.price * 100 < lowest_price * (100 - MAX_DROP_PCT) {
        return None; // unit or pack change
    }
    let mut prices: Vec<u64> = past.iter().map(|p| p.price).collect();
    prices.sort_unstable();
    let median = prices[prices.len() / 2];
    if let Some(list) = offer.list_price {
        let lowest_list = past.iter().filter_map(|p| p.list_price).min();
        let raised = lowest_list.is_some_and(|before| list * 100 > before * INFLATED_PCT);
        if raised && offer.price >= lowest_price && offer.price <= median {
            return Some(Signal::RaisedBefore);
        }
    } else if past.iter().all(|p| p.list_price.is_none()) && offer.price < median {
        // Chains that never publish a "before" price (DAR): compare with the usual price.
        let pct = (100 - offer.price * 100 / median) as u8;
        if pct >= DROP_MIN_PCT {
            return Some(Signal::Drop(pct));
        }
    }
    (offer.price < lowest_price).then_some(Signal::Lowest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Source;

    fn offer(chain: Chain, price: u64, list_price: Option<u64>) -> Offer {
        Offer {
            chain,
            store: "x".into(),
            id: "1".into(),
            ean: None,
            name: "Yerba".into(),
            price,
            list_price,
            discount_pct: None,
            promo: None,
            url: None,
            image: None,
            source: Source::Web,
            category: None,
        }
    }

    fn days(points: &[(u64, Option<u64>)]) -> Vec<PricePoint> {
        points.iter().map(|&(price, list_price)| PricePoint { price, list_price }).collect()
    }

    #[test]
    fn needs_a_few_earlier_days() {
        let past = days(&[(1000, Some(1500)), (1000, Some(1500))]);
        assert_eq!(signal(&offer(Chain::Coto, 500, Some(1500)), &past), None);
    }

    #[test]
    fn lowest_price_in_the_window() {
        let past = days(&[(1000, Some(1500)), (900, Some(1500)), (1000, Some(1500))]);
        assert_eq!(signal(&offer(Chain::Coto, 850, Some(1500)), &past), Some(Signal::Lowest));
        assert_eq!(signal(&offer(Chain::Coto, 900, Some(1500)), &past), None, "same as the lowest is not lower");
    }

    #[test]
    fn raised_before_price_is_flagged() {
        // Same price as always, but the crossed-out price jumped from 1.000 to 1.500.
        let past = days(&[(900, Some(1000)), (900, Some(1000)), (900, Some(1000))]);
        assert_eq!(signal(&offer(Chain::Carrefour, 900, Some(1500)), &past), Some(Signal::RaisedBefore));
        // A real cut with a slightly higher "before" price is not flagged.
        assert_eq!(signal(&offer(Chain::Carrefour, 700, Some(1050)), &past), Some(Signal::Lowest));
    }

    #[test]
    fn drop_for_offers_without_a_before_price() {
        let past = days(&[(1000, None), (1000, None), (980, None), (1200, None)]);
        assert_eq!(signal(&offer(Chain::Dar, 880, None), &past), Some(Signal::Drop(12)));
        // Less than 5% below the usual price: nothing, unless it is the lowest seen.
        assert_eq!(signal(&offer(Chain::Dar, 990, None), &past), None);
        assert_eq!(signal(&offer(Chain::Dar, 975, None), &past), Some(Signal::Lowest));
    }

    #[test]
    fn inflation_and_unit_changes_are_not_flagged() {
        // Price and "before" price both went up 20%: inflation, the discount is the same.
        let past = days(&[(1204, Some(1720)), (1204, Some(1720)), (1204, Some(1720))]);
        assert_eq!(signal(&offer(Chain::Carrefour, 1445, Some(2065)), &past), None);
        // A "before" price only 10% higher is not flagged either.
        assert_eq!(signal(&offer(Chain::Carrefour, 1204, Some(1892)), &past), None);
        // From $2.035 to $407 with no "before" price: a pack change, not an 80% cut.
        let pack = days(&[(203_500, Some(275_000)), (203_500, Some(275_000)), (203_500, Some(275_000))]);
        assert_eq!(signal(&offer(Chain::Carrefour, 40_700, None), &pack), None);
        // No "before" price today, but the product used to have one: no comparison with the median.
        assert_eq!(signal(&offer(Chain::Carrefour, 180_000, None), &pack), Some(Signal::Lowest));
    }

    #[test]
    fn labels() {
        assert_eq!(Signal::Lowest.label(), "El más bajo en 30 días");
        assert_eq!(Signal::Drop(12).label(), "12% menos que en los últimos 30 días");
        assert_eq!(Signal::RaisedBefore.kind(), "warn");
    }

    #[test]
    fn reads_nothing_outside_a_repo() {
        let dir = std::env::temp_dir().join(format!("ofertas-rosario-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(load(&dir, "2026-11-21").is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
