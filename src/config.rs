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
}

#[derive(Debug, Deserialize)]
pub struct CarrefourConfig {
    pub enabled: bool,
    pub store: String,
    pub seller: String,
    pub categories: Vec<String>,
    pub min_discount_pct: u8,
}

#[derive(Debug, Deserialize)]
pub struct CotoConfig {
    pub enabled: bool,
    pub store: String,
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
}
