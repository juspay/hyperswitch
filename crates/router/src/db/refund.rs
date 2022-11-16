use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{Refund, RefundNew, RefundUpdate},
};

#[async_trait::async_trait]
pub trait IRefund {
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        internal_reference_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Refund, errors::StorageError>;

    async fn find_refund_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<Refund>, errors::StorageError>;

    // async fn find_refund_by_payment_id_merchant_id_refund_id(
    //     &self,
    //     payment_id: &str,
    //     merchant_id: &str,
    //     refund_id: &str,
    // ) -> CustomResult<Refund, errors::StorageError>;

    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &str,
        refund_id: &str,
    ) -> CustomResult<Refund, errors::StorageError>;

    async fn update_refund(
        &self,
        this: Refund,
        refund: RefundUpdate,
    ) -> CustomResult<Refund, errors::StorageError>;

    async fn find_refund_by_merchant_id_transaction_id(
        &self,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<Vec<Refund>, errors::StorageError>;

    async fn insert_refund(&self, new: RefundNew) -> CustomResult<Refund, errors::StorageError>;
}

#[async_trait::async_trait]
impl IRefund for Store {
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        internal_reference_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Refund, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        Refund::find_by_internal_reference_id_merchant_id(&conn, internal_reference_id, merchant_id)
            .await
    }

    async fn insert_refund(&self, new: RefundNew) -> CustomResult<Refund, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        new.insert(&conn).await
    }
    async fn find_refund_by_merchant_id_transaction_id(
        &self,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<Vec<Refund>, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        Refund::find_by_merchant_id_transaction_id(&conn, merchant_id, txn_id).await
    }

    async fn update_refund(
        &self,
        this: Refund,
        refund: RefundUpdate,
    ) -> CustomResult<Refund, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        this.update(&conn, refund).await
    }

    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &str,
        refund_id: &str,
    ) -> CustomResult<Refund, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        Refund::find_by_merchant_id_refund_id(&conn, merchant_id, refund_id).await
    }

    // async fn find_refund_by_payment_id_merchant_id_refund_id(
    //     &self,
    //     payment_id: &str,
    //     merchant_id: &str,
    //     refund_id: &str,
    // ) -> CustomResult<Refund, errors::StorageError> {
    //     let conn = pg_connection(&self.pg_pool.conn).await;
    //     Refund::find_by_payment_id_merchant_id_refund_id(&conn, payment_id, merchant_id, refund_id)
    //         .await
    // }

    async fn find_refund_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<Refund>, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        Refund::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id).await
    }
}
