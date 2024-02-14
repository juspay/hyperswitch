use diesel_models::role as storage;
use error_stack::{IntoReport, ResultExt};

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

#[async_trait::async_trait]
pub trait RoleInterface {
    async fn insert_role(
        &self,
        role: storage::RoleNew,
    ) -> CustomResult<storage::Role, errors::StorageError>;

    async fn find_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError>;

    // async fn list_all_roles(
    //     &self,
    //     merchant_id: &str,
    //     org_id: &str,
    // ) -> CustomResult<Vec<storage::Role>, errors::StorageError>;
    // async fn update_permissions_by_role_id(
    //     &self,
    //     user_id: &str,
    //     merchant_id: &str,
    //     update: storage::RoleUpdate,
    // ) -> CustomResult<storage::Role, errors::StorageError>;
    async fn delete_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl RoleInterface for Store {
    async fn insert_role(
        &self,
        role: storage::RoleNew,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        role.insert(&conn).await.map_err(Into::into).into_report()
    }

    async fn find_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Role::find_by_role_id(&conn, role_id.to_owned())
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Role::delete_by_role_id(&conn, role_id.to_owned())
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl RoleInterface for MockDb {
    async fn insert_role(
        &self,
        role: storage::RoleNew,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let mut roles = self.roles.lock().await;
        if roles
            .iter()
            .any(|role_inner| role_inner.role_id == role.role_id)
        {
            Err(errors::StorageError::DuplicateValue {
                entity: "role_id",
                key: None,
            })?
        }
        let role = storage::Role {
            id: roles
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
            role_name: role.role_name,
            role_id: role.role_id,
            merchant_id: role.merchant_id,
            org_id: role.org_id,
            groups: role.groups,
            scope: role.scope,
            created_by: role.created_by,
            created_at: role.created_at,
            last_modified_at: role.last_modified_at,
            last_modified_by: role.last_modified_by,
        };
        roles.push(role.clone());
        Ok(role)
    }

    async fn find_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let roles = self.roles.lock().await;
        roles
            .iter()
            .find(|role| role.role_id == role_id)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No role available role_id  = {role_id}"
                ))
                .into(),
            )
    }

    async fn delete_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut roles = self.user_roles.lock().await;
        let role_index = roles
            .iter()
            .position(|role| role.user_id == role_id)
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "No user available for role_id = {role_id}"
            )))?;
        roles.remove(role_index);
        Ok(true)
    }
}
