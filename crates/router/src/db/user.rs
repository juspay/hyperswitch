use diesel_models::{enums::TotpStatus, user as storage};
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

    async fn find_active_user_by_user_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn find_user_by_user_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn find_active_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn update_active_user_by_user_id(
        &self,
        user_id: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn update_active_user_by_user_email(
        &self,
        user_email: &domain::UserEmail,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError>;

    async fn find_active_users_by_user_ids(
        &self,
        user_ids: Vec<String>,
    ) -> CustomResult<Vec<storage::User>, errors::StorageError>;

    async fn reactivate_user_by_user_id(
        &self,
        user_id: &str,
        user_update: storage::ReactivateUserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError>;
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
    async fn find_active_user_by_user_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_active_by_user_email(&conn, user_email.get_inner())
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_user_by_user_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_by_user_email(&conn, user_email.get_inner())
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_active_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_active_by_user_id(&conn, user_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_active_user_by_user_id(
        &self,
        user_id: &str,
        user_update: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        storage::User::update_active_by_user_id(&conn, user_id, user_update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_active_user_by_user_email(
        &self,
        user_email: &domain::UserEmail,
        user_update: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        storage::User::update_active_by_user_email(&conn, user_email.get_inner(), user_update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_active_users_by_user_ids(
        &self,
        user_ids: Vec<String>,
    ) -> CustomResult<Vec<storage::User>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_active_users_by_user_ids(&conn, user_ids)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn reactivate_user_by_user_id(
        &self,
        user_id: &str,
        user_update: storage::ReactivateUserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        storage::User::reactivate_by_user_id(&conn, user_id, user_update)
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
            lineage_context: user_data.lineage_context,
            is_active: user_data.is_active,
        };
        users.push(user.clone());
        Ok(user)
    }

    async fn find_active_user_by_user_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let users = self.users.lock().await;
        users
            .iter()
            .find(|user| user.email.eq(user_email.get_inner()) && user.is_active)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No Active user available for email = {user_email:?}"
                ))
                .into(),
            )
    }

    async fn find_user_by_user_email(
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

    async fn find_active_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let users = self.users.lock().await;
        users
            .iter()
            .find(|user| user.user_id == user_id && user.is_active)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No Active user available for user_id = {user_id}"
                ))
                .into(),
            )
    }

    async fn update_active_user_by_user_id(
        &self,
        user_id: &str,
        update_user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let mut users = self.users.lock().await;

        let user = users
            .iter_mut()
            .find(|user| user.user_id == user_id && user.is_active)
            .ok_or_else(|| {
                errors::StorageError::ValueNotFound(format!(
                    "No Active user available for user_id = {user_id}"
                ))
            })?;

        let last_modified_at = common_utils::date_time::now();

        *user = match update_user {
            storage::UserUpdate::VerifyUser => storage::User {
                last_modified_at,
                is_verified: true,
                ..user.to_owned()
            },

            storage::UserUpdate::AccountUpdate { name, is_verified } => storage::User {
                name: name.map(Secret::new).unwrap_or(user.name.clone()),
                last_modified_at,
                is_verified: is_verified.unwrap_or(user.is_verified),
                ..user.to_owned()
            },

            storage::UserUpdate::TotpUpdate {
                totp_status,
                totp_secret,
                totp_recovery_codes,
            } => storage::User {
                last_modified_at,
                totp_status: totp_status.unwrap_or(user.totp_status),
                totp_secret: totp_secret.or(user.totp_secret.clone()),
                totp_recovery_codes: totp_recovery_codes.or(user.totp_recovery_codes.clone()),
                ..user.to_owned()
            },

            storage::UserUpdate::PasswordUpdate { password } => storage::User {
                password: Some(password.clone()),
                last_password_modified_at: Some(common_utils::date_time::now()),
                ..user.to_owned()
            },

            storage::UserUpdate::LineageContextUpdate { lineage_context } => storage::User {
                last_modified_at,
                lineage_context: Some(lineage_context.clone()),
                ..user.to_owned()
            },

            storage::UserUpdate::DeactivateUpdate => storage::User {
                last_modified_at,
                password: None,
                last_password_modified_at: None,
                totp_status: TotpStatus::NotSet,
                totp_secret: None,
                totp_recovery_codes: None,
                is_verified: false,
                lineage_context: None,
                is_active: false,
                ..user.to_owned()
            },
        };

        Ok(user.to_owned())
    }

    async fn update_active_user_by_user_email(
        &self,
        user_email: &domain::UserEmail,
        update_user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let mut users = self.users.lock().await;

        let user = users
            .iter_mut()
            .find(|user| user.email.eq(user_email.get_inner()) && user.is_active)
            .ok_or_else(|| {
                errors::StorageError::ValueNotFound(format!(
                    "No Active user available for user_email = {user_email:?}"
                ))
            })?;

        let last_modified_at = common_utils::date_time::now();

        *user = match update_user {
            storage::UserUpdate::VerifyUser => storage::User {
                last_modified_at,
                is_verified: true,
                ..user.to_owned()
            },

            storage::UserUpdate::AccountUpdate { name, is_verified } => storage::User {
                name: name.map(Secret::new).unwrap_or_else(|| user.name.clone()),
                last_modified_at,
                is_verified: is_verified.unwrap_or(user.is_verified),
                ..user.to_owned()
            },

            storage::UserUpdate::TotpUpdate {
                totp_status,
                totp_secret,
                totp_recovery_codes,
            } => storage::User {
                last_modified_at,
                totp_status: totp_status.unwrap_or(user.totp_status),
                totp_secret: totp_secret.or_else(|| user.totp_secret.clone()),
                totp_recovery_codes: totp_recovery_codes
                    .or_else(|| user.totp_recovery_codes.clone()),
                ..user.to_owned()
            },

            storage::UserUpdate::PasswordUpdate { password } => storage::User {
                password: Some(password),
                last_password_modified_at: Some(common_utils::date_time::now()),
                ..user.to_owned()
            },

            storage::UserUpdate::LineageContextUpdate { lineage_context } => storage::User {
                last_modified_at,
                lineage_context: Some(lineage_context),
                ..user.to_owned()
            },

            storage::UserUpdate::DeactivateUpdate => storage::User {
                last_modified_at,
                password: None,
                last_password_modified_at: Some(last_modified_at),
                totp_status: TotpStatus::NotSet,
                totp_secret: None,
                totp_recovery_codes: None,
                is_verified: false,
                lineage_context: None,
                is_active: false,
                ..user.to_owned()
            },
        };

        Ok(user.to_owned())
    }

    async fn find_active_users_by_user_ids(
        &self,
        _user_ids: Vec<String>,
    ) -> CustomResult<Vec<storage::User>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn reactivate_user_by_user_id(
        &self,
        user_id: &str,
        user_update: storage::ReactivateUserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let mut users = self.users.lock().await;

        let last_modified_at = common_utils::date_time::now();
        users
            .iter_mut()
            .find(|user| user.user_id.eq(user_id) && !user.is_active)
            .map(|user| {
                *user = storage::User {
                    user_id: user.user_id.clone(),
                    email: user.email.clone(),
                    name: user_update
                        .new_name
                        .map(Secret::new)
                        .unwrap_or(user.name.clone()),
                    password: user_update.new_password,
                    is_verified: false,
                    created_at: last_modified_at,
                    last_modified_at,
                    totp_status: TotpStatus::NotSet,
                    totp_secret: None,
                    totp_recovery_codes: None,
                    last_password_modified_at: Some(last_modified_at),
                    lineage_context: None,
                    is_active: true,
                };
                user.to_owned()
            })
            .ok_or_else(|| {
                errors::StorageError::ValueNotFound(format!(
                    "No Inactive user available for user_id = {user_id:?}"
                ))
                .into()
            })
    }
}
