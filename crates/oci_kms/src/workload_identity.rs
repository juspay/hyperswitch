//! Obtains OCI Workload Identity credentials from OKE's proxymux service.
//!
//! OKE writes no credentials to a pod's filesystem. Instead the client generates an
//! ephemeral RSA keypair in memory and trades the pod's own Kubernetes service account
//! token for a session token bound to that keypair. Ported from `oci-go-sdk`'s
//! `x509FederationClientForOkeWorkloadIdentity`
//! (`common/auth/federation_client_oke_workload_identity.go`).

use base64::Engine;
use rsa::pkcs8::EncodePublicKey;

use crate::{
    credentials::{soft_expiry, OciCredentials},
    environment::Environment,
    error::OciKmsError,
    transport::{self, AttemptError},
};

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

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

pub(crate) fn in_kubernetes() -> bool {
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

pub(crate) async fn credentials(
    environment: &dyn Environment,
) -> Result<OciCredentials, OciKmsError> {
    let kubernetes_host = std::env::var(KUBERNETES_HOST_VAR).map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!("{KUBERNETES_HOST_VAR} is unusable: {error}"))
    })?;

    let service_account_token =
        std::fs::read_to_string(SERVICE_ACCOUNT_TOKEN_PATH).map_err(|error| {
            OciKmsError::CredentialsUnavailable(format!(
                "failed to read the pod's Kubernetes service account token ({error}); the pod needs `automountServiceAccountToken: true`"
            ))
        })?;

    let cluster_ca = std::fs::read(CLUSTER_CA_PATH).map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "failed to read the Kubernetes cluster CA certificate: {error}"
        ))
    })?;

    let private_key = generate_session_key().await?;
    let public_key_pem = private_key
        .to_public_key()
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .map_err(|error| {
            OciKmsError::CredentialsUnavailable(format!(
                "failed to encode the ephemeral session public key: {error}"
            ))
        })?;

    let body = serde_json::to_vec(&SessionTokenRequest {
        pod_key: &public_key_pem,
    })
    .map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "failed to serialize the proxymux session token request: {error}"
        ))
    })?;

    let client = proxymux_client(&cluster_ca)?;
    let url = proxymux_url(&kubernetes_host);
    let response_body = transport::with_retries(environment, "oci_proxymux_session_token", || {
        request_session_token(
            environment,
            &client,
            &url,
            service_account_token.trim(),
            &body,
        )
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
    environment: &dyn Environment,
    client: &reqwest::Client,
    url: &str,
    service_account_token: &str,
    body: &[u8],
) -> Result<String, AttemptError> {
    let response = client
        .post(url)
        .bearer_auth(service_account_token)
        .header("content-type", "application/json")
        .header("opc-request-id", transport::request_id(environment))
        .body(body.to_vec())
        .send()
        .await
        .map_err(|error| {
            AttemptError::Retryable(OciKmsError::CredentialsUnavailable(format!(
                "failed to reach the OKE proxymux service: {error}"
            )))
        })?;

    let status = response.status();
    let response_body = response.text().await.map_err(|error| {
        AttemptError::Retryable(OciKmsError::CredentialsUnavailable(format!(
            "failed to read the proxymux response body: {error}"
        )))
    })?;

    if !status.is_success() {
        // Proxymux answers 403 when the cluster isn't an *enhanced* OKE cluster, which
        // is the usual cause and isn't otherwise obvious from the response.
        let hint = match status {
            reqwest::StatusCode::FORBIDDEN => " (Workload Identity needs an enhanced OKE cluster and a policy granting this service account access)",
            _ => "",
        };
        return Err(AttemptError::from_status(
            status,
            OciKmsError::CredentialsUnavailable(format!(
                "proxymux rejected the session token request with status {status}{hint}: {response_body}"
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
/// No proxy is applied either: proxymux is reachable only from inside the cluster.
fn proxymux_client(cluster_ca_pem: &[u8]) -> Result<reqwest::Client, OciKmsError> {
    let certificates = reqwest::Certificate::from_pem_bundle(cluster_ca_pem).map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "failed to parse the Kubernetes cluster CA certificate: {error}"
        ))
    })?;

    certificates
        .into_iter()
        .fold(
            transport::client_builder()
                .use_rustls_tls()
                .tls_built_in_root_certs(false),
            |builder, certificate| builder.add_root_certificate(certificate),
        )
        .no_proxy()
        .build()
        .map_err(|error| {
            OciKmsError::CredentialsUnavailable(format!(
                "failed to build the proxymux HTTP client: {error}"
            ))
        })
}

/// RSA keygen is CPU-bound and can run for hundreds of milliseconds, so keep it off the
/// async worker threads.
async fn generate_session_key() -> Result<rsa::RsaPrivateKey, OciKmsError> {
    tokio::task::spawn_blocking(|| {
        rsa::RsaPrivateKey::new(&mut rand::rngs::OsRng, SESSION_KEY_BITS)
    })
    .await
    .map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "the session key generation task failed to complete: {error}"
        ))
    })?
    .map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "failed to generate the ephemeral session key: {error}"
        ))
    })
}

/// Proxymux answers with a JSON string whose base64-decoded value is itself the JSON
/// `{"token": "ST$<jwt>"}`.
fn parse_session_token(response_body: &str) -> Result<String, OciKmsError> {
    let encoded: String = serde_json::from_str(response_body).map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "the proxymux response was not a JSON string: {error}"
        ))
    })?;

    let decoded = BASE64_ENGINE.decode(encoded).map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "failed to base64 decode the proxymux response: {error}"
        ))
    })?;

    let response: SessionTokenResponse = serde_json::from_slice(&decoded).map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "failed to parse the decoded proxymux response: {error}"
        ))
    })?;

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
        serde_json::to_string(&BASE64_ENGINE.encode(inner)).expect("serializable")
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
