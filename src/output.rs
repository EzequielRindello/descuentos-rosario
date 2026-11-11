//! Writes the daily data files. The format is deterministic (sorted, one offer per line)
//! so that the daily git commit only shows real price changes.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, SecondsFormat};
use serde::{Deserialize, Serialize};

use crate::model::{Chain, Offer};

/// Summary of one run, written to `data/run.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RunReport {
    pub generated_at: String,
    pub chains: BTreeMap<String, ChainReport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainReport {
    pub ok: bool,
    pub offers: usize,
    pub seconds: u64,
    pub error: Option<String>,
}

impl RunReport {
    pub fn new(now: DateTime<FixedOffset>) -> RunReport {
        RunReport { generated_at: now.to_rfc3339_opts(SecondsFormat::Secs, false), chains: BTreeMap::new() }
    }
}

/// Number of offers in the chain's current data file, if there is one.
pub fn previous_count(data_dir: &Path, chain: Chain) -> Option<usize> {
    let path = data_dir.join("offers").join(format!("{}.json", chain.id()));
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Vec<serde_json::Value>>(&text).ok().map(|offers| offers.len())
}

/// Writes `data/offers/<chain>.json`, sorted by product id.
pub fn write_offers(data_dir: &Path, chain: Chain, offers: &mut [Offer]) -> Result<()> {
    offers.sort_by(|a, b| a.id.cmp(&b.id));
    let dir = data_dir.join("offers");
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let mut body = String::from("[\n");
    for (i, offer) in offers.iter().enumerate() {
        body.push_str(&serde_json::to_string(offer)?);
        body.push_str(if i + 1 < offers.len() { ",\n" } else { "\n" });
    }
    body.push_str("]\n");
    let path = dir.join(format!("{}.json", chain.id()));
    fs::write(&path, body).with_context(|| format!("writing {}", path.display()))
}

/// Merges this run into `data/run.json`, keeping the last report of chains that were not run.
pub fn write_report(data_dir: &Path, report: RunReport) -> Result<()> {
    let path = data_dir.join("run.json");
    let mut merged: RunReport = fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    merged.generated_at = report.generated_at;
    merged.chains.extend(report.chains);
    fs::create_dir_all(data_dir)?;
    fs::write(&path, serde_json::to_string_pretty(&merged)? + "\n").with_context(|| format!("writing {}", path.display()))
}
