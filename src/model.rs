use serde::{Deserialize, Serialize};

/// Supermarket chain covered by the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Carrefour,
    Coto,
    Dar,
}

impl Chain {
    pub const ALL: [Chain; 3] = [Chain::Carrefour, Chain::Coto, Chain::Dar];

    pub fn id(self) -> &'static str {
        match self {
            Chain::Carrefour => "carrefour",
            Chain::Coto => "coto",
            Chain::Dar => "dar",
        }
    }

    pub fn from_id(id: &str) -> Option<Chain> {
        Chain::ALL.into_iter().find(|c| c.id() == id)
    }
}

/// Where a price was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// The chain's own website or API (online price).
    Web,
}

/// One offer, normalized across chains. Money is in cents of ARS to avoid float rounding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Offer {
    pub chain: Chain,
    pub store: String,
    /// The chain's own product id.
    pub id: String,
    pub ean: Option<String>,
    pub name: String,
    /// Offer price, in cents.
    pub price: u64,
    /// Regular price before the offer, in cents, when the chain publishes it.
    pub list_price: Option<u64>,
    /// 1 - price / list_price, rounded, filled in by the detector.
    pub discount_pct: Option<u8>,
    /// Promo text as the chain shows it (e.g. "70% 2da (Llevando 2)").
    pub promo: Option<String>,
    pub url: Option<String>,
    pub image: Option<String>,
    pub source: Source,
}

/// Converts a peso amount to cents, rejecting non-positive or non-finite values.
pub fn pesos_to_cents(pesos: f64) -> Option<u64> {
    if pesos.is_finite() && pesos > 0.0 {
        Some((pesos * 100.0).round() as u64)
    } else {
        None
    }
}

/// Parses an Argentine-formatted amount such as "$1.250,50" or "$475,00" into cents.
pub fn parse_ars(text: &str) -> Option<u64> {
    let cleaned: String = text.chars().filter(|c| c.is_ascii_digit() || *c == ',' || *c == '.').collect();
    let (int_part, dec_part) = match cleaned.rsplit_once(',') {
        Some((i, d)) => (i.replace('.', ""), d.to_string()),
        None => (cleaned.replace('.', ""), String::new()),
    };
    let int: u64 = int_part.parse().ok()?;
    let dec: u64 = match dec_part.len() {
        0 => 0,
        1 => dec_part.parse::<u64>().ok()? * 10,
        _ => dec_part[..2].parse().ok()?,
    };
    let cents = int * 100 + dec;
    (cents > 0).then_some(cents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_argentine_amounts() {
        assert_eq!(parse_ars("$475,00"), Some(47_500));
        assert_eq!(parse_ars("$1.250,50"), Some(125_050));
        assert_eq!(parse_ars("$ 12.345"), Some(1_234_500));
        assert_eq!(parse_ars("$0,00"), None);
        assert_eq!(parse_ars("sin precio"), None);
    }

    #[test]
    fn converts_pesos_to_cents() {
        assert_eq!(pesos_to_cents(2837.65), Some(283_765));
        assert_eq!(pesos_to_cents(0.0), None);
        assert_eq!(pesos_to_cents(f64::NAN), None);
    }
}
