use std::{cmp, collections::HashSet};

use common_enums::{EntityType, PermissionGroup};
use common_utils::id_type;
use diesel_models::{
    enums::{UserRoleVersion, UserStatus},
    role::ListRolesByEntityPayload,
    user_role::{UserRole, UserRoleUpdate},
};
use error_stack::{report, Report, ResultExt};
use router_env::logger;
use storage_impl::errors::StorageError;

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    db::{
        errors::StorageErrorExt,
        user_role::{ListUserRolesByOrgIdPayload, ListUserRolesByUserIdPayload},
    },
    routes::SessionState,
    services::authorization::{self as authz, roles},
    types::domain,
};

pub fn validate_role_groups(groups: &[PermissionGroup]) -> UserResult<()> {
    if groups.is_empty() {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("Role groups cannot be empty");
    }

    let unique_groups: HashSet<_> = groups.iter().copied().collect();

    if unique_groups.contains(&PermissionGroup::OrganizationManage) {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("Organization manage group cannot be added to role");
    }

    if unique_groups.len() != groups.len() {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("Duplicate permission group found");
    }

    Ok(())
}

pub async fn validate_role_name(
    state: &SessionState,
    role_name: &domain::RoleName,
    merchant_id: &id_type::MerchantId,
    org_id: &id_type::OrganizationId,
    tenant_id: &id_type::TenantId,
    profile_id: &id_type::ProfileId,
    entity_type: &EntityType,
) -> UserResult<()> {
    let role_name_str = role_name.clone().get_role_name();

    let is_present_in_predefined_roles = roles::predefined_roles::PREDEFINED_ROLES
        .iter()
        .any(|(_, role_info)| role_info.get_role_name() == role_name_str);

    let entity_type_for_role = match entity_type {
        EntityType::Tenant | EntityType::Organization => ListRolesByEntityPayload::Organization,
        EntityType::Merchant => ListRolesByEntityPayload::Merchant(merchant_id.to_owned()),
        EntityType::Profile => {
            ListRolesByEntityPayload::Profile(merchant_id.to_owned(), profile_id.to_owned())
        }
    };

    let is_present_in_custom_role = match state
        .global_store
        .generic_list_roles_by_entity_type(
            entity_type_for_role,
            false,
            tenant_id.to_owned(),
            org_id.to_owned(),
        )
        .await
    {
        Ok(roles_list) => roles_list
            .iter()
            .any(|role| role.role_name == role_name_str),
        Err(e) => {
            if e.current_context().is_db_not_found() {
                false
            } else {
                return Err(UserErrors::InternalServerError.into());
            }
        }
    };

    if is_present_in_predefined_roles || is_present_in_custom_role {
        return Err(UserErrors::RoleNameAlreadyExists.into());
    }

    Ok(())
}

pub async fn set_role_info_in_cache_by_user_role(
    state: &SessionState,
    user_role: &UserRole,
) -> bool {
    let Some(ref org_id) = user_role.org_id else {
        return false;
    };
    set_role_info_in_cache_if_required(
        state,
        user_role.role_id.as_str(),
        org_id,
        &user_role.tenant_id,
    )
    .await
    .map_err(|e| logger::error!("Error setting permissions in cache {:?}", e))
    .is_ok()
}

pub async fn set_role_info_in_cache_by_role_id_org_id(
    state: &SessionState,
    role_id: &str,
    org_id: &id_type::OrganizationId,
    tenant_id: &id_type::TenantId,
) -> bool {
    set_role_info_in_cache_if_required(state, role_id, org_id, tenant_id)
        .await
        .map_err(|e| logger::error!("Error setting permissions in cache {:?}", e))
        .is_ok()
}

pub async fn set_role_info_in_cache_if_required(
    state: &SessionState,
    role_id: &str,
    org_id: &id_type::OrganizationId,
    tenant_id: &id_type::TenantId,
) -> UserResult<()> {
    if roles::predefined_roles::PREDEFINED_ROLES.contains_key(role_id) {
        return Ok(());
    }

    let role_info =
        roles::RoleInfo::from_role_id_org_id_tenant_id(state, role_id, org_id, tenant_id)
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error getting role_info from role_id")?;

    authz::set_role_info_in_cache(
        state,
        role_id,
        &role_info,
        i64::try_from(consts::JWT_TOKEN_TIME_IN_SECS)
            .change_context(UserErrors::InternalServerError)?,
    )
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Error setting permissions in redis")
}

