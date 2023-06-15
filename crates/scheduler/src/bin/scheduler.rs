#![recursion_limit = "256"]
use std::sync::Arc;

use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{self, CustomResult},
    logger, routes,
};
use scheduler::flow::SchedulerFlow;
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

pub trait ForeignFrom<F> {
    fn foreign_from(from: F) -> Self;
}

impl ForeignFrom<router::configs::settings::SchedulerSettings>
    for scheduler::settings::SchedulerSettings
{
    fn foreign_from(value: router::configs::settings::SchedulerSettings) -> Self {
        Self {
            stream: value.stream,
            producer: scheduler::settings::ProducerSettings {
                upper_fetch_limit: value.producer.upper_fetch_limit,
                lower_fetch_limit: value.producer.lower_fetch_limit,
                lock_key: value.producer.lock_key,
                lock_ttl: value.producer.lock_ttl,
                batch_size: value.producer.batch_size,
            },
            consumer: scheduler::settings::ConsumerSettings {
                disabled: value.consumer.disabled,
                consumer_group: value.consumer.consumer_group,
            },
            loop_interval: value.loop_interval,
            graceful_shutdown_interval: value.graceful_shutdown_interval,
        }
    }
}

async fn start_scheduler(
    state: &routes::AppState,
    channel: (mpsc::Sender<()>, mpsc::Receiver<()>),
) -> CustomResult<(), errors::ProcessTrackerError> {
    use std::str::FromStr;

    #[allow(clippy::expect_used)]
    let flow = std::env::var(SCHEDULER_FLOW).expect("SCHEDULER_FLOW environment variable not set");
    #[allow(clippy::expect_used)]
    let flow = SchedulerFlow::from_str(&flow)
        .expect("Unable to parse SchedulerFlow from environment variable");

    let scheduler_settings = state
        .conf
        .scheduler
        .clone()
        .ok_or(errors::ProcessTrackerError::ConfigurationError)?;
    scheduler::scheduler::start_process_tracker(
        state,
        flow,
        Arc::new(scheduler::settings::SchedulerSettings::foreign_from(
            scheduler_settings,
        )),
        channel,
    )
    .await
}
