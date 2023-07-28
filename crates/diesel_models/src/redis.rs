
pub use redis_interface::*;

// #TODO: These structs can be copied over to redis_interface or 
// redis_interface can be merged into diesel_models (which would be renamed to storage_impls)
// #[derive(Debug, serde::Deserialize, Clone)]
// #[serde(default)]
// pub struct RedisSettings {
//     /// Redis mode of operation standalone or cluster
//     #[serde(flatten)]
//     #[serde(alias = "cluster_enabled")]
//     pub mode: RedisMode,
//     pub use_legacy_version: bool,
//     pub pool_size: usize,
//     pub reconnect_max_attempts: u32,
//     /// Reconnect delay in milliseconds
//     pub reconnect_delay: u32,
//     /// TTL in seconds
//     pub default_ttl: u32,
//     /// TTL for hash-tables in seconds
//     pub default_hash_ttl: u32,
//     /// Batch size to read from stream
//     pub stream_read_count: u64,
// }

// #[serde(tag = "source")]
// #[serde(rename_all = "lowercase")]
// pub enum RedisMode {
//     #[serde(alias = "true")]
//     Cluster { 
//         // Redis cluster URL's
//         cluster_urls: Vec<String> 
//     },
//     #[serde(alias = "false")]
//     Standalone { 
//         /// Redis host
//         host: String,
//         /// Redis port
//         port: u16 
//     },
// }


// pub enum FRMCOnn {

// }

// pub enum PayConnec {

// }

// pub enum PayoutConnec {

// }


// pub enum AllConnectors {
//     FRM(FRMConn),
//     Payment(PayConnec),
//     Payout(PayoutConnec),
// }