pub async fn update_v1_and_v2_user_roles_in_db(
    state: &SessionState,
    user_id: &str,
    tenant_id: &id_type::TenantId,
    org_id: &id_type::OrganizationId,
    merchant_id: Option<&id_type::MerchantId>,
    profile_id: Option<&id_type::ProfileId>,
    update: UserRoleUpdate,
) -> (
    Result<UserRole, Report<StorageError>>,
    Result<UserRole, Report<StorageError>>,
) {
    let updated_v1_role = state
        .global_store
        .update_user_role_by_user_id_and_lineage(
            user_id,
            tenant_id,
            org_id,
            merchant_id,
            profile_id,
            update.clone(),
            UserRoleVersion::V1,
        )
        .await
        .map_err(|e| {
            logger::error!("Error updating user_role {e:?}");
            e
        });

    let updated_v2_role = state
        .global_store
        .update_user_role_by_user_id_and_lineage(
            user_id,
            tenant_id,
            org_id,
            merchant_id,
            profile_id,
            update,
            UserRoleVersion::V2,
        )
        .await
        .map_err(|e| {
            logger::error!("Error updating user_role {e:?}");
            e
        });

    (updated_v1_role, updated_v2_role)
}

pub async fn get_single_org_id(
    state: &SessionState,
    user_role: &UserRole,
) -> UserResult<id_type::OrganizationId> {
    let (_, entity_type) = user_role
        .get_entity_id_and_type()
        .ok_or(UserErrors::InternalServerError)?;
    match entity_type {
        EntityType::Tenant => Ok(state
            .store
            .list_merchant_and_org_ids(&state.into(), 1, None)
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to get merchants list for org")?
            .pop()
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("No merchants to get merchant or org id")?
            .1),
        EntityType::Organization | EntityType::Merchant | EntityType::Profile => user_role
            .org_id
            .clone()
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("Org_id not found"),
    }
}

pub async fn get_single_merchant_id(
    state: &SessionState,
    user_role: &UserRole,
    org_id: &id_type::OrganizationId,
) -> UserResult<id_type::MerchantId> {
    let (_, entity_type) = user_role
        .get_entity_id_and_type()
        .ok_or(UserErrors::InternalServerError)?;
    match entity_type {
        EntityType::Tenant | EntityType::Organization => Ok(state
            .store
            .list_merchant_accounts_by_organization_id(&state.into(), org_id)
            .await
            .to_not_found_response(UserErrors::InvalidRoleOperationWithMessage(
                "Invalid Org Id".to_string(),
            ))?
            .first()
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("No merchants found for org_id")?
            .get_id()
            .clone()),
        EntityType::Merchant | EntityType::Profile => user_role
            .merchant_id
            .clone()
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("merchant_id not found"),
    }
}

pub async fn get_single_profile_id(
    state: &SessionState,
    user_role: &UserRole,
    merchant_id: &id_type::MerchantId,
) -> UserResult<id_type::ProfileId> {
    let (_, entity_type) = user_role
        .get_entity_id_and_type()
        .ok_or(UserErrors::InternalServerError)?;
    match entity_type {
        EntityType::Tenant | EntityType::Organization | EntityType::Merchant => {
            let key_store = state
                .store
                .get_merchant_key_store_by_merchant_id(
                    &state.into(),
                    merchant_id,
                    &state.store.get_master_key().to_vec().into(),
                )
                .await
                .change_context(UserErrors::InternalServerError)?;

            Ok(state
                .store
                .list_profile_by_merchant_id(&state.into(), &key_store, merchant_id)
                .await
                .change_context(UserErrors::InternalServerError)?
                .pop()
                .ok_or(UserErrors::InternalServerError)?
                .get_id()
                .to_owned())
        }
        EntityType::Profile => user_role
            .profile_id
            .clone()
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("profile_id not found"),
    }
}

pub async fn get_lineage_for_user_id_and_entity_for_accepting_invite(
    state: &SessionState,
    user_id: &str,
    tenant_id: &id_type::TenantId,
    entity_id: String,
    entity_type: EntityType,
) -> UserResult<
    Option<(
        id_type::OrganizationId,
        Option<id_type::MerchantId>,
        Option<id_type::ProfileId>,
    )>,
