// Copyright (C) 2024-2026 Sjors Robroek
// SPDX-License-Identifier: AGPL-3.0-only

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
    #[must_use]
    #[allow(clippy::missing_const_for_fn, reason = "reqwest::Client is not const")]
    pub fn new(client: reqwest::Client, max_retries: u32) -> Self {
        Self {
            inner: client,
            max_retries,
        }
    }

    #[must_use]
    pub fn get(&self, url: &str) -> RetryRequestBuilder {
        RetryRequestBuilder {
            client: self.clone(),
            inner: self.inner.get(url),
        }
    }

    #[must_use]
    pub fn post(&self, url: &str) -> RetryRequestBuilder {
        RetryRequestBuilder {
            client: self.clone(),
            inner: self.inner.post(url),
        }
    }

    #[must_use]
    pub fn patch(&self, url: &str) -> RetryRequestBuilder {
        RetryRequestBuilder {
            client: self.clone(),
            inner: self.inner.patch(url),
        }
    }

    #[must_use]
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
    #[must_use]
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

    #[must_use]
    pub fn body(self, body: impl Into<reqwest::Body>) -> Self {
        Self {
            inner: self.inner.body(body),
            ..self
        }
    }

    /// Send the request with automatic retry on transient errors.
    ///
    /// # Errors
    ///
    /// Returns `reqwest::Error` if the request fails after all retry attempts.
    ///
    /// # Panics
    ///
    /// Panics if the request body is not cloneable (e.g., streaming body).
    pub async fn send(self) -> Result<reqwest::Response, reqwest::Error> {
        let request = self.inner.build()?;
        let max_retries = self.client.max_retries;

        for attempt in 0..=max_retries {
            #[expect(clippy::expect_used, reason = "retry requires cloneable body")]
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

    /// Get the response body as bytes.
    ///
    /// # Errors
    ///
    /// Returns `reqwest::Error` if the request or body read fails.
    pub async fn bytes(self) -> Result<bytes::Bytes, reqwest::Error> {
        self.send().await?.bytes().await
    }

    /// Get the response body as text.
    ///
    /// # Errors
    ///
    /// Returns `reqwest::Error` if the request or body read fails.
    pub async fn text(self) -> Result<String, reqwest::Error> {
        self.send().await?.text().await
    }
}

const fn backoff_delay(attempt: u32) -> Duration {
    // 500ms, 1s, 2s, ...
    Duration::from_millis(500 * 2u64.pow(attempt))
}

fn is_transient(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}
