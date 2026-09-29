//! Router active-user lookup operation.
use hyperswitch_masking::{PeekInterface, Secret};
use reqwest::{header, StatusCode};
use serde::Deserialize;

use super::{read_bounded, RouterClient, RouterError};

#[derive(Deserialize)]
struct UserDetailsResponse {
    email: String,
}

impl RouterClient {
    pub async fn get_user_email(&self, token: &Secret<String>) -> Result<String, RouterError> {
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
                let user: UserDetailsResponse =
                    serde_json::from_slice(&read_bounded(response).await?)
                        .map_err(|_| RouterError::Unavailable)?;
                Ok(user.email)
            }
            StatusCode::UNAUTHORIZED => Err(RouterError::InvalidCredential),
            _ => Err(RouterError::Unavailable),
        }
    }
}
