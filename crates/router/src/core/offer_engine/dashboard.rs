use api_models::offer_engine::{OfferEngineDashboardFlow, OfferEngineDashboardProxyRequest};
use common_utils::fp_utils;
use error_stack::ResultExt;

use super::{
    client::{DashboardFlow, OfferEngineClient},
    config::resolve_offer_engine_credential_source,
    types::{DashboardProxyRequest, OfferEngineCredentialSource, OfferEngineError},
};
use crate::{
    core::{
        configs::dimension_state::Dimensions,
        errors::{self, RouterResponse, RouterResult},
    },
    routes::SessionState,
    services::{
        api as service_api,
        authentication::UserFromToken,
        authorization::{permissions::Permission, roles::RoleInfo},
    },
};

/// Forwards a Control Center request to Offer Engine. Hyperswitch owns the caller's identity -
/// it authenticates the dashboard user, resolves the merchant's Offer Engine credentials and
/// stamps the merchant id onto the request - and passes the payload and response through
/// untouched, because Offer Engine owns those schemas.
pub async fn proxy_dashboard_request(
    state: SessionState,
    user: UserFromToken,
    request: OfferEngineDashboardProxyRequest,
) -> RouterResponse<serde_json::Value> {
    let flow = request.flow;
    authorize_flow(&state, &user, flow).await?;

    let offer_id = resolve_offer_id(flow, request.offer_id)?;
    let config = resolve_dashboard_offer_engine_config(&state, &user).await?;

    // The merchant behind the API key is the only merchant this caller may act on, so the id
    // Offer Engine reads from the body is set here rather than taken from the caller.
    let body = stamp_merchant_id(flow, request.payload, &config.merchant_id);

    let client = OfferEngineClient::new(config, &state.conf.trace_header.header_name);
    let response = client
        .dashboard_proxy(
            &state,
            dashboard_flow(flow),
            DashboardProxyRequest {
                offer_id,
                draft: request.draft,
                body,
            },
        )
        .await
        .map_err(map_dashboard_error)?;

    Ok(service_api::ApplicationResponse::Json(response))
}

fn dashboard_flow(flow: OfferEngineDashboardFlow) -> DashboardFlow {
    match flow {
        OfferEngineDashboardFlow::ListOffers => DashboardFlow::ListOffers,
        OfferEngineDashboardFlow::OfferDetail => DashboardFlow::OfferDetail,
        OfferEngineDashboardFlow::CreateOffer => DashboardFlow::CreateOffer,
        OfferEngineDashboardFlow::DeleteOffer => DashboardFlow::DeleteOffer,
        OfferEngineDashboardFlow::UpdateOfferStatus => DashboardFlow::UpdateOfferStatus,
    }
}

/// The route carries a single `Permission::MerchantOffersRead` check, so the flows that change
/// an offer are gated here instead.
fn requires_write_permission(flow: OfferEngineDashboardFlow) -> bool {
    match flow {
        OfferEngineDashboardFlow::ListOffers | OfferEngineDashboardFlow::OfferDetail => false,
        OfferEngineDashboardFlow::CreateOffer
        | OfferEngineDashboardFlow::DeleteOffer
        | OfferEngineDashboardFlow::UpdateOfferStatus => true,
    }
}

/// Whether Offer Engine's request schema for this flow carries `merchant_id`.
fn carries_merchant_id(flow: OfferEngineDashboardFlow) -> bool {
    match flow {
        OfferEngineDashboardFlow::ListOffers
        | OfferEngineDashboardFlow::CreateOffer
        | OfferEngineDashboardFlow::DeleteOffer => true,
        OfferEngineDashboardFlow::OfferDetail | OfferEngineDashboardFlow::UpdateOfferStatus => {
            false
        }
    }
}

/// Whether the Offer Engine route takes the offer id in its path.
fn takes_offer_id(flow: OfferEngineDashboardFlow) -> bool {
    match flow {
        OfferEngineDashboardFlow::DeleteOffer | OfferEngineDashboardFlow::UpdateOfferStatus => true,
        OfferEngineDashboardFlow::ListOffers
        | OfferEngineDashboardFlow::OfferDetail
        | OfferEngineDashboardFlow::CreateOffer => false,
    }
}

/// Reads need only the `MerchantOffersRead` the route already carries; changing an offer needs
/// the write permission as well.
async fn authorize_flow(
    state: &SessionState,
    user: &UserFromToken,
    flow: OfferEngineDashboardFlow,
) -> RouterResult<()> {
    match requires_write_permission(flow) {
        false => Ok(()),
        true => check_offers_write_permission(state, user).await,
    }
}

