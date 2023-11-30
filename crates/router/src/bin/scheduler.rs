#![recursion_limit = "256"]
use std::{str::FromStr, sync::Arc};

use common_utils::ext_traits::{OptionExt, StringExt};
use diesel_models::process_tracker as storage;
use error_stack::ResultExt;
use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{self, CustomResult},
    logger, routes, services,
    types::storage::ProcessTrackerExt,
    workflows,
};
use scheduler::{
    consumer::workflows::ProcessTrackerWorkflow, errors::ProcessTrackerError,
    workflows::ProcessTrackerWorkflows, SchedulerAppState,
};
use serde::{Deserialize, Serialize};
use strum::EnumString;
use tokio::sync::{mpsc, oneshot};

const SCHEDULER_FLOW: &str = "SCHEDULER_FLOW";
#[tokio::main]
async fn main() -> CustomResult<(), ProcessTrackerError> {
    // console_subscriber::init();

    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    let api_client = Box::new(
        services::ProxyClient::new(
            conf.proxy.clone(),
            services::proxy_bypass_urls(&conf.locker),
        )
        .change_context(errors::ProcessTrackerError::ConfigurationError)?,
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
    tokio::spawn(router::receiver_for_error(
        redis_shutdown_signal_rx,
        tx.clone(),
    ));

    #[allow(clippy::expect_used)]
    let scheduler_flow_str =
        std::env::var(SCHEDULER_FLOW).expect("SCHEDULER_FLOW environment variable not set");
    #[allow(clippy::expect_used)]
    let scheduler_flow = scheduler::SchedulerFlow::from_str(&scheduler_flow_str)
        .expect("Unable to parse SchedulerFlow from environment variable");

    let _guard = router_env::setup(
        &state.conf.log,
        &scheduler_flow_str,
        [router_env::service_name!()],
    );

    logger::debug!(startup_config=?state.conf);

    start_scheduler(&state, scheduler_flow, (tx, rx)).await?;

    eprintln!("Scheduler shut down");
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, EnumString)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum PTRunner {
    PaymentsSyncWorkflow,
    RefundWorkflowRouter,
    DeleteTokenizeDataWorkflow,
}

#[derive(Debug, Copy, Clone)]
pub struct WorkflowRunner;

#[async_trait::async_trait]
impl ProcessTrackerWorkflows<routes::AppState> for WorkflowRunner {
    async fn trigger_workflow<'a>(
        &'a self,
        state: &'a routes::AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), ProcessTrackerError> {
        let runner = process.runner.clone().get_required_value("runner")?;
        let runner: Option<PTRunner> = runner.parse_enum("PTRunner").ok();
        let operation: Box<dyn ProcessTrackerWorkflow<routes::AppState>> = match runner {
            Some(PTRunner::PaymentsSyncWorkflow) => {
                Box::new(workflows::payment_sync::PaymentsSyncWorkflow)
            }
            Some(PTRunner::RefundWorkflowRouter) => {
                Box::new(workflows::refund_router::RefundWorkflowRouter)
            }
            Some(PTRunner::DeleteTokenizeDataWorkflow) => {
                Box::new(workflows::tokenized_data::DeleteTokenizeDataWorkflow)
            }
            _ => Err(ProcessTrackerError::UnexpectedFlow)?,
        };
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
                    let status = process
                        .finish_with_status(
                            state.get_db().as_scheduler(),
                            "GLOBAL_FAILURE".to_string(),
                        )
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
        .ok_or(errors::ProcessTrackerError::ConfigurationError)?;
    scheduler::start_process_tracker(
        state,
        scheduler_flow,
        Arc::new(scheduler_settings),
        channel,
        WorkflowRunner {},
    )
    .await
}

#[cfg(test)]
mod workflow_tests {
    #![allow(clippy::unwrap_used)]
    use common_utils::ext_traits::StringExt;

    use super::PTRunner;

    #[test]
    fn test_enum_to_string() {
        let string_format = "PAYMENTS_SYNC_WORKFLOW".to_string();
        let enum_format: PTRunner = string_format.parse_enum("PTRunner").unwrap();
        assert_eq!(enum_format, PTRunner::PaymentsSyncWorkflow)
    }
}
