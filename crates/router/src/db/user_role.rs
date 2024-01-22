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

    async fn delete_user_role(&self, user_id: &str) -> CustomResult<bool, errors::StorageError>;

    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;
}

#[async_trait::async_trait]
impl UserRoleInterface for Store {
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

    async fn delete_user_role(&self, user_id: &str) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::delete_by_user_id(&conn, user_id.to_owned())
            .await
            .map_err(Into::into)
            .into_report()
    }

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

    async fn delete_user_role(&self, user_id: &str) -> CustomResult<bool, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;
        let user_role_index = user_roles
            .iter()
            .position(|user_role| user_role.user_id == user_id)
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "No user available for user_id = {user_id}"
            )))?;
        user_roles.remove(user_role_index);
        Ok(true)
    }

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
    async fn insert_user_role(
        &self,
        user_role: storage::UserRoleNew,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        self.diesel_store.insert_user_role(user_role).await
    }
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
    async fn find_user_role_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        self.diesel_store.find_user_role_by_user_id(user_id).await
    }
    async fn delete_user_role(&self, user_id: &str) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store.delete_user_role(user_id).await
    }
    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        self.diesel_store.list_user_roles_by_user_id(user_id).await
    }
}
