use error_stack::IntoReport;
use router_env::{instrument, tracing};

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait PayoutsInterface {
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError>;

    async fn update_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _payout: storage::PayoutsUpdate,
    ) -> CustomResult<storage::Payouts, errors::StorageError>;

    async fn insert_payout(
        &self,
        _payout: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError>;
}

#[async_trait::async_trait]
impl PayoutsInterface for Store {
    #[instrument(skip_all)]
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Payouts::find_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn update_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
        payout: storage::PayoutsUpdate,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Payouts::update_by_merchant_id_payout_id(&conn, merchant_id, payout_id, payout)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn insert_payout(
        &self,
        payout: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payout.insert(&conn).await.map_err(Into::into).into_report()
    }
}

#[async_trait::async_trait]
impl PayoutsInterface for MockDb {
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _payout: storage::PayoutsUpdate,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_payout(
        &self,
        _payout: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
