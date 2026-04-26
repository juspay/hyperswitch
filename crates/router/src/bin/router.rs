use std::sync::atomic::{AtomicUsize, Ordering};

use error_stack::ResultExt;
use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
    routes::metrics,
};

// Counters for tracking runtime activities
static THREAD_START_COUNTER: AtomicUsize = AtomicUsize::new(0);
static THREAD_STOP_COUNTER: AtomicUsize = AtomicUsize::new(0);
static THREAD_PARK_COUNTER: AtomicUsize = AtomicUsize::new(0);
static THREAD_UNPARK_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TASK_SPAWN_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TASK_POLL_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TASK_TERMINATE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn main() -> ApplicationResult<()> {
    let body = async {
        let cmd_line = <CmdLineConf as clap::Parser>::parse();
        #[allow(clippy::expect_used)]
        let conf = Settings::with_config_path(cmd_line.config_path)
            .expect("Unable to construct application configuration");
        #[allow(clippy::expect_used)]
        conf.validate()
            .expect("Failed to validate router configuration");
        #[allow(clippy::print_stdout)]
        #[cfg(feature = "vergen")]
        {
            println!("Starting router (Version: {})", router_env::git_tag!());
        }
        let _guard = router_env::setup(
            &conf.log,
            "UNRESOLVED_ENV_VAR",
            ["UNRESOLVED_ENV_VAR", "actix_server"],
        )
        .change_context(ApplicationError::ConfigurationError)?;
        logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);
        metrics::bg_metrics_collector::spawn_metrics_collector(
            conf.log.telemetry.bg_metrics_collection_interval_in_secs,
        );
        #[allow(clippy::expect_used)]
        let server = Box::pin(router::start_server(conf))
            .await
            .expect("Failed to create the server");
        let _ = server.await;
        Err(error_stack::Report::from(ApplicationError::from(
            std::io::Error::other("Server shut down"),
        )))
    };
    #[cfg(all())]
    #[allow(
        clippy::expect_used,
        clippy::diverging_sub_expression,
        clippy::needless_return,
        clippy::unwrap_in_result
    )]
    {
        return tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            // Thread lifecycle callbacks
            .on_thread_start(|| {
                let thread_id = THREAD_START_COUNTER.fetch_add(1, Ordering::SeqCst);
                logger::debug!("[tokio-runtime] [thread-{}] Thread started", thread_id);
            })
            .on_thread_stop(|| {
                let thread_id = THREAD_STOP_COUNTER.fetch_add(1, Ordering::SeqCst);
                logger::debug!("[tokio-runtime] [thread-{}] Thread stopping", thread_id);
            })
            .on_thread_park(|| {
                let park_id = THREAD_PARK_COUNTER.fetch_add(1, Ordering::SeqCst);
                logger::debug!("[tokio-runtime] [park-{}] Thread parking", park_id);
            })
            .on_thread_unpark(|| {
                let unpark_id = THREAD_UNPARK_COUNTER.fetch_add(1, Ordering::SeqCst);
                logger::debug!("[tokio-runtime] [unpark-{}] Thread unparking", unpark_id);
            })
            // Task lifecycle callbacks (requires tokio_unstable)
            .on_task_spawn(|meta| {
                let task_id = TASK_SPAWN_COUNTER.fetch_add(1, Ordering::SeqCst);
                logger::debug!(
                    "[tokio-runtime] [task-{}] Task spawned; id={:?}; location={:?}",
                    task_id,
                    meta.id(),
                    meta.spawned_at()
                );
            })
            .on_before_task_poll(|meta| {
                let poll_id = TASK_POLL_COUNTER.fetch_add(1, Ordering::SeqCst);
                logger::debug!(
                    "[tokio-runtime] [poll-{}] Task polling; id={:?}; location={:?}",
                    poll_id,
                    meta.id(),
                    meta.spawned_at()
                );
            })
            .on_after_task_poll(|meta| {
                let poll_done_id = TASK_POLL_COUNTER.fetch_add(1, Ordering::SeqCst);
                logger::debug!(
                    "[tokio-runtime] [poll-done-{}] Task polled; id={:?}; location={:?}",
                    poll_done_id,
                    meta.id(),
                    meta.spawned_at()
                );
            })
            .on_task_terminate(|meta| {
                let term_id = TASK_TERMINATE_COUNTER.fetch_add(1, Ordering::SeqCst);
                logger::debug!(
                    "[tokio-runtime] [term-{}] Task terminated; id={:?}; location={:?}",
                    term_id,
                    meta.id(),
                    meta.spawned_at()
                );
            })
            .build()
            .expect("Failed building the Runtime")
            .block_on(body);
    }
    #[cfg(not(all()))]
    {
        panic!("fell through checks")
    }
}
