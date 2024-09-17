use actix_web::{web, HttpRequest, HttpResponse};
use common_enums::EntityType;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{admin::*, api_locking},
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api::admin,
};

#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::OrganizationCreate))]
pub async fn organization_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::OrganizationRequest>,
) -> HttpResponse {
    let flow = Flow::OrganizationCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req, _| create_organization(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::OrganizationUpdate))]
pub async fn organization_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    org_id: web::Path<common_utils::id_type::OrganizationId>,
    json_payload: web::Json<admin::OrganizationRequest>,
) -> HttpResponse {
    let flow = Flow::OrganizationUpdate;
    let organization_id = org_id.into_inner();
    let org_id = admin::OrganizationId { organization_id };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req, _| update_organization(state, org_id.clone(), req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::OrganizationRetrieve))]
pub async fn organization_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    org_id: web::Path<common_utils::id_type::OrganizationId>,
) -> HttpResponse {
    let flow = Flow::OrganizationRetrieve;
    let organization_id = org_id.into_inner();
    let payload = admin::OrganizationId { organization_id };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req, _| get_organization(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountCreate))]
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
        |state, _, req, _| create_merchant_account(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Merchant Account - Retrieve
///
/// Retrieve a merchant account details.
#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountRetrieve))]
pub async fn retrieve_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    mid: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::MerchantsAccountRetrieve;
    let merchant_id = mid.into_inner();
    let payload = admin::MerchantId {
        merchant_id: merchant_id.clone(),
    };
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req, _| get_merchant_account(state, req, None),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantAccountRead,
                minimum_entity_level: EntityType::Profile,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all, fields(flow = ?Flow::MerchantAccountList))]
pub async fn merchant_account_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    organization_id: web::Path<common_utils::id_type::OrganizationId>,
) -> HttpResponse {
    let flow = Flow::MerchantAccountList;

    let organization_id = admin::OrganizationId {
        organization_id: organization_id.into_inner(),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        organization_id,
        |state, _, request, _| list_merchant_account(state, request),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantAccountRead,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::MerchantAccountList))]
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
        |state, _, request, _| list_merchant_account(state, request),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantAccountRead,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Merchant Account - Update
