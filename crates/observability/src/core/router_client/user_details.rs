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
            .join("user")
            .map_err(|_| RouterError::Unavailable)?;
        // The Router deployment uses force_cookies=false. Never forward browser cookies.
        let mut authorization = header::HeaderValue::from_str(&format!("Bearer {}", token.peek()))
            .map_err(|_| RouterError::InvalidCredential)?;
        authorization.set_sensitive(true);
        let response = self
            .client
            .get(url)
            .header(header::AUTHORIZATION, authorization)
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
