use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{admin::*, api_locking},
    services::{api, authentication as auth, authorization::permissions},
    types::api::admin,
};

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::ProfileCreate))]
pub async fn profile_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::ProfileCreate>,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::ProfileCreate;
    let payload = json_payload.into_inner();
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data, req, _| {
            create_profile(state, req, auth_data.merchant_account, auth_data.key_store)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: permissions::Permission::MerchantAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all, fields(flow = ?Flow::ProfileCreate))]
pub async fn profile_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::ProfileCreate>,
) -> HttpResponse {
    let flow = Flow::ProfileCreate;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state,
         auth::AuthenticationDataWithoutProfile {
             merchant_account,
             key_store,
         },
         req,
         _| { create_profile(state, req, merchant_account, key_store) },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: permissions::Permission::MerchantAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::ProfileRetrieve))]
pub async fn profile_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
    )>,
) -> HttpResponse {
    let flow = Flow::ProfileRetrieve;
    let (merchant_id, profile_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        profile_id,
        |state, auth_data, profile_id, _| retrieve_profile(state, profile_id, auth_data.key_store),
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: permissions::Permission::ProfileAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ProfileRetrieve))]
pub async fn profile_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
) -> HttpResponse {
    let flow = Flow::ProfileRetrieve;
    let profile_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        profile_id,
        |state, auth::AuthenticationDataWithoutProfile { key_store, .. }, profile_id, _| {
            retrieve_profile(state, profile_id, key_store)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: permissions::Permission::MerchantAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::ProfileUpdate))]
pub async fn profile_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
    )>,
    json_payload: web::Json<api_models::admin::ProfileUpdate>,
) -> HttpResponse {
    let flow = Flow::ProfileUpdate;
    let (merchant_id, profile_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth_data, req, _| update_profile(state, &profile_id, auth_data.key_store, req),
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantAndProfileFromRoute {
                merchant_id: merchant_id.clone(),
                profile_id: profile_id.clone(),
                required_permission: permissions::Permission::ProfileAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ProfileUpdate))]
pub async fn profile_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
    json_payload: web::Json<api_models::admin::ProfileUpdate>,
) -> HttpResponse {
    let flow = Flow::ProfileUpdate;
    let profile_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth::AuthenticationDataWithoutProfile { key_store, .. }, req, _| {
            update_profile(state, &profile_id, key_store, req)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: permissions::Permission::MerchantAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ProfileDelete))]
pub async fn profile_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
    )>,
) -> HttpResponse {
    let flow = Flow::ProfileDelete;
    let (merchant_id, profile_id) = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        profile_id,
        |state, _, profile_id, _| delete_profile(state, profile_id, &merchant_id),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::ProfileList))]
pub async fn profiles_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::ProfileList;
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.clone(),
        |state, _auth, merchant_id, _| list_profile(state, merchant_id, None),
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: permissions::Permission::MerchantAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ProfileList))]
pub async fn profiles_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::ProfileList;
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.clone(),
        |state, auth::AuthenticationDataWithoutProfile { .. }, merchant_id, _| {
            list_profile(state, merchant_id, None)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: permissions::Permission::MerchantAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::ProfileList))]
pub async fn profiles_list_at_profile_level(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::ProfileList;
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.clone(),
        |state, auth, merchant_id, _| {
            list_profile(
                state,
                merchant_id,
                auth.profile_id.map(|profile_id| vec![profile_id]),
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: permissions::Permission::ProfileAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ToggleConnectorAgnosticMit))]
pub async fn toggle_connector_agnostic_mit(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
    )>,
    json_payload: web::Json<api_models::admin::ConnectorAgnosticMitChoice>,
) -> HttpResponse {
    let flow = Flow::ToggleConnectorAgnosticMit;
    let (merchant_id, profile_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _: auth::AuthenticationData, req, _| {
            connector_agnostic_mit_toggle(state, &merchant_id, &profile_id, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: permissions::Permission::MerchantRoutingWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ToggleExtendedCardInfo))]
pub async fn toggle_extended_card_info(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
    )>,
    json_payload: web::Json<api_models::admin::ExtendedCardInfoChoice>,
) -> HttpResponse {
    let flow = Flow::ToggleExtendedCardInfo;
    let (merchant_id, profile_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req, _| extended_card_info_toggle(state, &merchant_id, &profile_id, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsList))]
pub async fn payment_connector_list_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsList;
    let merchant_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.to_owned(),
        |state, auth, merchant_id, _| {
            list_payment_connectors(
                state,
                merchant_id,
                auth.profile_id.map(|profile_id| vec![profile_id]),
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: permissions::Permission::ProfileConnectorRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
