pub use analytics::*;

pub mod routes {
    use actix_web::{web, Responder, Scope};
    use analytics::{
        api_event::api_events_core, errors::AnalyticsError, sdk_events::sdk_events_core,
    };
    use api_models::analytics::{
        GetApiEventFiltersRequest, GetApiEventMetricRequest, GetPaymentFiltersRequest,
        GetPaymentMetricRequest, GetRefundFilterRequest, GetRefundMetricRequest,
        GetSdkEventFiltersRequest, GetSdkEventMetricRequest, PaymentReportRequest,
    };
    use error_stack::ResultExt;
    use hyperswitch_oss::{
        core::api_locking,
        services::{authentication::AuthenticationData, ApplicationResponse},
    };
    use masking::PeekInterface;
    use router_env::VasFlow;

    use crate::{
        services::{api, authentication as auth, authorization::permissions::Permission},
        types::user::UserEmail,
        AppStateVas,
    };

    pub struct Analytics;

    impl Analytics {
        pub fn server(state: AppStateVas) -> Scope {
            let mut route = web::scope("/analytics/v1").app_data(web::Data::new(state));
            {
                route = route
                    .service(
                        web::resource("metrics/payments")
                            .route(web::post().to(get_payment_metrics)),
                    )
                    .service(
                        web::resource("metrics/refunds").route(web::post().to(get_refunds_metrics)),
                    )
                    .service(
                        web::resource("filters/payments")
                            .route(web::post().to(get_payment_filters)),
                    )
                    .service(
                        web::resource("filters/refunds").route(web::post().to(get_refund_filters)),
                    )
                    .service(web::resource("{domain}/info").route(web::get().to(get_info)))
            }
            route
        }
    }

    pub async fn get_info(
        state: web::Data<AppStateVas>,
        req: actix_web::HttpRequest,
        domain: actix_web::web::Path<analytics::AnalyticsDomain>,
    ) -> impl Responder {
        let flow = VasFlow::GetInfo;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            domain.into_inner(),
            |_, _, domain| async {
                analytics::core::get_domain_info(domain)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::NoAuth,
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetPaymentMetricRequest` element.
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
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req| async move {
                analytics::payments::get_metrics(
                    &state.pool,
                    &auth.merchant_account.merchant_id,
                    req,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetRefundMetricRequest` element.
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
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req| async move {
                analytics::refunds::get_metrics(
                    &state.pool,
                    &auth.merchant_account.merchant_id,
                    req,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_payment_filters(
        state: web::Data<AppStateVas>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetPaymentFiltersRequest>,
    ) -> impl Responder {
        let flow = VasFlow::GetPaymentFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req| async move {
                analytics::payments::get_filters(
                    &state.pool,
                    req,
                    &auth.merchant_account.merchant_id,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_refund_filters(
        state: web::Data<AppStateVas>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetRefundFilterRequest>,
    ) -> impl Responder {
        let flow = VasFlow::GetRefundFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req: GetRefundFilterRequest| async move {
                analytics::refunds::get_filters(
                    &state.pool,
                    req,
                    &auth.merchant_account.merchant_id,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}
