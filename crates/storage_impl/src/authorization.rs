use common_utils::errors::CustomResult;
use diesel_models::authorization as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::authorization::AuthorizationInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> AuthorizationInterface for RouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_authorization(
        &self,
        authorization: storage::AuthorizationNew,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        authorization
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_all_authorizations_by_merchant_id_payment_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
    ) -> CustomResult<Vec<storage::Authorization>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Authorization::find_by_merchant_id_payment_id(&conn, merchant_id, payment_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_authorization_by_merchant_id_authorization_id(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
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
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
