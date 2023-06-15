use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SchedulerSettings {
    pub stream: String,
    pub producer: ProducerSettings,
    pub consumer: ConsumerSettings,
    pub loop_interval: u64,
    pub graceful_shutdown_interval: u64,
}

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