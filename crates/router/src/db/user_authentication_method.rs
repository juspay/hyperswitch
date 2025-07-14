use diesel_models::user_authentication_method as storage;
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

    async fn get_user_authentication_method_by_id(
        &self,
        id: &str,
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

    async fn list_user_authentication_methods_for_email_domain(
        &self,
        email_domain: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError>;
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

#[async_trait::async_trait]
impl UserAuthenticationMethodInterface for MockDb {
    async fn insert_user_authentication_method(
        &self,
        user_authentication_method: storage::UserAuthenticationMethodNew,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        let mut user_authentication_methods = self.user_authentication_methods.lock().await;
        let existing_auth_id = user_authentication_methods
            .iter()
            .find(|uam| uam.owner_id == user_authentication_method.owner_id)
            .map(|uam| uam.auth_id.clone());

        let auth_id = existing_auth_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let user_authentication_method = storage::UserAuthenticationMethod {
            id: uuid::Uuid::new_v4().to_string(),
            auth_id,
            owner_id: user_authentication_method.auth_id,
            owner_type: user_authentication_method.owner_type,
            auth_type: user_authentication_method.auth_type,
            public_config: user_authentication_method.public_config,
            private_config: user_authentication_method.private_config,
            allow_signup: user_authentication_method.allow_signup,
            created_at: user_authentication_method.created_at,
            last_modified_at: user_authentication_method.last_modified_at,
            email_domain: user_authentication_method.email_domain,
        };

        user_authentication_methods.push(user_authentication_method.clone());
        Ok(user_authentication_method)
    }

    #[instrument(skip_all)]
    async fn get_user_authentication_method_by_id(
        &self,
        id: &str,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        let user_authentication_methods = self.user_authentication_methods.lock().await;

        let user_authentication_method = user_authentication_methods
            .iter()
            .find(|&auth_method_inner| auth_method_inner.id == id);

        if let Some(user_authentication_method) = user_authentication_method {
            Ok(user_authentication_method.to_owned())
        } else {
            return Err(errors::StorageError::ValueNotFound(format!(
                "No user authentication method found for id = {id}",
            ))
            .into());
        }
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
                "No user authentication method found for auth_id = {auth_id}",
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
                "No user authentication method found for owner_id = {owner_id}",
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
                    storage::UserAuthenticationMethodUpdate::UpdateConfig {
                        private_config,
                        public_config,
                    } => storage::UserAuthenticationMethod {
                        private_config,
                        public_config,
                        last_modified_at: common_utils::date_time::now(),
                        ..auth_method_inner.to_owned()
                    },
                    storage::UserAuthenticationMethodUpdate::EmailDomain { email_domain } => {
                        storage::UserAuthenticationMethod {
                            email_domain: email_domain.to_owned(),
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

    #[instrument(skip_all)]
    async fn list_user_authentication_methods_for_email_domain(
        &self,
        email_domain: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError> {
        let user_authentication_methods = self.user_authentication_methods.lock().await;

        let user_authentication_methods_list: Vec<_> = user_authentication_methods
            .iter()
            .filter(|auth_method_inner| auth_method_inner.email_domain == email_domain)
            .cloned()
            .collect();

        Ok(user_authentication_methods_list)
    }
}
