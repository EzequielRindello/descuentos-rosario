//! Discount rules and sanity checks applied to every offer, whatever chain it comes from.

use crate::model::Offer;

/// A "previous price" more than this many times the current one is treated as bogus
/// (seen on Jumbo: a $8,900 yerba with a $735,537 list price).
const MAX_LIST_TO_PRICE_RATIO: u64 = 5;

/// Drops unreasonable list prices and fills in `discount_pct`.
pub fn finalize(offers: &mut [Offer]) {
    for offer in offers.iter_mut() {
        if let Some(list) = offer.list_price
            && (list <= offer.price || list > offer.price * MAX_LIST_TO_PRICE_RATIO)
        {
            offer.list_price = None;
        }
        offer.discount_pct = offer.list_price.map(|list| discount_pct(offer.price, list));
    }
}

/// Rounded percentage saved: 1 - price / list.
pub fn discount_pct(price: u64, list: u64) -> u8 {
    if list == 0 || price >= list {
        return 0;
    }
    (((list - price) * 100 + list / 2) / list) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Chain, Source};

    fn offer(price: u64, list: Option<u64>) -> Offer {
        Offer {
            chain: Chain::Coto,
            store: "test".into(),
            id: "1".into(),
            ean: None,
            name: "test".into(),
            price,
            list_price: list,
            discount_pct: None,
            promo: None,
            url: None,
            image: None,
            source: Source::Web,
        }
    }

    #[test]
    fn computes_rounded_discount() {
        assert_eq!(discount_pct(283_765, 567_530), 50);
        assert_eq!(discount_pct(2_000, 3_000), 33);
        assert_eq!(discount_pct(100, 100), 0);
    }

    #[test]
    fn drops_bogus_list_prices() {
        let mut offers = vec![offer(890_000, Some(73_553_700)), offer(1_000, Some(800)), offer(750, Some(1_000)), offer(500, None)];
        finalize(&mut offers);
        assert_eq!(offers[0].list_price, None);
        assert_eq!(offers[0].discount_pct, None);
        assert_eq!(offers[1].list_price, None);
        assert_eq!(offers[2].discount_pct, Some(25));
        assert_eq!(offers[3].discount_pct, None);
    }
}
