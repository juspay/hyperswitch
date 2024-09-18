use std::collections::HashSet;

use api_models::user_role as user_role_api;
use common_enums::{EntityType, PermissionGroup};
use common_utils::id_type;
use diesel_models::{
    enums::{UserRoleVersion, UserStatus},
    user_role::{UserRole, UserRoleUpdate},
};
use error_stack::{report, Report, ResultExt};
use router_env::logger;
use storage_impl::errors::StorageError;

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    db::user_role::ListUserRolesByUserIdPayload,
    routes::SessionState,
    services::authorization::{self as authz, permissions::Permission, roles},
    types::domain,
};

impl From<Permission> for user_role_api::Permission {
    fn from(value: Permission) -> Self {
        match value {
            Permission::PaymentRead => Self::PaymentRead,
            Permission::PaymentWrite => Self::PaymentWrite,
            Permission::RefundRead => Self::RefundRead,
            Permission::RefundWrite => Self::RefundWrite,
            Permission::ApiKeyRead => Self::ApiKeyRead,
            Permission::ApiKeyWrite => Self::ApiKeyWrite,
            Permission::MerchantAccountRead => Self::MerchantAccountRead,
            Permission::MerchantAccountWrite => Self::MerchantAccountWrite,
            Permission::MerchantConnectorAccountRead => Self::MerchantConnectorAccountRead,
            Permission::MerchantConnectorAccountWrite => Self::MerchantConnectorAccountWrite,
            Permission::RoutingRead => Self::RoutingRead,
            Permission::RoutingWrite => Self::RoutingWrite,
            Permission::DisputeRead => Self::DisputeRead,
            Permission::DisputeWrite => Self::DisputeWrite,
            Permission::MandateRead => Self::MandateRead,
            Permission::MandateWrite => Self::MandateWrite,
            Permission::CustomerRead => Self::CustomerRead,
            Permission::CustomerWrite => Self::CustomerWrite,
            Permission::Analytics => Self::Analytics,
            Permission::ThreeDsDecisionManagerWrite => Self::ThreeDsDecisionManagerWrite,
            Permission::ThreeDsDecisionManagerRead => Self::ThreeDsDecisionManagerRead,
            Permission::SurchargeDecisionManagerWrite => Self::SurchargeDecisionManagerWrite,
            Permission::SurchargeDecisionManagerRead => Self::SurchargeDecisionManagerRead,
            Permission::UsersRead => Self::UsersRead,
            Permission::UsersWrite => Self::UsersWrite,
            Permission::MerchantAccountCreate => Self::MerchantAccountCreate,
            Permission::WebhookEventRead => Self::WebhookEventRead,
            Permission::WebhookEventWrite => Self::WebhookEventWrite,
            Permission::PayoutRead => Self::PayoutRead,
            Permission::PayoutWrite => Self::PayoutWrite,
            Permission::GenerateReport => Self::GenerateReport,
            Permission::ReconAdmin => Self::ReconAdmin,
        }
    }
}

