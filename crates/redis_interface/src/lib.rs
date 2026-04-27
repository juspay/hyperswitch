//! Redis interface — compile-time backend selection via Cargo features.
//! Enable exactly one of: `redis-rs` (default) or `fred-rs`.
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

pub mod errors;
pub mod types;
pub mod constant;

#[cfg(all(feature = "redis-rs", not(feature = "fred-rs")))]
mod backends {
    pub mod redis_rs;
}

#[cfg(all(feature = "fred-rs", not(feature = "redis-rs")))]
mod backends {
    pub mod fred;
}

// Re-export the active backend's public types under unified names.
// All external code imports `redis_interface::RedisConnectionPool` etc.
// and is never aware of which backend is active.

#[cfg(feature = "redis-rs")]
pub use backends::redis_rs::{
    PubSubMessage, PublisherClient, RedisConfig, RedisConn, RedisConnectionPool,
    SubscriberClient,
};

#[cfg(feature = "fred-rs")]
pub use backends::fred::{
    PubSubMessage, RedisClient, RedisConfig, RedisConnectionPool, SubscriberClient,
};

pub use self::types::*;
