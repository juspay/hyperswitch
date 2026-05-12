//! Redis interface — compile-time backend selection via Cargo feature.
//!
//! By default the `redis-rs` crate is used. Enable the `fred` feature to switch.
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

// Compile-time guards: exactly one backend must be active.
#[cfg(not(any(feature = "fred", feature = "redis-rs")))]
compile_error!("Either feature \"fred\" or \"redis-rs\" must be enabled for this crate.");

#[cfg(all(feature = "fred", feature = "redis-rs"))]
compile_error!("Features \"fred\" and \"redis-rs\" are mutually exclusive — enable only one.");

pub mod constant;
pub mod errors;
pub mod types;

#[cfg(feature = "fred")]
mod module {
    pub mod fred;
}

#[cfg(feature = "redis-rs")]
mod module {
    pub mod redis_rs;
}

// Re-export the active backend's public types under unified names.
// All external code imports `redis_interface::RedisConnectionPool` etc.
// and is never aware of which backend is active.

#[cfg(feature = "fred")]
pub use fred::interfaces::{EventInterface, PubsubInterface};
#[cfg(feature = "fred")]
pub use module::fred::{
    PubSubMessage, RedisClient, RedisConfig, RedisConnectionPool, SubscriberClient,
};
#[cfg(feature = "redis-rs")]
pub use module::redis_rs::{
    redis_value_to_option_string, PubSubMessage, PublisherClient, RedisConfig, RedisConn,
    RedisConnectionPool, SubscriberClient,
};

pub use self::types::*;

#[cfg(test)]
mod test;
