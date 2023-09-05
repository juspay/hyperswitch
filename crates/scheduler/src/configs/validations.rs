use common_utils::ext_traits::ConfigExt;
use diesel_models::errors::ApplicationError;

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
