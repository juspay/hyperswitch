use diesel_models::user_role as storage;
use error_stack::{IntoReport, ResultExt};

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

#[async_trait::async_trait]
pub trait UserRoleInterface {
    async fn insert_user_role(
        &self,
        user_role: storage::UserRoleNew,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn find_user_role_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn find_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
        update: storage::UserRoleUpdate,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;
    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;
}

#[async_trait::async_trait]
impl UserRoleInterface for Store {
        /// Asynchronously inserts a new user role into the database.
    ///
    /// This method takes a `storage::UserRoleNew` as input and returns a `CustomResult` containing the inserted `storage::UserRole` or a `errors::StorageError` if the operation fails. It first establishes a write connection to the database using `connection::pg_connection_write`, then inserts the user role using the `insert` method of the user role object. Any errors are converted into a `StorageError` and reported using the `into_report` method.
    ///
    /// # Arguments
    ///
    /// * `user_role` - The user role object to be inserted into the database.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the inserted `storage::UserRole` or a `errors::StorageError` if the operation fails.
    ///
    async fn insert_user_role(
        &self,
        user_role: storage::UserRoleNew,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        user_role
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds the role of a user based on their user ID.
    ///
    /// # Arguments
    ///
    /// * `user_id` - A string reference representing the user ID
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the user's role if found, or a `StorageError` if the user does not exist
    ///
    async fn find_user_role_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::find_by_user_id(&conn, user_id.to_owned())
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds a user role by user ID and merchant ID.
    /// 
    /// # Arguments
    /// 
    /// * `user_id` - A reference to a string representing the user ID.
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a `storage::UserRole` if successful, otherwise a `errors::StorageError`.
    /// 
    async fn find_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::find_by_user_id_merchant_id(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

        /// Asynchronously updates the role of a user for a specific merchant in the storage.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user whose role is to be updated.
    /// * `merchant_id` - The ID of the merchant for which the user's role is to be updated.
    /// * `update` - The UserRoleUpdate struct containing the updated role information.
    ///
    /// # Returns
    ///
    /// A Result containing a UserRole if the update was successful, or a StorageError if an error occurred.
    ///
    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
        update: storage::UserRoleUpdate,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::update_by_user_id_merchant_id(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
            update,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

        /// Asynchronously deletes a user role by user ID and merchant ID from the database.
    ///
    /// # Arguments
    ///
    /// * `user_id` - A string slice representing the user ID.
    /// * `merchant_id` - A string slice representing the merchant ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a boolean value indicating whether the deletion was successful,
    /// or a `StorageError` if an error occurred during the deletion process.
    ///
    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::delete_by_user_id_merchant_id(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

        /// Asynchronously retrieves a list of user roles by the user's ID from the database.
    ///
    /// # Arguments
    ///
    /// * `user_id` - A string slice representing the user's ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `storage::UserRole` if successful, otherwise an `errors::StorageError`.
    ///
    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::list_by_user_id(&conn, user_id.to_owned())
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl UserRoleInterface for MockDb {
        /// Inserts a new user role into the storage. It first checks if the user id already exists, and if so, returns a DuplicateValue error. Otherwise, it creates a new UserRole with an incremented id and adds it to the user roles list.
    async fn insert_user_role(
        &self,
        user_role: storage::UserRoleNew,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;
        if user_roles
            .iter()
            .any(|user_role_inner| user_role_inner.user_id == user_role.user_id)
        {
            Err(errors::StorageError::DuplicateValue {
                entity: "user_id",
                key: None,
            })?
        }
        let user_role = storage::UserRole {
            id: user_roles
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
            user_id: user_role.user_id,
            merchant_id: user_role.merchant_id,
            role_id: user_role.role_id,
            status: user_role.status,
            created_by: user_role.created_by,
            created_at: user_role.created_at,
            last_modified: user_role.last_modified,
            last_modified_by: user_role.last_modified_by,
            org_id: user_role.org_id,
        };
        user_roles.push(user_role.clone());
        Ok(user_role)
    }

        /// Asynchronously finds and returns the user role associated with the given user_id. 
    ///
    /// # Arguments
    ///
    /// * `user_id` - A string slice representing the user ID for which the user role needs to be found.
    ///
    /// # Returns
    ///
    /// * `CustomResult<storage::UserRole, errors::StorageError>` - A custom result indicating either the found user role or an error in case the user role is not available.
    ///
    async fn find_user_role_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;
        user_roles
            .iter()
            .find(|user_role| user_role.user_id == user_id)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No user role available for user_id = {user_id}"
                ))
                .into(),
            )
    }

        /// Asynchronously finds the user role by the given user ID and merchant ID. Returns a Result with the found UserRole or a StorageError if the user role is not found.
    async fn find_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;
        user_roles
            .iter()
            .find(|user_role| user_role.user_id == user_id && user_role.merchant_id == merchant_id)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No user role available for user_id = {user_id} and merchant_id = {merchant_id}"
                ))
                .into(),
            )
    }

        /// Asynchronously updates the user role for a specific user and merchant based on the provided user ID,
    /// merchant ID, and role update. If a matching user role is found, it is updated with the new role or status
    /// based on the provided `storage::UserRoleUpdate`. If no matching user role is found, a `StorageError`
    /// is returned indicating that no user role is available for the given user and merchant.
    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
        update: storage::UserRoleUpdate,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;
        user_roles
            .iter_mut()
            .find(|user_role| user_role.user_id == user_id && user_role.merchant_id == merchant_id)
            .map(|user_role| {
                *user_role = match &update {
                    storage::UserRoleUpdate::UpdateRole {
                        role_id,
                        modified_by,
                    } => storage::UserRole {
                        role_id: role_id.to_string(),
                        last_modified_by: modified_by.to_string(),
                        ..user_role.to_owned()
                    },
                    storage::UserRoleUpdate::UpdateStatus {
                        status,
                        modified_by,
                    } => storage::UserRole {
                        status: status.to_owned(),
                        last_modified_by: modified_by.to_owned(),
                        ..user_role.to_owned()
                    },
                };
                user_role.to_owned()
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No user role available for user_id = {user_id} and merchant_id = {merchant_id}"
                ))
                .into(),
            )
    }

        /// Asynchronously deletes a user role by user ID and merchant ID from the storage.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user whose role is to be deleted.
    /// * `merchant_id` - The ID of the merchant for which the user role is to be deleted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` with a boolean indicating whether the user role was successfully deleted or an error of type `StorageError`.
    ///
    /// # Errors
    ///
    /// Returns a `ValueNotFound` error if no user role is found for the given user ID.
    ///
    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;
        let user_role_index = user_roles
            .iter()
            .position(|user_role| {
                user_role.user_id == user_id && user_role.merchant_id == merchant_id
            })
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "No user available for user_id = {user_id}"
            )))?;
        user_roles.remove(user_role_index);
        Ok(true)
    }

        /// Asynchronously retrieves a list of user roles by the given user ID from the storage.
    /// 
    /// # Arguments
    /// * `user_id` - A reference to a string representing the user ID for which to retrieve the roles.
    ///
    /// # Returns
    /// A `CustomResult` containing a vector of `UserRole` instances if successful, or a `StorageError` if an error occurs.
    ///
    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;

        Ok(user_roles
            .iter()
            .cloned()
            .filter_map(|ele| {
                if ele.user_id == user_id {
                    return Some(ele);
                }
                None
            })
            .collect())
    }
}

