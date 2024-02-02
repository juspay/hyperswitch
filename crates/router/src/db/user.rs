use diesel_models::{user as storage, user_role::UserRole};
use error_stack::{IntoReport, ResultExt};
use masking::Secret;

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
        /// Asynchronously inserts a new user into the database using the provided user data.
    /// 
    /// # Arguments
    /// 
    /// * `user_data` - The user data to be inserted into the database.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the inserted `storage::User` if successful, or a `errors::StorageError` if an error occurs.
    /// 
    /// # Errors
    /// 
    /// This method returns a `errors::StorageError` if there is an issue with inserting the user data into the database.
    /// 
    async fn insert_user(
        &self,
        user_data: storage::UserNew,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        user_data
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds a user by their email in the storage. Returns a Result containing either the found User or a StorageError.
    async fn find_user_by_email(
        &self,
        user_email: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::find_by_user_email(&conn, user_email)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds a user by their ID in the database.
    ///
    /// # Arguments
    ///
    /// * `user_id` - A reference to the user's ID as a string
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the found `storage::User` if successful, or an `errors::StorageError` if an error occurs.
    ///
    async fn find_user_by_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::find_by_user_id(&conn, user_id)
            .await
            .map_err(Into::into)
            .into_report()
    }


        /// Asynchronously updates a user in the database using the provided user ID and user update information.
    /// Returns a result containing either the updated user or a storage error.
    async fn update_user_by_user_id(
        &self,
        user_id: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::update_by_user_id(&conn, user_id, user)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously updates a user with the specified email using the provided user update data.
    /// Returns a custom result containing the updated user if successful, or a storage error if the update fails.
    async fn update_user_by_email(
        &self,
        user_email: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::update_by_user_email(&conn, user_email, user)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously deletes a user from the storage by user ID. It first establishes a write connection to the database, then calls the `delete_by_user_id` method of the User model from the storage module to delete the user with the given user ID. If successful, it returns true, otherwise it returns a StorageError.
    async fn delete_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::delete_by_user_id(&conn, user_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds and returns a vector of tuples containing User and UserRole instances
    /// associated with the given merchant_id. This method first establishes a connection to the
    /// PostgreSQL database, then calls the `find_joined_users_and_roles_by_merchant_id` method
    /// from the `storage::User` module to retrieve the required data. Any encountered errors are
    /// converted into a `CustomResult` with a `StorageError` variant and reported back.
    async fn find_users_and_roles_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<(storage::User, UserRole)>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::find_joined_users_and_roles_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl UserInterface for MockDb {
        /// Asynchronously inserts a new user into the storage. It first checks if the user's email or user_id already exists in the storage, and if so, returns a DuplicateValue error. If not, it creates a new User object using the provided user_data and adds it to the storage. Returns the newly inserted user on success.
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
                .into_report()
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

        /// Asynchronously finds a user by their email in the storage. Returns a Result containing either the found user or a StorageError.
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

        /// Asynchronously finds a user by their user ID in the storage. Returns a Result containing either the found user or a StorageError.
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

        /// Asynchronously updates a user in the storage by user ID. 
    /// Returns a result containing the updated user if successful, or a StorageError if the user is not found.
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

        /// Asynchronously updates a user in the storage by their email address.
    /// If the user is found, their information is updated based on the provided UserUpdate enum.
    /// Returns a Result containing the updated user if successful, or a StorageError if the user is not found.
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

        /// Asynchronously deletes a user from the storage by user ID.
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

        /// Asynchronously finds users and their associated roles by the given merchant ID.
    async fn find_users_and_roles_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<(storage::User, UserRole)>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
