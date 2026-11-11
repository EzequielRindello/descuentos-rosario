use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;

const ATTEMPTS: u32 = 3;
const RETRY_BASE: Duration = Duration::from_secs(2);
const TIMEOUT: Duration = Duration::from_secs(60);

/// Polite HTTP client: identifiable User-Agent, a pause before every request and a few retries.
/// One instance per chain, so cookies and pacing never mix between sites.
pub struct Http {
    client: Client,
    delay: Duration,
}

impl Http {
    /// `legacy_tls` switches to OpenSSL (vendored), needed by servers that only offer old CBC
    /// cipher suites that rustls does not implement (DAR's IIS 8.5).
    pub fn new(user_agent: &str, delay_ms: u64, cookies: bool, legacy_tls: bool) -> Result<Http> {
        let mut headers = HeaderMap::new();
        // Plain browser-like content negotiation. Some old ASP sites reject requests without it.
        headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/json;q=0.9,*/*;q=0.8"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("es-AR,es;q=0.9"));
        let mut builder = Client::builder();
        if legacy_tls {
            builder = builder.tls_backend_native();
        }
        let client = builder
            .user_agent(user_agent)
            .default_headers(headers)
            .cookie_store(cookies)
            .gzip(true)
            .timeout(TIMEOUT)
            .build()
            .context("building HTTP client")?;
        Ok(Http { client, delay: Duration::from_millis(delay_ms) })
    }

    pub fn get(&self, url: &str) -> RequestBuilder {
        self.client.get(url)
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        self.client.post(url)
    }

    /// Sends the request (with pacing and retries) and returns the body as text.
    pub async fn text(&self, request: RequestBuilder) -> Result<String> {
        let mut last_error = anyhow!("request was never sent");
        for attempt in 1..=ATTEMPTS {
            tokio::time::sleep(self.delay).await;
            let attempt_request = request.try_clone().context("request body cannot be retried")?;
            match self.send_once(attempt_request).await {
                Ok(body) => return Ok(body),
                Err(error) => {
                    tracing::warn!("attempt {attempt}/{ATTEMPTS} failed: {error:#}");
                    last_error = error;
                    if attempt < ATTEMPTS {
                        tokio::time::sleep(RETRY_BASE * attempt).await;
                    }
                }
            }
        }
        Err(last_error)
    }

    /// Like `text`, then parses the body as JSON.
    pub async fn json<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T> {
        let body = self.text(request).await?;
        serde_json::from_str(&body).with_context(|| format!("invalid JSON (starts with {:?})", snippet(&body)))
    }

    async fn send_once(&self, request: RequestBuilder) -> Result<String> {
        let response = request.send().await?;
        let status = response.status();
        let url = response.url().to_string();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(anyhow!("HTTP {status} for {url} (body starts with {:?})", snippet(&body)));
        }
        Ok(body)
    }
}

fn snippet(body: &str) -> String {
    body.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(160).collect()
}
