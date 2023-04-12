#![recursion_limit = "256"]
use std::sync::Arc;

use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{self, CustomResult},
    logger, routes, scheduler,
};

const SCHEDULER_FLOW: &str = "SCHEDULER_FLOW";

#[tokio::main]
async fn main() -> CustomResult<(), errors::ProcessTrackerError> {
    // console_subscriber::init();

    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");

    let mut state = routes::AppState::new(conf).await;
    let _guard =
        logger::setup(&state.conf.log).map_err(|_| errors::ProcessTrackerError::UnexpectedFlow)?;

    logger::debug!(startup_config=?state.conf);

    start_scheduler(&state).await?;

    state.store.close().await;

    eprintln!("Scheduler shut down");
    Ok(())
}

async fn start_scheduler(
    state: &routes::AppState,
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
    scheduler::start_process_tracker(state, flow, Arc::new(scheduler_settings)).await
}
