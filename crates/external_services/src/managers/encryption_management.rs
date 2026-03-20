//! Encryption management util module

use std::sync::Arc;

use common_utils::errors::CustomResult;
use hyperswitch_interfaces::encryption_interface::{
    EncryptionError, EncryptionManagementInterface,
};

#[cfg(feature = "aws_kms")]
use crate::aws_kms;
use crate::no_encryption::core::NoEncryption;

/// Enum representing configuration options for encryption management.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(tag = "encryption_manager")]
#[serde(rename_all = "snake_case")]
pub enum EncryptionManagementConfig {
    /// AWS KMS configuration
    #[cfg(feature = "aws_kms")]
    AwsKms {
        /// AWS KMS config
        aws_kms: aws_kms::core::AwsKmsConfig,
    },

    /// Variant representing no encryption
    #[default]
    NoEncryption,
}

impl EncryptionManagementConfig {
    /// Verifies that the client configuration is usable
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            #[cfg(feature = "aws_kms")]
            Self::AwsKms { aws_kms } => {
                use common_utils::fp_utils::when;

                aws_kms.validate()?;

                when(aws_kms.key_id.is_none(), || {
                    Err("KMS AWS key ID must not be empty")
                })
            }

            Self::NoEncryption => Ok(()),
        }
    }

    /// Retrieves the appropriate encryption client based on the configuration.
    pub async fn get_encryption_management_client(
        &self,
    ) -> CustomResult<Arc<dyn EncryptionManagementInterface>, EncryptionError> {
        Ok(match self {
            #[cfg(feature = "aws_kms")]
            Self::AwsKms { aws_kms } => Arc::new(aws_kms::core::AwsKmsClient::new(aws_kms).await),

            Self::NoEncryption => Arc::new(NoEncryption),
        })
    }
}
