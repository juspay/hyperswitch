use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{
    tracing::{self, instrument},
    Flow,
};

use super::app::AppState;
use crate::{core::admin::*, services::api, types::api::admin};

#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountCreate))]
// #[post("")]
pub async fn merchant_account_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::CreateMerchantAccount>,
) -> HttpResponse {
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, _, req| create_merchant_account(&*state.store, req),
        api::MerchantAuthentication::AdminApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountRetrieve))]
// #[get("/{id}")]
pub async fn retrieve_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    mid: web::Path<String>,
) -> HttpResponse {
    let payload = web::Json(admin::MerchantId {
        merchant_id: mid.into_inner(),
    })
    .into_inner();
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, _, req| get_merchant_account(&*state.store, req),
        api::MerchantAuthentication::AdminApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountUpdate))]
// #[post["/{id}"]]
pub async fn update_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    mid: web::Path<String>,
    json_payload: web::Json<admin::CreateMerchantAccount>,
) -> HttpResponse {
    let merchant_id = mid.into_inner();
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, _, req| merchant_account_update(&*state.store, &merchant_id, req),
        api::MerchantAuthentication::AdminApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountDelete))]
// #[delete("/{id}")]
pub async fn delete_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    mid: web::Path<String>,
) -> HttpResponse {
    let payload = web::Json(admin::MerchantId {
        merchant_id: mid.into_inner(),
    })
    .into_inner();
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, _, req| merchant_account_delete(&*state.store, req.merchant_id),
        api::MerchantAuthentication::AdminApiKey,
    )
    .await
}

//payment connector api
#[instrument(skip_all, fields(flow = ?Flow::PaymentConnectorsCreate))]
// #[post("/{merchant_id}/connectors")]
pub async fn payment_connector_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<admin::PaymentConnectorCreate>,
) -> HttpResponse {
    let merchant_id = path.into_inner();
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, _, req| create_payment_connector(&*state.store, req, &merchant_id),
        api::MerchantAuthentication::AdminApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentConnectorsRetrieve))]
// #[get("/{merchant_id}/connectors/{merchant_connector_id}")]
pub async fn payment_connector_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, i32)>,
) -> HttpResponse {
    let (merchant_id, merchant_connector_id) = path.into_inner();
    let payload = web::Json(admin::MerchantConnectorId {
        merchant_id,
        merchant_connector_id,
    })
    .into_inner();
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, _, req| {
            retrieve_payment_connector(&*state.store, req.merchant_id, req.merchant_connector_id)
        },
        api::MerchantAuthentication::AdminApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentConnectorsList))]

pub async fn payment_connector_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let merchant_id = path.into_inner();
    api::server_wrap(
        &state,
        &req,
        merchant_id,
        |state, _, merchant_id| list_payment_connectors(&*state.store, merchant_id),
        api::MerchantAuthentication::AdminApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentConnectorsUpdate))]
// #[post("/{merchant_id}/connectors/{merchant_connector_id}")]
pub async fn payment_connector_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, i32)>,
    json_payload: web::Json<admin::PaymentConnectorCreate>,
) -> HttpResponse {
    let (merchant_id, merchant_connector_id) = path.into_inner();
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, _, req| {
            update_payment_connector(&*state.store, &merchant_id, merchant_connector_id, req)
        },
        api::MerchantAuthentication::AdminApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentConnectorsDelete))]
// #[delete("/{merchant_id}/connectors/{merchant_connector_id}")]
pub async fn payment_connector_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, i32)>,
) -> HttpResponse {
    let (merchant_id, merchant_connector_id) = path.into_inner();
    let payload = web::Json(admin::MerchantConnectorId {
        merchant_id,
        merchant_connector_id,
    })
    .into_inner();
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, _, req| {
            delete_payment_connector(&*state.store, req.merchant_id, req.merchant_connector_id)
        },
        api::MerchantAuthentication::AdminApiKey,
    )
    .await
}
