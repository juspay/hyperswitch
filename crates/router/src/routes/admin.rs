use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::admin::*,
    services::{api, authentication as auth},
    types::api::admin,
};

/// Merchant Account - Create
///
/// Create a new account for a merchant and the merchant could be a seller or retailer or client who likes to receive and send payments.
#[utoipa::path(
    post,
    path = "/accounts",
    request_body= MerchantAccountCreate,
    responses(
        (status = 200, description = "Merchant Account Created", body = MerchantAccountResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Merchant Account",
    operation_id = "Create a Merchant Account",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountCreate))]
pub async fn merchant_account_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::MerchantAccountCreate>,
) -> HttpResponse {
    api::server_wrap(
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        |state, _, req| create_merchant_account(state, req),
        &auth::AdminApiAuth,
    )
    .await
}

/// Merchant Account - Retrieve
///
/// Retrieve a merchant account details.
#[utoipa::path(
    get,
    path = "/accounts/{account_id}",
    params (("account_id" = String, Path, description = "The unique identifier for the merchant account")),
    responses(
        (status = 200, description = "Merchant Account Retrieved", body = MerchantAccountResponse),
        (status = 404, description = "Merchant account not found")
    ),
    tag = "Merchant Account",
    operation_id = "Retrieve a Merchant Account",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountRetrieve))]
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
        state.get_ref(),
        &req,
        payload,
        |state, _, req| get_merchant_account(&*state.store, req),
        &auth::AdminApiAuth,
    )
    .await
}

/// Merchant Account - Update
///
/// To update an existing merchant account. Helpful in updating merchant details such as email, contact details, or other configuration details like webhook, routing algorithm etc
#[utoipa::path(
    post,
    path = "/accounts/{account_id}",
    request_body = MerchantAccountUpdate,
    params (("account_id" = String, Path, description = "The unique identifier for the merchant account")),
    responses(
        (status = 200, description = "Merchant Account Updated", body = MerchantAccountResponse),
        (status = 404, description = "Merchant account not found")
    ),
    tag = "Merchant Account",
    operation_id = "Update a Merchant Account",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountUpdate))]
pub async fn update_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    mid: web::Path<String>,
    json_payload: web::Json<admin::MerchantAccountUpdate>,
) -> HttpResponse {
    let merchant_id = mid.into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        |state, _, req| merchant_account_update(&*state.store, &merchant_id, req),
        &auth::AdminApiAuth,
    )
    .await
}

/// Merchant Account - Delete
///
/// To delete a merchant account
#[utoipa::path(
    delete,
    path = "/accounts/{account_id}",
    params (("account_id" = String, Path, description = "The unique identifier for the merchant account")),
    responses(
        (status = 200, description = "Merchant Account Deleted", body = MerchantAccountDeleteResponse),
        (status = 404, description = "Merchant account not found")
    ),
    tag = "Merchant Account",
    operation_id = "Delete a Merchant Account",
    security(("admin_api_key" = []))
)]
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
        state.get_ref(),
        &req,
        payload,
        |state, _, req| merchant_account_delete(&*state.store, req.merchant_id),
        &auth::AdminApiAuth,
    )
    .await
}

/// PaymentsConnectors - Create
///
/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[utoipa::path(
    post,
    path = "/accounts/{account_id}/connectors",
    request_body = MerchantConnector,
    responses(
        (status = 200, description = "Merchant Connector Created", body = MerchantConnector),
        (status = 400, description = "Missing Mandatory fields"),
    ),
    tag = "Merchant Connector Account",
    operation_id = "Create a Merchant Connector",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsCreate))]
pub async fn payment_connector_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<admin::MerchantConnector>,
) -> HttpResponse {
    let merchant_id = path.into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        |state, _, req| create_payment_connector(&*state.store, req, &merchant_id),
        &auth::AdminApiAuth,
    )
    .await
}

/// Merchant Connector - Retrieve
///
/// Retrieve Merchant Connector Details
#[utoipa::path(
    get,
    path = "/accounts/{account_id}/connectors/{connector_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("connector_id" = i32, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector retrieved successfully", body = MerchantConnector),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "Retrieve a Merchant Connector",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsRetrieve))]
pub async fn payment_connector_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (merchant_id, merchant_connector_id) = path.into_inner();
    let payload = web::Json(admin::MerchantConnectorId {
        merchant_id,
        merchant_connector_id,
    })
    .into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        payload,
        |state, _, req| {
            retrieve_payment_connector(&*state.store, req.merchant_id, req.merchant_connector_id)
        },
        &auth::AdminApiAuth,
    )
    .await
}

/// Merchant Connector - List
///
/// List Merchant Connector Details for the merchant
#[utoipa::path(
    get,
    path = "/accounts/{account_id}/connectors",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
    ),
    responses(
        (status = 200, description = "Merchant Connector list retrieved successfully", body = Vec<MerchantConnector>),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "List all Merchant Connectors",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsList))]
pub async fn payment_connector_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let merchant_id = path.into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        merchant_id,
        |state, _, merchant_id| list_payment_connectors(&*state.store, merchant_id),
        &auth::AdminApiAuth,
    )
    .await
}

/// Merchant Connector - Update
///
/// To update an existing Merchant Connector. Helpful in enabling / disabling different payment methods and other settings for the connector etc.
#[utoipa::path(
    post,
    path = "/accounts/{account_id}/connectors/{connector_id}",
    request_body = MerchantConnector,
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("connector_id" = i32, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector Updated", body = MerchantConnector),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
   tag = "Merchant Connector Account",
   operation_id = "Update a Merchant Connector",
   security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsUpdate))]
pub async fn payment_connector_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    json_payload: web::Json<admin::MerchantConnector>,
) -> HttpResponse {
    let (merchant_id, merchant_connector_id) = path.into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        |state, _, req| {
            update_payment_connector(&*state.store, &merchant_id, &merchant_connector_id, req)
        },
        &auth::AdminApiAuth,
    )
    .await
}

/// Merchant Connector - Delete
///
/// Delete or Detach a Merchant Connector from Merchant Account
#[utoipa::path(
    delete,
    path = "/accounts/{account_id}/connectors/{connector_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("connector_id" = i32, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector Deleted", body = MerchantConnectorDeleteResponse),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "Delete a Merchant Connector",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsDelete))]
pub async fn payment_connector_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (merchant_id, merchant_connector_id) = path.into_inner();
    let payload = web::Json(admin::MerchantConnectorId {
        merchant_id,
        merchant_connector_id,
    })
    .into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        payload,
        |state, _, req| {
            delete_payment_connector(&*state.store, req.merchant_id, req.merchant_connector_id)
        },
        &auth::AdminApiAuth,
    )
    .await
}

/// Merchant Account - Toggle KV
///
/// Toggle KV mode for the Merchant Account
#[instrument(skip_all)]
pub async fn merchant_account_toggle_kv(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<admin::ToggleKVRequest>,
) -> HttpResponse {
    let payload = json_payload.into_inner();
    let merchant_id = path.into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        (merchant_id, payload),
        |state, _, (merchant_id, payload)| {
            kv_for_merchant(&*state.store, merchant_id, payload.kv_enabled)
        },
        &auth::AdminApiAuth,
    )
    .await
}

/// Merchant Account - KV Status
///
/// Toggle KV mode for the Merchant Account
#[instrument(skip_all)]
pub async fn merchant_account_kv_status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let merchant_id = path.into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        merchant_id,
        |state, _, req| check_merchant_account_kv_status(&*state.store, req),
        &auth::AdminApiAuth,
    )
    .await
}
