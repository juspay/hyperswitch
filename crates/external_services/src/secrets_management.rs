//!
//! Secrets management util module
//!

use common_utils::errors::CustomResult;
#[cfg(feature = "hashicorp-vault")]
use error_stack::ResultExt;
use secrets_interface::secrets_management::{SecretManagementInterface, SecretsManagementError};

#[cfg(feature = "aws_kms")]
use crate::aws_kms;
#[cfg(feature = "hashicorp-vault")]
use crate::hashicorp_vault;
use crate::no_encryption::NoEncryption;

/// Enum representing configuration options for secrets management.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(tag = "secrets_manager")]
#[serde(rename_all = "snake_case")]
pub enum SecretsManagementConfig {
    /// AWS KMS configuration
    #[cfg(feature = "aws_kms")]
    AwsKms {
        /// AWS KMS config
        aws_kms: aws_kms::AwsKmsConfig,
    },

    /// HshiCorp-Vault configuration
    #[cfg(feature = "hashicorp-vault")]
    HashiCorpVault {
        /// HC-Vault config
        hc_vault: hashicorp_vault::HashiCorpVaultConfig,
    },

    /// Varient representing no encryption
    #[default]
    NoEncryption,
}

impl SecretsManagementConfig {
    /// Verifies that the client configuration is usable
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            #[cfg(feature = "aws_kms")]
            Self::AwsKms { aws_kms } => aws_kms.validate(),
            #[cfg(feature = "hashicorp-vault")]
            Self::HashiCorpVault { hc_vault } => hc_vault.validate(),
            Self::NoEncryption => Ok(()),
        }
    }

    /// Retrieves the appropriate secret management client based on the configuration.
    pub async fn get_secret_management_client(
        &self,
    ) -> CustomResult<Box<dyn SecretManagementInterface>, SecretsManagementError> {
        match self {
            #[cfg(feature = "aws_kms")]
            Self::AwsKms { aws_kms } => {
                let res: Box<dyn SecretManagementInterface> =
                    Box::new(aws_kms::AwsKmsClient::new(aws_kms).await);
                Ok(res)
            }
            #[cfg(feature = "hashicorp-vault")]
            Self::HashiCorpVault { hc_vault } => hashicorp_vault::HashiCorpVault::new(hc_vault)
                .map(|val| Box::from(val).into())
                .change_context(SecretsManagementError::ClientCreationFailed),
            Self::NoEncryption => Ok(Box::new(NoEncryption)),
        }
    }
}
