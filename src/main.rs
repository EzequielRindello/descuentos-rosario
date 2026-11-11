mod config;
mod detector;
mod http;
mod model;
mod output;
mod scrapers;
mod site;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use chrono::{FixedOffset, Utc};

use crate::config::Config;
use crate::http::Http;
use crate::model::{Chain, Offer};
use crate::output::{ChainReport, RunReport};

const USAGE: &str = "\
Usage:
  ofertas-rosario scrape [--only carrefour,coto,dar] [--config config/stores.toml] [--data data]
  ofertas-rosario build  [--config config/stores.toml] [--site config/site.toml] [--data data] [--out dist]

Commands:
  scrape   Read today's offers from each chain and write data/offers/<chain>.json and data/run.json
  build    Generate the static site from data/ into dist/";

struct Args {
    command: String,
    only: Option<Vec<Chain>>,
    config: PathBuf,
    site: PathBuf,
    data: PathBuf,
    out: PathBuf,
}

fn parse_args() -> Result<Args> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let mut parsed = Args {
        command,
        only: None,
        config: PathBuf::from("config/stores.toml"),
        site: PathBuf::from("config/site.toml"),
        data: PathBuf::from("data"),
        out: PathBuf::from("dist"),
    };
    while let Some(flag) = args.next() {
        let mut value = || args.next().with_context(|| format!("missing value for {flag}"));
        match flag.as_str() {
            "--only" => {
                let chains = value()?
                    .split(',')
                    .map(|id| Chain::from_id(id.trim()).with_context(|| format!("unknown chain {id:?}")))
                    .collect::<Result<Vec<_>>>()?;
                parsed.only = Some(chains);
            }
            "--config" => parsed.config = PathBuf::from(value()?),
            "--data" => parsed.data = PathBuf::from(value()?),
            "--site" => parsed.site = PathBuf::from(value()?),
            "--out" => parsed.out = PathBuf::from(value()?),
            other => bail!("unknown argument {other:?}\n\n{USAGE}"),
        }
    }
    Ok(parsed)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_target(false).init();
    let args = parse_args()?;
    match args.command.as_str() {
        "scrape" => scrape(args).await,
        "build" => {
            let config = Config::load(&args.config)?;
            let site_config = site::SiteConfig::load(&args.site)?;
            site::build(&args.data, &args.out, &site_config, &config)
        }
        _ => {
            println!("{USAGE}");
            Ok(())
        }
    }
}

async fn scrape(args: Args) -> Result<()> {
    let config = Config::load(&args.config)?;
    let wanted = |chain: Chain, enabled: bool| enabled && args.only.as_ref().is_none_or(|only| only.contains(&chain));

    // Each chain runs in parallel with its own client; a failing chain never stops the others.
    let (carrefour, coto, dar) = tokio::join!(
        run_chain(wanted(Chain::Carrefour, config.carrefour.enabled), &config, Chain::Carrefour),
        run_chain(wanted(Chain::Coto, config.coto.enabled), &config, Chain::Coto),
        run_chain(wanted(Chain::Dar, config.dar.enabled), &config, Chain::Dar),
    );

    // Argentina has no daylight saving time: always UTC-3.
    let now = Utc::now().with_timezone(&FixedOffset::west_opt(3 * 3600).expect("valid offset"));
    let mut report = RunReport::new(now);
    for (chain, result) in [(Chain::Carrefour, carrefour), (Chain::Coto, coto), (Chain::Dar, dar)] {
        let Some((result, seconds)) = result else { continue };
        let result = result.and_then(|offers| check_volume(&args.data, chain, offers));
        let chain_report = match result {
            Ok(mut offers) => {
                detector::finalize(&mut offers);
                output::write_offers(&args.data, chain, &mut offers)?;
                tracing::info!("{}: {} offers in {seconds}s", chain.id(), offers.len());
                ChainReport { ok: true, offers: offers.len(), seconds, error: None }
            }
            Err(error) => {
                // Keep yesterday's file so the site still has data; the error is in run.json.
                tracing::error!("{}: failed after {seconds}s: {error:#}", chain.id());
                ChainReport { ok: false, offers: 0, seconds, error: Some(format!("{error:#}")) }
            }
        };
        report.chains.insert(chain.id().to_string(), chain_report);
    }

    let ran = report.chains.len();
    let failed = report.chains.values().filter(|c| !c.ok).count();
    output::write_report(&args.data, report)?;
    if ran > 0 && failed == ran {
        bail!("every chain failed");
    }
    Ok(())
}

/// Offer counts swing a lot from day to day (a chain can end most of its promos overnight), so
/// a drop is real data, not an error. Only zero offers after a non-empty day is treated as a
/// broken scraper (site redesign, blocked request) and keeps the old file. Big drops are logged.
fn check_volume(data_dir: &std::path::Path, chain: Chain, offers: Vec<Offer>) -> Result<Vec<Offer>> {
    let Some(previous) = output::previous_count(data_dir, chain) else {
        return Ok(offers);
    };
    if offers.is_empty() && previous > 0 {
        bail!("brought no offers (previous file had {previous}); keeping the previous file");
    }
    if offers.len() * 2 < previous {
        tracing::warn!("{}: {} offers, less than half of the previous {previous}", chain.id(), offers.len());
    }
    Ok(offers)
}

async fn run_chain(enabled: bool, config: &Config, chain: Chain) -> Option<(Result<Vec<Offer>>, u64)> {
    if !enabled {
        return None;
    }
    let start = Instant::now();
    let result = async {
        // DAR needs a session cookie and an old TLS cipher suite.
        let is_dar = chain == Chain::Dar;
        let http = Http::new(&config.user_agent, config.request_delay_ms, is_dar, is_dar)?;
        match chain {
            Chain::Carrefour => scrapers::carrefour::scrape(&config.carrefour, &http).await,
            Chain::Coto => scrapers::coto::scrape(&config.coto, &http).await,
            Chain::Dar => scrapers::dar::scrape(&config.dar, &http).await,
        }
    }
    .await;
    Some((result, start.elapsed().as_secs()))
}