///
/// To update an existing merchant account. Helpful in updating merchant details such as email, contact details, or other configuration details like webhook, routing algorithm etc
#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountUpdate))]
pub async fn update_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    mid: web::Path<common_utils::id_type::MerchantId>,
    json_payload: web::Json<admin::MerchantAccountUpdate>,
) -> HttpResponse {
    let flow = Flow::MerchantsAccountUpdate;
    let merchant_id = mid.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, req, _| merchant_account_update(state, &merchant_id, None, req),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantAccountWrite,
                minimum_entity_level: EntityType::Merchant,
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
#[instrument(skip_all, fields(flow = ?Flow::MerchantsAccountDelete))]
pub async fn delete_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    mid: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::MerchantsAccountDelete;
    let mid = mid.into_inner();

    let payload = web::Json(admin::MerchantId { merchant_id: mid }).into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req, _| merchant_account_delete(state, req.merchant_id),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// Merchant Connector - Create
///
/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsCreate))]
pub async fn connector_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
    json_payload: web::Json<admin::MerchantConnectorCreate>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsCreate;
    let payload = json_payload.into_inner();
    let merchant_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data, req, _| {
            create_connector(
                state,
                req,
                auth_data.merchant_account,
                auth_data.profile_id,
                auth_data.key_store,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantConnectorAccountWrite,
                minimum_entity_level: EntityType::Profile,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Merchant Connector - Create
///
/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsCreate))]
pub async fn connector_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::MerchantConnectorCreate>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsCreate;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data, req, _| {
            create_connector(
                state,
                req,
                auth_data.merchant_account,
                None,
                auth_data.key_store,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantConnectorAccountWrite,
                minimum_entity_level: EntityType::Merchant,
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
#[cfg(feature = "v1")]
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
pub async fn connector_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::MerchantConnectorAccountId,
    )>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsRetrieve;
    let (merchant_id, merchant_connector_id) = path.into_inner();
    let payload = web::Json(admin::MerchantConnectorId {
        merchant_id: merchant_id.clone(),
        merchant_connector_id,
    })
    .into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req, _| {
            retrieve_connector(
                state,
                req.merchant_id,
                auth.profile_id,
                req.merchant_connector_id,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantConnectorAccountRead,
                minimum_entity_level: EntityType::Profile,
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
#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsRetrieve))]
pub async fn connector_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantConnectorAccountId>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsRetrieve;
    let id = path.into_inner();
    let payload = web::Json(admin::MerchantConnectorId { id: id.clone() }).into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data, req, _| {
            retrieve_connector(
                state,
                auth_data.merchant_account,
                auth_data.key_store,
                req.id.clone(),
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantConnectorAccountRead,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsList))]
pub async fn connector_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsList;
    let profile_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        profile_id.to_owned(),
        |state, auth, _, _| {
            list_connectors_for_a_profile(state, auth.merchant_account.clone(), profile_id.clone())
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantConnectorAccountRead,
                minimum_entity_level: EntityType::Merchant,
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
#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsList))]
pub async fn connector_list(
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
        |state, _auth, merchant_id, _| list_payment_connectors(state, merchant_id, None),
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantConnectorAccountRead,
                minimum_entity_level: EntityType::Merchant,
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
    path = "/accounts/{account_id}/profile/connectors",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
    ),
    responses(
        (status = 200, description = "Merchant Connector list retrieved successfully", body = Vec<MerchantConnectorResponse>),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "List all Merchant Connectors for The given Profile",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsList))]
pub async fn connector_list_profile(
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
                required_permission: Permission::MerchantConnectorAccountRead,
                minimum_entity_level: EntityType::Profile,
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
#[cfg(feature = "v1")]
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
pub async fn connector_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::MerchantConnectorAccountId,
    )>,
    json_payload: web::Json<api_models::admin::MerchantConnectorUpdate>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsUpdate;
    let (merchant_id, merchant_connector_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req, _| {
            update_connector(
                state,
                &merchant_id,
                auth.profile_id,
                &merchant_connector_id,
                req,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantConnectorAccountWrite,
                minimum_entity_level: EntityType::Profile,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Merchant Connector - Update
///
/// To update an existing Merchant Connector. Helpful in enabling / disabling different payment methods and other settings for the connector etc.
#[cfg(feature = "v2")]
#[utoipa::path(
    post,
    path = "/connector_accounts/{id}",
    request_body = MerchantConnectorUpdate,
    params(
        ("id" = i32, Path, description = "The unique identifier for the Merchant Connector")
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
pub async fn connector_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantConnectorAccountId>,
    json_payload: web::Json<api_models::admin::MerchantConnectorUpdate>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsUpdate;
    let id = path.into_inner();
    let payload = json_payload.into_inner();
    let merchant_id = payload.merchant_id.clone();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req, _| update_connector(state, &merchant_id, None, &id, req),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantConnectorAccountWrite,
                minimum_entity_level: EntityType::Merchant,
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
#[cfg(feature = "v1")]
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
pub async fn connector_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::MerchantConnectorAccountId,
    )>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsDelete;
    let (merchant_id, merchant_connector_id) = path.into_inner();

    let payload = web::Json(admin::MerchantConnectorId {
        merchant_id: merchant_id.clone(),
        merchant_connector_id,
    })
    .into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req, _| delete_connector(state, req.merchant_id, req.merchant_connector_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantConnectorAccountWrite,
                minimum_entity_level: EntityType::Merchant,
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
#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::MerchantConnectorsDelete))]
pub async fn connector_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantConnectorAccountId>,
) -> HttpResponse {
    let flow = Flow::MerchantConnectorsDelete;
    let id = path.into_inner();

    let payload = web::Json(admin::MerchantConnectorId { id: id.clone() }).into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data, req, _| {
            delete_connector(
                state,
                auth_data.merchant_account,
                auth_data.key_store,
                req.id,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantConnectorAccountWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Merchant Account - Toggle KV
///
/// Toggle KV mode for the Merchant Account
#[instrument(skip_all)]
pub async fn merchant_account_toggle_kv(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
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
        |state, _, payload, _| kv_for_merchant(state, payload.merchant_id, payload.kv_enabled),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// Merchant Account - Transfer Keys
///
/// Transfer Merchant Encryption key to keymanager
#[instrument(skip_all)]
pub async fn merchant_account_toggle_all_kv(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::ToggleAllKVRequest>,
) -> HttpResponse {
    let flow = Flow::MerchantTransferKey;
    let payload = json_payload.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, payload, _| toggle_kv_for_all_merchants(state, payload.kv_enabled),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileCreate))]
pub async fn business_profile_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::BusinessProfileCreate>,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileCreate;
    let payload = json_payload.into_inner();
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data, req, _| {
            create_business_profile(state, req, auth_data.merchant_account, auth_data.key_store)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantAccountWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileCreate))]
pub async fn business_profile_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<admin::BusinessProfileCreate>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileCreate;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data, req, _| {
            create_business_profile(state, req, auth_data.merchant_account, auth_data.key_store)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantAccountWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileRetrieve))]
pub async fn business_profile_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
    )>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileRetrieve;
    let (merchant_id, profile_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        profile_id,
        |state, auth_data, profile_id, _| {
            retrieve_business_profile(state, profile_id, auth_data.key_store)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantAccountRead,
                minimum_entity_level: EntityType::Profile,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileRetrieve))]
