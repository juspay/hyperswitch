// TODO: Add crate & modules documentation for this crate

pub mod commands;
pub mod errors;
pub mod types;

use router_env::logger;

pub use self::{commands::*, types::*};

pub struct RedisConnectionPool {
    pub pool: fred::pool::RedisPool,
    config: RedisConfig,
    _join_handles: Vec<fred::types::ConnectHandle>,
}

impl RedisConnectionPool {
    /// Create a new Redis connection
    ///
    /// # Panics
    ///
    /// Panics if a connection to Redis is not successful.
    #[allow(clippy::expect_used)]
    pub async fn new(conf: &types::RedisSettings) -> Self {
        let redis_connection_url = match conf.cluster_enabled {
            // Fred relies on this format for specifying cluster where the host port is ignored & only query parameters are used for node addresses
            // redis-cluster://username:password@host:port?node=bar.com:30002&node=baz.com:30003
            true => format!(
                "redis-cluster://{}:{}?{}",
                conf.host,
                conf.port,
                conf.cluster_urls
                    .iter()
                    .flat_map(|url| vec!["&", url])
                    .skip(1)
                    .collect::<String>()
            ),
            false => format!(
                "redis://{}:{}", //URI Schema
                conf.host, conf.port,
            ),
        };
        let mut config = fred::types::RedisConfig::from_url(&redis_connection_url)
            .expect("Invalid Redis connection URL");
        if !conf.use_legacy_version {
            config.version = fred::types::RespVersion::RESP3;
        }
        config.tracing = true;
        let policy = fred::types::ReconnectPolicy::new_constant(
            conf.reconnect_max_attempts,
            conf.reconnect_delay,
        );
        let pool = fred::pool::RedisPool::new(config, conf.pool_size)
            .expect("Unable to construct Redis pool");

        let _join_handles = pool.connect(Some(policy));
        pool.wait_for_connect()
            .await
            .expect("Error connecting to Redis");
        let config = RedisConfig::from(conf);

        Self {
            pool,
            config,
            _join_handles,
        }
    }

    pub async fn close_connections(&mut self) {
        self.pool.quit_pool().await;
        for handle in self._join_handles.drain(..) {
            match handle.await {
                Ok(Ok(_)) => (),
                Ok(Err(error)) => logger::error!(%error),
                Err(error) => logger::error!(%error),
            };
        }
    }
}

struct RedisConfig {
    default_ttl: u32,
    default_stream_read_count: u64,
}

impl From<&types::RedisSettings> for RedisConfig {
    fn from(config: &types::RedisSettings) -> Self {
        Self {
            default_ttl: config.default_ttl,
            default_stream_read_count: config.stream_read_count,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_redis_error() {
        let x = errors::RedisError::ConsumerGroupClaimFailed.to_string();

        assert_eq!(x, "Failed to set Redis stream message owner".to_string())
    }
}
