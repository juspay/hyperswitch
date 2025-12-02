use actix_web::{
    body::{BoxBody, MessageBody},
    web, HttpRequest, HttpResponse, Responder,
};
#[cfg(feature = "v1")]
use api_models::payments::BrowserInformation;
use common_utils::id_type;
#[cfg(feature = "v2")]
use common_utils::types::BrowserInformation;
use hyperswitch_domain_models::payments::HeaderPayload;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, errors::RouterResult, payouts::*},
    logger,
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
    types::{api::payouts as payout_types, transformers::ForeignTryFrom},
};

/// Payouts - Create
#[instrument(skip_all, fields(flow = ?Flow::PayoutsCreate))]
pub async fn payouts_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
) -> HttpResponse {
    let flow = Flow::PayoutsCreate;

    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let mut payload = json_payload.into_inner();

    if let Err(err) = populate_browser_info_for_payouts(&req, &mut payload, &header_payload) {
        return api::log_and_return_error_response(err);
    }

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            payouts_create_core(state, platform, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", feature = "payouts"))]
/// Payouts - Retrieve
#[instrument(skip_all, fields(flow = ?Flow::PayoutsRetrieve))]
pub async fn payouts_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::PayoutId>,
    query_params: web::Query<payout_types::PayoutRetrieveBody>,
) -> HttpResponse {
    let payout_retrieve_request = payout_types::PayoutRetrieveRequest {
        payout_id: path.into_inner(),
        force_sync: query_params.force_sync.to_owned(),
        merchant_id: query_params.merchant_id.to_owned(),
    };
    let flow = Flow::PayoutsRetrieve;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payout_retrieve_request,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.clone().into();
            payouts_retrieve_core(state, platform, auth.profile_id, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfilePayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Payouts - Update
#[instrument(skip_all, fields(flow = ?Flow::PayoutsUpdate))]
pub async fn payouts_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::PayoutId>,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
) -> HttpResponse {
    let flow = Flow::PayoutsUpdate;
    let payout_id = path.into_inner();
    let mut payout_update_payload = json_payload.into_inner();
    payout_update_payload.payout_id = Some(payout_id);

    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(err) => return api::log_and_return_error_response(err),
    };
    if let Err(err) =
        populate_browser_info_for_payouts(&req, &mut payout_update_payload, &header_payload)
    {
        return api::log_and_return_error_response(err);
    }

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payout_update_payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            payouts_update_core(state, platform, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsConfirm))]
pub async fn payouts_confirm(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
    path: web::Path<id_type::PayoutId>,
) -> HttpResponse {
    let flow = Flow::PayoutsConfirm;
    let mut payload = json_payload.into_inner();
    let payout_id = path.into_inner();
    tracing::Span::current().record("payout_id", payout_id.get_string_repr());
    payload.payout_id = Some(payout_id);
    payload.confirm = Some(true);
    let api_auth = auth::ApiKeyAuth::default();

    let (auth_type, _auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth) {
            Ok(auth) => auth,
            Err(e) => return api::log_and_return_error_response(e),
        };

    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(err) => return api::log_and_return_error_response(err),
    };
    if let Err(err) = populate_browser_info_for_payouts(&req, &mut payload, &header_payload) {
        return api::log_and_return_error_response(err);
    }

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req, _| {
            let platform = auth.into();
            payouts_confirm_core(state, platform, req)
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Cancel
#[instrument(skip_all, fields(flow = ?Flow::PayoutsCancel))]
pub async fn payouts_cancel(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::PayoutId>,
) -> HttpResponse {
    let flow = Flow::PayoutsCancel;
    let payload = payout_types::PayoutActionRequest {
        payout_id: path.into_inner(),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            payouts_cancel_core(state, platform, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Payouts - Fulfill
#[instrument(skip_all, fields(flow = ?Flow::PayoutsFulfill))]
pub async fn payouts_fulfill(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::PayoutId>,
) -> HttpResponse {
    let flow = Flow::PayoutsFulfill;
    let payload = payout_types::PayoutActionRequest {
        payout_id: path.into_inner(),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            payouts_fulfill_core(state, platform, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - List
#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsList))]
pub async fn payouts_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Query<payout_types::PayoutListConstraints>,
) -> HttpResponse {
    let flow = Flow::PayoutsList;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            payouts_list_core(state, platform, None, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantPayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - List Profile
#[cfg(all(feature = "olap", feature = "payouts", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsList))]
pub async fn payouts_list_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Query<payout_types::PayoutListConstraints>,
) -> HttpResponse {
    let flow = Flow::PayoutsList;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.clone().into();
            payouts_list_core(
                state,
                platform,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfilePayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Filtered list
#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsList))]
pub async fn payouts_list_by_filter(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutListFilterConstraints>,
) -> HttpResponse {
    let flow = Flow::PayoutsList;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            payouts_filtered_list_core(state, platform, None, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantPayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Filtered list
#[cfg(all(feature = "olap", feature = "payouts", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsList))]
pub async fn payouts_list_by_filter_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutListFilterConstraints>,
) -> HttpResponse {
    let flow = Flow::PayoutsList;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.clone().into();
            payouts_filtered_list_core(
                state,
                platform,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfilePayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Available filters for Merchant
#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsFilter))]
pub async fn payouts_list_available_filters_for_merchant(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<common_utils::types::TimeRange>,
) -> HttpResponse {
    let flow = Flow::PayoutsFilter;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            payouts_list_available_filters_core(state, platform, None, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantPayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Available filters for Profile
#[cfg(all(feature = "olap", feature = "payouts", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsFilter))]
pub async fn payouts_list_available_filters_for_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<common_utils::types::TimeRange>,
) -> HttpResponse {
    let flow = Flow::PayoutsFilter;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.clone().into();
            payouts_list_available_filters_core(
                state,
                platform,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfilePayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsAccounts))]
// #[get("/accounts")]
pub async fn payouts_accounts() -> impl Responder {
    let _flow = Flow::PayoutsAccounts;
    http_response("accounts")
}

fn http_response<T: MessageBody + 'static>(response: T) -> HttpResponse<BoxBody> {
    HttpResponse::Ok().body(response)
}

/// Payouts - Available filters for Profile
#[cfg(all(feature = "olap", feature = "payouts", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsFilter))]
pub async fn get_payout_filters(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let flow = Flow::PayoutsFilter;

    api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            let platform = auth.into();
            get_payout_filters_core(state, platform)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfilePayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

pub fn populate_browser_info_for_payouts(
    req: &HttpRequest,
    payload: &mut payout_types::PayoutCreateRequest,
    header_payload: &HeaderPayload,
) -> RouterResult<()> {
    let mut browser_info = payload.browser_info.clone().unwrap_or(BrowserInformation {
        color_depth: None,
        java_enabled: None,
        java_script_enabled: None,
        language: None,
        screen_height: None,
        screen_width: None,
        time_zone: None,
        ip_address: None,
        accept_header: None,
        user_agent: None,
        os_type: None,
        os_version: None,
        device_model: None,
        accept_language: None,
        referer: None,
    });

    let ip_address = req
        .connection_info()
        .realip_remote_addr()
        .map(ToOwned::to_owned);

    if ip_address.is_some() {
        logger::debug!("Extracted IP address from payout request");
    }

    browser_info.ip_address = browser_info.ip_address.or_else(|| {
        ip_address
            .as_ref()
            .map(|ip| ip.parse())
            .transpose()
            .unwrap_or_else(|error| {
                logger::error!(
                    ?error,
                    "Failed to parse IP address extracted from payout request"
                );
                None
            })
    });

    if let Some(locale) = &header_payload.locale {
        browser_info.accept_language = browser_info.accept_language.or(Some(locale.clone()));
    }

    payload.browser_info = Some(browser_info);

    Ok(())
}
