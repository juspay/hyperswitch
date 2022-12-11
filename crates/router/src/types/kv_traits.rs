use async_trait::async_trait;
use common_utils::errors::CustomResult;
use diesel::pg::Pg;

use crate::connection::PgPooledConn;

use super::storage::{PaymentIntent, PaymentIntentNew};

#[async_trait]
pub trait KVInsertable<Quer> {
    async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<Quer, crate::core::errors::StorageError>
    where
        Quer: diesel::Queryable<Quer, Pg>,
        Self: std::marker::Sized;
}

#[async_trait]
impl KVInsertable<PaymentIntent> for PaymentIntentNew {
    async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<PaymentIntent, crate::core::errors::StorageError> {
        self.insert(conn).await
    }
}
