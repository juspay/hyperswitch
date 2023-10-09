use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait PaymentLinkInterface {
    async fn find_payment_link_by_payment_link_id(
        &self,
        payment_link_id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError>;

    async fn insert_payment_link(
        &self,
        _payment_link: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError>;
}

#[async_trait::async_trait]
impl PaymentLinkInterface for Store {
    async fn find_payment_link_by_payment_link_id(
        &self,
        payment_link_id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentLink::find_by_link_payment_id_merchant_id(
            &conn,
            payment_link_id,
            merchant_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn insert_payment_link(
        &self,
        payment_link_object: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payment_link_object
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl PaymentLinkInterface for MockDb {
    async fn insert_payment_link(
        &self,
        _payment_link: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_payment_link_by_payment_link_id(
        &self,
        _payment_link_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`x
        Err(errors::StorageError::MockDbError)?
    }
}
