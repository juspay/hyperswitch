use error_stack::Report;

use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult, DatabaseError, StorageError},
    types::storage::{Refund, RefundNew, RefundUpdate},
};

#[async_trait::async_trait]
pub trait RefundInterface {
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
impl RefundInterface for super::Store {
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        internal_reference_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Refund, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Refund::find_by_internal_reference_id_merchant_id(&conn, internal_reference_id, merchant_id)
            .await
    }

    async fn insert_refund(&self, new: RefundNew) -> CustomResult<Refund, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        new.insert(&conn).await
    }
    async fn find_refund_by_merchant_id_transaction_id(
        &self,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<Vec<Refund>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Refund::find_by_merchant_id_transaction_id(&conn, merchant_id, txn_id).await
    }

    async fn update_refund(
        &self,
        this: Refund,
        refund: RefundUpdate,
    ) -> CustomResult<Refund, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        this.update(&conn, refund).await
    }

    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &str,
        refund_id: &str,
    ) -> CustomResult<Refund, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Refund::find_by_merchant_id_refund_id(&conn, merchant_id, refund_id).await
    }

    // async fn find_refund_by_payment_id_merchant_id_refund_id(
    //     &self,
    //     payment_id: &str,
    //     merchant_id: &str,
    //     refund_id: &str,
    // ) -> CustomResult<Refund, errors::StorageError> {
    //     let conn = pg_connection(&self.master_pool.conn).await;
    //     Refund::find_by_payment_id_merchant_id_refund_id(&conn, payment_id, merchant_id, refund_id)
    //         .await
    // }

    async fn find_refund_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<Refund>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Refund::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id).await
    }
}

#[async_trait::async_trait]
impl RefundInterface for MockDb {
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        _internal_reference_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<Refund, errors::StorageError> {
        todo!()
    }

    async fn insert_refund(&self, new: RefundNew) -> CustomResult<Refund, errors::StorageError> {
        let mut refunds = self.refunds.lock().await;
        let current_time = common_utils::date_time::now();

        let refund = Refund {
            id: refunds.len() as i32,
            internal_reference_id: new.internal_reference_id,
            refund_id: new.refund_id,
            payment_id: new.payment_id,
            merchant_id: new.merchant_id,
            transaction_id: new.transaction_id,
            connector: new.connector,
            pg_refund_id: new.pg_refund_id,
            external_reference_id: new.external_reference_id,
            refund_type: new.refund_type,
            total_amount: new.total_amount,
            currency: new.currency,
            refund_amount: new.refund_amount,
            refund_status: new.refund_status,
            sent_to_gateway: new.sent_to_gateway,
            refund_error_message: new.refund_error_message,
            metadata: new.metadata,
            refund_arn: new.refund_arn,
            created_at: new.created_at.unwrap_or(current_time),
            updated_at: current_time,
            description: new.description,
        };
        refunds.push(refund.clone());
        Ok(refund)
    }
    async fn find_refund_by_merchant_id_transaction_id(
        &self,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<Vec<Refund>, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        Ok(refunds
            .iter()
            .take_while(|refund| {
                refund.merchant_id == merchant_id && refund.transaction_id == txn_id
            })
            .cloned()
            .collect::<Vec<_>>())
    }

    async fn update_refund(
        &self,
        _this: Refund,
        _refund: RefundUpdate,
    ) -> CustomResult<Refund, errors::StorageError> {
        todo!()
    }

    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &str,
        refund_id: &str,
    ) -> CustomResult<Refund, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        refunds
            .iter()
            .find(|refund| refund.merchant_id == merchant_id && refund.refund_id == refund_id)
            .cloned()
            .ok_or_else(|| Report::from(StorageError::DatabaseError(DatabaseError::NotFound)))
    }

    async fn find_refund_by_payment_id_merchant_id(
        &self,
        _payment_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<Vec<Refund>, errors::StorageError> {
        todo!()
    }
}
