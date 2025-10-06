use hyperswitch_interfaces::types::RedisInterface;
use redis_interface::{errors::RedisError, RedisConnectionPool};

pub struct RedisAdapter<'a> {
    pub redis_conn: &'a RedisConnectionPool,
}

impl<'a> RedisAdapter<'a> {
    pub fn new(redis_conn: &'a RedisConnectionPool) -> Self {
        Self { redis_conn }
    }
}

impl<'a> RedisInterface for RedisAdapter<'a> {
    async fn incr(&self, key: &str) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        self.redis_conn
            .incr(key, 1u64)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get(&self, key: &str) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
        let result: Result<u64, RedisError> = self.redis_conn.get(key).await;
        match result {
            Ok(value) => Ok(Some(value)),
            Err(RedisError::NotFound) => Ok(None),
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }
}