//! Redis interface — compile-time backend selection via Cargo feature.
//!
//! By default the `fred` crate is used. Enable `redis-rs` to switch.
//!
//! # Examples
//! ```
//! use redis_interface::{types::RedisSettings, RedisConnectionPool};
//!
//! #[tokio::main]
//! async fn main() {
//!     let redis_conn = RedisConnectionPool::new(&RedisSettings::default()).await;
//! }
//! ```

pub mod constant;
pub mod errors;
pub mod types;

#[cfg(not(feature = "redis-rs"))]
mod backends {
    pub mod fred;
}

#[cfg(feature = "redis-rs")]
mod backends {
    pub mod redis_rs;
}

// Re-export the active backend's public types under unified names.
// All external code imports `redis_interface::RedisConnectionPool` etc.
// and is never aware of which backend is active.

#[cfg(not(feature = "redis-rs"))]
pub use backends::fred::{
    PubSubMessage, RedisClient, RedisConfig, RedisConnectionPool, SubscriberClient,
};
#[cfg(feature = "redis-rs")]
pub use backends::redis_rs::{
    PubSubMessage, PublisherClient, RedisConfig, RedisConn, RedisConnectionPool, SubscriberClient,
};

pub use self::types::*;

#[cfg(test)]
mod test;
