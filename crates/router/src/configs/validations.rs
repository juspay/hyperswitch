use common_utils::ext_traits::ConfigExt;
use storage_impl::errors::ApplicationError;

impl super::settings::Secrets {
        /// Validates the configuration values for the application.
    /// 
    /// # Returns
    /// 
    /// * `Result<(), ApplicationError>` - A result indicating success if the configuration values are valid,
    ///   or an error of type `ApplicationError` if any of the configuration values are invalid.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        #[cfg(not(feature = "kms"))]
        {
            when(self.jwt_secret.is_default_or_empty(), || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "JWT secret must not be empty".into(),
                ))
            })?;

            when(self.admin_api_key.is_default_or_empty(), || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "admin API key must not be empty".into(),
                ))
            })?;
        }

        #[cfg(feature = "kms")]
        {
            when(self.kms_encrypted_jwt_secret.is_default_or_empty(), || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "KMS encrypted JWT secret must not be empty".into(),
                ))
            })?;

            when(
                self.kms_encrypted_admin_api_key.is_default_or_empty(),
                || {
                    Err(ApplicationError::InvalidConfigurationValueError(
                        "KMS encrypted admin API key must not be empty".into(),
                    ))
                },
            )?;
        }
        when(self.master_enc_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Master encryption key must not be empty".into(),
            ))
        })
    }
}

impl super::settings::Locker {
        /// Validates the configuration values and returns a Result.
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

impl super::settings::Server {
        /// Validates the current state of the object and returns a Result indicating
    /// whether the validation was successful or if an ApplicationError occurred.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "server host must not be empty".into(),
            ))
        })
    }
}

impl super::settings::Database {
        /// Validates the database configuration by checking if the host, dbname, username, and password are not empty or default values. 
    /// If any of the values are empty or default, it returns an error with a specific message indicating which configuration value is invalid.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database host must not be empty".into(),
            ))
        })?;

        when(self.dbname.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database name must not be empty".into(),
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
        })
    }
}

impl super::settings::SupportedConnectors {
        /// Validates the configuration by checking if the list of connectors supporting wallets is not empty.
    /// If the list is empty, it returns an Err with an ApplicationError containing a message indicating the invalid configuration value.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.wallets.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "list of connectors supporting wallets must not be empty".into(),
            ))
        })
    }
}

#[cfg(feature = "kv_store")]
impl super::settings::DrainerSettings {
        /// Validates the configuration of the drainer stream name. 
    /// Returns a Result with Ok(()) if the stream name is not empty, 
    /// otherwise returns an Err with an ApplicationError indicating 
    /// an invalid configuration value error.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.stream_name.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "drainer stream name must not be empty".into(),
            ))
        })
    }
}

impl super::settings::ApiKeys {
        /// Validates the configuration values related to API key hashing key, and returns a Result
    /// indicating whether the validation was successful or an ApplicationError if the validation
    /// failed.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        #[cfg(feature = "kms")]
        return when(self.kms_encrypted_hash_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "API key hashing key must not be empty when KMS feature is enabled".into(),
            ))
        });

        #[cfg(not(feature = "kms"))]
        when(self.hash_key.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "API key hashing key must not be empty".into(),
            ))
        })
    }
}

impl super::settings::LockSettings {
        /// Validates the configuration values for redis_lock_expiry_seconds, delay_between_retries_in_milliseconds, and lock_retries.
    /// If any of the values are empty or 0, returns an ApplicationError indicating the invalid configuration value.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.redis_lock_expiry_seconds.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "redis_lock_expiry_seconds must not be empty or 0".into(),
            ))
        })?;

        when(
            self.delay_between_retries_in_milliseconds
                .is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "delay_between_retries_in_milliseconds must not be empty or 0".into(),
                ))
            },
        )?;

        when(self.lock_retries.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "lock_retries must not be empty or 0".into(),
            ))
        })
    }
}
