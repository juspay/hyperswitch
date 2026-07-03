use error_stack::ResultExt;
use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
    routes::metrics,
};

fn main() -> ApplicationResult<()> {
    // In debug builds the deeply-nested async payment flows (notably the
    // auto-retry path, which re-enters the entire payment execution) can exceed
    // the default worker-thread stack and abort the process with a stack
    // overflow. Raise the stack size for every worker thread *before* the Tokio
    // runtime is created, because `std` caches `RUST_MIN_STACK` on the first
    // thread spawn — setting it after the runtime is built has no effect on the
    // already-spawned Tokio/actix worker threads.
    //
    // Release builds are unaffected: this block is compiled out and, thanks to
    // optimization, they never reach this stack depth. An explicit
    // `RUST_MIN_STACK` from the environment is always respected.
    #[cfg(debug_assertions)]
    if std::env::var_os("RUST_MIN_STACK").is_none() {
        std::env::set_var("RUST_MIN_STACK", "67108864"); // 64 MiB
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .change_context(ApplicationError::ConfigurationError)?
        .block_on(run())
}

async fn run() -> ApplicationResult<()> {
    // get commandline config before initializing config
    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate router configuration");

    #[allow(clippy::print_stdout)] // The logger has not yet been initialized
    #[cfg(feature = "vergen")]
    {
        println!("Starting router (Version: {})", router_env::git_tag!());
    }

    let _guard = router_env::setup(
        &conf.log,
        router_env::service_name!(),
        [
            router_env::service_name!(),
            "actix_server",
            "open_feature",
            "superposition_provider",
            "superposition_sdk",
        ],
    )
    .change_context(ApplicationError::ConfigurationError)?;

    logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);

    // Spawn a thread for collecting metrics at fixed intervals
    metrics::bg_metrics_collector::spawn_metrics_collector(
        conf.log.telemetry.bg_metrics_collection_interval_in_secs,
    );

    #[allow(clippy::expect_used)]
    let server = Box::pin(router::start_server(conf, router_env::service_name!()))
        .await
        .expect("Failed to create the server");
    let _ = server.await;

    Err(error_stack::Report::from(ApplicationError::from(
        std::io::Error::other("Server shut down"),
    )))
}
