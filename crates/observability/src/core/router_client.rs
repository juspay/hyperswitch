//! Dedicated Router transport. No JWT decoding, caller-header forwarding or body logging.

use std::time::Duration;

use hyperswitch_masking::{PeekInterface, Secret};
use reqwest::{header, Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, PartialEq, Eq)]
pub enum RouterError {
    InvalidCredential,
    PermissionDenied,
    Unavailable,
}

/// Shared connection pool, constructed once in application state.
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

    pub async fn authorize_token(
        &self,
        token: &Secret<String>,
        permission: &str,
    ) -> Result<(), RouterError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            token: &'a Secret<String>,
            permission: &'a str,
        }
        let url = self
            .base_url
            .join("/user/internal/authorize")
            .map_err(|_| RouterError::Unavailable)?;
        let response = self
            .client
            .post(url)
            .json(&Payload { token, permission })
            .send()
            .await
            .map_err(|_| RouterError::Unavailable)?;
        match response.status() {
            StatusCode::OK => {
                // Router StatusOk has an empty body. Do not accept an unrelated JSON endpoint.
                if read_bounded(response).await?.is_empty() {
                    Ok(())
                } else {
                    Err(RouterError::Unavailable)
                }
            }
            StatusCode::UNAUTHORIZED => Err(RouterError::InvalidCredential),
            StatusCode::FORBIDDEN => Err(RouterError::PermissionDenied),
            _ => Err(RouterError::Unavailable),
        }
    }

    pub async fn get_user_email(&self, token: &Secret<String>) -> Result<String, RouterError> {
        #[derive(Deserialize)]
        struct User {
            email: String,
        }
        let url = self
            .base_url
            .join("/user")
            .map_err(|_| RouterError::Unavailable)?;
        // Router's force_cookies mode selects the cookie; the normal mode selects Authorization.
        // Populate both from the SAME token and mark them sensitive even for transport debugging.
        let sensitive = |value: String| {
            let mut header = header::HeaderValue::from_str(&value)
                .map_err(|_| RouterError::InvalidCredential)?;
            header.set_sensitive(true);
            Ok::<_, RouterError>(header)
        };
        let response = self
            .client
            .get(url)
            .header(
                header::AUTHORIZATION,
                sensitive(format!("Bearer {}", token.peek()))?,
            )
            .header(
                header::COOKIE,
                sensitive(format!("login_token={}", token.peek()))?,
            )
            .send()
            .await
            .map_err(|_| RouterError::Unavailable)?;
        match response.status() {
            StatusCode::OK => {
                let user: User = serde_json::from_slice(&read_bounded(response).await?)
                    .map_err(|_| RouterError::Unavailable)?;
                Ok(user.email)
            }
            StatusCode::UNAUTHORIZED => Err(RouterError::InvalidCredential),
            _ => Err(RouterError::Unavailable),
        }
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
