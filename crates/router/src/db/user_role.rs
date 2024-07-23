use std::collections::HashSet;

use async_bb8_diesel::AsyncConnection;
use diesel_models::{enums, user_role as storage};
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    connection, consts,
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
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn find_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn update_user_roles_by_user_id_org_id(
        &self,
        user_id: &str,
        org_id: &str,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

    async fn list_user_roles_by_merchant_id(
        &self,
        merchant_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

    async fn transfer_org_ownership_between_users(
        &self,
        from_user_id: &str,
        to_user_id: &str,
        org_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<(), errors::StorageError>;
}

#[async_trait::async_trait]
impl UserRoleInterface for Store {
    #[instrument(skip_all)]
    async fn insert_user_role(
        &self,
        user_role: storage::UserRoleNew,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        user_role
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
        merchant_id: &str,
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
        merchant_id: &str,
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
    async fn update_user_roles_by_user_id_org_id(
        &self,
        user_id: &str,
        org_id: &str,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::update_by_user_id_org_id(
            &conn,
            user_id.to_owned(),
            org_id.to_owned(),
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
        merchant_id: &str,
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
        merchant_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::list_by_merchant_id(&conn, merchant_id.to_owned(), version)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn transfer_org_ownership_between_users(
        &self,
        from_user_id: &str,
        to_user_id: &str,
        org_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<(), errors::StorageError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        conn.transaction_async(|conn| async move {
            let old_org_admin_user_roles = storage::UserRole::update_by_user_id_org_id(
                &conn,
                from_user_id.to_owned(),
                org_id.to_owned(),
                storage::UserRoleUpdate::UpdateRole {
                    role_id: consts::user_role::ROLE_ID_MERCHANT_ADMIN.to_string(),
                    modified_by: from_user_id.to_owned(),
                },
                version,
            )
            .await
            .map_err(|e| *e.current_context())?;

            let new_org_admin_user_roles = storage::UserRole::update_by_user_id_org_id(
                &conn,
                to_user_id.to_owned(),
                org_id.to_owned(),
                storage::UserRoleUpdate::UpdateRole {
                    role_id: consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
                    modified_by: from_user_id.to_owned(),
                },
                version,
            )
            .await
            .map_err(|e| *e.current_context())?;

            let new_org_admin_merchant_ids = new_org_admin_user_roles
                .iter()
                .map(|user_role| {
                    user_role
                        .merchant_id
                        .to_owned()
                        .ok_or(errors::DatabaseError::NotFound)
                })
                .collect::<Result<HashSet<_>, _>>()?;

            let now = common_utils::date_time::now();

            let mut missing_new_user_roles = Vec::new();

            for old_role in old_org_admin_user_roles {
                let Some(old_role_merchant_id) = &old_role.merchant_id else {
                    return Err(errors::DatabaseError::NotFound);
                };
                if !new_org_admin_merchant_ids.contains(old_role_merchant_id) {
                    missing_new_user_roles.push(storage::UserRoleNew {
                        user_id: to_user_id.to_string(),
                        merchant_id: Some(old_role_merchant_id.to_string()),
                        role_id: consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
                        org_id: Some(org_id.to_string()),
                        status: enums::UserStatus::Active,
                        created_by: from_user_id.to_string(),
                        last_modified_by: from_user_id.to_string(),
                        created_at: now,
                        last_modified: now,
                        profile_id: None,
                        entity_id: None,
                        entity_type: None,
                        version: enums::UserRoleVersion::V1,
                    });
                }
            }

            futures::future::try_join_all(missing_new_user_roles.into_iter().map(
                |user_role| async {
                    user_role
                        .insert(&conn)
                        .await
                        .map_err(|e| *e.current_context())
                },
            ))
            .await?;

            Ok::<_, errors::DatabaseError>(())
        })
        .await
        .map_err(|error| report!(errors::StorageError::from(report!(error))))?;

        Ok(())
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
            id: i32::try_from(user_roles.len())
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
            profile_id: None,
            entity_id: None,
            entity_type: None,
            version: enums::UserRoleVersion::V1,
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
        merchant_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;

        for user_role in user_roles.iter() {
            let Some(user_role_merchant_id) = &user_role.merchant_id else {
                return Err(errors::StorageError::ValueNotFound(
                    "Merchant id not found".to_string(),
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
            user_id, merchant_id
        ))
        .into())
    }

    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;

        for user_role in user_roles.iter_mut() {
            let Some(user_role_merchant_id) = &user_role.merchant_id else {
                return Err(errors::StorageError::ValueNotFound(
                    "Merchant id not found".to_string(),
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
            "No user role available for user_id = {} and merchant_id = {}",
            user_id, merchant_id
        ))
        .into())
    }

    async fn update_user_roles_by_user_id_org_id(
        &self,
        user_id: &str,
        org_id: &str,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;
        let mut updated_user_roles = Vec::new();
        for user_role in user_roles.iter_mut() {
            let Some(user_role_org_id) = &user_role.org_id else {
                return Err(errors::StorageError::ValueNotFound(
                    "No user org_id is available".to_string(),
                )
                .into());
            };
            if user_role.user_id == user_id
                && user_role_org_id == org_id
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
                        status.clone_into(&mut user_role.status);
                        modified_by.clone_into(&mut user_role.last_modified_by);
                    }
                }
                updated_user_roles.push(user_role.to_owned());
            }
        }
        if updated_user_roles.is_empty() {
            Err(errors::StorageError::ValueNotFound(format!(
                "No user role available for user_id = {user_id} and org_id = {org_id}"
            ))
            .into())
        } else {
            Ok(updated_user_roles)
        }
    }

    async fn transfer_org_ownership_between_users(
        &self,
        from_user_id: &str,
        to_user_id: &str,
        org_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<(), errors::StorageError> {
        let old_org_admin_user_roles = self
            .update_user_roles_by_user_id_org_id(
                from_user_id,
                org_id,
                storage::UserRoleUpdate::UpdateRole {
                    role_id: consts::user_role::ROLE_ID_MERCHANT_ADMIN.to_string(),
                    modified_by: from_user_id.to_string(),
                },
                version,
            )
            .await?;

        let new_org_admin_user_roles = self
            .update_user_roles_by_user_id_org_id(
                to_user_id,
                org_id,
                storage::UserRoleUpdate::UpdateRole {
                    role_id: consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
                    modified_by: from_user_id.to_string(),
                },
                version,
            )
            .await?;

        let new_org_admin_merchant_ids = new_org_admin_user_roles
            .iter()
            .map(|user_role| {
                user_role.merchant_id.to_owned().ok_or(report!(
                    errors::StorageError::ValueNotFound(
                        "Cannot find merchnat id for the user role".to_string(),
                    )
                ))
            })
            .collect::<Result<HashSet<_>, _>>()?;

        let now = common_utils::date_time::now();
        let mut missing_new_user_roles = Vec::new();

        for old_roles in old_org_admin_user_roles {
            let Some(merchant_id) = &old_roles.merchant_id else {
                return Err(errors::StorageError::ValueNotFound(
                    "Cannot find merchnat id for the user role".to_string(),
                )
                .into());
            };
            if !new_org_admin_merchant_ids.contains(merchant_id) {
                let new_user_role = storage::UserRoleNew {
                    user_id: to_user_id.to_string(),
                    merchant_id: Some(merchant_id.to_string()),
                    role_id: consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
                    org_id: Some(org_id.to_string()),
                    status: enums::UserStatus::Active,
                    created_by: from_user_id.to_string(),
                    last_modified_by: from_user_id.to_string(),
                    created_at: now,
                    last_modified: now,
                    profile_id: None,
                    entity_id: None,
                    entity_type: None,
                    version: enums::UserRoleVersion::V1,
                };

                missing_new_user_roles.push(new_user_role);
            }
        }

        for user_role in missing_new_user_roles {
            self.insert_user_role(user_role).await?;
        }

        Ok(())
    }

    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
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
        merchant_id: &str,
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
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        self.diesel_store
            .find_user_role_by_user_id(user_id, version)
            .await
    }
    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        self.diesel_store
            .delete_user_role_by_user_id_merchant_id(user_id, merchant_id)
            .await
    }
    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        self.diesel_store.list_user_roles_by_user_id(user_id).await
    }
    async fn list_user_roles_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        self.diesel_store
            .list_user_roles_by_merchant_id(merchant_id)
            .await
    }
}
