use api_models::external_service_auth as external_service_auth_api;
use common_enums::Resource;
use common_utils::fp_utils;
use error_stack::ResultExt;
use hyperswitch_masking::ExposeInterface;
use strum::IntoEnumIterator;

use crate::{
    core::{
        configs::dimension_state::Dimensions,
        errors::{self, RouterResponse, RouterResult},
        offer_engine,
    },
    services::{
        api as service_api,
        authentication::{self, blacklist::BlackList, ExternalServiceType, ExternalToken},
        authorization::{self, permissions::Permission, roles},
    },
    SessionState,
};

const OFFER_ENGINE_CONTEXT: &str = "MERCHANT";

pub async fn generate_external_token(
    state: SessionState,
    user: authentication::UserFromToken,
    external_service_type: ExternalServiceType,
) -> RouterResponse<external_service_auth_api::ExternalTokenResponse> {
    let token = ExternalToken::new_token(
        user.user_id.clone(),
        user.merchant_id.clone(),
        &state.conf,
        external_service_type.clone(),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "Failed to create external token for params [user_id, mid, external_service_type] [{}, {:?}, {:?}]",
            user.user_id, user.merchant_id, external_service_type,
        )
    })?;

    Ok(service_api::ApplicationResponse::Json(
        external_service_auth_api::ExternalTokenResponse {
            token: token.into(),
        },
    ))
}

pub async fn signout_external_token(
    state: SessionState,
    json_payload: external_service_auth_api::ExternalSignoutTokenRequest,
) -> RouterResponse<()> {
    let token = authentication::decode_jwt::<ExternalToken>(&json_payload.token.expose(), &state)
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)?;

    authentication::blacklist::insert_user_in_blacklist(&state, &token.user_id)
        .await
        .change_context(errors::ApiErrorResponse::InvalidJwtToken)?;

    Ok(service_api::ApplicationResponse::StatusOk)
}

pub async fn verify_external_token(
    state: SessionState,
    json_payload: external_service_auth_api::ExternalVerifyTokenRequest,
    external_service_type: ExternalServiceType,
) -> RouterResponse<external_service_auth_api::ExternalVerifyTokenResponse> {
    let token_from_payload = json_payload.token.expose();

    let token = authentication::decode_jwt::<ExternalToken>(&token_from_payload, &state)
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)?;

    fp_utils::when(
        authentication::blacklist::check_user_in_blacklist(&state, &token.user_id, token.exp)
            .await?,
        || Err(errors::ApiErrorResponse::InvalidJwtToken),
    )?;

    token.check_service_type(&external_service_type)?;

    let user_in_db = state
        .global_store
        .find_active_user_by_user_id(&token.user_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("User not found in database")?;

    let email = user_in_db.email.clone();
    let name = user_in_db.name;

    Ok(service_api::ApplicationResponse::Json(
        external_service_auth_api::ExternalVerifyTokenResponse::Hypersense {
            user_id: user_in_db.user_id,
            merchant_id: token.merchant_id,
            name,
            email,
        },
    ))
}

/// Validates a dashboard token for an external service and returns the identity and
/// permissions behind it.
pub async fn validate_token(
    state: SessionState,
    req: external_service_auth_api::ValidateTokenRequest,
) -> RouterResponse<external_service_auth_api::ExternalVerifyTokenResponse> {
    let token = req.token.expose();

    let payload = authentication::decode_jwt::<authentication::AuthToken>(&token, &state).await?;

    fp_utils::when(payload.check_in_blacklist(&state).await?, || {
        Err(errors::ApiErrorResponse::InvalidJwtToken)
    })?;

    authorization::check_tenant(payload.tenant_id.clone(), &state.tenant.tenant_id)?;

    let role_info = authorization::get_role_info(&state, &payload).await?;

    match req.service {
        external_service_auth_api::ValidatingService::OfferEngine => {
            let merchant_id = resolve_offer_engine_merchant_id(&state, &payload).await?;

            Ok(service_api::ApplicationResponse::Json(
                external_service_auth_api::ExternalVerifyTokenResponse::OfferEngine {
                    merchant_id,
                    context: OFFER_ENGINE_CONTEXT.to_string(),
                    token: token.into(),
                    permissions: offer_engine_permissions(&role_info),
                },
            ))
        }
    }
}

/// Resolves the Offer Engine merchant id from the same config the payment flow uses. An
/// unresolved config means Offer Engine is not enabled for this merchant.
async fn resolve_offer_engine_merchant_id(
    state: &SessionState,
    payload: &authentication::AuthToken,
) -> RouterResult<String> {
    let dimensions = Dimensions::new()
        .with_processor_merchant_id(payload.merchant_id.clone().into())
        .with_organization_id(payload.org_id.clone())
        .with_profile_id(payload.profile_id.clone());

    offer_engine::resolve_offer_engine_config(state, &dimensions)
        .await
        .inspect_err(|error| {
            router_env::logger::warn!(?error, "failed to resolve Offer Engine config");
        })
        .ok()
        .flatten()
        .map(|config| config.merchant_id)
        .ok_or_else(|| {
            error_stack::report!(errors::ApiErrorResponse::AccessForbidden {
                resource: "offer_engine".to_string(),
            })
        })
}

/// Offer permissions held by the role. Empty means no offer access.
fn offer_engine_permissions(role_info: &roles::RoleInfo) -> Vec<String> {
    Permission::iter()
        .filter(|permission| permission.resource() == Resource::Offers)
        .filter(|permission| role_info.check_permission_exists(*permission))
        .map(|permission| permission.to_string())
        .collect()
}
