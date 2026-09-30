//! Gateway-facing monitoring authorization contract.

use hyperswitch_masking::Secret;

/// Only a token is accepted; the gateway cannot choose an identity or permission.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrafanaAuthRequest {
    pub token: Secret<String>,
}

/// A namespaced Grafana login, not Router's canonical user ID.
#[derive(Debug, serde::Serialize)]
pub struct GrafanaAuthResponse {
    pub grafana_login: String,
}

/// Configured embed destination for an authenticated Grafana session.
#[derive(Debug, serde::Serialize)]
pub struct GrafanaSessionResponse {
    pub embed_url: String,
}
