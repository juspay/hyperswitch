use diesel_models::{user as storage, user_role::UserRole};
use error_stack::ResultExt;
use masking::Secret;
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};
pub mod sample_data;

#[async_trait::async_trait]
pub trait UserInterface {
    async fn insert_user(
        &self,
        user_data: storage::UserNew,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn find_user_by_email(
        &self,
        user_email: &str,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn find_user_by_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn update_user_by_user_id(
        &self,
        user_id: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn update_user_by_email(
        &self,
        user_email: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn delete_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn find_users_and_roles_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<(storage::User, UserRole)>, errors::StorageError>;
}

#[async_trait::async_trait]
impl UserInterface for Store {
    #[instrument(skip_all)]
    async fn insert_user(
        &self,
        user_data: storage::UserNew,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        user_data.insert(&conn).await.map_err(Into::into)
    }

    #[instrument(skip_all)]
    async fn find_user_by_email(
        &self,
        user_email: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::find_by_user_email(&conn, user_email)
            .await
            .map_err(Into::into)
    }

    #[instrument(skip_all)]
    async fn find_user_by_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::find_by_user_id(&conn, user_id)
            .await
            .map_err(Into::into)
    }

    #[instrument(skip_all)]
    async fn update_user_by_user_id(
        &self,
        user_id: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::update_by_user_id(&conn, user_id, user)
            .await
            .map_err(Into::into)
    }

    #[instrument(skip_all)]
    async fn update_user_by_email(
        &self,
        user_email: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::update_by_user_email(&conn, user_email, user)
            .await
            .map_err(Into::into)
    }

    #[instrument(skip_all)]
    async fn delete_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::delete_by_user_id(&conn, user_id)
            .await
            .map_err(Into::into)
    }

    #[instrument(skip_all)]
    async fn find_users_and_roles_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<(storage::User, UserRole)>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::find_joined_users_and_roles_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(Into::into)
    }
}

#[async_trait::async_trait]
impl UserInterface for MockDb {
    async fn insert_user(
        &self,
        user_data: storage::UserNew,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let mut users = self.users.lock().await;
        if users
            .iter()
            .any(|user| user.email == user_data.email || user.user_id == user_data.user_id)
        {
            Err(errors::StorageError::DuplicateValue {
                entity: "email or user_id",
                key: None,
            })?
        }
        let time_now = common_utils::date_time::now();
        let user = storage::User {
            id: users
                .len()
                .try_into()
                .change_context(errors::StorageError::MockDbError)?,
            user_id: user_data.user_id,
            email: user_data.email,
            name: user_data.name,
            password: user_data.password,
            is_verified: user_data.is_verified,
            created_at: user_data.created_at.unwrap_or(time_now),
            last_modified_at: user_data.created_at.unwrap_or(time_now),
            preferred_merchant_id: user_data.preferred_merchant_id,
        };
        users.push(user.clone());
        Ok(user)
    }

    async fn find_user_by_email(
        &self,
        user_email: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let users = self.users.lock().await;
        let user_email_pii: common_utils::pii::Email = user_email
            .to_string()
            .try_into()
            .map_err(|_| errors::StorageError::MockDbError)?;
        users
            .iter()
            .find(|user| user.email == user_email_pii)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No user available for email = {user_email}"
                ))
                .into(),
            )
    }

    async fn find_user_by_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let users = self.users.lock().await;
        users
            .iter()
            .find(|user| user.user_id == user_id)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No user available for user_id = {user_id}"
                ))
                .into(),
            )
    }

    async fn update_user_by_user_id(
        &self,
        user_id: &str,
        update_user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let mut users = self.users.lock().await;
        users
            .iter_mut()
            .find(|user| user.user_id == user_id)
            .map(|user| {
                *user = match &update_user {
                    storage::UserUpdate::VerifyUser => storage::User {
                        is_verified: true,
                        ..user.to_owned()
                    },
                    storage::UserUpdate::AccountUpdate {
                        name,
                        password,
                        is_verified,
                        preferred_merchant_id,
                    } => storage::User {
                        name: name.clone().map(Secret::new).unwrap_or(user.name.clone()),
                        password: password.clone().unwrap_or(user.password.clone()),
                        is_verified: is_verified.unwrap_or(user.is_verified),
                        preferred_merchant_id: preferred_merchant_id
                            .clone()
                            .or(user.preferred_merchant_id.clone()),
                        ..user.to_owned()
                    },
                };
                user.to_owned()
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No user available for user_id = {user_id}"
                ))
                .into(),
            )
    }

    async fn update_user_by_email(
        &self,
        user_email: &str,
        update_user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let mut users = self.users.lock().await;
        let user_email_pii: common_utils::pii::Email = user_email
            .to_string()
            .try_into()
            .map_err(|_| errors::StorageError::MockDbError)?;
        users
            .iter_mut()
            .find(|user| user.email == user_email_pii)
            .map(|user| {
                *user = match &update_user {
                    storage::UserUpdate::VerifyUser => storage::User {
                        is_verified: true,
                        ..user.to_owned()
                    },
                    storage::UserUpdate::AccountUpdate {
                        name,
                        password,
                        is_verified,
                        preferred_merchant_id,
                    } => storage::User {
                        name: name.clone().map(Secret::new).unwrap_or(user.name.clone()),
                        password: password.clone().unwrap_or(user.password.clone()),
                        is_verified: is_verified.unwrap_or(user.is_verified),
                        preferred_merchant_id: preferred_merchant_id
                            .clone()
                            .or(user.preferred_merchant_id.clone()),
                        ..user.to_owned()
                    },
                };
                user.to_owned()
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No user available for user_email = {user_email}"
                ))
                .into(),
            )
    }

    async fn delete_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut users = self.users.lock().await;
        let user_index = users
            .iter()
            .position(|user| user.user_id == user_id)
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "No user available for user_id = {user_id}"
            )))?;
        users.remove(user_index);
        Ok(true)
    }

    async fn find_users_and_roles_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<(storage::User, UserRole)>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
