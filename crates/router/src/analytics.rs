pub use analytics::*;

pub mod routes {
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };

    use actix_web::{web, Responder, Scope};
    use analytics::{
        api_event::api_events_core, connector_events::connector_events_core, enums::AuthInfo,
        errors::AnalyticsError, lambda_utils::invoke_lambda, opensearch::OpenSearchError,
        outgoing_webhook_event::outgoing_webhook_events_core, routing_events::routing_events_core,
        sdk_events::sdk_events_core, AnalyticsFlow,
    };
    use api_models::analytics::{
        api_event::QueryType,
        search::{
            GetGlobalSearchRequest, GetSearchRequest, GetSearchRequestWithIndex, SearchIndex,
        },
        AnalyticsRequest, GenerateReportRequest, GetActivePaymentsMetricRequest,
        GetApiEventFiltersRequest, GetApiEventMetricRequest, GetAuthEventFilterRequest,
        GetAuthEventMetricRequest, GetDisputeMetricRequest, GetFrmFilterRequest,
        GetFrmMetricRequest, GetPaymentFiltersRequest, GetPaymentIntentFiltersRequest,
        GetPaymentIntentMetricRequest, GetPaymentMetricRequest, GetRefundFilterRequest,
        GetRefundMetricRequest, GetSdkEventFiltersRequest, GetSdkEventMetricRequest, ReportRequest,
    };
    use common_enums::EntityType;
    use common_utils::types::TimeRange;
    use error_stack::{report, ResultExt};
    use futures::{stream::FuturesUnordered, StreamExt};

    use crate::{
        analytics_validator::request_validator,
        consts::opensearch::SEARCH_INDEXES,
        core::{api_locking, errors::user::UserErrors, verification::utils},
        db::{user::UserInterface, user_role::ListUserRolesByUserIdPayload},
        routes::AppState,
        services::{
            api,
            authentication::{self as auth, AuthenticationData, UserFromToken},
            authorization::{permissions::Permission, roles::RoleInfo},
            ApplicationResponse,
        },
        types::{domain::UserEmail, storage::UserRole},
    };

    pub struct Analytics;

    impl Analytics {
        #[cfg(feature = "v2")]
        pub fn server(state: AppState) -> Scope {
            web::scope("/analytics").app_data(web::Data::new(state))
        }
        #[cfg(feature = "v1")]
        pub fn server(state: AppState) -> Scope {
            web::scope("/analytics")
                .app_data(web::Data::new(state))
                .service(
                    web::scope("/v1")
                        .service(
                            web::resource("metrics/payments")
                                .route(web::post().to(get_merchant_payment_metrics)),
                        )
                        .service(
                            web::resource("metrics/routing")
                                .route(web::post().to(get_merchant_payment_metrics)),
                        )
                        .service(
                            web::resource("metrics/refunds")
                                .route(web::post().to(get_merchant_refund_metrics)),
                        )
                        .service(
                            web::resource("filters/payments")
                                .route(web::post().to(get_merchant_payment_filters)),
                        )
                        .service(
                            web::resource("filters/routing")
                                .route(web::post().to(get_merchant_payment_filters)),
                        )
                        .service(
                            web::resource("filters/frm").route(web::post().to(get_frm_filters)),
                        )
                        .service(
                            web::resource("filters/refunds")
                                .route(web::post().to(get_merchant_refund_filters)),
                        )
                        .service(web::resource("{domain}/info").route(web::get().to(get_info)))
                        .service(
                            web::resource("report/dispute")
                                .route(web::post().to(generate_merchant_dispute_report)),
                        )
                        .service(
                            web::resource("report/refunds")
                                .route(web::post().to(generate_merchant_refund_report)),
                        )
                        .service(
                            web::resource("report/payments")
                                .route(web::post().to(generate_merchant_payment_report)),
                        )
                        .service(
                            web::resource("report/payouts")
                                .route(web::post().to(generate_merchant_payout_report)),
                        )
                        .service(
                            web::resource("report/authentications")
                                .route(web::post().to(generate_merchant_authentication_report)),
                        )
                        .service(
                            web::resource("metrics/sdk_events")
                                .route(web::post().to(get_sdk_event_metrics)),
                        )
                        .service(
                            web::resource("metrics/active_payments")
                                .route(web::post().to(get_active_payments_metrics)),
                        )
                        .service(
                            web::resource("filters/sdk_events")
                                .route(web::post().to(get_sdk_event_filters)),
                        )
                        .service(
                            web::resource("metrics/auth_events")
                                .route(web::post().to(get_merchant_auth_event_metrics)),
                        )
                        .service(
                            web::resource("filters/auth_events")
                                .route(web::post().to(get_merchant_auth_events_filters)),
                        )
                        .service(
                            web::resource("metrics/frm").route(web::post().to(get_frm_metrics)),
                        )
                        .service(
                            web::resource("api_event_logs")
                                .route(web::get().to(get_profile_api_events)),
                        )
                        .service(
                            web::resource("sdk_event_logs")
                                .route(web::post().to(get_profile_sdk_events)),
                        )
                        .service(
                            web::resource("connector_event_logs")
                                .route(web::get().to(get_profile_connector_events)),
                        )
                        .service(
                            web::resource("routing_event_logs")
                                .route(web::get().to(get_profile_routing_events)),
                        )
                        .service(
                            web::resource("outgoing_webhook_event_logs")
                                .route(web::get().to(get_profile_outgoing_webhook_events)),
                        )
                        .service(
                            web::resource("metrics/api_events")
                                .route(web::post().to(get_merchant_api_events_metrics)),
                        )
                        .service(
                            web::resource("filters/api_events")
                                .route(web::post().to(get_merchant_api_event_filters)),
                        )
                        .service(
                            web::resource("search")
                                .route(web::post().to(get_global_search_results)),
                        )
                        .service(
                            web::resource("search/{domain}")
                                .route(web::post().to(get_search_results)),
                        )
                        .service(
                            web::resource("metrics/disputes")
                                .route(web::post().to(get_merchant_dispute_metrics)),
                        )
                        .service(
                            web::resource("filters/disputes")
                                .route(web::post().to(get_merchant_dispute_filters)),
                        )
                        .service(
                            web::resource("metrics/sankey")
                                .route(web::post().to(get_merchant_sankey)),
                        )
                        .service(
                            web::resource("metrics/auth_events/sankey")
                                .route(web::post().to(get_merchant_auth_event_sankey)),
                        )
                        .service(
                            web::scope("/merchant")
                                .service(
                                    web::resource("metrics/payments")
                                        .route(web::post().to(get_merchant_payment_metrics)),
                                )
                                .service(
                                    web::resource("metrics/routing")
                                        .route(web::post().to(get_merchant_payment_metrics)),
                                )
                                .service(
                                    web::resource("metrics/refunds")
                                        .route(web::post().to(get_merchant_refund_metrics)),
                                )
                                .service(
                                    web::resource("metrics/auth_events")
                                        .route(web::post().to(get_merchant_auth_event_metrics)),
                                )
                                .service(
                                    web::resource("filters/payments")
                                        .route(web::post().to(get_merchant_payment_filters)),
                                )
                                .service(
                                    web::resource("filters/routing")
                                        .route(web::post().to(get_merchant_payment_filters)),
                                )
                                .service(
                                    web::resource("filters/refunds")
                                        .route(web::post().to(get_merchant_refund_filters)),
                                )
                                .service(
                                    web::resource("filters/auth_events")
                                        .route(web::post().to(get_merchant_auth_events_filters)),
                                )
                                .service(
                                    web::resource("{domain}/info").route(web::get().to(get_info)),
                                )
                                .service(
                                    web::resource("report/dispute")
                                        .route(web::post().to(generate_merchant_dispute_report)),
                                )
                                .service(
                                    web::resource("report/refunds")
                                        .route(web::post().to(generate_merchant_refund_report)),
                                )
                                .service(
                                    web::resource("report/payments")
                                        .route(web::post().to(generate_merchant_payment_report)),
                                )
                                .service(
                                    web::resource("report/payouts")
                                        .route(web::post().to(generate_merchant_payout_report)),
                                )
                                .service(
                                    web::resource("report/authentications").route(
                                        web::post().to(generate_merchant_authentication_report),
                                    ),
                                )
                                .service(
                                    web::resource("metrics/api_events")
                                        .route(web::post().to(get_merchant_api_events_metrics)),
                                )
                                .service(
                                    web::resource("filters/api_events")
                                        .route(web::post().to(get_merchant_api_event_filters)),
                                )
                                .service(
                                    web::resource("metrics/disputes")
                                        .route(web::post().to(get_merchant_dispute_metrics)),
                                )
                                .service(
                                    web::resource("filters/disputes")
                                        .route(web::post().to(get_merchant_dispute_filters)),
                                )
                                .service(
                                    web::resource("metrics/sankey")
                                        .route(web::post().to(get_merchant_sankey)),
                                )
                                .service(
                                    web::resource("metrics/auth_events/sankey")
                                        .route(web::post().to(get_merchant_auth_event_sankey)),
                                ),
                        )
                        .service(
                            web::scope("/org")
                                .service(
                                    web::resource("{domain}/info").route(web::get().to(get_info)),
                                )
                                .service(
                                    web::resource("metrics/payments")
                                        .route(web::post().to(get_org_payment_metrics)),
                                )
                                .service(
                                    web::resource("filters/payments")
                                        .route(web::post().to(get_org_payment_filters)),
                                )
                                .service(
                                    web::resource("metrics/routing")
                                        .route(web::post().to(get_org_payment_metrics)),
                                )
                                .service(
                                    web::resource("filters/routing")
                                        .route(web::post().to(get_org_payment_filters)),
                                )
                                .service(
                                    web::resource("metrics/refunds")
                                        .route(web::post().to(get_org_refund_metrics)),
                                )
                                .service(
                                    web::resource("filters/refunds")
                                        .route(web::post().to(get_org_refund_filters)),
                                )
                                .service(
                                    web::resource("metrics/disputes")
                                        .route(web::post().to(get_org_dispute_metrics)),
                                )
                                .service(
                                    web::resource("metrics/auth_events")
                                        .route(web::post().to(get_org_auth_event_metrics)),
                                )
                                .service(
                                    web::resource("filters/disputes")
                                        .route(web::post().to(get_org_dispute_filters)),
                                )
                                .service(
                                    web::resource("filters/auth_events")
                                        .route(web::post().to(get_org_auth_events_filters)),
                                )
                                .service(
                                    web::resource("report/dispute")
                                        .route(web::post().to(generate_org_dispute_report)),
                                )
                                .service(
                                    web::resource("report/refunds")
                                        .route(web::post().to(generate_org_refund_report)),
                                )
                                .service(
                                    web::resource("report/payments")
                                        .route(web::post().to(generate_org_payment_report)),
                                )
                                .service(
                                    web::resource("report/payouts")
                                        .route(web::post().to(generate_org_payout_report)),
                                )
                                .service(
                                    web::resource("report/authentications")
                                        .route(web::post().to(generate_org_authentication_report)),
                                )
                                .service(
                                    web::resource("metrics/sankey")
                                        .route(web::post().to(get_org_sankey)),
                                )
                                .service(
                                    web::resource("metrics/auth_events/sankey")
                                        .route(web::post().to(get_org_auth_event_sankey)),
                                ),
                        )
                        .service(
                            web::scope("/profile")
                                .service(
                                    web::resource("{domain}/info").route(web::get().to(get_info)),
                                )
                                .service(
                                    web::resource("metrics/payments")
                                        .route(web::post().to(get_profile_payment_metrics)),
                                )
                                .service(
                                    web::resource("filters/payments")
                                        .route(web::post().to(get_profile_payment_filters)),
                                )
                                .service(
                                    web::resource("metrics/routing")
                                        .route(web::post().to(get_profile_payment_metrics)),
                                )
                                .service(
                                    web::resource("filters/routing")
                                        .route(web::post().to(get_profile_payment_filters)),
                                )
                                .service(
                                    web::resource("metrics/refunds")
                                        .route(web::post().to(get_profile_refund_metrics)),
                                )
                                .service(
                                    web::resource("filters/refunds")
                                        .route(web::post().to(get_profile_refund_filters)),
                                )
                                .service(
                                    web::resource("metrics/disputes")
                                        .route(web::post().to(get_profile_dispute_metrics)),
                                )
                                .service(
                                    web::resource("metrics/auth_events")
                                        .route(web::post().to(get_profile_auth_event_metrics)),
                                )
                                .service(
                                    web::resource("filters/disputes")
                                        .route(web::post().to(get_profile_dispute_filters)),
                                )
                                .service(
                                    web::resource("filters/auth_events")
                                        .route(web::post().to(get_profile_auth_events_filters)),
                                )
                                .service(
                                    web::resource("connector_event_logs")
                                        .route(web::get().to(get_profile_connector_events)),
                                )
                                .service(
                                    web::resource("routing_event_logs")
                                        .route(web::get().to(get_profile_routing_events)),
                                )
                                .service(
                                    web::resource("outgoing_webhook_event_logs")
                                        .route(web::get().to(get_profile_outgoing_webhook_events)),
                                )
                                .service(
                                    web::resource("report/dispute")
                                        .route(web::post().to(generate_profile_dispute_report)),
                                )
                                .service(
                                    web::resource("report/refunds")
                                        .route(web::post().to(generate_profile_refund_report)),
                                )
                                .service(
                                    web::resource("report/payments")
                                        .route(web::post().to(generate_profile_payment_report)),
                                )
                                .service(
                                    web::resource("report/payouts")
                                        .route(web::post().to(generate_profile_payout_report)),
                                )
                                .service(
                                    web::resource("report/authentications").route(
                                        web::post().to(generate_profile_authentication_report),
                                    ),
                                )
                                .service(
                                    web::resource("api_event_logs")
                                        .route(web::get().to(get_profile_api_events)),
                                )
                                .service(
                                    web::resource("sdk_event_logs")
                                        .route(web::post().to(get_profile_sdk_events)),
                                )
                                .service(
                                    web::resource("metrics/sankey")
                                        .route(web::post().to(get_profile_sankey)),
                                )
                                .service(
                                    web::resource("metrics/auth_events/sankey")
                                        .route(web::post().to(get_profile_auth_event_sankey)),
                                ),
                        ),
                )
                .service(
                    web::scope("/v2")
                        .service(
                            web::resource("/metrics/payments")
                                .route(web::post().to(get_merchant_payment_intent_metrics)),
                        )
                        .service(
                            web::resource("/filters/payments")
                                .route(web::post().to(get_payment_intents_filters)),
                        )
                        .service(
                            web::scope("/merchant")
                                .service(
                                    web::resource("/metrics/payments")
                                        .route(web::post().to(get_merchant_payment_intent_metrics)),
                                )
                                .service(
                                    web::resource("/filters/payments")
                                        .route(web::post().to(get_payment_intents_filters)),
                                ),
                        )
                        .service(
                            web::scope("/org").service(
                                web::resource("/metrics/payments")
                                    .route(web::post().to(get_org_payment_intent_metrics)),
                            ),
                        )
                        .service(
                            web::scope("/profile").service(
                                web::resource("/metrics/payments")
                                    .route(web::post().to(get_profile_payment_intent_metrics)),
                            ),
                        ),
                )
        }
    }

    pub async fn get_info(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        domain: web::Path<analytics::AnalyticsDomain>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetInfo;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            domain.into_inner(),
            |_, _: (), domain: analytics::AnalyticsDomain, _| async {
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
    pub async fn get_merchant_payment_metrics(
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
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };
                let validator_response = request_validator(
                    AnalyticsRequest {
                        payment_attempt: Some(req.clone()),
                        ..Default::default()
                    },
                    &state,
                )
                .await?;
                let ex_rates = validator_response;
                analytics::payments::get_metrics(&state.pool, &ex_rates, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetPaymentMetricRequest` element.
    pub async fn get_org_payment_metrics(
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
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };

                let validator_response = request_validator(
                    AnalyticsRequest {
                        payment_attempt: Some(req.clone()),
                        ..Default::default()
                    },
                    &state,
                )
                .await?;
                let ex_rates = validator_response;
                analytics::payments::get_metrics(&state.pool, &ex_rates, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetPaymentMetricRequest` element.
    pub async fn get_profile_payment_metrics(
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
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };

                let validator_response = request_validator(
                    AnalyticsRequest {
                        payment_attempt: Some(req.clone()),
                        ..Default::default()
                    },
                    &state,
                )
                .await?;
                let ex_rates = validator_response;
                analytics::payments::get_metrics(&state.pool, &ex_rates, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetPaymentIntentMetricRequest` element.
    pub async fn get_merchant_payment_intent_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetPaymentIntentMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetPaymentIntentMetricRequest");
        let flow = AnalyticsFlow::GetPaymentIntentMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };

                let validator_response = request_validator(
                    AnalyticsRequest {
                        payment_intent: Some(req.clone()),
                        ..Default::default()
                    },
                    &state,
                )
                .await?;
                let ex_rates = validator_response;
                analytics::payment_intents::get_metrics(&state.pool, &ex_rates, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetPaymentIntentMetricRequest` element.
    pub async fn get_org_payment_intent_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetPaymentIntentMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetPaymentIntentMetricRequest");
        let flow = AnalyticsFlow::GetPaymentIntentMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };

                let validator_response = request_validator(
                    AnalyticsRequest {
                        payment_intent: Some(req.clone()),
                        ..Default::default()
                    },
                    &state,
                )
                .await?;
                let ex_rates = validator_response;
                analytics::payment_intents::get_metrics(&state.pool, &ex_rates, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetPaymentIntentMetricRequest` element.
    pub async fn get_profile_payment_intent_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetPaymentIntentMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetPaymentIntentMetricRequest");
        let flow = AnalyticsFlow::GetPaymentIntentMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };

                let validator_response = request_validator(
                    AnalyticsRequest {
                        payment_intent: Some(req.clone()),
                        ..Default::default()
                    },
                    &state,
                )
                .await?;
                let ex_rates = validator_response;
                analytics::payment_intents::get_metrics(&state.pool, &ex_rates, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetRefundMetricRequest` element.
    pub async fn get_merchant_refund_metrics(
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
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };

                let validator_response = request_validator(
                    AnalyticsRequest {
                        refund: Some(req.clone()),
                        ..Default::default()
                    },
                    &state,
                )
                .await?;
                let ex_rates = validator_response;
                analytics::refunds::get_metrics(&state.pool, &ex_rates, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetRefundMetricRequest` element.
    pub async fn get_org_refund_metrics(
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
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };

                let validator_response = request_validator(
                    AnalyticsRequest {
                        refund: Some(req.clone()),
                        ..Default::default()
                    },
                    &state,
                )
                .await?;
                let ex_rates = validator_response;
                analytics::refunds::get_metrics(&state.pool, &ex_rates, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetRefundMetricRequest` element.
    pub async fn get_profile_refund_metrics(
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
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };

                let validator_response = request_validator(
                    AnalyticsRequest {
                        refund: Some(req.clone()),
                        ..Default::default()
                    },
                    &state,
                )
                .await?;
                let ex_rates = validator_response;
                analytics::refunds::get_metrics(&state.pool, &ex_rates, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetFrmMetricRequest` element.
    pub async fn get_frm_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetFrmMetricRequest; 1]>,
    ) -> impl Responder {
        #[allow(clippy::expect_used)]
        // safety: This shouldn't panic owing to the data type
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetFrmMetricRequest");
        let flow = AnalyticsFlow::GetFrmMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                analytics::frm::get_metrics(&state.pool, auth.merchant_account.get_id(), req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetSdkEventMetricRequest` element.
    pub async fn get_sdk_event_metrics(
        state: web::Data<AppState>,
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
        let flow = AnalyticsFlow::GetSdkMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                analytics::sdk_events::get_metrics(
                    &state.pool,
                    &auth.merchant_account.publishable_key,
                    req,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetActivePaymentsMetricRequest` element.
    pub async fn get_active_payments_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetActivePaymentsMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetActivePaymentsMetricRequest");
        let flow = AnalyticsFlow::GetActivePaymentsMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                analytics::active_payments::get_metrics(
                    &state.pool,
                    &auth.merchant_account.publishable_key,
                    auth.merchant_account.get_id(),
                    req,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetAuthEventMetricRequest` element.
    pub async fn get_merchant_auth_event_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetAuthEventMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetAuthEventMetricRequest");
        let flow = AnalyticsFlow::GetAuthMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };

                analytics::auth_events::get_metrics(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetAuthEventMetricRequest` element.
    pub async fn get_profile_auth_event_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetAuthEventMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetAuthEventMetricRequest");
        let flow = AnalyticsFlow::GetAuthMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };
                analytics::auth_events::get_metrics(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetAuthEventMetricRequest` element.
    pub async fn get_org_auth_event_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetAuthEventMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetAuthEventMetricRequest");
        let flow = AnalyticsFlow::GetAuthMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };
                analytics::auth_events::get_metrics(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_merchant_payment_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetPaymentFiltersRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetPaymentFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };
                analytics::payments::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_merchant_auth_events_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetAuthEventFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetAuthEventFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();

                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };
                analytics::auth_events::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_org_auth_events_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetAuthEventFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetAuthEventFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };

                analytics::auth_events::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_profile_auth_events_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetAuthEventFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetAuthEventFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;

                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };
                analytics::auth_events::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_org_payment_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetPaymentFiltersRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetPaymentFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };
                analytics::payments::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_profile_payment_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetPaymentFiltersRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetPaymentFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };
                analytics::payments::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_payment_intents_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetPaymentIntentFiltersRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetPaymentIntentFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                analytics::payment_intents::get_filters(
                    &state.pool,
                    req,
                    auth.merchant_account.get_id(),
                )
                .await
                .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::MerchantAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_merchant_refund_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetRefundFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetRefundFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req: GetRefundFilterRequest, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };
                analytics::refunds::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_org_refund_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetRefundFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetRefundFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req: GetRefundFilterRequest, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };
                analytics::refunds::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_profile_refund_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetRefundFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetRefundFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req: GetRefundFilterRequest, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };
                analytics::refunds::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_frm_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetFrmFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetFrmFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req: GetFrmFilterRequest, _| async move {
                analytics::frm::get_filters(&state.pool, req, auth.merchant_account.get_id())
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_sdk_event_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetSdkEventFiltersRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetSdkEventFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                analytics::sdk_events::get_filters(
                    &state.pool,
                    req,
                    &auth.merchant_account.publishable_key,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_profile_api_events(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Query<api_models::analytics::api_event::ApiLogsRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetApiEvents;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let payment_id = match req.query_param.clone() {
                    QueryType::Payment { payment_id } => payment_id,
                    QueryType::Refund { payment_id, .. } => payment_id,
                    QueryType::Dispute { payment_id, .. } => payment_id,
                };
                utils::check_if_profile_id_is_present_in_payment_intent(payment_id, &state, &auth)
                    .await
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                api_events_core(&state.pool, req, auth.merchant_account.get_id())
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_profile_outgoing_webhook_events(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Query<
            api_models::analytics::outgoing_webhook_event::OutgoingWebhookLogsRequest,
        >,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetOutgoingWebhookEvents;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                utils::check_if_profile_id_is_present_in_payment_intent(
                    req.payment_id.clone(),
                    &state,
                    &auth,
                )
                .await
                .change_context(AnalyticsError::AccessForbiddenError)?;
                outgoing_webhook_events_core(&state.pool, req, auth.merchant_account.get_id())
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_profile_sdk_events(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<api_models::analytics::sdk_events::SdkEventsRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetSdkEvents;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                utils::check_if_profile_id_is_present_in_payment_intent(
                    req.payment_id.clone(),
                    &state,
                    &auth,
                )
                .await
                .change_context(AnalyticsError::AccessForbiddenError)?;
                sdk_events_core(&state.pool, req, &auth.merchant_account.publishable_key)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_merchant_refund_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GenerateRefundReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::MerchantLevel {
                        org_id: org_id.clone(),
                        merchant_ids: vec![merchant_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.refund_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_org_refund_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GenerateRefundReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: None,
                    auth: AuthInfo::OrgLevel {
                        org_id: org_id.clone(),
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.refund_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::OrganizationReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_profile_refund_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GenerateRefundReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::ProfileLevel {
                        org_id: org_id.clone(),
                        merchant_id: merchant_id.clone(),
                        profile_ids: vec![profile_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.refund_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
    #[cfg(feature = "v1")]
    pub async fn generate_merchant_dispute_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GenerateDisputeReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::MerchantLevel {
                        org_id: org_id.clone(),
                        merchant_ids: vec![merchant_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.dispute_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
    #[cfg(feature = "v1")]
    pub async fn generate_org_dispute_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GenerateDisputeReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: None,
                    auth: AuthInfo::OrgLevel {
                        org_id: org_id.clone(),
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.dispute_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::OrganizationReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_profile_dispute_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GenerateDisputeReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::ProfileLevel {
                        org_id: org_id.clone(),
                        merchant_id: merchant_id.clone(),
                        profile_ids: vec![profile_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.dispute_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_merchant_payout_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GeneratePayoutReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::MerchantLevel {
                        org_id: org_id.clone(),
                        merchant_ids: vec![merchant_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.payout_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_org_payout_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GeneratePayoutReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: None,
                    auth: AuthInfo::OrgLevel {
                        org_id: org_id.clone(),
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.payout_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::OrganizationReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_profile_payout_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GeneratePayoutReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::ProfileLevel {
                        org_id: org_id.clone(),
                        merchant_id: merchant_id.clone(),
                        profile_ids: vec![profile_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.payout_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_merchant_payment_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GeneratePaymentReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::MerchantLevel {
                        org_id: org_id.clone(),
                        merchant_ids: vec![merchant_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.payment_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_org_payment_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GeneratePaymentReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: None,
                    auth: AuthInfo::OrgLevel {
                        org_id: org_id.clone(),
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.payment_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::OrganizationReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_profile_payment_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GeneratePaymentReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::ProfileLevel {
                        org_id: org_id.clone(),
                        merchant_id: merchant_id.clone(),
                        profile_ids: vec![profile_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.payment_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_merchant_authentication_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GenerateAuthenticationReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::MerchantLevel {
                        org_id: org_id.clone(),
                        merchant_ids: vec![merchant_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.authentication_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_org_authentication_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GenerateAuthenticationReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let org_id = auth.merchant_account.get_org_id();
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: None,
                    auth: AuthInfo::OrgLevel {
                        org_id: org_id.clone(),
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.authentication_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::OrganizationReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn generate_profile_authentication_report(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<ReportRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GenerateAuthenticationReport;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload, _| async move {
                let user = UserInterface::find_user_by_id(&*state.global_store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: Some(merchant_id.clone()),
                    auth: AuthInfo::ProfileLevel {
                        org_id: org_id.clone(),
                        merchant_id: merchant_id.clone(),
                        profile_ids: vec![profile_id.clone()],
                    },
                    email: user_email,
                };

                let json_bytes =
                    serde_json::to_vec(&lambda_req).map_err(|_| AnalyticsError::UnknownError)?;
                invoke_lambda(
                    &state.conf.report_download_config.authentication_function,
                    &state.conf.report_download_config.region,
                    &json_bytes,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileReportRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetApiEventMetricRequest` element.
    pub async fn get_merchant_api_events_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetApiEventMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetApiEventMetricRequest");
        let flow = AnalyticsFlow::GetApiEventMetrics;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                analytics::api_event::get_api_event_metrics(
                    &state.pool,
                    auth.merchant_account.get_id(),
                    req,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_merchant_api_event_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetApiEventFiltersRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetApiEventFilters;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                analytics::api_event::get_filters(&state.pool, req, auth.merchant_account.get_id())
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_profile_connector_events(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Query<api_models::analytics::connector_events::ConnectorEventsRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetConnectorEvents;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                utils::check_if_profile_id_is_present_in_payment_intent(
                    req.payment_id.clone(),
                    &state,
                    &auth,
                )
                .await
                .change_context(AnalyticsError::AccessForbiddenError)?;
                connector_events_core(&state.pool, req, auth.merchant_account.get_id())
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_profile_routing_events(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Query<api_models::analytics::routing_events::RoutingEventsRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetRoutingEvents;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                utils::check_if_profile_id_is_present_in_payment_intent(
                    req.payment_id.clone(),
                    &state,
                    &auth,
                )
                .await
                .change_context(AnalyticsError::AccessForbiddenError)?;
                routing_events_core(&state.pool, req, auth.merchant_account.get_id())
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::HeaderAuth(auth::ApiKeyAuth {
                    is_connected_allowed: false,
                    is_platform_allowed: false,
                }),
                &auth::JWTAuth {
                    permission: Permission::ProfileAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_global_search_results(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetGlobalSearchRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetGlobalSearchResults;
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            json_payload.into_inner(),
            |state, auth: UserFromToken, req, _| async move {
                let role_id = auth.role_id;
                let role_info = RoleInfo::from_role_id_org_id_tenant_id(
                    &state,
                    &role_id,
                    &auth.org_id,
                    auth.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
                )
                .await
                .change_context(UserErrors::InternalServerError)
                .change_context(OpenSearchError::UnknownError)?;
                let permission_groups = role_info.get_permission_groups();
                if !permission_groups.contains(&common_enums::PermissionGroup::OperationsView) {
                    return Err(OpenSearchError::AccessForbiddenError)?;
                }
                let user_roles: HashSet<UserRole> = match role_info.get_entity_type() {
                    EntityType::Tenant => state
                        .global_store
                        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                            user_id: &auth.user_id,
                            tenant_id: auth.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
                            org_id: None,
                            merchant_id: None,
                            profile_id: None,
                            entity_id: None,
                            version: None,
                            status: None,
                            limit: None,
                        })
                        .await
                        .change_context(UserErrors::InternalServerError)
                        .change_context(OpenSearchError::UnknownError)?
                        .into_iter()
                        .collect(),
                    EntityType::Organization | EntityType::Merchant | EntityType::Profile => state
                        .global_store
                        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                            user_id: &auth.user_id,
                            tenant_id: auth.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
                            org_id: Some(&auth.org_id),
                            merchant_id: None,
                            profile_id: None,
                            entity_id: None,
                            version: None,
                            status: None,
                            limit: None,
                        })
                        .await
                        .change_context(UserErrors::InternalServerError)
                        .change_context(OpenSearchError::UnknownError)?
                        .into_iter()
                        .collect(),
                };

                let state = Arc::new(state);
                let role_info_map: HashMap<String, RoleInfo> = user_roles
                    .iter()
                    .map(|user_role| {
                        let state = Arc::clone(&state);
                        let role_id = user_role.role_id.clone();
                        let org_id = user_role.org_id.clone().unwrap_or_default();
                        let tenant_id = &user_role.tenant_id;
                        async move {
                            RoleInfo::from_role_id_org_id_tenant_id(
                                &state, &role_id, &org_id, tenant_id,
                            )
                            .await
                            .change_context(UserErrors::InternalServerError)
                            .change_context(OpenSearchError::UnknownError)
                            .map(|role_info| (role_id, role_info))
                        }
                    })
                    .collect::<FuturesUnordered<_>>()
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .collect::<Result<HashMap<_, _>, _>>()?;

                let filtered_user_roles: Vec<&UserRole> = user_roles
                    .iter()
                    .filter(|user_role| {
                        let user_role_id = &user_role.role_id;
                        if let Some(role_info) = role_info_map.get(user_role_id) {
                            let permissions = role_info.get_permission_groups();
                            permissions.contains(&common_enums::PermissionGroup::OperationsView)
                        } else {
                            false
                        }
                    })
                    .collect();

                let search_params: Vec<AuthInfo> = filtered_user_roles
                    .iter()
                    .filter_map(|user_role| {
                        user_role
                            .get_entity_id_and_type()
                            .and_then(|(_, entity_type)| match entity_type {
                                EntityType::Profile => Some(AuthInfo::ProfileLevel {
                                    org_id: user_role.org_id.clone()?,
                                    merchant_id: user_role.merchant_id.clone()?,
                                    profile_ids: vec![user_role.profile_id.clone()?],
                                }),
                                EntityType::Merchant => Some(AuthInfo::MerchantLevel {
                                    org_id: user_role.org_id.clone()?,
                                    merchant_ids: vec![user_role.merchant_id.clone()?],
                                }),
                                EntityType::Organization => Some(AuthInfo::OrgLevel {
                                    org_id: user_role.org_id.clone()?,
                                }),
                                EntityType::Tenant => Some(AuthInfo::OrgLevel {
                                    org_id: auth.org_id.clone(),
                                }),
                            })
                    })
                    .collect();

                analytics::search::msearch_results(
                    state
                        .opensearch_client
                        .as_ref()
                        .ok_or_else(|| error_stack::report!(OpenSearchError::NotEnabled))?,
                    req,
                    search_params,
                    SEARCH_INDEXES.to_vec(),
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_search_results(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<GetSearchRequest>,
        index: web::Path<SearchIndex>,
    ) -> impl Responder {
        let index = index.into_inner();
        let flow = AnalyticsFlow::GetSearchResults;
        let indexed_req = GetSearchRequestWithIndex {
            search_req: json_payload.into_inner(),
            index,
        };
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            indexed_req,
            |state, auth: UserFromToken, req, _| async move {
                let role_id = auth.role_id;
                let role_info = RoleInfo::from_role_id_org_id_tenant_id(
                    &state,
                    &role_id,
                    &auth.org_id,
                    auth.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
                )
                .await
                .change_context(UserErrors::InternalServerError)
                .change_context(OpenSearchError::UnknownError)?;
                let permission_groups = role_info.get_permission_groups();
                if !permission_groups.contains(&common_enums::PermissionGroup::OperationsView) {
                    return Err(OpenSearchError::AccessForbiddenError)?;
                }
                let user_roles: HashSet<UserRole> = match role_info.get_entity_type() {
                    EntityType::Tenant => state
                        .global_store
                        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                            user_id: &auth.user_id,
                            tenant_id: auth.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
                            org_id: None,
                            merchant_id: None,
                            profile_id: None,
                            entity_id: None,
                            version: None,
                            status: None,
                            limit: None,
                        })
                        .await
                        .change_context(UserErrors::InternalServerError)
                        .change_context(OpenSearchError::UnknownError)?
                        .into_iter()
                        .collect(),
                    EntityType::Organization | EntityType::Merchant | EntityType::Profile => state
                        .global_store
                        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                            user_id: &auth.user_id,
                            tenant_id: auth.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
                            org_id: Some(&auth.org_id),
                            merchant_id: None,
                            profile_id: None,
                            entity_id: None,
                            version: None,
                            status: None,
                            limit: None,
                        })
                        .await
                        .change_context(UserErrors::InternalServerError)
                        .change_context(OpenSearchError::UnknownError)?
                        .into_iter()
                        .collect(),
                };
                let state = Arc::new(state);
                let role_info_map: HashMap<String, RoleInfo> = user_roles
                    .iter()
                    .map(|user_role| {
                        let state = Arc::clone(&state);
                        let role_id = user_role.role_id.clone();
                        let org_id = user_role.org_id.clone().unwrap_or_default();
                        let tenant_id = &user_role.tenant_id;
                        async move {
                            RoleInfo::from_role_id_org_id_tenant_id(
                                &state, &role_id, &org_id, tenant_id,
                            )
                            .await
                            .change_context(UserErrors::InternalServerError)
                            .change_context(OpenSearchError::UnknownError)
                            .map(|role_info| (role_id, role_info))
                        }
                    })
                    .collect::<FuturesUnordered<_>>()
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .collect::<Result<HashMap<_, _>, _>>()?;

                let filtered_user_roles: Vec<&UserRole> = user_roles
                    .iter()
                    .filter(|user_role| {
                        let user_role_id = &user_role.role_id;
                        if let Some(role_info) = role_info_map.get(user_role_id) {
                            let permissions = role_info.get_permission_groups();
                            permissions.contains(&common_enums::PermissionGroup::OperationsView)
                        } else {
                            false
                        }
                    })
                    .collect();

                let search_params: Vec<AuthInfo> = filtered_user_roles
                    .iter()
                    .filter_map(|user_role| {
                        user_role
                            .get_entity_id_and_type()
                            .and_then(|(_, entity_type)| match entity_type {
                                EntityType::Profile => Some(AuthInfo::ProfileLevel {
                                    org_id: user_role.org_id.clone()?,
                                    merchant_id: user_role.merchant_id.clone()?,
                                    profile_ids: vec![user_role.profile_id.clone()?],
                                }),
                                EntityType::Merchant => Some(AuthInfo::MerchantLevel {
                                    org_id: user_role.org_id.clone()?,
                                    merchant_ids: vec![user_role.merchant_id.clone()?],
                                }),
                                EntityType::Organization => Some(AuthInfo::OrgLevel {
                                    org_id: user_role.org_id.clone()?,
                                }),
                                EntityType::Tenant => Some(AuthInfo::OrgLevel {
                                    org_id: auth.org_id.clone(),
                                }),
                            })
                    })
                    .collect();
                analytics::search::search_results(
                    state
                        .opensearch_client
                        .as_ref()
                        .ok_or_else(|| error_stack::report!(OpenSearchError::NotEnabled))?,
                    req,
                    search_params,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_merchant_dispute_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<api_models::analytics::GetDisputeFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetDisputeFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };
                analytics::disputes::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_profile_dispute_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<api_models::analytics::GetDisputeFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetDisputeFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };
                analytics::disputes::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_org_dispute_filters(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<api_models::analytics::GetDisputeFilterRequest>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetDisputeFilters;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            json_payload.into_inner(),
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };
                analytics::disputes::get_filters(&state.pool, req, &auth)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetDisputeMetricRequest` element.
    pub async fn get_merchant_dispute_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetDisputeMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetDisputeMetricRequest");
        let flow = AnalyticsFlow::GetDisputeMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };
                analytics::disputes::get_metrics(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetDisputeMetricRequest` element.
    pub async fn get_profile_dispute_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetDisputeMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetDisputeMetricRequest");
        let flow = AnalyticsFlow::GetDisputeMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };
                analytics::disputes::get_metrics(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetDisputeMetricRequest` element.
    pub async fn get_org_dispute_metrics(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<[GetDisputeMetricRequest; 1]>,
    ) -> impl Responder {
        // safety: This shouldn't panic owing to the data type
        #[allow(clippy::expect_used)]
        let payload = json_payload
            .into_inner()
            .to_vec()
            .pop()
            .expect("Couldn't get GetDisputeMetricRequest");
        let flow = AnalyticsFlow::GetDisputeMetrics;
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };
                analytics::disputes::get_metrics(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_merchant_sankey(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<TimeRange>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetSankey;
        let payload = json_payload.into_inner();
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };
                analytics::payment_intents::get_sankey(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    pub async fn get_merchant_auth_event_sankey(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<TimeRange>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetSankey;
        let payload = json_payload.into_inner();
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let auth: AuthInfo = AuthInfo::MerchantLevel {
                    org_id: org_id.clone(),
                    merchant_ids: vec![merchant_id.clone()],
                };
                analytics::auth_events::get_sankey(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::MerchantAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_org_auth_event_sankey(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<TimeRange>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetSankey;
        let payload = json_payload.into_inner();
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };
                analytics::auth_events::get_sankey(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_profile_auth_event_sankey(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<TimeRange>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetSankey;
        let payload = json_payload.into_inner();
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };
                analytics::auth_events::get_sankey(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_org_sankey(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<TimeRange>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetSankey;
        let payload = json_payload.into_inner();
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let auth: AuthInfo = AuthInfo::OrgLevel {
                    org_id: org_id.clone(),
                };
                analytics::payment_intents::get_sankey(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            auth::auth_type(
                &auth::PlatformOrgAdminAuth {
                    is_admin_auth_allowed: false,
                    organization_id: None,
                },
                &auth::JWTAuth {
                    permission: Permission::OrganizationAnalyticsRead,
                },
                req.headers(),
            ),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn get_profile_sankey(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        json_payload: web::Json<TimeRange>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetSankey;
        let payload = json_payload.into_inner();
        Box::pin(api::server_wrap(
            flow,
            state,
            &req,
            payload,
            |state: crate::routes::SessionState, auth: AuthenticationData, req, _| async move {
                let org_id = auth.merchant_account.get_org_id();
                let merchant_id = auth.merchant_account.get_id();
                let profile_id = auth
                    .profile_id
                    .ok_or(report!(UserErrors::JwtProfileIdMissing))
                    .change_context(AnalyticsError::AccessForbiddenError)?;
                let auth: AuthInfo = AuthInfo::ProfileLevel {
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                    profile_ids: vec![profile_id.clone()],
                };
                analytics::payment_intents::get_sankey(&state.pool, &auth, req)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAnalyticsRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}
