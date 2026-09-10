//! Reads OCI credentials from `~/.oci/config`, the file the `oci` CLI writes.
//!
//! This is the off-cluster path — local development, CI, or any container that isn't an
//! OKE pod — and the counterpart to `~/.aws/credentials` and gcloud's ADC file. Supports
//! the two profile shapes the CLI produces: session-token auth (`oci session
//! authenticate`) and API-key auth. Only the `keyId` differs between them; both sign
//! identically, so `signing.rs` is unaware of which one is in play.

use std::{collections::HashMap, path::PathBuf};

use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use rsa::{pkcs1::DecodeRsaPrivateKey, pkcs8::DecodePrivateKey};

use super::{
    core::OciKmsError,
    credentials::{soft_expiry, OciCredentials},
};

const CONFIG_PATH_VAR: &str = "OCI_CLI_CONFIG_FILE";
const PROFILE_VAR: &str = "OCI_CLI_PROFILE";
const DEFAULT_CONFIG_PATH: &str = "~/.oci/config";
const DEFAULT_PROFILE: &str = "DEFAULT";

pub(super) fn credentials() -> CustomResult<OciCredentials, OciKmsError> {
    let config_path =
        std::env::var(CONFIG_PATH_VAR).unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_owned());
    let profile_name = std::env::var(PROFILE_VAR).unwrap_or_else(|_| DEFAULT_PROFILE.to_owned());

    let config = std::fs::read_to_string(expand_home(&config_path))
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable_lazy(|| {
            format!(
                "Not running in Kubernetes, and no OCI config file at {config_path}. Run `oci session authenticate`, or set {CONFIG_PATH_VAR}"
            )
        })?;

    let profile = parse_profile(&config, &profile_name)
        .ok_or_else(|| report!(OciKmsError::CredentialsUnavailable))
        .attach_printable_lazy(|| format!("No profile named [{profile_name}] in {config_path}"))?;

    let private_key = load_private_key(required(&profile, "key_file")?)?;

    // Precedence matches `oci-go-sdk`'s `fileConfigurationProvider::KeyID`: a profile
    // carrying `user` is API-key auth, even when a session token sits alongside it.
    match profile.get("user") {
        Some(user) => {
            let tenancy = required(&profile, "tenancy")?;
            let fingerprint = required(&profile, "fingerprint")?;

            Ok(OciCredentials {
                key_id: format!("{tenancy}/{user}/{fingerprint}"),
                private_key,
                soft_expires_at: None,
            })
        }
        None => {
            let token_path = required(&profile, "security_token_file")?;
            let token = std::fs::read_to_string(expand_home(token_path))
                .change_context(OciKmsError::CredentialsUnavailable)
                .attach_printable("Failed to read the OCI session token file")?
                .trim()
                .to_owned();

            Ok(OciCredentials {
                soft_expires_at: Some(soft_expiry(&token)?),
                key_id: format!("ST${token}"),
                private_key,
            })
        }
    }
}

fn parse_profile(config: &str, profile_name: &str) -> Option<HashMap<String, String>> {
    let mut current_section = None;
    let mut values = HashMap::new();

    for line in config.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current_section = Some(section.trim().to_owned());
            continue;
        }

        if current_section.as_deref() == Some(profile_name) {
            if let Some((key, value)) = line.split_once('=') {
                values.insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
            }
        }
    }

    (!values.is_empty()).then_some(values)
}

fn required<'a>(
    profile: &'a HashMap<String, String>,
    key: &'static str,
) -> CustomResult<&'a str, OciKmsError> {
    profile
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| report!(OciKmsError::CredentialsUnavailable))
        .attach_printable_lazy(|| format!("OCI config profile is missing `{key}`"))
}

fn load_private_key(path: &str) -> CustomResult<rsa::RsaPrivateKey, OciKmsError> {
    let contents = std::fs::read_to_string(expand_home(path))
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable_lazy(|| format!("Failed to read the OCI private key at {path}"))?;
    let pem = pem_block(&contents);

    rsa::RsaPrivateKey::from_pkcs8_pem(&pem)
        .or_else(|_| rsa::RsaPrivateKey::from_pkcs1_pem(&pem))
        .change_context(OciKmsError::CredentialsUnavailable)
        .attach_printable_lazy(|| format!("Failed to parse the OCI private key at {path}"))
}

/// The `oci` CLI appends an `OCI_API_KEY` label line after the PEM footer, which strict
/// PKCS#8 parsers reject. Keep only through the end of the PEM block.
fn pem_block(contents: &str) -> String {
    let mut block = String::new();
    for line in contents.lines() {
        block.push_str(line);
        block.push('\n');
        if line.starts_with("-----END") {
            break;
        }
    }
    block
}

fn expand_home(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(rest),
            None => PathBuf::from(path),
        },
        None => PathBuf::from(path),
    }
}