#[cfg(feature = "kafka_events")]
#[async_trait::async_trait]
impl UserRoleInterface for super::KafkaStore {
        /// Inserts a new user role into the database.
    /// 
    /// # Arguments
    /// 
    /// * `user_role` - The user role to be inserted into the database.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the inserted `UserRole` if successful, otherwise an `StorageError`.
    /// 
    async fn insert_user_role(
        &self,
        user_role: storage::UserRoleNew,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        self.diesel_store.insert_user_role(user_role).await
    }
        /// Asynchronously updates the role of a user for a specific merchant based on user ID and merchant ID.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user whose role is to be updated.
    /// * `merchant_id` - The ID of the merchant for which the user's role is to be updated.
    /// * `update` - The details of the role update to be performed.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing the updated `UserRole` if the update is successful, otherwise returns a `StorageError`.
    ///
    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
        update: storage::UserRoleUpdate,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        self.diesel_store
            .update_user_role_by_user_id_merchant_id(user_id, merchant_id, update)
            .await
    }
        /// Asynchronously finds the role of a user based on the user ID. Returns a Result containing the user role if found, or a StorageError if an error occurs during the operation.
    async fn find_user_role_by_user_id(
            &self,
            user_id: &str,
        ) -> CustomResult<storage::UserRole, errors::StorageError> {
            self.diesel_store.find_user_role_by_user_id(user_id).await
        }
        /// Deletes a user role for a specific user and merchant.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user whose role is to be deleted.
    /// * `merchant_id` - The ID of the merchant for which the user role is to be deleted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` indicating whether the user role was successfully deleted or an error occurred.
    ///
    /// # Errors
    ///
    /// An `errors::StorageError` is returned if there is an issue with the storage operation.
    ///
    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_user_role_by_user_id_merchant_id(user_id, merchant_id)
            .await
    }
        /// Asynchronously retrieves a list of user roles for a given user ID from the database.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user for whom the roles are to be retrieved.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `Vec` of `storage::UserRole` if the operation is successful,
    /// otherwise a `StorageError` is returned.
    ///
    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        self.diesel_store.list_user_roles_by_user_id(user_id).await
    }
}
