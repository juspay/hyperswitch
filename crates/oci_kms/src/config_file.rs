//! Reads OCI credentials from `~/.oci/config`, the file the `oci` CLI writes.
//!
//! This is the off-cluster path — local development, CI, or any container that isn't an
//! OKE pod — and the counterpart to `~/.aws/credentials` and gcloud's ADC file. Supports
//! the two profile shapes the CLI produces: session-token auth (`oci session
//! authenticate`) and API-key auth. Only the `keyId` differs between them; both sign
//! identically, so `signing.rs` is unaware of which one is in play.

use std::{collections::HashMap, path::PathBuf};

use rsa::{pkcs1::DecodeRsaPrivateKey, pkcs8::DecodePrivateKey};

use crate::{
    credentials::{soft_expiry, OciCredentials},
    error::OciKmsError,
};

const CONFIG_PATH_VAR: &str = "OCI_CLI_CONFIG_FILE";
const PROFILE_VAR: &str = "OCI_CLI_PROFILE";
const DEFAULT_CONFIG_PATH: &str = "~/.oci/config";
const DEFAULT_PROFILE: &str = "DEFAULT";

pub(crate) fn credentials() -> Result<OciCredentials, OciKmsError> {
    let config_path =
        std::env::var(CONFIG_PATH_VAR).unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_owned());
    let profile_name = std::env::var(PROFILE_VAR).unwrap_or_else(|_| DEFAULT_PROFILE.to_owned());

    let config = std::fs::read_to_string(expand_home(&config_path)).map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "not running in Kubernetes, and no OCI config file at {config_path} ({error}); run `oci session authenticate`, or set {CONFIG_PATH_VAR}"
        ))
    })?;

    let profile = parse_profile(&config, &profile_name).ok_or_else(|| {
        OciKmsError::CredentialsUnavailable(format!(
            "no profile named [{profile_name}] in {config_path}"
        ))
    })?;

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
                .map_err(|error| {
                    OciKmsError::CredentialsUnavailable(format!(
                        "failed to read the OCI session token file: {error}"
                    ))
                })?
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
) -> Result<&'a str, OciKmsError> {
    profile.get(key).map(String::as_str).ok_or_else(|| {
        OciKmsError::CredentialsUnavailable(format!("OCI config profile is missing `{key}`"))
    })
}

fn load_private_key(path: &str) -> Result<rsa::RsaPrivateKey, OciKmsError> {
    let contents = std::fs::read_to_string(expand_home(path)).map_err(|error| {
        OciKmsError::CredentialsUnavailable(format!(
            "failed to read the OCI private key at {path}: {error}"
        ))
    })?;
    let pem = pem_block(&contents);

    rsa::RsaPrivateKey::from_pkcs8_pem(&pem)
        .or_else(|_| rsa::RsaPrivateKey::from_pkcs1_pem(&pem))
        .map_err(|error| {
            OciKmsError::CredentialsUnavailable(format!(
                "failed to parse the OCI private key at {path}: {error}"
            ))
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "\
[DEFAULT]
user=ocid1.user.oc1..default
fingerprint=aa:bb
# a comment
Tenancy = ocid1.tenancy.oc1..default
key_file=~/.oci/default.pem

[SESSION]
security_token_file=~/.oci/sessions/token
key_file=~/.oci/sessions/key.pem
";

    #[test]
    fn parse_profile_reads_only_the_requested_section() {
        let profile = parse_profile(CONFIG, "SESSION").expect("profile exists");
        assert_eq!(profile.len(), 2);
        assert_eq!(
            profile.get("security_token_file").map(String::as_str),
            Some("~/.oci/sessions/token")
        );
        assert!(!profile.contains_key("user"));
    }

    #[test]
    fn parse_profile_trims_and_lowercases_keys_and_skips_comments() {
        let profile = parse_profile(CONFIG, "DEFAULT").expect("profile exists");
        assert_eq!(
            profile.get("tenancy").map(String::as_str),
            Some("ocid1.tenancy.oc1..default")
        );
        assert_eq!(profile.len(), 4);
    }

    #[test]
    fn parse_profile_returns_none_for_a_missing_section() {
        assert!(parse_profile(CONFIG, "MISSING").is_none());
    }

    #[test]
    fn pem_block_drops_the_cli_label_after_the_footer() {
        // `pem_block` only looks for the `-----END` line, so the label doesn't matter.
        let contents = "-----BEGIN TEST BLOCK-----\nabc\n-----END TEST BLOCK-----\nOCI_API_KEY\n";
        assert_eq!(
            pem_block(contents),
            "-----BEGIN TEST BLOCK-----\nabc\n-----END TEST BLOCK-----\n"
        );
    }

    #[test]
    fn expand_home_leaves_absolute_paths_untouched() {
        assert_eq!(
            expand_home("/etc/oci/config"),
            PathBuf::from("/etc/oci/config")
        );
    }
}
