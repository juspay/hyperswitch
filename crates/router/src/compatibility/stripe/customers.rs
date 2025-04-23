pub mod types;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use common_utils::id_type;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use error_stack::report;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use router_env::{instrument, tracing, Flow};

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use crate::{
    compatibility::{stripe::errors, wrap},
    core::{api_locking, customers, payment_methods::cards},
    routes,
    services::{api, authentication as auth},
    types::{
        api::{customers as customer_types, payment_methods},
        domain,
    },
};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "payment_methods_v2")
))]
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
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            customers::create_customer(state, merchant_context, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "payment_methods_v2")
))]
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
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            customers::retrieve_customer(state, merchant_context, None, customer_id)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "payment_methods_v2")
))]
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
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            customers::update_customer(state, merchant_context, request_internal)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "payment_methods_v2")
))]
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
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            customers::delete_customer(state, merchant_context, customer_id)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "payment_methods_v2")
))]
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
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            cards::do_list_customer_pm_fetch_customer_if_not_passed(
                state,
                merchant_context,
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
