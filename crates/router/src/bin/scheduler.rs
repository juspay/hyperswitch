#![recursion_limit = "256"]
use std::{str::FromStr, sync::Arc};

use actix_web::{dev::Server, web, Scope};
use api_models::health_check::SchedulerHealthCheckResponse;
use common_utils::ext_traits::{OptionExt, StringExt};
use diesel_models::process_tracker as storage;
use error_stack::ResultExt;
use router::{
    configs::settings::{CmdLineConf, Settings},
    core::{
        errors::{self, CustomResult},
        health_check::HealthCheckInterface,
    },
    logger, routes,
    services::{self, api},
    workflows,
};
use router_env::{
    instrument,
    tracing::{self, Instrument},
};
use scheduler::{
    consumer::workflows::ProcessTrackerWorkflow, errors::ProcessTrackerError,
    workflows::ProcessTrackerWorkflows, SchedulerAppState,
};
use storage_impl::errors::ApplicationError;
use tokio::sync::{mpsc, oneshot};

const SCHEDULER_FLOW: &str = "SCHEDULER_FLOW";
#[tokio::main]
async fn main() -> CustomResult<(), ProcessTrackerError> {
    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    let api_client = Box::new(
        services::ProxyClient::new(
            conf.proxy.clone(),
            services::proxy_bypass_urls(&conf.locker),
        )
        .change_context(ProcessTrackerError::ConfigurationError)?,
    );
    // channel for listening to redis disconnect events
    let (redis_shutdown_signal_tx, redis_shutdown_signal_rx) = oneshot::channel();
    let state = Box::pin(routes::AppState::new(
        conf,
        redis_shutdown_signal_tx,
        api_client,
    ))
    .await;
    // channel to shutdown scheduler gracefully
    let (tx, rx) = mpsc::channel(1);
    let _task_handle = tokio::spawn(
        router::receiver_for_error(redis_shutdown_signal_rx, tx.clone()).in_current_span(),
    );

    #[allow(clippy::expect_used)]
    let scheduler_flow_str =
        std::env::var(SCHEDULER_FLOW).expect("SCHEDULER_FLOW environment variable not set");
    #[allow(clippy::expect_used)]
    let scheduler_flow = scheduler::SchedulerFlow::from_str(&scheduler_flow_str)
        .expect("Unable to parse SchedulerFlow from environment variable");

    #[cfg(feature = "vergen")]
    println!(
        "Starting {scheduler_flow} (Version: {})",
        router_env::git_tag!()
    );

    let _guard = router_env::setup(
        &state.conf.log,
        &scheduler_flow_str,
        [router_env::service_name!()],
    );

    #[allow(clippy::expect_used)]
    let web_server = Box::pin(start_web_server(
        state.clone(),
        scheduler_flow_str.to_string(),
    ))
    .await
    .expect("Failed to create the server");

    let _task_handle = tokio::spawn(
        async move {
            let _ = web_server.await;
            logger::error!("The health check probe stopped working!");
        }
        .in_current_span(),
    );

    logger::debug!(startup_config=?state.conf);

    start_scheduler(&state, scheduler_flow, (tx, rx)).await?;

    eprintln!("Scheduler shut down");
    Ok(())
}

pub async fn start_web_server(
    state: routes::AppState,
    service: String,
) -> errors::ApplicationResult<Server> {
    let server = state
        .conf
        .scheduler
        .as_ref()
        .ok_or(ApplicationError::InvalidConfigurationValueError(
            "Scheduler server is invalidly configured".into(),
        ))?
        .server
        .clone();

    let web_server = actix_web::HttpServer::new(move || {
        actix_web::App::new().service(Health::server(state.clone(), service.clone()))
    })
    .bind((server.host.as_str(), server.port))?
    .workers(server.workers)
    .run();
    let _ = web_server.handle();

    Ok(web_server)
}

pub struct Health;

impl Health {
    pub fn server(state: routes::AppState, service: String) -> Scope {
        web::scope("health")
            .app_data(web::Data::new(state))
            .app_data(web::Data::new(service))
            .service(web::resource("").route(web::get().to(health)))
            .service(web::resource("/ready").route(web::get().to(deep_health_check)))
    }
}

#[instrument(skip_all)]
pub async fn health() -> impl actix_web::Responder {
    logger::info!("Scheduler health was called");
    actix_web::HttpResponse::Ok().body("Scheduler health is good")
}
#[instrument(skip_all)]
pub async fn deep_health_check(
    state: web::Data<routes::AppState>,
    service: web::Data<String>,
) -> impl actix_web::Responder {
    let report = deep_health_check_func(state, service).await;
    match report {
        Ok(response) => services::http_response_json(
            serde_json::to_string(&response)
                .map_err(|err| {
                    logger::error!(serialization_error=?err);
                })
                .unwrap_or_default(),
        ),
        Err(err) => api::log_and_return_error_response(err),
    }
}
#[instrument(skip_all)]
pub async fn deep_health_check_func(
    state: web::Data<routes::AppState>,
    service: web::Data<String>,
) -> errors::RouterResult<SchedulerHealthCheckResponse> {
    logger::info!("{} deep health check was called", service.into_inner());

    logger::debug!("Database health check begin");

    let db_status = state.health_check_db().await.map(|_| true).map_err(|err| {
        error_stack::report!(errors::ApiErrorResponse::HealthCheckError {
            component: "Database",
            message: err.to_string()
        })
    })?;

    logger::debug!("Database health check end");

    logger::debug!("Redis health check begin");

    let redis_status = state
        .health_check_redis()
        .await
        .map(|_| true)
        .map_err(|err| {
            error_stack::report!(errors::ApiErrorResponse::HealthCheckError {
                component: "Redis",
                message: err.to_string()
            })
        })?;

    let outgoing_req_check = state
        .health_check_outgoing()
        .await
        .map(|_| true)
        .map_err(|err| {
            error_stack::report!(errors::ApiErrorResponse::HealthCheckError {
                component: "Outgoing Request",
                message: err.to_string()
            })
        })?;

    logger::debug!("Redis health check end");

    let response = SchedulerHealthCheckResponse {
        database: db_status,
        redis: redis_status,
        outgoing_request: outgoing_req_check,
    };

    Ok(response)
}

