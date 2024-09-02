use common_utils::id_type;
use diesel_models::{
    enums::{self, UserStatus},
    user_role as storage,
};
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

pub enum InsertUserRolePayload {
    OnlyV1(storage::UserRoleNew),
    OnlyV2(storage::UserRoleNew),
    V1AndV2(Box<[storage::UserRoleNew; 2]>),
}

impl InsertUserRolePayload {
    fn convert_to_vec(self) -> Vec<storage::UserRoleNew> {
        match self {
            Self::OnlyV1(user_role) | Self::OnlyV2(user_role) => vec![user_role],
            Self::V1AndV2(user_roles) => user_roles.to_vec(),
        }
    }
}

pub struct ListUserRolesByOrgIdPayload<'a> {
    pub user_id: Option<&'a String>,
    pub org_id: &'a id_type::OrganizationId,
    pub merchant_id: Option<&'a id_type::MerchantId>,
    pub profile_id: Option<&'a id_type::ProfileId>,
    pub version: Option<enums::UserRoleVersion>,
}

#[async_trait::async_trait]
pub trait UserRoleInterface {
    async fn insert_user_role(
        &self,
        user_role: InsertUserRolePayload,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

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

    async fn list_user_roles_by_user_id_and_version(
        &self,
        user_id: &str,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

    async fn list_user_roles_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

    async fn find_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: Option<&id_type::ProfileId>,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn update_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: Option<&id_type::ProfileId>,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn delete_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: Option<&id_type::ProfileId>,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError>;

    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
        org_id: Option<&id_type::OrganizationId>,
        merchant_id: Option<&id_type::MerchantId>,
        profile_id: Option<&id_type::ProfileId>,
        entity_id: Option<&String>,
        version: Option<enums::UserRoleVersion>,
        status: Option<UserStatus>,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;

    async fn list_user_roles_by_org_id<'a>(
        &self,
        payload: ListUserRolesByOrgIdPayload<'a>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError>;
}

#[async_trait::async_trait]
impl UserRoleInterface for Store {
    #[instrument(skip_all)]
    async fn insert_user_role(
        &self,
        user_role: InsertUserRolePayload,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        storage::UserRole::insert_multiple_user_roles(&conn, user_role.convert_to_vec())
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

    async fn list_user_roles_by_user_id_and_version(
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

    #[instrument(skip_all)]
    async fn find_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: Option<&id_type::ProfileId>,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::find_by_user_id_org_id_merchant_id_profile_id(
            &conn,
            user_id.to_owned(),
            org_id.to_owned(),
            merchant_id.to_owned(),
            profile_id.cloned(),
            version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: Option<&id_type::ProfileId>,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::update_by_user_id_org_id_merchant_id_profile_id(
            &conn,
            user_id.to_owned(),
            org_id.to_owned(),
            merchant_id.to_owned(),
            profile_id.cloned(),
            update,
            version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: Option<&id_type::ProfileId>,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UserRole::delete_by_user_id_org_id_merchant_id_profile_id(
            &conn,
            user_id.to_owned(),
            org_id.to_owned(),
            merchant_id.to_owned(),
            profile_id.cloned(),
            version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
        org_id: Option<&id_type::OrganizationId>,
        merchant_id: Option<&id_type::MerchantId>,
        profile_id: Option<&id_type::ProfileId>,
        entity_id: Option<&String>,
        version: Option<enums::UserRoleVersion>,
        status: Option<UserStatus>,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserRole::generic_user_roles_list_for_user(
            &conn,
            user_id.to_owned(),
            org_id.cloned(),
            merchant_id.cloned(),
            profile_id.cloned(),
            entity_id.cloned(),
            status,
            version,
            limit,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn list_user_roles_by_org_id<'a>(
        &self,
        payload: ListUserRolesByOrgIdPayload<'a>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::UserRole::generic_user_roles_list_for_org_and_extra(
            &conn,
            payload.user_id.cloned(),
            payload.org_id.to_owned(),
            payload.merchant_id.cloned(),
            payload.profile_id.cloned(),
            payload.version,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl UserRoleInterface for MockDb {
    async fn insert_user_role(
        &self,
        user_role: InsertUserRolePayload,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let mut db_user_roles = self.user_roles.lock().await;

        user_role
            .convert_to_vec()
            .into_iter()
            .map(|user_role| {
                if db_user_roles
                    .iter()
                    .any(|user_role_inner| user_role_inner.user_id == user_role.user_id)
                {
                    Err(errors::StorageError::DuplicateValue {
                        entity: "user_id",
                        key: None,
                    })?
                }
                let user_role = storage::UserRole {
                    id: i32::try_from(db_user_roles.len())
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
                db_user_roles.push(user_role.clone());
                Ok(user_role)
            })
            .collect::<Result<Vec<_>, _>>()
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
                continue;
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

    async fn list_user_roles_by_user_id_and_version(
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

    async fn find_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: Option<&id_type::ProfileId>,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;

        for user_role in user_roles.iter() {
            let org_level_check = user_role.org_id.as_ref() == Some(org_id)
                && user_role.merchant_id.is_none()
                && user_role.profile_id.is_none();

            let merchant_level_check = user_role.org_id.as_ref() == Some(org_id)
                && user_role.merchant_id.as_ref() == Some(merchant_id)
                && user_role.profile_id.is_none();

            let profile_level_check = user_role.org_id.as_ref() == Some(org_id)
                && user_role.merchant_id.as_ref() == Some(merchant_id)
                && user_role.profile_id.as_ref() == profile_id;

            // Check if any condition matches and the version matches
            if user_role.user_id == user_id
                && (org_level_check || merchant_level_check || profile_level_check)
                && user_role.version == version
            {
                return Ok(user_role.clone());
            }
        }

        Err(errors::StorageError::ValueNotFound(format!(
            "No user role available for user_id = {} in the current token hierarchy",
            user_id
        ))
        .into())
    }

    async fn update_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: Option<&id_type::ProfileId>,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;

        for user_role in user_roles.iter_mut() {
            let org_level_check = user_role.org_id.as_ref() == Some(org_id)
                && user_role.merchant_id.is_none()
                && user_role.profile_id.is_none();

            let merchant_level_check = user_role.org_id.as_ref() == Some(org_id)
                && user_role.merchant_id.as_ref() == Some(merchant_id)
                && user_role.profile_id.is_none();

            let profile_level_check = user_role.org_id.as_ref() == Some(org_id)
                && user_role.merchant_id.as_ref() == Some(merchant_id)
                && user_role.profile_id.as_ref() == profile_id;

            // Check if the user role matches the conditions and the version matches
            if user_role.user_id == user_id
                && (org_level_check || merchant_level_check || profile_level_check)
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
                }
                return Ok(user_role.clone());
            }
        }
        Err(
            errors::StorageError::ValueNotFound("Cannot find user role to update".to_string())
                .into(),
        )
    }

    async fn delete_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: Option<&id_type::ProfileId>,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        let mut user_roles = self.user_roles.lock().await;

        // Find the position of the user role to delete
        let index = user_roles.iter().position(|role| {
            let org_level_check = role.org_id.as_ref() == Some(org_id)
                && role.merchant_id.is_none()
                && role.profile_id.is_none();

            let merchant_level_check = role.org_id.as_ref() == Some(org_id)
                && role.merchant_id.as_ref() == Some(merchant_id)
                && role.profile_id.is_none();

            let profile_level_check = role.org_id.as_ref() == Some(org_id)
                && role.merchant_id.as_ref() == Some(merchant_id)
                && role.profile_id.as_ref() == profile_id;

            // Check if the user role matches the conditions and the version matches
            role.user_id == user_id
                && (org_level_check || merchant_level_check || profile_level_check)
                && role.version == version
        });

        // Remove and return the user role if found
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
        org_id: Option<&id_type::OrganizationId>,
        merchant_id: Option<&id_type::MerchantId>,
        profile_id: Option<&id_type::ProfileId>,
        entity_id: Option<&String>,
        version: Option<enums::UserRoleVersion>,
        status: Option<UserStatus>,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;

        let mut filtered_roles: Vec<_> = user_roles
            .iter()
            .filter_map(|role| {
                let mut filter_condition = role.user_id == user_id;

                role.org_id
                    .as_ref()
                    .zip(org_id)
                    .inspect(|(role_org_id, org_id)| {
                        filter_condition = filter_condition && role_org_id == org_id
                    });
                role.merchant_id.as_ref().zip(merchant_id).inspect(
                    |(role_merchant_id, merchant_id)| {
                        filter_condition = filter_condition && role_merchant_id == merchant_id
                    },
                );
                role.profile_id.as_ref().zip(profile_id).inspect(
                    |(role_profile_id, profile_id)| {
                        filter_condition = filter_condition && role_profile_id == profile_id
                    },
                );
                role.entity_id
                    .as_ref()
                    .zip(entity_id)
                    .inspect(|(role_entity_id, entity_id)| {
                        filter_condition = filter_condition && role_entity_id == entity_id
                    });
                version.inspect(|ver| filter_condition = filter_condition && ver == &role.version);
                status.inspect(|status| {
                    filter_condition = filter_condition && status == &role.status
                });

                filter_condition.then(|| role.to_owned())
            })
            .collect();

        if let Some(Ok(limit)) = limit.map(|val| val.try_into()) {
            filtered_roles = filtered_roles.into_iter().take(limit).collect();
        }
        Ok(filtered_roles)
    }

    async fn list_user_roles_by_org_id<'a>(
        &self,
        payload: ListUserRolesByOrgIdPayload<'a>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        let user_roles = self.user_roles.lock().await;

        let mut filtered_roles = Vec::new();

        for role in user_roles.iter() {
            let role_org_id = role
                .org_id
                .as_ref()
                .ok_or(report!(errors::StorageError::MockDbError))?;

            let mut filter_condition = role_org_id == payload.org_id;

            if let Some(user_id) = payload.user_id {
                filter_condition = filter_condition && user_id == &role.user_id
            }

            role.merchant_id.as_ref().zip(payload.merchant_id).inspect(
                |(role_merchant_id, merchant_id)| {
                    filter_condition = filter_condition && role_merchant_id == merchant_id
                },
            );

            role.profile_id.as_ref().zip(payload.profile_id).inspect(
                |(role_profile_id, profile_id)| {
                    filter_condition = filter_condition && role_profile_id == profile_id
                },
            );

            payload
                .version
                .inspect(|ver| filter_condition = filter_condition && ver == &role.version);

            if filter_condition {
                filtered_roles.push(role.clone())
            }
        }

        Ok(filtered_roles)
    }
}
