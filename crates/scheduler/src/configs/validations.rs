use common_utils::ext_traits::ConfigExt;
use storage_impl::errors::ApplicationError;

impl super::settings::SchedulerSettings {
        /// Validates the configuration values for the scheduler.
    /// Returns a Result indicating success or an ApplicationError if the configuration values are invalid.
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

        self.server.validate()?;

        Ok(())
    }
}

impl super::settings::ProducerSettings {
        /// This method is used to validate the lock key of a producer. It checks if the lock key is empty, and if so, it returns an `ApplicationError` with a message indicating that the producer lock key must not be empty. If the lock key is not empty, it returns `Ok(())`.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.lock_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "producer lock key must not be empty".into(),
            ))
        })
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
