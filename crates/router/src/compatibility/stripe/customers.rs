pub mod types;
#[cfg(feature = "v1")]
use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "v1")]
use common_utils::id_type;
#[cfg(feature = "v1")]
use error_stack::report;
#[cfg(feature = "v1")]
use router_env::{instrument, tracing, Flow};

#[cfg(feature = "v1")]
use crate::{
    compatibility::{stripe::errors, wrap},
    core::{api_locking, customers, payment_methods::cards},
    routes,
    services::{api, authentication as auth},
    types::api::{customers as customer_types, payment_methods},
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersCreate))]
pub async fn customer_create(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::CreateCustomerRequest = match qs_config.deserialize_bytes(&form_payload) {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    let create_cust_req: customer_types::CustomerRequest = payload.into();

    let flow = Flow::CustomersCreate;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::CreateCustomerResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        create_cust_req,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            customers::create_customer(state, platform, req, None)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersRetrieve))]
pub async fn customer_retrieve(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
) -> HttpResponse {
    let customer_id = path.into_inner();

    let flow = Flow::CustomersRetrieve;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::CustomerRetrieveResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        customer_id,
        |state, auth: auth::AuthenticationData, customer_id, _| {
            let platform = auth.into();
            customers::retrieve_customer(state, platform, None, customer_id)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersUpdate))]
pub async fn customer_update(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::CustomerUpdateRequest = match qs_config.deserialize_bytes(&form_payload) {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    let customer_id = path.into_inner().clone();
    let request = customer_types::CustomerUpdateRequest::from(payload);
    let request_internal = customer_types::CustomerUpdateRequestInternal {
        customer_id,
        request,
    };

    let flow = Flow::CustomersUpdate;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::CustomerUpdateResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        request_internal,
        |state, auth: auth::AuthenticationData, request_internal, _| {
            let platform = auth.into();
            customers::update_customer(state, platform, request_internal)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomersDelete))]
pub async fn customer_delete(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
) -> HttpResponse {
    let customer_id = path.into_inner();

    let flow = Flow::CustomersDelete;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::CustomerDeleteResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        customer_id,
        |state, auth: auth::AuthenticationData, customer_id, _| {
            let platform = auth.into();
            customers::delete_customer(state, platform, customer_id)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
pub async fn list_customer_payment_method_api(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
    json_payload: web::Query<payment_methods::PaymentMethodListRequest>,
) -> HttpResponse {
    let payload = json_payload.into_inner();
    let customer_id = path.into_inner();
    let flow = Flow::CustomerPaymentMethodsList;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::CustomerPaymentMethodListResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            cards::do_list_customer_pm_fetch_customer_if_not_passed(
                state,
                platform,
                Some(req),
                Some(&customer_id),
                None,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