async fn check_offers_write_permission(
    state: &SessionState,
    user: &UserFromToken,
) -> RouterResult<()> {
    let tenant_id = user
        .tenant_id
        .clone()
        .unwrap_or_else(|| state.tenant.tenant_id.clone());

    let role_info = RoleInfo::from_role_id_in_lineage(
        state,
        &user.role_id,
        &user.merchant_id,
        &user.org_id,
        &user.profile_id,
        &tenant_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to fetch role info for the Offer Engine dashboard proxy")?;

    fp_utils::when(
        !role_info.check_permission_exists(Permission::MerchantOffersWrite),
        || {
            Err(error_stack::report!(
                errors::ApiErrorResponse::AccessForbidden {
                    resource: "offers".to_string(),
                }
            ))
        },
    )
}

/// The id lands in a path, so only the characters an offer id can contain are accepted.
fn resolve_offer_id(
    flow: OfferEngineDashboardFlow,
    offer_id: Option<String>,
) -> RouterResult<Option<String>> {
    takes_offer_id(flow)
        .then(|| validate_offer_id(offer_id))
        .transpose()
}

fn validate_offer_id(offer_id: Option<String>) -> RouterResult<String> {
    let offer_id = offer_id.filter(|id| !id.is_empty()).ok_or_else(|| {
        error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "offer_id is required for this Offer Engine dashboard flow".to_string(),
        })
    })?;

    fp_utils::when(
        !offer_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')),
        || {
            Err(error_stack::report!(
                errors::ApiErrorResponse::InvalidRequestData {
                    message: "offer_id contains unsupported characters".to_string(),
                }
            ))
        },
    )?;

    Ok(offer_id)
}

fn stamp_merchant_id(
    flow: OfferEngineDashboardFlow,
    payload: serde_json::Value,
    merchant_id: &str,
) -> serde_json::Value {
    match carries_merchant_id(flow) {
        false => payload,
        true => {
            let mut object = match payload {
                serde_json::Value::Object(object) => object,
                _ => serde_json::Map::new(),
            };
            object.insert(
                "merchant_id".to_string(),
                serde_json::Value::String(merchant_id.to_string()),
            );
            serde_json::Value::Object(object)
        }
    }
}

/// Resolves the merchant's Offer Engine credentials from the same config the payment flow
/// uses. An unresolved config means Offer Engine is not enabled for this merchant.
async fn resolve_dashboard_offer_engine_config(
    state: &SessionState,
    user: &UserFromToken,
) -> RouterResult<super::types::ResolvedOfferEngineConfig> {
    let dimensions = Dimensions::new()
        .with_processor_merchant_id(user.merchant_id.clone().into())
        .with_organization_id(user.org_id.clone())
        .with_profile_id(user.profile_id.clone());

    match resolve_offer_engine_credential_source(state, &dimensions).await {
        OfferEngineCredentialSource::None => {
            return Err(error_stack::report!(
                errors::ApiErrorResponse::AccessForbidden {
                    resource: "offer_engine".to_string(),
                }
            ))
        }
        OfferEngineCredentialSource::Application => {
            OfferEngineCredentialSource::resolve_application_offer_config(state)
        }
        OfferEngineCredentialSource::Merchant => {
            let db = &*state.store;
            let key_store = db
                .get_merchant_key_store_by_merchant_id(
                    &user.merchant_id,
                    &db.get_master_key().to_vec().into(),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
            let merchant_account = db
                .find_merchant_account_by_merchant_id(&user.merchant_id, &key_store)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
            OfferEngineCredentialSource::resolve_merchant_offer_config(state, &merchant_account)
        }
    }
    .change_context(errors::ApiErrorResponse::AccessForbidden {
        resource: "offer_engine".to_string(),
    })
}

/// Offer Engine's own rejections belong to the dashboard user who caused them, so they are
/// surfaced rather than collapsed into a 500.
fn map_dashboard_error(
    err: error_stack::Report<OfferEngineError>,
) -> error_stack::Report<errors::ApiErrorResponse> {
    match err.current_context() {
        OfferEngineError::UpstreamRejected { status, body } => match *status {
            401 | 403 => error_stack::report!(errors::ApiErrorResponse::AccessForbidden {
                resource: "offer_engine".to_string(),
            }),
            400..=499 => error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
                message: body.clone(),
            }),
            _ => err
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Offer Engine dashboard call failed"),
        },
        _ => err
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Offer Engine dashboard call failed"),
    }
}
