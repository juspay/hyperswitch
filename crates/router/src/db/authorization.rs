use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait AuthorizationInterface {
    async fn insert_authorization(
        &self,
        authorization: storage::AuthorizationNew,
    ) -> CustomResult<storage::Authorization, errors::StorageError>;

    async fn find_all_authorizations_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Authorization>, errors::StorageError>;

    async fn update_authorization_by_merchant_id_authorization_id(
        &self,
        merchant_id: String,
        authorization_id: String,
        authorization: storage::AuthorizationUpdate,
    ) -> CustomResult<storage::Authorization, errors::StorageError>;
}

#[async_trait::async_trait]
impl AuthorizationInterface for Store {
    async fn insert_authorization(
        &self,
        authorization: storage::AuthorizationNew,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        authorization
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_all_authorizations_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Authorization>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Authorization::find_by_merchant_id_payment_id(&conn, merchant_id, payment_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_authorization_by_merchant_id_authorization_id(
        &self,
        merchant_id: String,
        authorization_id: String,
        authorization: storage::AuthorizationUpdate,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Authorization::update_by_merchant_id_authorization_id(
            &conn,
            merchant_id,
            authorization_id,
            authorization,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }
}

#[async_trait::async_trait]
impl AuthorizationInterface for MockDb {
    async fn insert_authorization(
        &self,
        _authorization: storage::AuthorizationNew,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_all_authorizations_by_merchant_id_payment_id(
        &self,
        _merchant_id: &str,
        _payment_id: &str,
    ) -> CustomResult<Vec<storage::Authorization>, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_authorization_by_merchant_id_authorization_id(
        &self,
        _merchant_id: String,
        _authorization_id: String,
        _authorization: storage::AuthorizationUpdate,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
