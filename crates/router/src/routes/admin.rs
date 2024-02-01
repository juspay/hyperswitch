use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{admin::*, api_locking},
    services::{api, authentication as auth, authorization::permissions::Permission},
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
/// Asynchronously handles the creation of a merchant account by taking in the App state, HTTP request, and JSON payload for the merchant account creation. It then wraps the process in a server_wrap function and awaits the result before returning the HTTP response.
pub async fn merchant_account_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::MerchantAccountCreate>,
) -> HttpResponse {
    let flow = Flow::MerchantsAccountCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req| create_merchant_account(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
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
/// Asynchronously retrieves a merchant account by making a server wrap API call with the provided merchant ID. 
pub async fn retrieve_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    mid: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::MerchantsAccountRetrieve;
    let merchant_id = mid.into_inner();
    let payload = web::Json(admin::MerchantId {
        merchant_id: merchant_id.to_owned(),
    })
    .into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req| get_merchant_account(state, req),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::MerchantAccountList))]
/// This method handles the HTTP request for listing merchant accounts. It takes the application state, the HTTP request, and the query parameters as input, and returns an HTTP response. It creates a flow for listing merchant accounts, wraps the server operation, and awaits the result of the operation before returning the response.
pub async fn merchant_account_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_params: web::Query<api_models::admin::MerchantAccountListRequest>,
) -> HttpResponse {
    let flow = Flow::MerchantAccountList;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query_params.into_inner(),
        |state, _, request| list_merchant_account(state, request),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
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
/// Asynchronously updates a merchant account using the provided state, request, merchant ID, and JSON payload. It wraps the update process in a server wrap and performs authentication and authorization checks before executing the merchant account update. 
pub async fn update_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    mid: web::Path<String>,
    json_payload: web::Json<admin::MerchantAccountUpdate>,
) -> HttpResponse {
    let flow = Flow::MerchantsAccountUpdate;
    let merchant_id = mid.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req| merchant_account_update(state, &merchant_id, req),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
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
    let flow = Flow::MerchantsAccountDelete;
    let mid = mid.into_inner();

    let payload = web::Json(admin::MerchantId { merchant_id: mid }).into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req| merchant_account_delete(state, req.merchant_id),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
/// Merchant Connector - Create
///
/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[utoipa::path(
    post,
    path = "/accounts/{account_id}/connectors",
    request_body = MerchantConnectorCreate,
    responses(
        (status = 200, description = "Merchant Connector Created", body = MerchantConnectorResponse),
        (status = 400, description = "Missing Mandatory fields"),
    ),
    tag = "Merchant Connector Account",
    operation_id = "Create a Merchant Connector",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsCreate))]
/// Creates a new payment connector for a merchant. This method takes in the necessary data including the application state, HTTP request, merchant ID, and JSON payload containing the details of the payment connector to be created. It then calls the `create_payment_connector` function with the provided data and performs authentication checks using `auth_type`. Finally, it uses `api_locking::LockAction` to determine the locking action and returns the HTTP response.
pub async fn payment_connector_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<admin::MerchantConnectorCreate>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsCreate;
    let merchant_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req| create_payment_connector(state, req, &merchant_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantConnectorAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
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
        (status = 200, description = "Merchant Connector retrieved successfully", body = MerchantConnectorResponse),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "Retrieve a Merchant Connector",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsRetrieve))]
/// This method is used to retrieve a payment connector based on the provided merchant ID and connector ID. It constructs a payload with the merchant ID and connector ID, then calls the `server_wrap` function to handle the API request. The `server_wrap` function provides authentication, authorization, and API locking before awaiting the result of the `retrieve_payment_connector` function, which actually retrieves the payment connector from the state.
pub async fn payment_connector_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsRetrieve;
    let (merchant_id, merchant_connector_id) = path.into_inner();
    let payload = web::Json(admin::MerchantConnectorId {
        merchant_id: merchant_id.clone(),
        merchant_connector_id,
    })
    .into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req| {
            retrieve_payment_connector(state, req.merchant_id, req.merchant_connector_id)
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantConnectorAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
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
        (status = 200, description = "Merchant Connector list retrieved successfully", body = Vec<MerchantConnectorResponse>),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "List all Merchant Connectors",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsList))]
/// This method handles the request to list payment connectors for a specific merchant. It extracts the merchant ID from the request path, then calls the `list_payment_connectors` function with the extracted merchant ID as a parameter. It also performs authentication checks based on the provided JWT token and required permissions. Finally, it awaits the result of the API server wrap operation and returns the HTTP response.
pub async fn payment_connector_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsList;
    let merchant_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.to_owned(),
        |state, _, merchant_id| list_payment_connectors(state, merchant_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantConnectorAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
/// Merchant Connector - Update
///
/// To update an existing Merchant Connector. Helpful in enabling / disabling different payment methods and other settings for the connector etc.
#[utoipa::path(
    post,
    path = "/accounts/{account_id}/connectors/{connector_id}",
    request_body = MerchantConnectorUpdate,
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("connector_id" = i32, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector Updated", body = MerchantConnectorResponse),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
   tag = "Merchant Connector Account",
   operation_id = "Update a Merchant Connector",
   security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsUpdate))]
