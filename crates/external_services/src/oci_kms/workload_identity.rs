//! Obtains OCI Workload Identity credentials from OKE's proxymux service.
//!
//! OKE writes no credentials to a pod's filesystem. Instead the client generates an
//! ephemeral RSA keypair in memory and trades the pod's own Kubernetes service account
//! token for a session token bound to that keypair. Ported from `oci-go-sdk`'s
//! `x509FederationClientForOkeWorkloadIdentity`
//! (`common/auth/federation_client_oke_workload_identity.go`).

use base64::Engine;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use rsa::pkcs8::EncodePublicKey;

use super::{
    core::OciKmsError,
    credentials::{soft_expiry, OciCredentials},
    transport::{self, AttemptError},
};
use crate::consts;

/// Injected into every pod by kubelet. Proxymux answers on port 12250 at that same
/// address — an Oracle-managed control-plane service, not a node-local agent and not
/// anything deployed alongside this app.
const KUBERNETES_HOST_VAR: &str = "KUBERNETES_SERVICE_HOST";
const PROXYMUX_PORT: u16 = 12250;
const PROXYMUX_PATH: &str = "/resourcePrincipalSessionTokens";

const SERVICE_ACCOUNT_TOKEN_PATH: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
const CLUSTER_CA_PATH: &str = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";

const SESSION_KEY_BITS: usize = 2048;

/// Proxymux returns the token already prefixed, and the `keyId` re-adds it.
const SECURITY_TOKEN_PREFIX: &str = "ST$";

pub(super) fn in_kubernetes() -> bool {
    std::env::var_os(KUBERNETES_HOST_VAR).is_some()
}

#[derive(serde::Serialize)]
struct SessionTokenRequest<'a> {
    #[serde(rename = "podKey")]
    pod_key: &'a str,
}

#[derive(serde::Deserialize)]
struct SessionTokenResponse {
    token: String,
}

pub(super) async fn credentials() -> CustomResult<OciCredentials, OciKmsError> {
    let kubernetes_host = std::env::var(KUBERNETES_HOST_VAR)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable_lazy(|| format!("Missing environment variable: {KUBERNETES_HOST_VAR}"))?;

    let service_account_token = std::fs::read_to_string(SERVICE_ACCOUNT_TOKEN_PATH)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable(
            "Failed to read the pod's Kubernetes service account token; the pod needs `automountServiceAccountToken: true`",
        )?;

    let cluster_ca = std::fs::read(CLUSTER_CA_PATH)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to read the Kubernetes cluster CA certificate")?;

    let private_key = generate_session_key().await?;
    let public_key_pem = private_key
        .to_public_key()
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to encode the ephemeral session public key")?;

    let body = serde_json::to_vec(&SessionTokenRequest {
        pod_key: &public_key_pem,
    })
    .change_context(OciKmsError::CredentialsUnavailable)
    .attach_printable("Failed to serialize the proxymux session token request")?;

    let client = proxymux_client(&cluster_ca)?;
    let url = proxymux_url(&kubernetes_host);
    let response_body = transport::with_retries("oci_proxymux_session_token", || {
        request_session_token(&client, &url, service_account_token.trim(), &body)
    })
    .await?;

    let session_token = parse_session_token(&response_body)?;

    Ok(OciCredentials {
        soft_expires_at: Some(soft_expiry(&session_token)?),
        key_id: format!("{SECURITY_TOKEN_PREFIX}{session_token}"),
        private_key,
    })
}

async fn request_session_token(
    client: &reqwest::Client,
    url: &str,
    service_account_token: &str,
    body: &[u8],
) -> Result<String, AttemptError> {
    let response = client
        .post(url)
        .bearer_auth(service_account_token)
        .header("content-type", "application/json")
        .header(
            "opc-request-id",
            hex::encode(common_utils::generate_random_bytes(16)),
        )
        .body(body.to_vec())
        .send()
        .await
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to reach the OKE proxymux service")
        .map_err(AttemptError::Retryable)?;

    let status = response.status();
    let response_body = response
        .text()
        .await
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to read the proxymux response body")
        .map_err(AttemptError::Retryable)?;

    if !status.is_success() {
        // Proxymux answers 403 when the cluster isn't an *enhanced* OKE cluster, which
        // is the usual cause and isn't otherwise obvious from the response.
        let hint = match status {
            reqwest::StatusCode::FORBIDDEN => " (Workload Identity needs an enhanced OKE cluster and a policy granting this service account access)",
            _ => "",
        };
        return Err(AttemptError::from_status(
            status,
            report!(OciKmsError::CredentialsUnavailable).attach_printable(format!(
                "Proxymux rejected the session token request with status {status}{hint}: {response_body}"
            )),
        ));
    }

    Ok(response_body)
}

