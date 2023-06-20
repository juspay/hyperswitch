#![recursion_limit = "256"]
use std::sync::Arc;

use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{CustomResult},
    logger, routes, workflows,
};
use scheduler::{
    errors as sch_errors,
    consumer::workflows::ProcessTrackerWorkflow
};
use tokio::sync::{mpsc, oneshot};

const SCHEDULER_FLOW: &str = "SCHEDULER_FLOW";

#[tokio::main]
async fn main() -> CustomResult<(), errors::ProcessTrackerError> {
    // console_subscriber::init();

    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    // channel for listening to redis disconnect events
    let (redis_shutdown_signal_tx, redis_shutdown_signal_rx) = oneshot::channel();
    let state = routes::AppState::new(conf, redis_shutdown_signal_tx).await;
    // channel to shutdown scheduler gracefully
    let (tx, rx) = mpsc::channel(1);
    tokio::spawn(router::receiver_for_error(
        redis_shutdown_signal_rx,
        tx.clone(),
    ));
    let _guard = logger::setup(&state.conf.log);

    logger::debug!(startup_config=?state.conf);

    start_scheduler(&state, (tx, rx)).await?;

    eprintln!("Scheduler shut down");
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, EnumString)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum PTRunner {
    PaymentsSyncWorkflow,
}

pub fn runner_from_task(
    task: &storage::ProcessTracker,
) -> Result<Option<Box<dyn ProcessTrackerWorkflow<routes::AppState>>>, sch_errors::ProcessTrackerError> {
    let runner = task.runner.clone().get_required_value("runner")?;
    let runner: Option<PTRunner> = runner.parse_enum("PTRunner").ok();
    Ok(match runner {
        Some(PTRunner::PaymentsSyncWorkflow) => Some(Box::new(workflows::payment_sync::PaymentsSyncWorkflow)),
        None => None,
    })
}

async fn start_scheduler(
    state: &routes::AppState,
    channel: (mpsc::Sender<()>, mpsc::Receiver<()>),
) -> CustomResult<(), errors::ProcessTrackerError> {
    use std::str::FromStr;

    #[allow(clippy::expect_used)]
    let flow = std::env::var(SCHEDULER_FLOW).expect("SCHEDULER_FLOW environment variable not set");
    #[allow(clippy::expect_used)]
    let flow = scheduler::SchedulerFlow::from_str(&flow)
        .expect("Unable to parse SchedulerFlow from environment variable");

    let scheduler_settings = state
        .conf
        .scheduler
        .clone()
        .ok_or(errors::ProcessTrackerError::ConfigurationError)?;
    scheduler::start_process_tracker(state, flow, Arc::new(scheduler_settings), channel, runner_from_task).await
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