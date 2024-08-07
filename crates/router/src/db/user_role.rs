use common_utils::id_type;
use diesel_models::{enums, user_role as storage};
use error_stack::report;
use router_env::{instrument, tracing};

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
        user_role: storage::NewUserRole,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn find_user_role_by_user_id(
        &self,
        user_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn find_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

    async fn list_user_roles_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;
}

#[async_trait::async_trait]
impl UserRoleInterface for Store {
    #[instrument(skip_all)]
    async fn insert_user_role(
        &self,
        user_role: storage::NewUserRole,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        let v1_role = user_role.clone().to_v1_role();

        v1_role
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        let v2_role = user_role.to_v2_role();

        v2_role
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_user_role_by_user_id(
        &self,
        user_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::find_by_user_id(&conn, user_id.to_owned(), version)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::find_by_user_id_merchant_id(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
            version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::update_by_user_id_merchant_id(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
            update,
            version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        storage::UserRole::delete_by_user_id_merchant_id(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
            version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::list_by_user_id(&conn, user_id.to_owned(), version)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_user_roles_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::list_by_merchant_id(&conn, merchant_id.to_owned(), version)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl UserRoleInterface for MockDb {
    async fn insert_user_role(
        &self,
        user_role: storage::NewUserRole,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;
        let v1_role = user_role_new.clone().to_v1_role();

        let user_role = storage::UserRole {
            id: i32::try_from(user_roles.len())
                .change_context(errors::StorageError::MockDbError)?,
            user_id: v1_role.user_id,
            merchant_id: v1_role.merchant_id,
            role_id: v1_role.role_id,
            status: v1_role.status,
            created_by: v1_role.created_by,
            created_at: v1_role.created_at,
            last_modified: v1_role.last_modified,
            last_modified_by: v1_role.last_modified_by,
            org_id: v1_role.org_id,
            profile_id: v1_role.profile_id,
            entity_id: v1_role.entity_id,
            entity_type: v1_role.entity_type,
            version: v1_role.version,
        };
        user_roles.push(user_role);

        let v2_role = user_role_new.to_v2_role();
        let user_role = storage::UserRole {
            id: i32::try_from(user_roles.len())
                .change_context(errors::StorageError::MockDbError)?,
            user_id: v2_role.user_id,
            merchant_id: v2_role.merchant_id,
            role_id: v2_role.role_id,
            status: v2_role.status,
            created_by: v2_role.created_by,
            created_at: v2_role.created_at,
            last_modified: v2_role.last_modified,
            last_modified_by: v2_role.last_modified_by,
            org_id: v2_role.org_id,
            profile_id: v2_role.profile_id,
            entity_id: v2_role.entity_id,
            entity_type: v2_role.entity_type,
            version: v2_role.version,
        };
        user_roles.push(user_role.clone());

        Ok(user_role)
    }

    async fn find_user_role_by_user_id(
        &self,
        user_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;
        user_roles
            .iter()
            .find(|user_role| user_role.user_id == user_id && user_role.version == version)
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
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;

        for user_role in user_roles.iter() {
            let Some(user_role_merchant_id) = &user_role.merchant_id else {
                return Err(errors::StorageError::DatabaseError(
                    report!(errors::DatabaseError::Others)
                        .attach_printable("merchant_id not found for user_role"),
                )
                .into());
            };
            if user_role.user_id == user_id
                && user_role_merchant_id == merchant_id
                && user_role.version == version
            {
                return Ok(user_role.clone());
            }
        }

        Err(errors::StorageError::ValueNotFound(format!(
            "No user role available for user_id = {} and merchant_id = {}",
            user_id,
            merchant_id.get_string_repr()
        ))
        .into())
    }

    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;

        for user_role in user_roles.iter_mut() {
            let Some(user_role_merchant_id) = &user_role.merchant_id else {
                return Err(errors::StorageError::DatabaseError(
                    report!(errors::DatabaseError::Others)
                        .attach_printable("merchant_id not found for user_role"),
                )
                .into());
            };
            if user_role.user_id == user_id
                && user_role_merchant_id == merchant_id
                && user_role.version == version
            {
                match &update {
                    storage::UserRoleUpdate::UpdateRole {
                        role_id,
                        modified_by,
                    } => {
                        user_role.role_id = role_id.to_string();
                        user_role.last_modified_by = modified_by.to_string();
                    }
                    storage::UserRoleUpdate::UpdateStatus {
                        status,
                        modified_by,
                    } => {
                        user_role.status = *status;
                        user_role.last_modified_by = modified_by.to_string();
                    }
                };
                return Ok(user_role.clone());
            }
        }

        Err(errors::StorageError::ValueNotFound(format!(
            "No user role available for user_id = {user_id} and merchant_id = {merchant_id:?}"
        ))
        .into())
    }

    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;

        let index = user_roles.iter().position(|role| {
            role.user_id == user_id
                && role.version == version
                && match role.merchant_id {
                    Some(ref mid) => mid == merchant_id,
                    None => false,
                }
        });

        match index {
            Some(idx) => Ok(user_roles.remove(idx)),
            None => Err(errors::StorageError::ValueNotFound(
                "Cannot find user role to delete".to_string(),
            )
            .into()),
        }
    }

    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;

        Ok(user_roles
            .iter()
            .cloned()
            .filter_map(|ele| {
                if ele.user_id == user_id && ele.version == version {
                    return Some(ele);
                }
                None
            })
            .collect())
    }

    async fn list_user_roles_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;

        let filtered_roles: Vec<_> = user_roles
            .iter()
            .filter_map(|role| {
                if let Some(role_merchant_id) = &role.merchant_id {
                    if role_merchant_id == merchant_id && role.version == version {
                        Some(role.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(filtered_roles)
    }
}
