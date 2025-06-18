//! Encryption management util module

use std::sync::Arc;

use common_utils::errors::CustomResult;
use hyperswitch_interfaces::encryption_interface::{
    EncryptionError, EncryptionManagementInterface,
};
#[cfg(feature = "gcp_kms")]
use error_stack::ResultExt;

#[cfg(feature = "aws_kms")]
use crate::aws_kms;
#[cfg(feature = "gcp_kms")]
use crate::gcp_kms;
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

    /// Google Cloud KMS configuration
    #[cfg(feature = "gcp_kms")]
    GcpKms {
        /// GCP KMS config
        gcp_kms: gcp_kms::core::GcpKmsConfig,
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
            Self::AwsKms { aws_kms } => aws_kms.validate(),

            #[cfg(feature = "gcp_kms")]
            Self::GcpKms { gcp_kms } => gcp_kms.validate(),

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

            #[cfg(feature = "gcp_kms")]
            Self::GcpKms { gcp_kms } => Arc::new(gcp_kms::core::GcpKmsClient::new(gcp_kms).await.change_context(EncryptionError::EncryptionFailed)?),

            Self::NoEncryption => Arc::new(NoEncryption),
        })
    }
}
