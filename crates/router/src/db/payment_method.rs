use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{PaymentMethod, PaymentMethodNew},
};

#[async_trait::async_trait]
pub trait IPaymentMethod {
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<PaymentMethod>, errors::StorageError>;

    async fn insert_payment_method(
        &self,
        m: PaymentMethodNew,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;

    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;
}

#[async_trait::async_trait]
impl IPaymentMethod for Store {
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
    ) -> CustomResult<PaymentMethod, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        PaymentMethod::find_by_payment_method_id(&conn, payment_method_id).await
    }

    async fn insert_payment_method(
        &self,
        m: PaymentMethodNew,
    ) -> CustomResult<PaymentMethod, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        m.insert(&conn).await
    }

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<PaymentMethod>, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        PaymentMethod::find_by_customer_id_merchant_id(&conn, customer_id, merchant_id).await
    }

    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<PaymentMethod, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        PaymentMethod::delete_by_merchant_id_payment_method_id(
            &conn,
            merchant_id,
            payment_method_id,
        )
        .await
    }
}
