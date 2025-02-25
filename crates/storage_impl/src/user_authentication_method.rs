
use common_utils::errors::CustomResult;
use diesel_models::user_authentication_method as storage;
use router_env::{instrument, tracing};
use error_stack::report;
use sample::user_authentication_method::UserAuthenticationMethodInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> UserAuthenticationMethodInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_user_authentication_method(
        &self,
        user_authentication_method: storage::UserAuthenticationMethodNew,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        user_authentication_method
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn get_user_authentication_method_by_id(
        &self,
        id: &str,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserAuthenticationMethod::get_user_authentication_method_by_id(&conn, id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_user_authentication_methods_for_auth_id(
        &self,
        auth_id: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserAuthenticationMethod::list_user_authentication_methods_for_auth_id(
            &conn, auth_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_user_authentication_methods_for_owner_id(
        &self,
        owner_id: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserAuthenticationMethod::list_user_authentication_methods_for_owner_id(
            &conn, owner_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_user_authentication_method(
        &self,
        id: &str,
        user_authentication_method_update: storage::UserAuthenticationMethodUpdate,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserAuthenticationMethod::update_user_authentication_method(
            &conn,
            id,
            user_authentication_method_update,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_user_authentication_methods_for_email_domain(
        &self,
        email_domain: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserAuthenticationMethod::list_user_authentication_methods_for_email_domain(
            &conn,
            email_domain,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}