/// `KUBERNETES_SERVICE_HOST` is a bare IP, so an IPv6 address needs brackets in a URL.
fn proxymux_url(kubernetes_host: &str) -> String {
    if kubernetes_host.contains(':') && !kubernetes_host.starts_with('[') {
        format!("https://[{kubernetes_host}]:{PROXYMUX_PORT}{PROXYMUX_PATH}")
    } else {
        format!("https://{kubernetes_host}:{PROXYMUX_PORT}{PROXYMUX_PATH}")
    }
}

/// Trusts the cluster CA alone: this request carries the pod's Kubernetes identity as a
/// bearer token, so accepting any other issuer would risk handing it to an impostor.
/// Deliberately not `http_client::create_client` — that applies the outbound proxy
/// config, and proxymux is reachable only from inside the cluster.
fn proxymux_client(cluster_ca_pem: &[u8]) -> CustomResult<reqwest::Client, OciKmsError> {
    let certificates = reqwest::Certificate::from_pem_bundle(cluster_ca_pem)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to parse the Kubernetes cluster CA certificate")?;

    certificates
        .into_iter()
        .fold(
            transport::client_builder()
                .use_rustls_tls()
                .tls_built_in_root_certs(false),
            |builder, certificate| builder.add_root_certificate(certificate),
        )
        .build()
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to build the proxymux HTTP client")
}

/// RSA keygen is CPU-bound and can run for hundreds of milliseconds, so keep it off the
/// async worker threads.
async fn generate_session_key() -> CustomResult<rsa::RsaPrivateKey, OciKmsError> {
    tokio::task::spawn_blocking(|| {
        rsa::RsaPrivateKey::new(&mut rand::rngs::OsRng, SESSION_KEY_BITS)
    })
    .await
    .change_context(OciKmsError::CredentialsUnavailable)
    .attach_printable("The session key generation task failed to complete")?
    .change_context(OciKmsError::CredentialsUnavailable)
    .attach_printable("Failed to generate the ephemeral session key")
}

/// Proxymux answers with a JSON string whose base64-decoded value is itself the JSON
/// `{"token": "ST$<jwt>"}`.
fn parse_session_token(response_body: &str) -> CustomResult<String, OciKmsError> {
    let encoded: String = serde_json::from_str(response_body)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("The proxymux response was not a JSON string")?;

    let decoded = consts::BASE64_ENGINE
        .decode(encoded)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to base64 decode the proxymux response")?;

    let response: SessionTokenResponse = serde_json::from_slice(&decoded)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to parse the decoded proxymux response")?;

    Ok(response
        .token
        .strip_prefix(SECURITY_TOKEN_PREFIX)
        .unwrap_or(&response.token)
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wraps `inner` the way proxymux does: a JSON string holding base64 of the JSON body.
    fn proxymux_response(inner: &str) -> String {
        serde_json::to_string(&consts::BASE64_ENGINE.encode(inner)).expect("serializable")
    }

    #[test]
    fn parse_session_token_strips_the_security_token_prefix() {
        let body = proxymux_response(r#"{"token":"ST$header.payload.signature"}"#);
        assert_eq!(
            parse_session_token(&body).expect("valid response"),
            "header.payload.signature"
        );
    }

    #[test]
    fn parse_session_token_accepts_an_unprefixed_token() {
        let body = proxymux_response(r#"{"token":"header.payload.signature"}"#);
        assert_eq!(
            parse_session_token(&body).expect("valid response"),
            "header.payload.signature"
        );
    }

    #[test]
    fn parse_session_token_rejects_a_plain_json_object() {
        assert!(parse_session_token(r#"{"token":"ST$abc"}"#).is_err());
    }

    #[test]
    fn parse_session_token_rejects_invalid_base64() {
        assert!(parse_session_token(r#""not base64!""#).is_err());
    }

    #[test]
    fn proxymux_url_uses_an_ipv4_host_as_is() {
        assert_eq!(
            proxymux_url("10.96.0.1"),
            "https://10.96.0.1:12250/resourcePrincipalSessionTokens"
        );
    }

    #[test]
    fn proxymux_url_brackets_an_ipv6_host() {
        assert_eq!(
            proxymux_url("fd00::1"),
            "https://[fd00::1]:12250/resourcePrincipalSessionTokens"
        );
    }

    #[test]
    fn proxymux_url_keeps_an_already_bracketed_ipv6_host() {
        assert_eq!(
            proxymux_url("[fd00::1]"),
            "https://[fd00::1]:12250/resourcePrincipalSessionTokens"
        );
    }
}
