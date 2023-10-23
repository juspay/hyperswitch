use actix_web::{web, Responder, Scope};
use api_models::analytics::{
    GetPaymentFiltersRequest, GetPaymentMetricRequest, GetRefundFilterRequest,
    GetRefundMetricRequest,
};
use api_models::analytics::{GetSdkEventFiltersRequest, GetSdkEventMetricRequest};
use hyperswitch_oss::core::api_locking;
use router_env::VasFlow;

use super::sdk_events;
use super::{core::*, payments, refunds, types::AnalyticsDomain};

pub struct Analytics;

impl Analytics {
    pub fn server(state: AppStateVas) -> Scope {
        let mut route = web::scope("/analytics/v1").app_data(web::Data::new(state));
        {
            route = route
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
        {
            route = route
                .service(
                    web::resource("metrics/sdk_events")
                        .route(web::post().to(get_sdk_event_metrics)),
                )
                .service(
                    web::resource("filters/sdk_events")
                        .route(web::post().to(get_sdk_event_filters)),
                )
        }
        route
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
        state,
        &req,
        domain.into_inner(),
        |_, _, domain| get_domain_info(domain),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
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
        state,
        &req,
        payload,
        |state, auth: AuthenticationData, req| {
            payments::get_metrics(&state.pool, auth.merchant_account, req)
        },
        &auth::JWTAuth(Permission::Analytics),
        api_locking::LockAction::NotApplicable,
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
        state,
        &req,
        payload,
        |state, auth: AuthenticationData, req| {
            refunds::get_metrics(&state.pool, auth.merchant_account, req)
        },
        &auth::JWTAuth(Permission::Analytics),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

pub async fn get_sdk_event_metrics(
    state: web::Data<AppStateVas>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<[GetSdkEventMetricRequest; 1]>,
) -> impl Responder {
    // safety: This shouldn't panic owing to the data type
    #[allow(clippy::expect_used)]
    let payload = json_payload
        .into_inner()
        .to_vec()
        .pop()
        .expect("Couldn't get GetSdkEventMetricRequest");
    let flow = VasFlow::GetPaymentMetrics;
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: AuthenticationData, req| {
            sdk_events::get_metrics(&state.pool, auth.merchant_account, req)
        },
        &auth::JWTAuth(Permission::Analytics),
        api_locking::LockAction::NotApplicable,
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
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: AuthenticationData, req| {
            payment_filters_core(&state.pool, req, auth.merchant_account)
        },
        &auth::JWTAuth(Permission::Analytics),
        api_locking::LockAction::NotApplicable,
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
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: AuthenticationData, req: GetRefundFilterRequest| {
            refund_filter_core(&state.pool, req, auth.merchant_account)
        },
        &auth::JWTAuth(Permission::Analytics),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

pub async fn get_sdk_event_filters(
    state: web::Data<AppStateVas>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<GetSdkEventFiltersRequest>,
) -> impl Responder {
    let flow = VasFlow::GetSdkEventFilters;
    api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: AuthenticationData, req| {
            sdk_event_filter_core(&state.pool, req, auth.merchant_account)
        },
        &auth::JWTAuth(Permission::Analytics),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
