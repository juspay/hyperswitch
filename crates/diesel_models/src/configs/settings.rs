#[cfg(feature = "kms")]
use external_services::kms;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use serde::Deserialize;
#[cfg(feature = "kms")]
pub type Password = kms::KmsValue;
#[cfg(not(feature = "kms"))]
pub type Password = masking::Secret<String>;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchedulerSettings {
    pub stream: String,
    pub producer: ProducerSettings,
    pub consumer: ConsumerSettings,
    pub loop_interval: u64,
    pub graceful_shutdown_interval: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProducerSettings {
    pub upper_fetch_limit: i64,
    pub lower_fetch_limit: i64,

    pub lock_key: String,
    pub lock_ttl: i64,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ConsumerSettings {
    pub disabled: bool,
    pub consumer_group: String,
}
