use common_utils::ext_traits::ConfigExt;
use masking::PeekInterface;
use storage_impl::errors::ApplicationError;

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
        })?;

        when(self.master_enc_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Master encryption key must not be empty".into(),
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

impl super::settings::Server {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "server host must not be empty".into(),
            ))
        })?;

        when(self.workers == 0, || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "number of workers must be greater than 0".into(),
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
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.wallets.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "list of connectors supporting wallets must not be empty".into(),
            ))
        })
    }
}

impl super::settings::CorsSettings {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.wildcard_origin && !self.origins.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Allowed Origins must be empty when wildcard origin is true".to_string(),
            ))
        })?;

        common_utils::fp_utils::when(!self.wildcard_origin && self.origins.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Allowed origins must not be empty. Please either enable wildcard origin or provide Allowed Origin".to_string(),
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

impl super::settings::ApiKeys {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.hash_key.peek().is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "API key hashing key must not be empty".into(),
            ))
        })?;

        #[cfg(feature = "email")]
        when(self.expiry_reminder_days.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "API key expiry reminder days must not be empty".into(),
            ))
        })?;

        Ok(())
    }
}

impl super::settings::LockSettings {
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

impl super::settings::WebhooksSettings {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.redis_lock_expiry_seconds.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "redis_lock_expiry_seconds must not be empty or 0".into(),
            ))
        })
    }
}

impl super::settings::GenericLinkEnvConfig {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.expiry == 0, || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "link's expiry should not be 0".into(),
            ))
        })
    }
}

#[cfg(feature = "v2")]
impl super::settings::CellInformation {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::{fp_utils::when, id_type};

        when(self == &Self::default(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "CellId cannot be set to a default".into(),
            ))
        })
    }
}

impl super::settings::NetworkTokenizationService {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.token_service_api_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "token_service_api_key must not be empty".into(),
            ))
        })?;

        when(self.public_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "public_key must not be empty".into(),
            ))
        })?;

        when(self.key_id.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "key_id must not be empty".into(),
            ))
        })?;

        when(self.private_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "private_key must not be empty".into(),
            ))
        })?;

        when(
            self.webhook_source_verification_key.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "webhook_source_verification_key must not be empty".into(),
                ))
            },
        )
    }
}

impl super::settings::PazeDecryptConfig {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.paze_private_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "paze_private_key must not be empty".into(),
            ))
        })?;

        when(
            self.paze_private_key_passphrase.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "paze_private_key_passphrase must not be empty".into(),
                ))
            },
        )
    }
}

impl super::settings::GooglePayDecryptConfig {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(
            self.google_pay_root_signing_keys.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "google_pay_root_signing_keys must not be empty".into(),
                ))
            },
        )
    }
}

impl super::settings::KeyManagerConfig {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        #[cfg(feature = "keymanager_mtls")]
        when(
            self.enabled && (self.ca.is_default_or_empty() || self.cert.is_default_or_empty()),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "Invalid CA or Certificate for Keymanager.".into(),
                ))
            },
        )?;

        when(self.enabled && self.url.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "Invalid URL for Keymanager".into(),
            ))
        })
    }
}

impl super::settings::Platform {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(!self.enabled && self.allow_connected_merchants, || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "platform.allow_connected_merchants cannot be true when platform.enabled is false"
                    .into(),
            ))
        })
    }
}

impl super::settings::OpenRouter {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(
            (self.dynamic_routing_enabled || self.static_routing_enabled)
                && self.url.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "OpenRouter base URL must not be empty when it is enabled".into(),
                ))
            },
        )
    }
}

impl super::settings::ChatSettings {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.enabled && self.hyperswitch_ai_host.is_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "hyperswitch ai host must be set if chat is enabled".into(),
            ))
        })
    }
}
