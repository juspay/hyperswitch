pub mod api;
pub mod encryption;
pub mod logger;
pub mod redis;

use std::sync::Arc;

pub use self::{api::*, encryption::*};

#[derive(Clone)]
pub struct Store {
    pub pg_pool: crate::db::SqlDb,
    pub redis_conn: Arc<crate::services::redis::RedisConnectionPool>,
}
