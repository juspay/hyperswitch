use std::{collections::HashSet, ops::Not};

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

    async fn update_user_roles_by_user_id_org_id(
        &self,
        user_id: &str,
        org_id: &str,
        update: storage::UserRoleUpdate,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

    async fn transfer_org_ownership_between_users(
        &self,
        from_user_id: &str,
        to_user_id: &str,
        org_id: &str,
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
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::find_by_user_id(&conn, user_id.to_owned())
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
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
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
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
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_user_roles_by_user_id_org_id(
        &self,
        user_id: &str,
        org_id: &str,
        update: storage::UserRoleUpdate,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::update_by_user_id_org_id(
            &conn,
            user_id.to_owned(),
            org_id.to_owned(),
            update,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
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
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::list_by_user_id(&conn, user_id.to_owned())
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn transfer_org_ownership_between_users(
        &self,
        from_user_id: &str,
        to_user_id: &str,
        org_id: &str,
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
            )
            .await
            .map_err(|e| *e.current_context())?;

            let new_org_admin_merchant_ids = new_org_admin_user_roles
                .iter()
                .map(|user_role| user_role.merchant_id.to_owned())
                .collect::<HashSet<String>>();

            let now = common_utils::date_time::now();

            let missing_new_user_roles =
                old_org_admin_user_roles.into_iter().filter_map(|old_role| {
                    new_org_admin_merchant_ids
                        .contains(&old_role.merchant_id)
                        .not()
                        .then_some({
                            storage::UserRoleNew {
                                user_id: to_user_id.to_string(),
                                merchant_id: old_role.merchant_id,
                                role_id: consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
                                org_id: org_id.to_string(),
                                status: enums::UserStatus::Active,
                                created_by: from_user_id.to_string(),
                                last_modified_by: from_user_id.to_string(),
                                created_at: now,
                                last_modified: now,
                            }
                        })
                });

            futures::future::try_join_all(missing_new_user_roles.map(|user_role| async {
                user_role
                    .insert(&conn)
                    .await
                    .map_err(|e| *e.current_context())
            }))
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

    async fn update_user_roles_by_user_id_org_id(
        &self,
        user_id: &str,
        org_id: &str,
        update: storage::UserRoleUpdate,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;
        let mut updated_user_roles = Vec::new();
        for user_role in user_roles.iter_mut() {
            if user_role.user_id == user_id && user_role.org_id == org_id {
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
                        user_role.status = status.to_owned();
                        user_role.last_modified_by = modified_by.to_owned();
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
    ) -> CustomResult<(), errors::StorageError> {
        let old_org_admin_user_roles = self
            .update_user_roles_by_user_id_org_id(
                from_user_id,
                org_id,
                storage::UserRoleUpdate::UpdateRole {
                    role_id: consts::user_role::ROLE_ID_MERCHANT_ADMIN.to_string(),
                    modified_by: from_user_id.to_string(),
                },
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
            )
            .await?;

        let new_org_admin_merchant_ids = new_org_admin_user_roles
            .iter()
            .map(|user_role| user_role.merchant_id.to_owned())
            .collect::<HashSet<String>>();

        let now = common_utils::date_time::now();

        let missing_new_user_roles = old_org_admin_user_roles
            .into_iter()
            .filter_map(|old_roles| {
                if !new_org_admin_merchant_ids.contains(&old_roles.merchant_id) {
                    Some(storage::UserRoleNew {
                        user_id: to_user_id.to_string(),
                        merchant_id: old_roles.merchant_id,
                        role_id: consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
                        org_id: org_id.to_string(),
                        status: enums::UserStatus::Active,
                        created_by: from_user_id.to_string(),
                        last_modified_by: from_user_id.to_string(),
                        created_at: now,
                        last_modified: now,
                    })
                } else {
                    None
                }
            });

        for user_role in missing_new_user_roles {
            self.insert_user_role(user_role).await?;
        }

        Ok(())
    }

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
    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
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
}
