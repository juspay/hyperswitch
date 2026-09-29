//! Dedicated Router transport. No JWT decoding, caller-header forwarding or body logging.

use std::time::Duration;

use reqwest::{Client, Response};
use url::Url;

mod authorize_token;
mod user_details;

#[derive(Debug, PartialEq, Eq)]
pub enum RouterError {
    InvalidCredential,
    PermissionDenied,
    Unavailable,
}

/// Shared connection pool, constructed once in application state.
#[derive(Clone)]
pub struct RouterClient {
    client: Client,
    base_url: Url,
}

impl RouterClient {
    pub fn new(base_url: Url) -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(3))
                .build()?,
            base_url,
        })
    }
}

async fn read_bounded(mut response: Response) -> Result<Vec<u8>, RouterError> {
    const LIMIT: usize = 64 * 1024;
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| RouterError::Unavailable)?
    {
        if chunk.len() > LIMIT - body.len() {
            return Err(RouterError::Unavailable);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
