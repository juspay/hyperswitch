use api_models::user_role as user_role_api;
use diesel_models::enums::UserStatus;
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    routes::AppState,
    services::authorization::{
        permissions::Permission,
        predefined_permissions::{self, RoleInfo},
    },
};

pub fn is_internal_role(role_id: &str) -> bool {
    role_id == consts::user_role::ROLE_ID_INTERNAL_ADMIN
        || role_id == consts::user_role::ROLE_ID_INTERNAL_VIEW_ONLY_USER
}

pub async fn get_merchant_ids_for_user(state: AppState, user_id: &str) -> UserResult<Vec<String>> {
    Ok(state
        .store
        .list_user_roles_by_user_id(user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .filter_map(|ele| {
            if ele.status == UserStatus::Active {
                return Some(ele.merchant_id);
            }
            None
        })
        .collect())
}

pub fn validate_role_id(role_id: &str) -> UserResult<()> {
    if predefined_permissions::is_role_invitable(role_id) {
        return Ok(());
    }
    Err(UserErrors::InvalidRoleId.into())
}

pub fn get_role_name_and_permission_response(
    role_info: &RoleInfo,
) -> Option<(Vec<user_role_api::Permission>, &'static str)> {
    role_info
        .get_permissions()
        .iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<user_role_api::Permission>, _>>()
        .ok()
        .zip(role_info.get_name())
}

impl TryFrom<&Permission> for user_role_api::Permission {
    type Error = ();
    fn try_from(value: &Permission) -> Result<Self, Self::Error> {
        match value {
            Permission::PaymentRead => Ok(Self::PaymentRead),
            Permission::PaymentWrite => Ok(Self::PaymentWrite),
            Permission::RefundRead => Ok(Self::RefundRead),
            Permission::RefundWrite => Ok(Self::RefundWrite),
            Permission::ApiKeyRead => Ok(Self::ApiKeyRead),
            Permission::ApiKeyWrite => Ok(Self::ApiKeyWrite),
            Permission::MerchantAccountRead => Ok(Self::MerchantAccountRead),
            Permission::MerchantAccountWrite => Ok(Self::MerchantAccountWrite),
            Permission::MerchantConnectorAccountRead => Ok(Self::MerchantConnectorAccountRead),
            Permission::MerchantConnectorAccountWrite => Ok(Self::MerchantConnectorAccountWrite),
            Permission::ForexRead => Ok(Self::ForexRead),
            Permission::RoutingRead => Ok(Self::RoutingRead),
            Permission::RoutingWrite => Ok(Self::RoutingWrite),
            Permission::DisputeRead => Ok(Self::DisputeRead),
            Permission::DisputeWrite => Ok(Self::DisputeWrite),
            Permission::MandateRead => Ok(Self::MandateRead),
            Permission::MandateWrite => Ok(Self::MandateWrite),
            Permission::CustomerRead => Ok(Self::CustomerRead),
            Permission::CustomerWrite => Ok(Self::CustomerWrite),
            Permission::FileRead => Ok(Self::FileRead),
            Permission::FileWrite => Ok(Self::FileWrite),
            Permission::Analytics => Ok(Self::Analytics),
            Permission::ThreeDsDecisionManagerWrite => Ok(Self::ThreeDsDecisionManagerWrite),
            Permission::ThreeDsDecisionManagerRead => Ok(Self::ThreeDsDecisionManagerRead),
            Permission::SurchargeDecisionManagerWrite => Ok(Self::SurchargeDecisionManagerWrite),
            Permission::SurchargeDecisionManagerRead => Ok(Self::SurchargeDecisionManagerRead),
            Permission::UsersRead => Ok(Self::UsersRead),
            Permission::UsersWrite => Ok(Self::UsersWrite),

            Permission::MerchantAccountCreate => {
                logger::error!("Invalid use of internal permission");
                Err(())
            }
        }
    }
}
