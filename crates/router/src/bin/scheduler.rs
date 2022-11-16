#![recursion_limit = "256"]
use std::sync::Arc;

use router::{
    configs::settings::{CmdLineConf, Settings},
    connection,
    core::errors::{self, CustomResult},
    db::SqlDb,
    logger, routes, scheduler,
    services::Store,
};
use structopt::StructOpt;

const SCHEDULER_FLOW: &str = "SCHEDULER_FLOW";

#[tokio::main]
async fn main() -> CustomResult<(), errors::ProcessTrackerError> {
    // console_subscriber::init();

    let cmd_line = CmdLineConf::from_args();
    let conf = Settings::with_config_path(cmd_line.config_path).unwrap();
    let mut state = routes::AppState {
        flow_name: String::from("default"),
        store: Store {
            pg_pool: SqlDb::new(&conf.database).await,
            redis_conn: Arc::new(connection::redis_connection(&conf).await),
        },
        conf,
    };
    let _guard =
        logger::setup(&state.conf.log).map_err(|_| errors::ProcessTrackerError::UnexpectedFlow)?;

    logger::debug!(startup_config=?state.conf);

    start_scheduler(&state).await?;

    std::sync::Arc::get_mut(&mut state.store.redis_conn)
        .expect("Redis connection pool cannot be closed")
        .close_connections()
        .await;

    eprintln!("Scheduler shut down");
    Ok(())
}

async fn start_scheduler(
    state: &routes::AppState,
) -> CustomResult<(), errors::ProcessTrackerError> {
    use std::str::FromStr;

    let options = scheduler::SchedulerOptions {
        looper_interval: scheduler::Milliseconds(5_000),
        db_name: "".to_string(),
        cache_name: "".to_string(),
        schema_name: "".to_string(),
        cache_expiry: scheduler::Milliseconds(30_000_000),
        runners: vec![],
        fetch_limit: 30,
        fetch_limit_product_factor: 1,
        query_order: "".to_string(),
        readiness: scheduler::options::ReadinessOptions {
            is_ready: true,
            graceful_termination_duration: scheduler::Milliseconds(60_000),
        },
    };

    let flow = std::env::var(SCHEDULER_FLOW).expect("SCHEDULER_FLOW environment variable not set");
    let flow = scheduler::SchedulerFlow::from_str(&flow).unwrap();
    let scheduler_settings = state
        .conf
        .scheduler
        .clone()
        .ok_or(errors::ProcessTrackerError::ConfigurationError)?;
    scheduler::start_process_tracker(state, Arc::new(options), flow, scheduler_settings).await
}
