use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub user_agent: String,
    pub request_delay_ms: u64,
    pub carrefour: CarrefourConfig,
    pub coto: CotoConfig,
    pub dar: DarConfig,
    /// The site's own categories, in display order, and how each chain's categories map to them.
    #[serde(default)]
    pub categories: Vec<Category>,
}

/// One of the site's categories. A product gets the first one that matches.
#[derive(Debug, Clone, Deserialize)]
pub struct Category {
    /// Stable id, used in the data files and in the URL (`?categoria=carnes`).
    pub id: String,
    /// Name shown to users.
    pub name: String,
    /// Carrefour category slugs (from `carrefour.categories`) that belong here.
    #[serde(default)]
    pub carrefour: Vec<String>,
    /// Coto subcategory names (`allAncestors.displayName`), checked before any section.
    #[serde(default)]
    pub coto: Vec<String>,
    /// Whole Coto sections (top level), used when no subcategory matched.
    #[serde(default)]
    pub coto_section: Vec<String>,
}

/// Site category for a product scraped from this Carrefour category slug.
pub fn carrefour_category(categories: &[Category], slug: &str) -> Option<String> {
    categories.iter().find(|c| c.carrefour.iter().any(|s| s == slug)).map(|c| c.id.clone())
}

/// Site category for a Coto product, from all its ancestor category names (an unordered set):
/// subcategories first, so "Carniceria" wins over its section "Frescos".
pub fn coto_category(categories: &[Category], ancestors: &[&str]) -> Option<String> {
    let has = |names: &[String]| names.iter().any(|n| ancestors.contains(&n.as_str()));
    categories
        .iter()
        .find(|c| has(&c.coto))
        .or_else(|| categories.iter().find(|c| has(&c.coto_section)))
        .map(|c| c.id.clone())
}

#[derive(Debug, Deserialize)]
pub struct CarrefourConfig {
    pub enabled: bool,
    pub store: String,
    /// Shown next to the chain's offers: how to see the same price on the chain's own site.
    #[serde(default)]
    pub hint: String,
    pub seller: String,
    pub categories: Vec<String>,
    pub min_discount_pct: u8,
}

#[derive(Debug, Deserialize)]
pub struct CotoConfig {
    pub enabled: bool,
    pub store: String,
    /// Shown next to the chain's offers: how to see the same price on the chain's own site.
    #[serde(default)]
    pub hint: String,
    pub store_id: String,
    pub page_size: u32,
    /// Top-level Coto categories that are not groceries (matched against the product's ancestors).
    #[serde(default)]
    pub exclude_categories: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DarConfig {
    pub enabled: bool,
    pub store: String,
    /// Shown next to the chain's offers: how to see the same price on the chain's own site.
    #[serde(default)]
    pub hint: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_config_parses() {
        let config = Config::load(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/config/stores.toml"))).unwrap();
        assert_eq!(config.coto.store_id, "165");
        assert!(!config.carrefour.categories.is_empty());
    }

    fn repo_config() -> Config {
        Config::load(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/config/stores.toml"))).unwrap()
    }

    #[test]
    fn every_carrefour_category_maps_to_exactly_one_site_category() {
        let config = repo_config();
        for slug in &config.carrefour.categories {
            let owners = config.categories.iter().filter(|c| c.carrefour.contains(slug)).count();
            assert_eq!(owners, 1, "{slug} should belong to one site category");
        }
    }

    #[test]
    fn site_categories_are_unique_and_url_safe() {
        let config = repo_config();
        let mut ids: Vec<&str> = config.categories.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.iter().all(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c == '-')));
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), config.categories.len(), "category ids must be unique");
        let mut coto: Vec<&String> = config.categories.iter().flat_map(|c| c.coto.iter().chain(&c.coto_section)).collect();
        let total = coto.len();
        coto.sort_unstable();
        coto.dedup();
        assert_eq!(coto.len(), total, "a Coto name must map to a single site category");
    }

    #[test]
    fn coto_products_get_the_most_specific_category() {
        let config = repo_config();
        let category = |ancestors: &[&str]| coto_category(&config.categories, ancestors);
        // Unordered sets like the ones Coto sends; names from its category menus on 21/11/2026.
        assert_eq!(category(&["CotoDigital", "Frescos", "Carniceria", "Vacuna"]).as_deref(), Some("carnes"));
        assert_eq!(category(&["Congelados", "CotoDigital", "Pescaderia"]).as_deref(), Some("carnes"));
        assert_eq!(category(&["Congelados", "CotoDigital", "Helados y Postres"]).as_deref(), Some("congelados"));
        assert_eq!(category(&["Congelados", "CotoDigital", "Hamburguesas y Milanesas"]).as_deref(), Some("congelados"));
        // Cookies sit inside Coto's "Panaderia": the earlier category (breakfast) wins.
        assert_eq!(category(&["Almacén", "CotoDigital", "Galletitas", "Panaderia"]).as_deref(), Some("desayuno"));
        assert_eq!(category(&["Almacén", "CotoDigital", "Panaderia", "Panificados"]).as_deref(), Some("panaderia"));
        assert_eq!(category(&["Almacén", "CotoDigital", "Infusiones", "Yerba"]).as_deref(), Some("desayuno"));
        // "Huevo" is a kind of dry pasta, not the eggs subcategory ("Huevos").
        assert_eq!(
            category(&["Almacén", "CotoDigital", "Huevo", "Pasta Seca, Lista y Rellenas", "Pastas Secas"]).as_deref(),
            Some("almacen")
        );
        assert_eq!(category(&["CotoDigital", "Frescos", "Huevos"]).as_deref(), Some("lacteos"));
        assert_eq!(category(&["CotoDigital", "Pañales y Productos para Incontinencia", "Perfumería"]).as_deref(), Some("bebes"));
        assert_eq!(category(&["CotoDigital", "Perfumería", "Cuidado Bucal"]).as_deref(), Some("perfumeria"));
        assert_eq!(category(&["CotoDigital"]), None);
    }

    #[test]
    fn carrefour_products_get_their_category() {
        let config = repo_config();
        assert_eq!(carrefour_category(&config.categories, "carnes-y-pescados").as_deref(), Some("carnes"));
        assert_eq!(carrefour_category(&config.categories, "mascotas").as_deref(), Some("mascotas"));
        assert_eq!(carrefour_category(&config.categories, "no-existe"), None);
    }
}
