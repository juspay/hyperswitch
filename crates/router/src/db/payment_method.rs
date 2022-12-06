use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait PaymentMethodInterface {
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError>;

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError>;

    async fn insert_payment_method(
        &self,
        m: storage::PaymentMethodNew,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError>;

    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError>;
}

#[async_trait::async_trait]
impl PaymentMethodInterface for super::Store {
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let conn = crate::connection::pg_connection(&self.master_pool).await;
        storage::PaymentMethod::find_by_payment_method_id(&conn, payment_method_id).await
    }

    async fn insert_payment_method(
        &self,
        m: storage::PaymentMethodNew,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let conn = crate::connection::pg_connection(&self.master_pool).await;
        m.insert(&conn).await
    }

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError> {
        let conn = crate::connection::pg_connection(&self.master_pool).await;
        storage::PaymentMethod::find_by_customer_id_merchant_id(&conn, customer_id, merchant_id)
            .await
    }

    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let conn = crate::connection::pg_connection(&self.master_pool).await;
        storage::PaymentMethod::delete_by_merchant_id_payment_method_id(
            &conn,
            merchant_id,
            payment_method_id,
        )
        .await
    }
}

#[async_trait::async_trait]
impl PaymentMethodInterface for MockDb {
    async fn find_payment_method(
        &self,
        _payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        todo!()
    }

    async fn insert_payment_method(
        &self,
        _m: storage::PaymentMethodNew,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        todo!()
    }

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError> {
        todo!()
    }

    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        _merchant_id: &str,
        _payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        todo!()
    }
}
