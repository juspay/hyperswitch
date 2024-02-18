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

    async fn update_role_by_role_id(
        &self,
        role_id: &str,
        role_update: storage::RoleUpdate,
    ) -> CustomResult<storage::Role, errors::StorageError>;

    async fn delete_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError>;

    async fn list_all_roles(
        &self,
        merchant_id: &str,
        org_id: &str,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError>;
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
        storage::Role::find_by_role_id(&conn, role_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_role_by_role_id(
        &self,
        role_id: &str,
        role_update: storage::RoleUpdate,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Role::update_by_role_id(&conn, role_id, role_update)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Role::delete_by_role_id(&conn, role_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn list_all_roles(
        &self,
        merchant_id: &str,
        org_id: &str,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Role::list_roles(&conn, merchant_id, org_id)
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

    async fn update_role_by_role_id(
        &self,
        role_id: &str,
        role_update: storage::RoleUpdate,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let mut roles = self.roles.lock().await;
        let last_modified_at = common_utils::date_time::now();

        roles
            .iter_mut()
            .find(|role| role.role_id == role_id)
            .map(|role| {
                *role = match role_update {
                    storage::RoleUpdate::UpdateGroup {
                        groups,
                        last_modified_by,
                    } => storage::Role {
                        groups,
                        last_modified_by,
                        last_modified_at,
                        ..role.to_owned()
                    },
                };
                role.to_owned()
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No role available for role_id = {role_id}"
                ))
                .into(),
            )
    }

    async fn delete_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let mut roles = self.roles.lock().await;
        let role_index = roles
            .iter()
            .position(|role| role.role_id == role_id)
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "No role available for role_id = {role_id}"
            )))?;

        Ok(roles.remove(role_index))
    }

    async fn list_all_roles(
        &self,
        merchant_id: &str,
        org_id: &str,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        let roles = self.roles.lock().await;

        let roles_list: Vec<_> = roles
            .iter()
            .filter(|role| {
                role.merchant_id == merchant_id
                    || (role.org_id == org_id
                        && role.scope == diesel_models::enums::RoleScope::Organization)
            })
            .cloned()
            .collect();

        if roles_list.is_empty() {
            return Err(errors::StorageError::ValueNotFound(format!(
                "No role found for merchant id = {} and org_id = {}",
                merchant_id, org_id
            ))
            .into());
        }

        Ok(roles_list)
    }
}