pub async fn business_profile_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileRetrieve;
    let profile_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        profile_id,
        |state, auth_data, profile_id, _| {
            retrieve_business_profile(state, profile_id, auth_data.key_store)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantAccountRead,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileUpdate))]
pub async fn business_profile_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
    )>,
    json_payload: web::Json<api_models::admin::BusinessProfileUpdate>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileUpdate;
    let (merchant_id, profile_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth_data, req, _| {
            update_business_profile(state, &profile_id, auth_data.key_store, req)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantAndProfileFromRoute {
                merchant_id: merchant_id.clone(),
                profile_id: profile_id.clone(),
                required_permission: Permission::MerchantAccountWrite,
                minimum_entity_level: EntityType::Profile,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileUpdate))]
pub async fn business_profile_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
    json_payload: web::Json<api_models::admin::BusinessProfileUpdate>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileUpdate;
    let profile_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth_data, req, _| {
            update_business_profile(state, &profile_id, auth_data.key_store, req)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantAccountWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileDelete))]
pub async fn business_profile_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
    )>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileDelete;
    let (merchant_id, profile_id) = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        profile_id,
        |state, _, profile_id, _| delete_business_profile(state, profile_id, &merchant_id),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileList))]
pub async fn business_profiles_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileList;
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.clone(),
        |state, _auth, merchant_id, _| list_business_profile(state, merchant_id, None),
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantAccountRead,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::BusinessProfileList))]
pub async fn business_profiles_list_at_profile_level(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::BusinessProfileList;
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.clone(),
        |state, auth, merchant_id, _| {
            list_business_profile(
                state,
                merchant_id,
                auth.profile_id.map(|profile_id| vec![profile_id]),
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantAccountRead,
                minimum_entity_level: EntityType::Profile,
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
        |state, _, req, _| connector_agnostic_mit_toggle(state, &merchant_id, &profile_id, req),
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RoutingWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Merchant Account - KV Status
///
/// Toggle KV mode for the Merchant Account
#[instrument(skip_all)]
pub async fn merchant_account_kv_status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::ConfigKeyFetch;
    let merchant_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        merchant_id,
        |state, _, req, _| check_merchant_account_kv_status(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// Merchant Account - KV Status
///
/// Toggle KV mode for the Merchant Account
#[instrument(skip_all)]
pub async fn merchant_account_transfer_keys(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<api_models::admin::MerchantKeyTransferRequest>,
) -> HttpResponse {
    let flow = Flow::ConfigKeyFetch;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.into_inner(),
        |state, _, req, _| transfer_key_store_to_key_manager(state, req),
        &auth::AdminApiAuth,
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