> {
    match entity_type {
        EntityType::Tenant => Err(UserErrors::InvalidRoleOperationWithMessage(
            "Tenant roles are not allowed for this operation".to_string(),
        )
        .into()),
        EntityType::Organization => {
            let Ok(org_id) =
                id_type::OrganizationId::try_from(std::borrow::Cow::from(entity_id.clone()))
            else {
                return Ok(None);
            };

            let user_roles = state
                .global_store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id,
                    tenant_id,
                    org_id: Some(&org_id),
                    merchant_id: None,
                    profile_id: None,
                    entity_id: None,
                    version: None,
                    status: Some(UserStatus::InvitationSent),
                    limit: None,
                })
                .await
                .change_context(UserErrors::InternalServerError)?
                .into_iter()
                .collect::<HashSet<_>>();

            if user_roles.len() > 1 {
                return Ok(None);
            }

            if let Some(user_role) = user_roles.into_iter().next() {
                let (_entity_id, entity_type) = user_role
                    .get_entity_id_and_type()
                    .ok_or(UserErrors::InternalServerError)?;

                if entity_type != EntityType::Organization {
                    return Ok(None);
                }

                return Ok(Some((
                    user_role.org_id.ok_or(UserErrors::InternalServerError)?,
                    None,
                    None,
                )));
            }

            Ok(None)
        }
        EntityType::Merchant => {
            let Ok(merchant_id) = id_type::MerchantId::wrap(entity_id) else {
                return Ok(None);
            };

            let user_roles = state
                .global_store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id,
                    tenant_id,
                    org_id: None,
                    merchant_id: Some(&merchant_id),
                    profile_id: None,
                    entity_id: None,
                    version: None,
                    status: Some(UserStatus::InvitationSent),
                    limit: None,
                })
                .await
                .change_context(UserErrors::InternalServerError)?
                .into_iter()
                .collect::<HashSet<_>>();

            if user_roles.len() > 1 {
                return Ok(None);
            }

            if let Some(user_role) = user_roles.into_iter().next() {
                let (_entity_id, entity_type) = user_role
                    .get_entity_id_and_type()
                    .ok_or(UserErrors::InternalServerError)?;

                if entity_type != EntityType::Merchant {
                    return Ok(None);
                }

                return Ok(Some((
                    user_role.org_id.ok_or(UserErrors::InternalServerError)?,
                    Some(merchant_id),
                    None,
                )));
            }

            Ok(None)
        }
        EntityType::Profile => {
            let Ok(profile_id) = id_type::ProfileId::try_from(std::borrow::Cow::from(entity_id))
            else {
                return Ok(None);
            };

            let user_roles = state
                .global_store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id,
                    tenant_id: &state.tenant.tenant_id,
                    org_id: None,
                    merchant_id: None,
                    profile_id: Some(&profile_id),
                    entity_id: None,
                    version: None,
                    status: Some(UserStatus::InvitationSent),
                    limit: None,
                })
                .await
                .change_context(UserErrors::InternalServerError)?
                .into_iter()
                .collect::<HashSet<_>>();

            if user_roles.len() > 1 {
                return Ok(None);
            }

            if let Some(user_role) = user_roles.into_iter().next() {
                let (_entity_id, entity_type) = user_role
                    .get_entity_id_and_type()
                    .ok_or(UserErrors::InternalServerError)?;

                if entity_type != EntityType::Profile {
                    return Ok(None);
                }

                return Ok(Some((
                    user_role.org_id.ok_or(UserErrors::InternalServerError)?,
                    Some(
                        user_role
                            .merchant_id
                            .ok_or(UserErrors::InternalServerError)?,
                    ),
                    Some(profile_id),
                )));
            }

            Ok(None)
        }
    }
}

pub async fn get_single_merchant_id_and_profile_id(
    state: &SessionState,
    user_role: &UserRole,
) -> UserResult<(id_type::MerchantId, id_type::ProfileId)> {
    let org_id = get_single_org_id(state, user_role).await?;
    let merchant_id = get_single_merchant_id(state, user_role, &org_id).await?;
    let profile_id = get_single_profile_id(state, user_role, &merchant_id).await?;

    Ok((merchant_id, profile_id))
}

pub async fn fetch_user_roles_by_payload(
    state: &SessionState,
    payload: ListUserRolesByOrgIdPayload<'_>,
    request_entity_type: Option<EntityType>,
) -> UserResult<HashSet<UserRole>> {
    Ok(state
        .global_store
        .list_user_roles_by_org_id(payload)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .filter_map(|user_role| {
            let (_entity_id, entity_type) = user_role.get_entity_id_and_type()?;
            request_entity_type
                .map_or(true, |req_entity_type| entity_type == req_entity_type)
                .then_some(user_role)
        })
        .collect::<HashSet<_>>())
}

pub fn get_min_entity(
    user_entity: EntityType,
    filter_entity: Option<EntityType>,
) -> UserResult<EntityType> {
    let Some(filter_entity) = filter_entity else {
        return Ok(user_entity);
    };

    if user_entity < filter_entity {
        return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
            "{} level user requesting data for {:?} level",
            user_entity, filter_entity
        ));
    }

    Ok(cmp::min(user_entity, filter_entity))
}
