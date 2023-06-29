use common_utils::ext_traits::ConfigExt;
use storage_models::errors::ApplicationError;

impl super::settings::SupportedConnectors {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.wallets.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "list of connectors supporting wallets must not be empty".into(),
            ))
        })
    }
}

impl super::settings::ConnectorParamsWithFileUploadUrl {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.base_url.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "connector base URL must not be empty".into(),
            ))
        })?;
        common_utils::fp_utils::when(self.base_url_file_upload.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "connector file upload base URL must not be empty".into(),
            ))
        })
    }
}

#[cfg(feature = "s3")]
impl super::settings::FileUploadConfig {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.region.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "s3 region must not be empty".into(),
            ))
        })?;

        when(self.bucket_name.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "s3 bucket name must not be empty".into(),
            ))
        })
    }
}
