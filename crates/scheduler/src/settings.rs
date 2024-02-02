use common_utils::ext_traits::ConfigExt;
use serde::Deserialize;
use storage_impl::errors::ApplicationError;

pub use crate::configs::settings::SchedulerSettings;

#[derive(Debug, Clone, Deserialize)]
pub struct ProducerSettings {
    pub upper_fetch_limit: i64,
    pub lower_fetch_limit: i64,

    pub lock_key: String,
    pub lock_ttl: i64,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConsumerSettings {
    pub disabled: bool,
    pub consumer_group: String,
}

#[cfg(feature = "kv_store")]
#[derive(Debug, Clone, Deserialize)]
pub struct DrainerSettings {
    pub stream_name: String,
    pub num_partitions: u8,
    pub max_read_count: u64,
    pub shutdown_interval: u32, // in milliseconds
    pub loop_interval: u32,     // in milliseconds
}

impl ProducerSettings {
        /// Validates the current state of the object and returns a Result.
    /// If the lock key is empty or has a default value, an ApplicationError
    /// with a message indicating the invalid configuration value is returned.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.lock_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "producer lock key must not be empty".into(),
            ))
        })
    }
}

#[cfg(feature = "kv_store")]
impl DrainerSettings {
        /// This method is used to validate the stream name. It checks if the stream name is default or empty, and if so, it returns an error of type ApplicationError with a message indicating that the drainer stream name must not be empty.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        common_utils::fp_utils::when(self.stream_name.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "drainer stream name must not be empty".into(),
            ))
        })
    }
}

impl Default for ProducerSettings {
        /// Returns a new instance of Self with default values for upper_fetch_limit, lower_fetch_limit,
    /// lock_key, lock_ttl, and batch_size.
    fn default() -> Self {
        Self {
            upper_fetch_limit: 0,
            lower_fetch_limit: 1800,
            lock_key: "PRODUCER_LOCKING_KEY".into(),
            lock_ttl: 160,
            batch_size: 200,
        }
    }
}

impl Default for ConsumerSettings {
        /// Returns a new instance of the struct with default values.
    fn default() -> Self {
        Self {
            disabled: false,
            consumer_group: "SCHEDULER_GROUP".into(),
        }
    }
}

#[cfg(feature = "kv_store")]
impl Default for DrainerSettings {
        /// Creates a new instance of the struct with default values for the stream name, number of partitions, maximum read count, shutdown interval, and loop interval.
    fn default() -> Self {
        Self {
            stream_name: "DRAINER_STREAM".into(),
            num_partitions: 64,
            max_read_count: 100,
            shutdown_interval: 1000,
            loop_interval: 500,
        }
    }
}
