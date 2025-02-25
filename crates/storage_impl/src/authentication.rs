use common_utils::errors::CustomResult;
use diesel_models::authentication as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::authentication::AuthenticationInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> AuthenticationInterface for RouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_authentication(
        &self,
        authentication: storage::AuthenticationNew,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        authentication
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: String,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Authentication::find_by_merchant_id_authentication_id(
            &conn,
            merchant_id,
            &authentication_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_authentication_by_merchant_id_connector_authentication_id(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
        connector_authentication_id: String,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Authentication::find_authentication_by_merchant_id_connector_authentication_id(
            &conn,
            &merchant_id,
            &connector_authentication_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: storage::Authentication,
        authentication_update: storage::AuthenticationUpdate,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Authentication::update_by_merchant_id_authentication_id(
            &conn,
            previous_state.merchant_id,
            previous_state.authentication_id,
            authentication_update,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
