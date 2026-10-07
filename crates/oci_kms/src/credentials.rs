//! Resolves OCI signing credentials from the ambient environment, and caches them.
//!
//! Mirrors what the AWS and GCP SDKs do for their backends: no credentials in
//! service config, just a source picked from the environment. Inside Kubernetes that
//! is OKE Workload Identity; anywhere else it is the `~/.oci/config` the `oci` CLI writes.

use std::sync::Arc;

use base64::Engine;
use tokio::sync::Mutex;

use crate::{config_file, environment::Environment, error::OciKmsError, workload_identity};

/// Refresh at half the token's lifetime, matching `oci-go-sdk`'s `rpstValidForRatio`
/// and what `oci-python-sdk`'s `valid_with_half_expiration_time` ships.
const SOFT_EXPIRY_LIFETIME_RATIO: i64 = 2;
const REFRESH_BUFFER_SECONDS: i64 = 5 * 60;

/// A signing identity: the `keyId` header value and the key that signs for it.
/// Deliberately not `Debug` — neither field belongs in a log or a panic message.
pub(crate) struct OciCredentials {
    pub(crate) key_id: String,
    pub(crate) private_key: rsa::RsaPrivateKey,
    /// `None` for credentials that never expire, such as an API key.
    pub(crate) soft_expires_at: Option<i64>,
}

/// Caches credentials until their own soft-expiry. The lock is held across the refresh so
/// a burst of concurrent calls resolves once, not once each.
#[derive(Default)]
pub(crate) struct CredentialCache {
    cached: Mutex<Option<Arc<OciCredentials>>>,
}

impl CredentialCache {
    pub(crate) async fn current(
        &self,
        environment: &dyn Environment,
    ) -> Result<Arc<OciCredentials>, OciKmsError> {
        let mut cached = self.cached.lock().await;

        if let Some(credentials) = cached.as_ref() {
            if !is_stale(
                credentials.soft_expires_at,
                environment.now_unix_timestamp(),
            ) {
                return Ok(Arc::clone(credentials));
            }
        }

        let credentials = Arc::new(resolve(environment).await?);
        *cached = Some(Arc::clone(&credentials));

        Ok(credentials)
    }
}

/// Inside Kubernetes this is Workload Identity and nothing else — on-disk credentials are
/// never a fallback there, so a production pod can't quietly end up signing as whoever
/// last logged in with the `oci` CLI.
async fn resolve(environment: &dyn Environment) -> Result<OciCredentials, OciKmsError> {
    if workload_identity::in_kubernetes() {
        workload_identity::credentials(environment).await
    } else {
        config_file::credentials()
    }
}

#[derive(serde::Deserialize)]
struct TokenClaims {
    iat: i64,
    exp: i64,
}

/// Halfway point of a session token's real lifetime, from its own `iat`/`exp` claims. The
/// signature isn't verified: the token is trusted because of how it was obtained, exactly
/// as Oracle's own SDKs treat it.
pub(crate) fn soft_expiry(session_token: &str) -> Result<i64, OciKmsError> {
    let payload = session_token.split('.').nth(1).ok_or_else(|| {
        OciKmsError::CredentialsUnavailable("the OCI session token is not a JWT".to_owned())
    })?;

    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|error| {
            OciKmsError::CredentialsUnavailable(format!(
                "failed to base64 decode the session token payload: {error}"
            ))
        })?;

    let claims: TokenClaims = serde_json::from_slice(&payload).map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "the session token payload is missing `iat`/`exp`: {error}"
        ))
    })?;

    Ok(claims.iat + (claims.exp - claims.iat) / SOFT_EXPIRY_LIFETIME_RATIO)
}

fn is_stale(soft_expires_at: Option<i64>, now: i64) -> bool {
    let Some(soft_expires_at) = soft_expires_at else {
        return false;
    };
    soft_expires_at <= now + REFRESH_BUFFER_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jwt_with_payload(payload: &str) -> String {
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
        format!("header.{encoded}.signature")
    }

    #[test]
    fn soft_expiry_is_halfway_through_the_token_lifetime() {
        let token = jwt_with_payload(r#"{"iat":1000,"exp":4600}"#);
        assert_eq!(soft_expiry(&token).expect("valid token"), 2800);
    }

    #[test]
    fn soft_expiry_rejects_a_token_that_is_not_a_jwt() {
        assert!(soft_expiry("not-a-jwt").is_err());
    }

    #[test]
    fn soft_expiry_rejects_a_token_without_lifetime_claims() {
        let token = jwt_with_payload(r#"{"sub":"workload"}"#);
        assert!(soft_expiry(&token).is_err());
    }

    #[test]
    fn credentials_without_an_expiry_are_never_stale() {
        assert!(!is_stale(None, 1_000_000));
    }

    #[test]
    fn credentials_past_their_soft_expiry_are_stale() {
        assert!(is_stale(Some(0), 1_000_000));
    }

    #[test]
    fn credentials_within_the_refresh_buffer_are_stale() {
        let now = 1_000_000;
        assert!(is_stale(Some(now + REFRESH_BUFFER_SECONDS - 1), now));
        assert!(!is_stale(Some(now + REFRESH_BUFFER_SECONDS + 60), now));
    }
}
