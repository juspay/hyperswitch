//! Router token authorization operation.
use hyperswitch_masking::Secret;
use reqwest::StatusCode;
use serde::Serialize;

use super::{read_bounded, RouterClient, RouterError};

#[derive(Serialize)]
struct AuthorizeTokenRequest<'a> {
    token: &'a Secret<String>,
    permission: &'a str,
}

impl RouterClient {
    pub async fn authorize_token(
        &self,
        token: &Secret<String>,
        permission: &str,
    ) -> Result<(), RouterError> {
        let url = self
            .base_url
            .join("/user/internal/authorize")
            .map_err(|_| RouterError::Unavailable)?;
        let response = self
            .client
            .post(url)
            .json(&AuthorizeTokenRequest { token, permission })
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
}
