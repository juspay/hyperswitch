use common_utils::ext_traits::ConfigExt;

use crate::core::errors::ApplicationError;

impl super::settings::Secrets {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.jwt_secret.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "JWT secret must not be empty".into(),
            ))
        })?;

        when(self.admin_api_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "admin API key must not be empty".into(),
            ))
        })
    }
}

impl super::settings::Locker {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(!self.mock_locker && self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "locker host must not be empty when mock locker is disabled".into(),
            ))
        })?;

        when(
            !self.mock_locker && self.basilisk_host.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "basilisk host must not be empty when mock locker is disabled".into(),
                ))
            },
        )
    }
}

impl super::settings::Jwekey {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        #[cfg(feature = "kms")]
        common_utils::fp_utils::when(self.aws_key_id.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "AWS key ID must not be empty when KMS feature is enabled".into(),
            ))
        })?;

        #[cfg(feature = "kms")]
        common_utils::fp_utils::when(self.aws_region.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "AWS region must not be empty when KMS feature is enabled".into(),
            ))
        })?;

        Ok(())
    }
}

impl super::settings::Server {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "server host must not be empty".into(),
            ))
        })
    }
}

impl super::settings::Database {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database host must not be empty".into(),
            ))
        })?;

        when(self.username.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database user username must not be empty".into(),
            ))
        })?;

        when(self.password.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database user password must not be empty".into(),
            ))
        })?;

        when(self.dbname.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database name must not be empty".into(),
            ))
        })
    }
}

impl super::settings::SupportedConnectors {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.wallets.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "list of connectors supporting wallets must not be empty".into(),
            ))
        })
    }
}

impl super::settings::Connectors {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        self.aci.validate()?;
        self.adyen.validate()?;
        self.applepay.validate()?;
        self.authorizedotnet.validate()?;
        self.braintree.validate()?;
        self.checkout.validate()?;
        self.cybersource.validate()?;
        self.globalpay.validate()?;
        self.klarna.validate()?;
        self.shift4.validate()?;
        self.stripe.validate()?;
        self.worldpay.validate()?;

        self.supported.validate()?;

        Ok(())
    }
}

impl super::settings::ConnectorParams {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.base_url.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "connector base URL must not be empty".into(),
            ))
        })
    }
}

impl super::settings::SchedulerSettings {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.stream.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "scheduler stream must not be empty".into(),
            ))
        })?;

        when(self.consumer.consumer_group.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "scheduler consumer group must not be empty".into(),
            ))
        })?;

        self.producer.validate()?;

        Ok(())
    }
}

impl super::settings::ProducerSettings {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.lock_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "producer lock key must not be empty".into(),
            ))
        })
    }
}

#[cfg(feature = "kv_store")]
impl super::settings::DrainerSettings {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.stream_name.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "drainer stream name must not be empty".into(),
            ))
        })
    }
}
