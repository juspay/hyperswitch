use actix_web::{web, Responder, Scope};
use api_models::analytics::{
    GetPaymentFiltersRequest, GetPaymentMetricRequest, GetRefundFilterRequest,
    GetRefundMetricRequest,
};
#[cfg(feature = "clickhouse_analytics")]
use hyperswitch_oss::services::authentication::AuthenticationData;
use router_env::VasFlow;

use super::{core::*, payments, refunds, types::AnalyticsDomain};
use crate::{
    services::{api, authentication as auth, authorization::Permission},
    AppStateVas,
};

pub struct Analytics;

impl Analytics {
    pub fn server(state: AppStateVas) -> Scope {
        let mut route = web::scope("/analytics/v1").app_data(web::Data::new(state));
            route
                .service(
                    web::resource("metrics/payments").route(web::post().to(get_payment_metrics)),
                )
                .service(
                    web::resource("metrics/refunds").route(web::post().to(get_refunds_metrics)),
                )
                .service(
                    web::resource("filters/payments").route(web::post().to(get_payment_filters)),
                )
                .service(web::resource("filters/refunds").route(web::post().to(get_refund_filters)))
                .service(web::resource("{domain}/info").route(web::get().to(get_info)))
    }
}

pub async fn get_info(
    state: web::Data<AppStateVas>,
    req: actix_web::HttpRequest,
    domain: actix_web::web::Path<AnalyticsDomain>,
) -> impl Responder {
    let flow = VasFlow::GetInfo;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        domain.into_inner(),
        |_, _, domain| get_domain_info(domain),
        &auth::NoAuth,
    )
    .await
}

pub async fn get_payment_metrics(
    state: web::Data<AppStateVas>,
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
    let flow = VasFlow::GetPaymentMetrics;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload,
        |state, auth: AuthenticationData, req| {
            payments::get_metrics(&state.pool, auth.merchant_account, req)
        },
        &auth::JWTAuth(Permission::Analytics),
    )
    .await
}

pub async fn get_refunds_metrics(
    state: web::Data<AppStateVas>,
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
    let flow = VasFlow::GetRefundsMetrics;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload,
        |state, auth: AuthenticationData, req| {
            refunds::get_metrics(&state.pool, auth.merchant_account, req)
        },
        &auth::JWTAuth(Permission::Analytics),
    )
    .await
}

pub async fn get_payment_filters(
    state: web::Data<AppStateVas>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<GetPaymentFiltersRequest>,
) -> impl Responder {
    let flow = VasFlow::GetPaymentFilters;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        |state, auth: AuthenticationData, req| {
            payment_filters_core(&state.pool, req, auth.merchant_account)
        },
        &auth::JWTAuth(Permission::Analytics),
    )
    .await
}

pub async fn get_refund_filters(
    state: web::Data<AppStateVas>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<GetRefundFilterRequest>,
) -> impl Responder {
    let flow = VasFlow::GetRefundFilters;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        |state, auth: AuthenticationData, req: GetRefundFilterRequest| {
            refund_filter_core(&state.pool, req, auth.merchant_account)
        },
        &auth::JWTAuth(Permission::Analytics),
    )
    .await
}
