use actix_web::{web, Responder, Scope};
use api_models::analytics::{
    GetPaymentFiltersRequest, GetPaymentMetricRequest, GetRefundFilterRequest,
    GetRefundMetricRequest,
};
use router_env::AnalyticsFlow;

use super::{core::*, payments, refunds, types::AnalyticsDomain};
use crate::{
    core::api_locking,
    services::{
        api, authentication as auth, authentication::AuthenticationData,
        authorization::permissions::Permission,
    },
    AppState,
};

pub struct Analytics;

impl Analytics {
    pub fn server(state: AppState) -> Scope {
        let route = web::scope("/analytics/v1").app_data(web::Data::new(state));
        route
            .service(web::resource("metrics/payments").route(web::post().to(get_payment_metrics)))
            .service(web::resource("metrics/refunds").route(web::post().to(get_refunds_metrics)))
            .service(web::resource("filters/payments").route(web::post().to(get_payment_filters)))
            .service(web::resource("filters/refunds").route(web::post().to(get_refund_filters)))
            .service(web::resource("{domain}/info").route(web::get().to(get_info)))
    }
}

pub async fn get_info(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    domain: actix_web::web::Path<AnalyticsDomain>,
) -> impl Responder {
    let flow = AnalyticsFlow::GetInfo;
    api::server_wrap(
        flow,
        state,
        &req,
        domain.into_inner(),
        |_, _, domain| get_domain_info(domain),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// # Panics
///
/// Panics if `json_payload` array does not contain one `GetPaymentMetricRequest` element.
pub async fn get_payment_metrics(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<[GetPaymentMetricRequest; 1]>,
) -> impl Responder {
    // safety: This shouldn't panic owing to the data type
    #[allow(clippy::expect_used)]
    let payload = json_payload
        .into_inner()
        .to_vec()
        .pop()
        .expect("Couldn't get GetPaymentMetricRequest");
    let flow = AnalyticsFlow::GetPaymentMetrics;
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: AuthenticationData, req| {
            payments::get_metrics(state.pool.clone(), auth.merchant_account, req)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::Analytics),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// # Panics
///
/// Panics if `json_payload` array does not contain one `GetRefundMetricRequest` element.
pub async fn get_refunds_metrics(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<[GetRefundMetricRequest; 1]>,
) -> impl Responder {
    #[allow(clippy::expect_used)]
    // safety: This shouldn't panic owing to the data type
    let payload = json_payload
        .into_inner()
        .to_vec()
        .pop()
        .expect("Couldn't get GetRefundMetricRequest");
    let flow = AnalyticsFlow::GetRefundsMetrics;
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: AuthenticationData, req| {
            refunds::get_metrics(state.pool.clone(), auth.merchant_account, req)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::Analytics),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

pub async fn get_payment_filters(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<GetPaymentFiltersRequest>,
) -> impl Responder {
    let flow = AnalyticsFlow::GetPaymentFilters;
    api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: AuthenticationData, req| {
            payment_filters_core(state.pool.clone(), req, auth.merchant_account)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::Analytics),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

pub async fn get_refund_filters(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<GetRefundFilterRequest>,
) -> impl Responder {
    let flow = AnalyticsFlow::GetRefundFilters;
    api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: AuthenticationData, req: GetRefundFilterRequest| {
            refund_filter_core(state.pool.clone(), req, auth.merchant_account)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::Analytics),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
