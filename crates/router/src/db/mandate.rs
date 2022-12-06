use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{Mandate, MandateNew, MandateUpdate},
};

#[async_trait::async_trait]
pub trait MandateInterface {
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
impl MandateInterface for super::Store {
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
    ) -> CustomResult<Mandate, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        Mandate::find_by_merchant_id_mandate_id(&conn, merchant_id, mandate_id).await
    }

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<Mandate>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        Mandate::find_by_merchant_id_customer_id(&conn, merchant_id, customer_id).await
    }

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate: MandateUpdate,
    ) -> CustomResult<Mandate, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        Mandate::update_by_merchant_id_mandate_id(&conn, merchant_id, mandate_id, mandate).await
    }

    async fn insert_mandate(
        &self,
        mandate: MandateNew,
    ) -> CustomResult<Mandate, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        mandate.insert(&conn).await
    }
}

#[async_trait::async_trait]
impl MandateInterface for MockDb {
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        _merchant_id: &str,
        _mandate_id: &str,
    ) -> CustomResult<Mandate, errors::StorageError> {
        todo!()
    }

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        _merchant_id: &str,
        _customer_id: &str,
    ) -> CustomResult<Vec<Mandate>, errors::StorageError> {
        todo!()
    }

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        _merchant_id: &str,
        _mandate_id: &str,
        _mandate: MandateUpdate,
    ) -> CustomResult<Mandate, errors::StorageError> {
        todo!()
    }

    async fn insert_mandate(
        &self,
        _mandate: MandateNew,
    ) -> CustomResult<Mandate, errors::StorageError> {
        todo!()
    }
}
