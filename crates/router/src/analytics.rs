pub use analytics::*;

pub mod routes {
    use actix_web::{web, Responder, Scope};
    use analytics::{
        api_event::api_events_core, connector_events::connector_events_core,
        errors::AnalyticsError, lambda_utils::invoke_lambda,
        outgoing_webhook_event::outgoing_webhook_events_core, sdk_events::sdk_events_core,
    };
    use api_models::analytics::{
        GenerateReportRequest, GetApiEventFiltersRequest, GetApiEventMetricRequest,
        GetPaymentFiltersRequest, GetPaymentMetricRequest, GetRefundFilterRequest,
        GetRefundMetricRequest, GetSdkEventFiltersRequest, GetSdkEventMetricRequest, ReportRequest,
    };
    use error_stack::ResultExt;
    use router_env::AnalyticsFlow;

    use crate::{
        core::api_locking,
        db::user::UserInterface,
        routes::AppState,
        services::{
            api,
            authentication::{self as auth, AuthenticationData},
            authorization::permissions::Permission,
            ApplicationResponse,
        },
        types::domain::UserEmail,
    };

    pub struct Analytics;

    impl Analytics {
                /// This method creates and configures routes for various analytics endpoints in the server. It takes an `AppState` as input and returns a `Scope` with all the configured routes.
        pub fn server(state: AppState) -> Scope {
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
                    .service(
                        web::resource("report/dispute")
                            .route(web::post().to(generate_dispute_report)),
                    )
                    .service(
                        web::resource("report/refunds")
                            .route(web::post().to(generate_refund_report)),
                    )
                    .service(
                        web::resource("report/payments")
                            .route(web::post().to(generate_payment_report)),
                    )
                    .service(
                        web::resource("metrics/sdk_events")
                            .route(web::post().to(get_sdk_event_metrics)),
                    )
                    .service(
                        web::resource("filters/sdk_events")
                            .route(web::post().to(get_sdk_event_filters)),
                    )
                    .service(web::resource("api_event_logs").route(web::get().to(get_api_events)))
                    .service(web::resource("sdk_event_logs").route(web::post().to(get_sdk_events)))
                    .service(
                        web::resource("connector_event_logs")
                            .route(web::get().to(get_connector_events)),
                    )
                    .service(
                        web::resource("outgoing_webhook_event_logs")
                            .route(web::get().to(get_outgoing_webhook_events)),
                    )
                    .service(
                        web::resource("filters/api_events")
                            .route(web::post().to(get_api_event_filters)),
                    )
                    .service(
                        web::resource("metrics/api_events")
                            .route(web::post().to(get_api_events_metrics)),
                    )
            }
            route
        }
    }

        /// Asynchronously retrieves information about the specified domain from the analytics service.
    pub async fn get_info(
        state: web::Data<AppState>,
        req: actix_web::HttpRequest,
        domain: actix_web::web::Path<analytics::AnalyticsDomain>,
    ) -> impl Responder {
        let flow = AnalyticsFlow::GetInfo;
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
            |state, auth: AuthenticationData, req| async move {
                analytics::sdk_events::get_metrics(
                    &state.pool,
                    auth.merchant_account.publishable_key.as_ref(),
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

        /// Asynchronously handles a request to get payment filters by extracting the necessary data from the request, calling the appropriate analytics method to retrieve the filters, and returning the result as a JSON response. This method requires the AppState, HttpRequest, and a JSON payload containing the GetPaymentFiltersRequest.
    pub async fn get_payment_filters(
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

        /// Asynchronously handles a HTTP request to retrieve refund filters from the analytics server.
    pub async fn get_refund_filters(
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

        /// Asynchronously handles the HTTP POST request to fetch SDK event filters and returns a responder. It extracts the `AppState` from the web data, the `HttpRequest` and a JSON payload of type `GetSdkEventFiltersRequest`. It then creates an `AnalyticsFlow` of type `GetSdkEventFilters` and uses it to call the `api::server_wrap` method with necessary parameters to fetch the SDK event filters from the database. Finally, it returns the result as a responder.
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
            |state, auth: AuthenticationData, req| async move {
                analytics::sdk_events::get_filters(
                    &state.pool,
                    req,
                    auth.merchant_account.publishable_key.as_ref(),
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

        /// Retrieves API events using the provided request data and returns a responder.
    pub async fn get_api_events(
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
            |state, auth: AuthenticationData, req| async move {
                api_events_core(&state.pool, req, auth.merchant_account.merchant_id)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

        /// Asynchronously handles the incoming request to retrieve outgoing webhook events for analytics purposes. It takes in the current application state, the HTTP request, and the JSON payload containing the outgoing webhook logs request. It then wraps the processing of the request in a server_wrap function, which handles authentication, authorization, and locking, before calling the core logic for retrieving outgoing webhook events. The method returns a responder for the HTTP response.
    pub async fn get_outgoing_webhook_events(
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
            |state, auth: AuthenticationData, req| async move {
                outgoing_webhook_events_core(&state.pool, req, auth.merchant_account.merchant_id)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

        /// This method is used to handle the HTTP request for retrieving SDK events for analytics. It takes the application state, HTTP request, and JSON payload as input and returns a responder. It creates an analytics flow for getting SDK events, then wraps the API server call with the necessary parameters and authentication data. Finally, it awaits the result of the API call and returns the application response as JSON.
    pub async fn get_sdk_events(
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
            |state, auth: AuthenticationData, req| async move {
                sdk_events_core(
                    &state.pool,
                    req,
                    auth.merchant_account.publishable_key.unwrap_or_default(),
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

        /// Asynchronously generates a refund report by invoking a lambda function with the provided payload and user information. 
    /// 
    /// # Arguments
    /// 
    /// * `state` - The application state data
    /// * `req` - The HTTP request
    /// * `json_payload` - The JSON payload containing the report request
    /// 
    /// # Returns
    /// 
    /// The response from the lambda function as a Responder
    /// 
    pub async fn generate_refund_report(
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
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload| async move {
                let user = UserInterface::find_user_by_id(&*state.store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: auth.merchant_account.merchant_id.to_string(),
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
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

        /// Generate a dispute report by invoking a Lambda function and returning the result as a JSON response. Requires authentication with JWT and permission for analytics.
    pub async fn generate_dispute_report(
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
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload| async move {
                let user = UserInterface::find_user_by_id(&*state.store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: auth.merchant_account.merchant_id.to_string(),
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
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

        /// Asynchronously generates a payment report by invoking a Lambda function. 
    ///
    /// # Arguments
    ///
    /// * `state` - the web AppState
    /// * `req` - the actix_web HttpRequest
    /// * `json_payload` - the web Json containing the ReportRequest payload
    ///
    /// # Returns
    ///
    /// The generated payment report as a Responder
    pub async fn generate_payment_report(
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
            |state, (auth, user_id): auth::AuthenticationDataWithUserId, payload| async move {
                let user = UserInterface::find_user_by_id(&*state.store, &user_id)
                    .await
                    .change_context(AnalyticsError::UnknownError)?;

                let user_email = UserEmail::from_pii_email(user.email)
                    .change_context(AnalyticsError::UnknownError)?
                    .get_secret();

                let lambda_req = GenerateReportRequest {
                    request: payload,
                    merchant_id: auth.merchant_account.merchant_id.to_string(),
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
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    /// # Panics
    ///
    /// Panics if `json_payload` array does not contain one `GetApiEventMetricRequest` element.
    pub async fn get_api_events_metrics(
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
            |state, auth: AuthenticationData, req| async move {
                analytics::api_event::get_api_event_metrics(
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

        /// Asynchronously handles the request to retrieve API event filters. It wraps the flow in a server and executes the get_filters function from the analytics module, returning the result as a JSON response. Requires authentication with the 'Analytics' permission. 
    pub async fn get_api_event_filters(
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
            |state, auth: AuthenticationData, req| async move {
                analytics::api_event::get_filters(
                    &state.pool,
                    req,
                    auth.merchant_account.merchant_id,
                )
                .await
                .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

        /// Asynchronously handles the HTTP request to retrieve connector events by calling the `connector_events_core` function and returning the result as a JSON response. It requires the `AppState` data, the HTTP request, and a JSON payload containing the request parameters. This method uses the `api::server_wrap` function to wrap the flow, state, request, payload, and authentication data, and then waits for the result using `Box::pin` and `await`.
    pub async fn get_connector_events(
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
            |state, auth: AuthenticationData, req| async move {
                connector_events_core(&state.pool, req, auth.merchant_account.merchant_id)
                    .await
                    .map(ApplicationResponse::Json)
            },
            &auth::JWTAuth(Permission::Analytics),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}
