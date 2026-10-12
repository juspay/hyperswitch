use redis_interface::errors::RedisError;

use super::MockDb;
use crate::redis::{
    cache::{CacheInterface, Caches},
    kv_store::RedisConnInterface,
};

impl RedisConnInterface for MockDb {
    fn get_redis_conn(
        &self,
    ) -> Result<redis_interface::RedisConnectionWithContext, error_stack::Report<RedisError>> {
        let pool = self.redis.get_redis_pool()?;

        Ok(redis_interface::RedisConnectionWithContext::new_without_context(pool))
    }
}

impl CacheInterface for MockDb {
    fn caches(&self) -> &Caches {
        &self.caches
    }

    fn cache_key_prefix(&self) -> &str {
        self.redis.key_prefix()
    }
}