pub fn validate_role_groups(groups: &[PermissionGroup]) -> UserResult<()> {
    if groups.is_empty() {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("Role groups cannot be empty");
    }

    let unique_groups: HashSet<_> = groups.iter().cloned().collect();

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
) -> UserResult<()> {
    let role_name_str = role_name.clone().get_role_name();

    let is_present_in_predefined_roles = roles::predefined_roles::PREDEFINED_ROLES
        .iter()
        .any(|(_, role_info)| role_info.get_role_name() == role_name_str);

    // TODO: Create and use find_by_role_name to make this efficient
    let is_present_in_custom_roles = state
        .store
        .list_all_roles(merchant_id, org_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .iter()
        .any(|role| role.role_name == role_name_str);

    if is_present_in_predefined_roles || is_present_in_custom_roles {
        return Err(UserErrors::RoleNameAlreadyExists.into());
    }

    Ok(())
}

pub async fn set_role_permissions_in_cache_by_user_role(
    state: &SessionState,
    user_role: &UserRole,
) -> bool {
    let Some(ref merchant_id) = user_role.merchant_id else {
        return false;
    };

    let Some(ref org_id) = user_role.org_id else {
        return false;
    };
    set_role_permissions_in_cache_if_required(
        state,
        user_role.role_id.as_str(),
        merchant_id,
        org_id,
    )
    .await
    .map_err(|e| logger::error!("Error setting permissions in cache {:?}", e))
    .is_ok()
}

pub async fn set_role_permissions_in_cache_by_role_id_merchant_id_org_id(
    state: &SessionState,
    role_id: &str,
    merchant_id: &id_type::MerchantId,
    org_id: &id_type::OrganizationId,
) -> bool {
    set_role_permissions_in_cache_if_required(state, role_id, merchant_id, org_id)
        .await
        .map_err(|e| logger::error!("Error setting permissions in cache {:?}", e))
        .is_ok()
}

pub async fn set_role_permissions_in_cache_if_required(
    state: &SessionState,
    role_id: &str,
    merchant_id: &id_type::MerchantId,
    org_id: &id_type::OrganizationId,
) -> UserResult<()> {
    if roles::predefined_roles::PREDEFINED_ROLES.contains_key(role_id) {
        return Ok(());
    }

    let role_info =
        roles::RoleInfo::from_role_id_in_merchant_scope(state, role_id, merchant_id, org_id)
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
    org_id: &id_type::OrganizationId,
    merchant_id: &id_type::MerchantId,
    profile_id: Option<&id_type::ProfileId>,
    update: UserRoleUpdate,
) -> (
    Result<UserRole, Report<StorageError>>,
    Result<UserRole, Report<StorageError>>,
) {
    let updated_v1_role = state
        .store
        .update_user_role_by_user_id_and_lineage(
            user_id,
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
        .store
        .update_user_role_by_user_id_and_lineage(
            user_id,
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

pub async fn get_single_merchant_id(
    state: &SessionState,
    user_role: &UserRole,
) -> UserResult<id_type::MerchantId> {
    match user_role.entity_type {
        Some(EntityType::Organization) => Ok(state
            .store
            .list_merchant_accounts_by_organization_id(
                &state.into(),
                user_role
                    .org_id
                    .as_ref()
                    .ok_or(UserErrors::InternalServerError)
                    .attach_printable("org_id not found")?,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to get merchant list for org")?
            .first()
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("No merchants found for org_id")?
            .get_id()
            .clone()),
        Some(EntityType::Merchant)
        | Some(EntityType::Internal)
        | Some(EntityType::Profile)
        | None => user_role
            .merchant_id
            .clone()
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("merchant_id not found"),
    }
}

pub async fn get_lineage_for_user_id_and_entity_for_accepting_invite(
    state: &SessionState,
    user_id: &str,
    entity_id: String,
    entity_type: EntityType,
) -> UserResult<
    Option<(
        id_type::OrganizationId,
        id_type::MerchantId,
        Option<id_type::ProfileId>,
    )>,
> {
    match entity_type {
        EntityType::Internal | EntityType::Organization => {
            Err(UserErrors::InvalidRoleOperation.into())
        }
        EntityType::Merchant => {
            let Ok(merchant_id) = id_type::MerchantId::wrap(entity_id) else {
                return Ok(None);
            };

            let user_roles = state
                .store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id,
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
                    merchant_id,
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
                .store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id,
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
                    user_role
                        .merchant_id
                        .ok_or(UserErrors::InternalServerError)?,
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
    let merchant_id = get_single_merchant_id(state, user_role).await?;
    let (_, entity_type) = user_role
        .get_entity_id_and_type()
        .ok_or(UserErrors::InternalServerError)?;
    let profile_id = match entity_type {
        EntityType::Organization | EntityType::Merchant | EntityType::Internal => {
            let key_store = state
                .store
                .get_merchant_key_store_by_merchant_id(
                    &state.into(),
                    &merchant_id,
                    &state.store.get_master_key().to_vec().into(),
                )
                .await
                .change_context(UserErrors::InternalServerError)?;

            state
                .store
                .list_business_profile_by_merchant_id(&state.into(), &key_store, &merchant_id)
                .await
                .change_context(UserErrors::InternalServerError)?
                .pop()
                .ok_or(UserErrors::InternalServerError)?
                .get_id()
                .to_owned()
        }
        EntityType::Profile => user_role
            .profile_id
            .clone()
            .ok_or(UserErrors::InternalServerError)?,
    };

    Ok((merchant_id, profile_id))
}
