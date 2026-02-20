use actix_web::{web, HttpRequest, HttpResponse, Responder};
use common_utils::id_type;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, customers::*},
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api::customers,
};
#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersCreate))]
pub async fn customers_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<customers::CustomerRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            create_customer(
                state,
                auth.platform.get_provider().clone(),
                auth.platform.get_initiator().cloned(),
                req,
                None,
            )
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerWrite,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersCreate))]
pub async fn customers_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<customers::CustomerRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            create_customer(
                state,
                auth.platform.get_provider().clone(),
                auth.platform.get_initiator().cloned(),
                req,
                None,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerWrite,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersRetrieve))]
pub async fn customers_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
) -> HttpResponse {
    let flow = Flow::CustomersRetrieve;

    let customer_id = path.into_inner();

    let auth = if auth::is_jwt_auth(req.headers()) {
        Box::new(auth::JWTAuth {
            permission: Permission::MerchantCustomerRead,
            allow_connected: true,
            allow_platform: true,
        })
    } else {
        let api_auth = auth::ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: true,
        };
        match auth::is_ephemeral_auth(req.headers(), api_auth) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(err),
        }
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        customer_id,
        |state, auth, customer_id, _| {
            let profile_id = auth.profile.map(|profile| profile.get_id().clone());
            retrieve_customer(
                state,
                auth.platform.get_provider().clone(),
                profile_id,
                customer_id,
            )
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersRetrieve))]
pub async fn customers_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalCustomerId>,
) -> HttpResponse {
    use crate::services::authentication::sdk_or_api_or_client_auth;

    let flow = Flow::CustomersRetrieve;

    let id = path.into_inner();

    let v2_client_auth = auth::V2ClientAuth(
        common_utils::types::authentication::ResourceId::Customer(id.clone()),
    );
    let sdk_auth = auth::SdkAuthorizationAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
        resource_id: common_utils::types::authentication::ResourceId::Customer(id.clone()),
    };
    let auth = if auth::is_jwt_auth(req.headers()) {
        &auth::JWTAuth {
            permission: Permission::MerchantCustomerRead,
            allow_connected: true,
            allow_platform: true,
        }
    } else {
        sdk_or_api_or_client_auth(
            &sdk_auth,
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &v2_client_auth,
            req.headers(),
        )
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        id,
        |state, auth: auth::AuthenticationData, id, _| {
            retrieve_customer(state, auth.platform.get_provider().clone(), id)
        },
        auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersList))]
pub async fn customers_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<customers::CustomerListRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersList;
    let payload = query.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, request, _| {
            list_customers(state, auth.platform.get_provider().clone(), None, request)
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerRead,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersList))]
pub async fn customers_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<customers::CustomerListRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersList;
    let payload = query.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, request, _| {
            list_customers(state, auth.platform.get_provider().clone(), None, request)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerRead,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersListWithConstraints))]
pub async fn customers_list_with_count(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<customers::CustomerListRequestWithConstraints>,
) -> HttpResponse {
    let flow = Flow::CustomersListWithConstraints;
    let payload = query.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, request, _| {
            list_customers_with_count(state, auth.platform.get_provider().clone(), request)
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerRead,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersListWithConstraints))]
pub async fn customers_list_with_count(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<customers::CustomerListRequestWithConstraints>,
) -> HttpResponse {
    let flow = Flow::CustomersListWithConstraints;
    let payload = query.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, request, _| {
            list_customers_with_count(state, auth.platform.get_provider().clone(), request)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerRead,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersUpdate))]
pub async fn customers_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
    json_payload: web::Json<customers::CustomerUpdateRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersUpdate;
    let customer_id = path.into_inner();
    let request = json_payload.into_inner();
    let request_internal = customers::CustomerUpdateRequestInternal {
        customer_id,
        request,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, auth: auth::AuthenticationData, request_internal, _| {
            update_customer(
                state,
                auth.platform.get_provider().clone(),
                auth.platform.get_initiator().cloned(),
                request_internal,
            )
        },
        auth::auth_type(
            &auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerWrite,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersUpdate))]
pub async fn customers_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalCustomerId>,
    json_payload: web::Json<customers::CustomerUpdateRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersUpdate;
    let id = path.into_inner();
    let request = json_payload.into_inner();
    let request_internal = customers::CustomerUpdateRequestInternal { id, request };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, auth: auth::AuthenticationData, request_internal, _| {
            update_customer(
                state,
                auth.platform.get_provider().clone(),
                auth.platform.get_initiator().cloned(),
                request_internal,
            )
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerWrite,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersDelete))]
pub async fn customers_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalCustomerId>,
) -> impl Responder {
    let flow = Flow::CustomersDelete;
    let id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        id,
        |state, auth: auth::AuthenticationData, id, _| {
            delete_customer(state, auth.platform, id, auth.profile)
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            },
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerWrite,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersDelete))]
pub async fn customers_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
) -> impl Responder {
    let flow = Flow::CustomersDelete;
    let customer_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        customer_id,
        |state, auth: auth::AuthenticationData, customer_id, _| {
            delete_customer(
                state,
                auth.platform.get_provider().clone(),
                auth.platform.get_initiator().cloned(),
                customer_id,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantCustomerWrite,
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersGetMandates))]
pub async fn get_customer_mandates(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
) -> impl Responder {
    let flow = Flow::CustomersGetMandates;
    let customer_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        customer_id,
        |state, auth: auth::AuthenticationData, customer_id, _| {
            crate::core::mandate::get_customer_mandates(state, auth.platform, customer_id)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantMandateRead,
                allow_connected: false,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
