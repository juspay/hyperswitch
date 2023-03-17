use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait DisputeInterface {
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;

    async fn find_by_payment_id_connector_dispute_id(
        &self,
        payment_id: &str,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError>;

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;
}

#[async_trait::async_trait]
impl DisputeInterface for Store {
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        dispute
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_by_payment_id_connector_dispute_id(
        &self,
        payment_id: &str,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::Dispute::find_by_payment_id_connector_dispute_id(
            &conn,
            payment_id,
            connector_dispute_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        this.update(&conn, dispute)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl DisputeInterface for MockDb {
    async fn insert_dispute(
        &self,
        _dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
    async fn find_by_payment_id_connector_dispute_id(
        &self,
        _payment_id: &str,
        _connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_dispute(
        &self,
        _this: storage::Dispute,
        _dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
