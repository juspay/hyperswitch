use error_stack::IntoReport;
use router_env::{instrument, tracing};

use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    services::Store,
    types::storage::{self, PaymentLinkDbExt},
};

#[async_trait::async_trait]
pub trait PaymentLinkInterface {
    async fn find_payment_link_by_payment_link_id(
        &self,
        payment_link_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError>;

    async fn insert_payment_link(
        &self,
        _payment_link: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError>;

    async fn list_payment_link_by_merchant_id(
        &self,
        merchant_id: &str,
        payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
    ) -> CustomResult<Vec<storage::PaymentLink>, errors::StorageError>;
}

#[async_trait::async_trait]
impl PaymentLinkInterface for Store {
    #[instrument(skip_all)]
    async fn find_payment_link_by_payment_link_id(
        &self,
        payment_link_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentLink::find_link_by_payment_link_id(&conn, payment_link_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn insert_payment_link(
        &self,
        payment_link_config: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payment_link_config
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn list_payment_link_by_merchant_id(
        &self,
        merchant_id: &str,
        payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
    ) -> CustomResult<Vec<storage::PaymentLink>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentLink::filter_by_constraints(&conn, merchant_id, payment_link_constraints)
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
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`x
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_payment_link_by_merchant_id(
        &self,
        _merchant_id: &str,
        _payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
    ) -> CustomResult<Vec<storage::PaymentLink>, errors::StorageError> {
        // TODO: Implement function for `MockDb`x
        Err(errors::StorageError::MockDbError)?
    }
}
