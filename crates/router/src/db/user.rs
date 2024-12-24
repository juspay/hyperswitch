use diesel_models::user as storage;
use error_stack::report;
use masking::Secret;
use router_env::{instrument, tracing};

use super::{domain, MockDb};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};
pub mod sample_data;
pub mod theme;

#[async_trait::async_trait]
pub trait UserInterface {
    async fn insert_user(
        &self,
        user_data: storage::UserNew,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn find_user_by_email(
        &self,
        user_email: &domain::UserEmail,
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
        user_email: &domain::UserEmail,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn delete_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn find_users_by_user_ids(
        &self,
        user_ids: Vec<String>,
    ) -> CustomResult<Vec<storage::User>, errors::StorageError>;
}

#[async_trait::async_trait]
impl UserInterface for Store {
    #[instrument(skip_all)]
    async fn insert_user(
        &self,
        user_data: storage::UserNew,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        user_data
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_user_by_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_by_user_email(&conn, user_email.get_inner())
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_user_by_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_by_user_id(&conn, user_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
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
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_user_by_email(
        &self,
        user_email: &domain::UserEmail,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::update_by_user_email(&conn, user_email.get_inner(), user)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::delete_by_user_id(&conn, user_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_users_by_user_ids(
        &self,
        user_ids: Vec<String>,
    ) -> CustomResult<Vec<storage::User>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_users_by_user_ids(&conn, user_ids)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
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
            user_id: user_data.user_id,
            email: user_data.email,
            name: user_data.name,
            password: user_data.password,
            is_verified: user_data.is_verified,
            created_at: user_data.created_at.unwrap_or(time_now),
            last_modified_at: user_data.created_at.unwrap_or(time_now),
            totp_status: user_data.totp_status,
            totp_secret: user_data.totp_secret,
            totp_recovery_codes: user_data.totp_recovery_codes,
            last_password_modified_at: user_data.last_password_modified_at,
        };
        users.push(user.clone());
        Ok(user)
    }

    async fn find_user_by_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let users = self.users.lock().await;
        users
            .iter()
            .find(|user| user.email.eq(user_email.get_inner()))
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No user available for email = {user_email:?}"
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
                    storage::UserUpdate::AccountUpdate { name, is_verified } => storage::User {
                        name: name.clone().map(Secret::new).unwrap_or(user.name.clone()),
                        is_verified: is_verified.unwrap_or(user.is_verified),
                        ..user.to_owned()
                    },
                    storage::UserUpdate::TotpUpdate {
                        totp_status,
                        totp_secret,
                        totp_recovery_codes,
                    } => storage::User {
                        totp_status: totp_status.unwrap_or(user.totp_status),
                        totp_secret: totp_secret.clone().or(user.totp_secret.clone()),
                        totp_recovery_codes: totp_recovery_codes
                            .clone()
                            .or(user.totp_recovery_codes.clone()),
                        ..user.to_owned()
                    },
                    storage::UserUpdate::PasswordUpdate { password } => storage::User {
                        password: Some(password.clone()),
                        last_password_modified_at: Some(common_utils::date_time::now()),
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
        user_email: &domain::UserEmail,
        update_user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let mut users = self.users.lock().await;
        users
            .iter_mut()
            .find(|user| user.email.eq(user_email.get_inner()))
            .map(|user| {
                *user = match &update_user {
                    storage::UserUpdate::VerifyUser => storage::User {
                        is_verified: true,
                        ..user.to_owned()
                    },
                    storage::UserUpdate::AccountUpdate { name, is_verified } => storage::User {
                        name: name.clone().map(Secret::new).unwrap_or(user.name.clone()),
                        is_verified: is_verified.unwrap_or(user.is_verified),
                        ..user.to_owned()
                    },
                    storage::UserUpdate::TotpUpdate {
                        totp_status,
                        totp_secret,
                        totp_recovery_codes,
                    } => storage::User {
                        totp_status: totp_status.unwrap_or(user.totp_status),
                        totp_secret: totp_secret.clone().or(user.totp_secret.clone()),
                        totp_recovery_codes: totp_recovery_codes
                            .clone()
                            .or(user.totp_recovery_codes.clone()),
                        ..user.to_owned()
                    },
                    storage::UserUpdate::PasswordUpdate { password } => storage::User {
                        password: Some(password.clone()),
                        last_password_modified_at: Some(common_utils::date_time::now()),
                        ..user.to_owned()
                    },
                };
                user.to_owned()
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No user available for user_email = {user_email:?}"
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

    async fn find_users_by_user_ids(
        &self,
        _user_ids: Vec<String>,
    ) -> CustomResult<Vec<storage::User>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
