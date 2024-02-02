use std::sync::Arc;

use redis_interface::errors::RedisError;

use super::MockDb;
use crate::redis::kv_store::RedisConnInterface;

impl RedisConnInterface for MockDb {
        /// This method returns a result containing an Arc of RedisConnectionPool or a RedisError report. It retrieves a Redis connection from the internal Redis instance.
    fn get_redis_conn(
        &self,
    ) -> Result<Arc<redis_interface::RedisConnectionPool>, error_stack::Report<RedisError>> {
        self.redis.get_redis_conn()
    }
}