#[derive(Debug, Copy, Clone)]
pub struct WorkflowRunner;

#[async_trait::async_trait]
impl ProcessTrackerWorkflows<routes::AppState> for WorkflowRunner {
    async fn trigger_workflow<'a>(
        &'a self,
        state: &'a routes::AppState,
        process: storage::ProcessTracker,
    ) -> CustomResult<(), ProcessTrackerError> {
        let runner = process
            .runner
            .clone()
            .get_required_value("runner")
            .change_context(ProcessTrackerError::MissingRequiredField)
            .attach_printable("Missing runner field in process information")?;
        let runner: storage::ProcessTrackerRunner = runner
            .parse_enum("ProcessTrackerRunner")
            .change_context(ProcessTrackerError::UnexpectedFlow)
            .attach_printable("Failed to parse workflow runner name")?;

        let get_operation = |runner: storage::ProcessTrackerRunner| -> CustomResult<
            Box<dyn ProcessTrackerWorkflow<routes::AppState>>,
            ProcessTrackerError,
        > {
            match runner {
                storage::ProcessTrackerRunner::PaymentsSyncWorkflow => {
                    Ok(Box::new(workflows::payment_sync::PaymentsSyncWorkflow))
                }
                storage::ProcessTrackerRunner::RefundWorkflowRouter => {
                    Ok(Box::new(workflows::refund_router::RefundWorkflowRouter))
                }
                storage::ProcessTrackerRunner::DeleteTokenizeDataWorkflow => Ok(Box::new(
                    workflows::tokenized_data::DeleteTokenizeDataWorkflow,
                )),
                storage::ProcessTrackerRunner::ApiKeyExpiryWorkflow => {
                    #[cfg(feature = "email")]
                    {
                        Ok(Box::new(workflows::api_key_expiry::ApiKeyExpiryWorkflow))
                    }

                    #[cfg(not(feature = "email"))]
                    {
                        Err(error_stack::report!(ProcessTrackerError::UnexpectedFlow))
                            .attach_printable(
                                "Cannot run API key expiry workflow when email feature is disabled",
                            )
                    }
                }
                storage::ProcessTrackerRunner::OutgoingWebhookRetryWorkflow => Ok(Box::new(
                    workflows::outgoing_webhook_retry::OutgoingWebhookRetryWorkflow,
                )),
                storage::ProcessTrackerRunner::AttachPayoutAccountWorkflow => {
                    #[cfg(feature = "payouts")]
                    {
                        Ok(Box::new(
                            workflows::attach_payout_account_workflow::AttachPayoutAccountWorkflow,
                        ))
                    }
                    #[cfg(not(feature = "payouts"))]
                    {
                        Err(
                            error_stack::report!(ProcessTrackerError::UnexpectedFlow),
                        )
                        .attach_printable(
                            "Cannot run Stripe external account workflow when payouts feature is disabled",
                        )
                    }
                }
            }
        };

        let operation = get_operation(runner)?;

        let app_state = &state.clone();
        let output = operation.execute_workflow(app_state, process.clone()).await;
        match output {
            Ok(_) => operation.success_handler(app_state, process).await,
            Err(error) => match operation
                .error_handler(app_state, process.clone(), error)
                .await
            {
                Ok(_) => (),
                Err(error) => {
                    logger::error!(%error, "Failed while handling error");
                    let status = state
                        .get_db()
                        .as_scheduler()
                        .finish_process_with_business_status(process, "GLOBAL_FAILURE".to_string())
                        .await;
                    if let Err(err) = status {
                        logger::error!(%err, "Failed while performing database operation: GLOBAL_FAILURE");
                    }
                }
            },
        };
        Ok(())
    }
}

async fn start_scheduler(
    state: &routes::AppState,
    scheduler_flow: scheduler::SchedulerFlow,
    channel: (mpsc::Sender<()>, mpsc::Receiver<()>),
) -> CustomResult<(), ProcessTrackerError> {
    let scheduler_settings = state
        .conf
        .scheduler
        .clone()
        .ok_or(ProcessTrackerError::ConfigurationError)?;
    scheduler::start_process_tracker(
        state,
        scheduler_flow,
        Arc::new(scheduler_settings),
        channel,
        WorkflowRunner {},
    )
    .await
}
