use common_enums::enums;
use common_utils::id_type;
use diesel_models::role as storage;
use error_stack::report;
use router_env::{instrument, tracing};

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

    async fn find_role_by_role_id_in_merchant_scope(
        &self,
        role_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<storage::Role, errors::StorageError>;

    async fn find_role_by_role_id_in_org_scope(
        &self,
        role_id: &str,
        org_id: &id_type::OrganizationId,
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
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError>;

    async fn list_roles_for_org_by_parameters(
        &self,
        org_id: &id_type::OrganizationId,
        merchant_id: Option<&id_type::MerchantId>,
        profile_id: Option<&id_type::ProfileId>,
        entity_type: Option<enums::EntityType>,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError>;
}

#[async_trait::async_trait]
impl RoleInterface for Store {
    #[instrument(skip_all)]
    async fn insert_role(
        &self,
        role: storage::RoleNew,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        role.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::find_by_role_id(&conn, role_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_role_by_role_id_in_merchant_scope(
        &self,
        role_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::find_by_role_id_in_merchant_scope(&conn, role_id, merchant_id, org_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_role_by_role_id_in_org_scope(
        &self,
        role_id: &str,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::find_by_role_id_in_org_scope(&conn, role_id, org_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_role_by_role_id(
        &self,
        role_id: &str,
        role_update: storage::RoleUpdate,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Role::update_by_role_id(&conn, role_id, role_update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Role::delete_by_role_id(&conn, role_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_all_roles(
        &self,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::list_roles(&conn, merchant_id, org_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_roles_for_org_by_parameters(
        &self,
        org_id: &id_type::OrganizationId,
        merchant_id: Option<&id_type::MerchantId>,
        profile_id: Option<&id_type::ProfileId>,
        entity_type: Option<enums::EntityType>,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Role::generic_roles_list_for_org(
            &conn,
            org_id.to_owned(),
            merchant_id.cloned(),
            profile_id.cloned(),
            entity_type,
            limit,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
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
            role_name: role.role_name,
            role_id: role.role_id,
            merchant_id: role.merchant_id,
            org_id: role.org_id,
            groups: role.groups,
            scope: role.scope,
            entity_type: role.entity_type,
            created_by: role.created_by,
            created_at: role.created_at,
            last_modified_at: role.last_modified_at,
            last_modified_by: role.last_modified_by,
            profile_id: role.profile_id,
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

    async fn find_role_by_role_id_in_merchant_scope(
        &self,
        role_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let roles = self.roles.lock().await;
        roles
            .iter()
            .find(|role| {
                role.role_id == role_id
                    && (role.merchant_id == *merchant_id
                        || (role.org_id == *org_id && role.scope == enums::RoleScope::Organization))
            })
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No role available in merchant scope for role_id = {role_id}, \
                    merchant_id = {merchant_id:?} and org_id = {org_id:?}"
                ))
                .into(),
            )
    }

    async fn find_role_by_role_id_in_org_scope(
        &self,
        role_id: &str,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        let roles = self.roles.lock().await;
        roles
            .iter()
            .find(|role| role.role_id == role_id && role.org_id == *org_id)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No role available in org scope for role_id = {role_id} and org_id = {org_id:?}"
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
        roles
            .iter_mut()
            .find(|role| role.role_id == role_id)
            .map(|role| {
                *role = match role_update {
                    storage::RoleUpdate::UpdateDetails {
                        groups,
                        role_name,
                        last_modified_at,
                        last_modified_by,
                    } => storage::Role {
                        groups: groups.unwrap_or(role.groups.to_owned()),
                        role_name: role_name.unwrap_or(role.role_name.to_owned()),
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
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        let roles = self.roles.lock().await;

        let roles_list: Vec<_> = roles
            .iter()
            .filter(|role| {
                role.merchant_id == *merchant_id
                    || (role.org_id == *org_id
                        && role.scope == diesel_models::enums::RoleScope::Organization)
            })
            .cloned()
            .collect();

        if roles_list.is_empty() {
            return Err(errors::StorageError::ValueNotFound(format!(
                "No role found for merchant id = {:?} and org_id = {:?}",
                merchant_id, org_id
            ))
            .into());
        }

        Ok(roles_list)
    }

    #[instrument(skip_all)]
    async fn list_roles_for_org_by_parameters(
        &self,
        org_id: &id_type::OrganizationId,
        merchant_id: Option<&id_type::MerchantId>,
        profile_id: Option<&id_type::ProfileId>,
        entity_type: Option<enums::EntityType>,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        let roles = self.roles.lock().await;
        let limit_usize = limit.unwrap_or(u32::MAX).try_into().unwrap_or(usize::MAX);
        let roles_list: Vec<_> = roles
            .iter()
            .filter(|role| {
                let matches_merchant = match merchant_id {
                    Some(merchant_id) => {
                        role.merchant_id == *merchant_id
                            && role.scope == diesel_models::enums::RoleScope::Merchant
                    }
                    None => true,
                };

                let matches_profile = role
                    .profile_id
                    .as_ref()
                    .zip(profile_id)
                    .map(|(role_profile_id, user_profile_id)| {
                        user_profile_id == role_profile_id
                            && role.scope == diesel_models::enums::RoleScope::Profile
                    })
                    .unwrap_or(true);
                let matches_org_scope = role.scope == diesel_models::enums::RoleScope::Organization;

                (matches_profile || matches_merchant || matches_org_scope)
                    && role.org_id == *org_id
                    && role.entity_type == entity_type
            })
            .take(limit_usize)
            .cloned()
            .collect();

        Ok(roles_list)
    }
}
