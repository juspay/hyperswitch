use diesel_models::user_authentication_method::{self as storage};
use error_stack::report;
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

#[async_trait::async_trait]
pub trait UserAuthenticationMethodInterface {
    async fn insert_user_authentication_method(
        &self,
        user_authentication_method: storage::UserAuthenticationMethodNew,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError>;

    async fn list_user_authentication_methods_for_auth_id(
        &self,
        auth_id: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError>;

    async fn list_user_authentication_methods_for_owner_id(
        &self,
        owner_id: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError>;

    async fn update_user_authentication_method(
        &self,
        id: &str,
        user_authentication_method_update: storage::UserAuthenticationMethodUpdate,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError>;
}

#[async_trait::async_trait]
impl UserAuthenticationMethodInterface for Store {
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
    async fn list_user_authentication_methods_for_auth_id(
        &self,
        auth_id: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
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
        let conn = connection::pg_connection_write(self).await?;
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
}

#[async_trait::async_trait]
impl UserAuthenticationMethodInterface for MockDb {
    async fn insert_user_authentication_method(
        &self,
        user_authentication_method: storage::UserAuthenticationMethodNew,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        let mut user_authentication_methods = self.user_authentication_methods.lock().await;
        let user_authentication_method = storage::UserAuthenticationMethod {
            id: uuid::Uuid::new_v4().to_string(),
            auth_id: uuid::Uuid::new_v4().to_string(),
            owner_id: user_authentication_method.auth_id,
            owner_type: user_authentication_method.owner_type,
            auth_method: user_authentication_method.auth_method,
            config: user_authentication_method.config,
            allow_signup: user_authentication_method.allow_signup,
            created_at: user_authentication_method.created_at,
            last_modified_at: user_authentication_method.last_modified_at,
        };

        user_authentication_methods.push(user_authentication_method.clone());
        Ok(user_authentication_method)
    }

    async fn list_user_authentication_methods_for_auth_id(
        &self,
        auth_id: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError> {
        let user_authentication_methods = self.user_authentication_methods.lock().await;

        let user_authentication_methods_list: Vec<_> = user_authentication_methods
            .iter()
            .filter(|auth_method_inner| auth_method_inner.auth_id == auth_id)
            .cloned()
            .collect();
        if user_authentication_methods_list.is_empty() {
            return Err(errors::StorageError::ValueNotFound(format!(
                "No user authentication method found for auth_id = {}",
                auth_id
            ))
            .into());
        }

        Ok(user_authentication_methods_list)
    }

    async fn list_user_authentication_methods_for_owner_id(
        &self,
        owner_id: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError> {
        let user_authentication_methods = self.user_authentication_methods.lock().await;

        let user_authentication_methods_list: Vec<_> = user_authentication_methods
            .iter()
            .filter(|auth_method_inner| auth_method_inner.owner_id == owner_id)
            .cloned()
            .collect();
        if user_authentication_methods_list.is_empty() {
            return Err(errors::StorageError::ValueNotFound(format!(
                "No user authentication method found for owner_id = {}",
                owner_id
            ))
            .into());
        }

        Ok(user_authentication_methods_list)
    }

    async fn update_user_authentication_method(
        &self,
        id: &str,
        user_authentication_method_update: storage::UserAuthenticationMethodUpdate,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        let mut user_authentication_methods = self.user_authentication_methods.lock().await;
        user_authentication_methods
            .iter_mut()
            .find(|auth_method_inner| auth_method_inner.id == id)
            .map(|auth_method_inner| {
                *auth_method_inner = match user_authentication_method_update {
                    storage::UserAuthenticationMethodUpdate::UpdateConfig { config } => {
                        storage::UserAuthenticationMethod {
                            config,
                            last_modified_at: common_utils::date_time::now(),
                            ..auth_method_inner.to_owned()
                        }
                    }
                };
                auth_method_inner.to_owned()
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No authentication method available for the id = {id}"
                ))
                .into(),
            )
    }
}
