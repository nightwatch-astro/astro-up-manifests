use reqwest::header::{HeaderName, HeaderValue};
use std::convert::TryFrom;
use std::time::Duration;

/// A thin retry wrapper around `reqwest::Client`.
///
/// Retries on transient errors (5xx, timeout, connection failure)
/// with exponential backoff.
#[derive(Clone)]
pub struct RetryClient {
    inner: reqwest::Client,
    max_retries: u32,
}

impl RetryClient {
    pub fn new(client: reqwest::Client, max_retries: u32) -> Self {
        Self {
            inner: client,
            max_retries,
        }
    }

    pub fn get(&self, url: &str) -> RetryRequestBuilder {
        RetryRequestBuilder {
            client: self.clone(),
            inner: self.inner.get(url),
        }
    }

    pub fn post(&self, url: &str) -> RetryRequestBuilder {
        RetryRequestBuilder {
            client: self.clone(),
            inner: self.inner.post(url),
        }
    }

    pub fn patch(&self, url: &str) -> RetryRequestBuilder {
        RetryRequestBuilder {
            client: self.clone(),
            inner: self.inner.patch(url),
        }
    }

    pub fn head(&self, url: &str) -> RetryRequestBuilder {
        RetryRequestBuilder {
            client: self.clone(),
            inner: self.inner.head(url),
        }
    }
}

pub struct RetryRequestBuilder {
    client: RetryClient,
    inner: reqwest::RequestBuilder,
}

impl RetryRequestBuilder {
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self {
            inner: self.inner.header(key, value),
            ..self
        }
    }

    pub fn body(self, body: impl Into<reqwest::Body>) -> Self {
        Self {
            inner: self.inner.body(body),
            ..self
        }
    }

    pub async fn send(self) -> Result<reqwest::Response, reqwest::Error> {
        let request = self.inner.build()?;
        let max_retries = self.client.max_retries;

        for attempt in 0..=max_retries {
            let cloned = request.try_clone().expect("request body must be cloneable");
            match self.client.inner.execute(cloned).await {
                Ok(resp) if resp.status().is_server_error() && attempt < max_retries => {
                    let delay = backoff_delay(attempt);
                    tracing::debug!(
                        "retry {}/{max_retries}: server error {}, waiting {delay:?}",
                        attempt + 1,
                        resp.status(),
                    );
                    tokio::time::sleep(delay).await;
                }
                Ok(resp) => return Ok(resp),
                Err(e) if is_transient(&e) && attempt < max_retries => {
                    let delay = backoff_delay(attempt);
                    tracing::debug!(
                        "retry {}/{max_retries}: {e}, waiting {delay:?}",
                        attempt + 1,
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }

        // Unreachable — the loop always returns
        unreachable!()
    }

    pub async fn bytes(self) -> Result<bytes::Bytes, reqwest::Error> {
        self.send().await?.bytes().await
    }

    pub async fn text(self) -> Result<String, reqwest::Error> {
        self.send().await?.text().await
    }
}

fn backoff_delay(attempt: u32) -> Duration {
    // 500ms, 1s, 2s, ...
    Duration::from_millis(500 * 2u64.pow(attempt))
}

fn is_transient(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}
