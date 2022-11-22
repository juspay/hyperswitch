use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{Mandate, MandateNew, MandateUpdate},
};

#[async_trait::async_trait]
pub trait IMandate {
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
    ) -> CustomResult<Mandate, errors::StorageError>;

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<Mandate>, errors::StorageError>;

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate: MandateUpdate,
    ) -> CustomResult<Mandate, errors::StorageError>;

    async fn insert_mandate(
        &self,
        mandate: MandateNew,
    ) -> CustomResult<Mandate, errors::StorageError>;
}

#[async_trait::async_trait]
impl IMandate for Store {
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
    ) -> CustomResult<Mandate, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Mandate::find_by_merchant_id_mandate_id(&conn, merchant_id, mandate_id).await
    }

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<Mandate>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Mandate::find_by_merchant_id_customer_id(&conn, merchant_id, customer_id).await
    }

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate: MandateUpdate,
    ) -> CustomResult<Mandate, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Mandate::update_by_merchant_id_mandate_id(&conn, merchant_id, mandate_id, mandate).await
    }

    async fn insert_mandate(
        &self,
        mandate: MandateNew,
    ) -> CustomResult<Mandate, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        mandate.insert(&conn).await
    }
}
