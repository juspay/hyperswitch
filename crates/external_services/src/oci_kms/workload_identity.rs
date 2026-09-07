//! Reads OCI Workload Identity credentials the OKE sidecar writes to local files.
//!
//! Follows OCI's resource-principal-v2 environment contract:
//!
//! - `OCI_RESOURCE_PRINCIPAL_RPST` — path to the file holding the current session
//!   token (used as `keyId="ST$<token>"` when signing requests)
//! - `OCI_RESOURCE_PRINCIPAL_PRIVATE_PEM` — path to the file holding the ephemeral
//!   private key the session token is bound to
//! - `OCI_RESOURCE_PRINCIPAL_REGION` — the canonical region name (inline, not a path
//!   — it doesn't rotate)
//!
//! CAUTION: this file-based contract is confirmed for OCI *Functions*' resource
//! principals, but genuine OKE Workload Identity (per `oracle/oci-go-sdk`'s
//! `auth.OkeWorkloadIdentityConfigurationProvider`, the code External Secrets
//! Operator and the Secrets Store CSI driver both call) instead holds an in-memory
//! ephemeral keypair and does a live handshake with a node-local proxymux service —
//! no files at all. Verify against a real OKE pod before trusting this in production.

use std::{
    sync::{Arc, RwLock},
    time::SystemTime,
};

use base64::Engine;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use rsa::pkcs8::DecodePrivateKey;

use super::core::OciKmsError;

/// Mirrors `oci-go-sdk`'s `rpstValidForRatio` / `bufferTimeBeforeTokenExpiration` (`jwt.go`).
const SOFT_EXPIRY_LIFETIME_RATIO: i64 = 2;
const REFRESH_BUFFER_SECONDS: i64 = 5 * 60;

const RPST_PATH_VAR: &str = "OCI_RESOURCE_PRINCIPAL_RPST";
const PRIVATE_KEY_PATH_VAR: &str = "OCI_RESOURCE_PRINCIPAL_PRIVATE_PEM";
const REGION_VAR: &str = "OCI_RESOURCE_PRINCIPAL_REGION";

/// Ambient OCI Workload Identity credentials: a session token and its bound private key.
pub(crate) struct WorkloadIdentityCredentials {
    /// Used as `keyId="ST$<token>"` when signing requests.
    pub(crate) session_token: String,
    pub(crate) private_key: rsa::RsaPrivateKey,
    pub(crate) region: String,
}

impl std::fmt::Debug for WorkloadIdentityCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately redact `session_token`/`private_key` — this is credential
        // material, not something to expose via a Debug/panic-message format.
        f.debug_struct("WorkloadIdentityCredentials")
            .field("region", &self.region)
            .finish_non_exhaustive()
    }
}

struct CachedCredentials {
    credentials: Arc<WorkloadIdentityCredentials>,
    source_modified: SystemTime,
    soft_expires_at: Option<i64>,
}

/// Caches parsed credentials by the session-token file's mtime, avoiding a PEM
/// re-parse per request. Owned by `OciKmsClient` behind an `Arc` so cache lifetime
/// matches client lifetime, not a process-global.
#[derive(Default)]
pub(crate) struct WorkloadIdentityCache {
    cached: RwLock<Option<CachedCredentials>>,
}

impl WorkloadIdentityCache {
    /// Returns current credentials, reloading if the file's mtime changed or the
    /// cached token is past its own soft-expiry.
    pub(crate) fn current(&self) -> CustomResult<Arc<WorkloadIdentityCredentials>, OciKmsError> {
        let rpst_path = env_var(RPST_PATH_VAR)?;
        let modified = std::fs::metadata(&rpst_path)
            .and_then(|metadata| metadata.modified())
            .change_context(OciKmsError::CredentialsUnavailable)
            .attach_printable("Failed to stat the resource-principal session token file")?;

        {
            let cached = self
                .cached
                .read()
                .map_err(|_| report!(OciKmsError::CredentialsUnavailable))?;
            if let Some(cached) = cached.as_ref() {
                if cached.source_modified == modified && !is_stale(cached.soft_expires_at) {
                    return Ok(cached.credentials.clone());
                }
            }
        }

        let credentials = Arc::new(load(&rpst_path)?);
        let soft_expires_at = soft_expiry(&credentials.session_token);

        let mut cached = self
            .cached
            .write()
            .map_err(|_| report!(OciKmsError::CredentialsUnavailable))?;
        *cached = Some(CachedCredentials {
            credentials: credentials.clone(),
            source_modified: modified,
            soft_expires_at,
        });

        Ok(credentials)
    }
}

#[derive(serde::Deserialize)]
struct RpstClaims {
    iat: i64,
    exp: i64,
}

/// Halfway point of the token's real lifetime, from its own `iat`/`exp` JWT claims.
fn soft_expiry(session_token: &str) -> Option<i64> {
    let payload_b64 = session_token.split('.').nth(1)?;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .ok()?;
    let claims: RpstClaims = serde_json::from_slice(&payload).ok()?;
    Some(claims.iat + (claims.exp - claims.iat) / SOFT_EXPIRY_LIFETIME_RATIO)
}

/// `None` means "couldn't compute a soft-expiry" — trust it until mtime says otherwise.
fn is_stale(soft_expires_at: Option<i64>) -> bool {
    let Some(soft_expires_at) = soft_expires_at else {
        return false;
    };
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(i64::MAX);
    soft_expires_at <= now + REFRESH_BUFFER_SECONDS
}

fn load(rpst_path: &str) -> CustomResult<WorkloadIdentityCredentials, OciKmsError> {
    let session_token = std::fs::read_to_string(rpst_path)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to read the resource-principal session token file")?
        .trim()
        .to_owned();

    let private_key_path = env_var(PRIVATE_KEY_PATH_VAR)?;
    let private_key_pem = std::fs::read_to_string(&private_key_path)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to read the resource-principal private key file")?;
    let private_key = rsa::RsaPrivateKey::from_pkcs8_pem(&private_key_pem)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable("Failed to parse the resource-principal private key")?;

    let region = env_var(REGION_VAR)?;

    Ok(WorkloadIdentityCredentials {
        session_token,
        private_key,
        region,
    })
}

fn env_var(name: &'static str) -> CustomResult<String, OciKmsError> {
    std::env::var(name)
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable_lazy(|| format!("Missing environment variable: {name}"))
}
