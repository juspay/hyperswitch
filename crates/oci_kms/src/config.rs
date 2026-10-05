//! Configuration for an [`OciKmsClient`](crate::OciKmsClient).

/// Configuration parameters required for constructing an [`OciKmsClient`](crate::OciKmsClient).
///
/// Holds no credentials: those come from the environment (see the crate docs).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct OciKmsConfig {
    /// The vault's crypto endpoint (e.g. `https://<vault>-crypto.kms.<region>.oci.oraclecloud.com`),
    /// obtained once when the vault is created; doesn't change afterward.
    pub vault_crypto_endpoint: String,

    /// The full OCID of the KMS key used to encrypt/decrypt data.
    pub key_id: String,
}

impl OciKmsConfig {
    /// Verifies that the configuration is usable.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.vault_crypto_endpoint.trim().is_empty() {
            return Err("OCI KMS vault crypto endpoint must not be empty");
        }
        if self.key_id.trim().is_empty() {
            return Err("OCI KMS key ID must not be empty");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> OciKmsConfig {
        OciKmsConfig {
            vault_crypto_endpoint: "https://abc-crypto.kms.ap-mumbai-1.oci.oraclecloud.com"
                .to_string(),
            key_id: "ocid1.key.oc1.ap-mumbai-1.test".to_string(),
        }
    }

    #[test]
    fn validate_succeeds_when_all_fields_are_set() {
        assert!(config().validate().is_ok());
    }

    #[test]
    fn validate_fails_when_vault_crypto_endpoint_is_empty() {
        let config = OciKmsConfig {
            vault_crypto_endpoint: String::new(),
            ..config()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_fails_when_key_id_is_blank() {
        let config = OciKmsConfig {
            key_id: "  ".to_string(),
            ..config()
        };
        assert!(config.validate().is_err());
    }
}