/// Asynchronously handles the update of a merchant payment connector. 
pub async fn payment_connector_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    json_payload: web::Json<api_models::admin::MerchantConnectorUpdate>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsUpdate;
    let (merchant_id, merchant_connector_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req| update_payment_connector(state, &merchant_id, &merchant_connector_id, req),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantConnectorAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
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
/// Delete a payment connector for a specific merchant.
pub async fn payment_connector_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsDelete;
    let (merchant_id, merchant_connector_id) = path.into_inner();

    let payload = web::Json(admin::MerchantConnectorId {
        merchant_id: merchant_id.clone(),
        merchant_connector_id,
    })
    .into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req| delete_payment_connector(state, req.merchant_id, req.merchant_connector_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantConnectorAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
/// Merchant Account - Toggle KV
///
/// Toggle KV mode for the Merchant Account
#[instrument(skip_all)]
/// Handles the HTTP request to toggle the key-value store for a merchant account. It 
/// takes the state of the application, the HTTP request, the merchant account ID from the 
/// path, and a JSON payload containing the toggle request. It then updates the payload 
/// with the merchant ID from the path, calls the `api::server_wrap` function to handle 
/// the API request, and awaits the result. The `api::server_wrap` function wraps the 
/// logic for handling the API request, including authentication and locking. 
pub async fn merchant_account_toggle_kv(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<admin::ToggleKVRequest>,
) -> HttpResponse {
    let flow = Flow::ConfigKeyUpdate;
    let mut payload = json_payload.into_inner();
    payload.merchant_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, payload| kv_for_merchant(state, payload.merchant_id, payload.kv_enabled),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileCreate))]
/// Handles the creation of a business profile by extracting the necessary data from the request, 
/// verifying the authentication, and then calling the 'create_business_profile' function to perform
/// the actual creation. Returns an HttpResponse representing the result of the operation.
pub async fn business_profile_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::BusinessProfileCreate>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileCreate;
    let payload = json_payload.into_inner();
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req| create_business_profile(state, req, &merchant_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileRetrieve))]
/// Retrieves a business profile using the provided merchant_id and profile_id. 
/// 
/// # Arguments
/// 
/// * `state` - The web data state of the application
/// * `req` - The HTTP request
/// * `path` - The path containing the merchant_id and profile_id
/// 
/// # Returns
/// 
/// Returns an HTTP response with the retrieved business profile.
pub async fn business_profile_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileRetrieve;
    let (merchant_id, profile_id) = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        profile_id,
        |state, _, profile_id| retrieve_business_profile(state, profile_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileUpdate))]
/// Updates a business profile for a specific merchant, based on the provided JSON payload.
pub async fn business_profile_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    json_payload: web::Json<api_models::admin::BusinessProfileUpdate>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileUpdate;
    let (merchant_id, profile_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req| update_business_profile(state, &profile_id, &merchant_id, req),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileDelete))]
/// Handles the deletion of a business profile by calling the `delete_business_profile` function with the provided `profile_id` and `merchant_id`. It wraps the operation in the `api::server_wrap` function, passing the necessary parameters including the `Flow`, `AppState`, `HttpRequest`, and authentication details. The method is asynchronous and returns an HttpResponse.
pub async fn business_profile_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileDelete;
    let (merchant_id, profile_id) = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        profile_id,
        |state, _, profile_id| delete_business_profile(state, profile_id, &merchant_id),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileList))]
/// This method handles the HTTP request for listing business profiles. It takes in the application state,
/// the HTTP request, and the merchant ID as parameters. It initializes the flow as BusinessProfileList and
/// extracts the merchant ID from the path. It then calls the server_wrap function from the api module with
/// the provided parameters, including the business profile listing function, authentication type, and locking action.
/// The function returns an HTTP response asynchronously.
pub async fn business_profiles_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileList;
    let merchant_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.clone(),
        |state, _, merchant_id| list_business_profile(state, merchant_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
/// Merchant Account - KV Status
///
/// Toggle KV mode for the Merchant Account
#[instrument(skip_all)]
/// Handles the HTTP request to fetch the status of a merchant account key-value pair.
pub async fn merchant_account_kv_status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::ConfigKeyFetch;
    let merchant_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        merchant_id,
        |state, _, req| check_merchant_account_kv_status(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
