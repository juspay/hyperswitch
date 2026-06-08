use error_stack::ResultExt;
use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
    routes::metrics,
};

// 256 MiB per worker stack. Default 2 MiB overflows on deeply nested generic
// dispatch paths for UCS-only connectors (tsys_transit /payments).
// Actix worker threads pick up `RUST_MIN_STACK` from the environment — see
// scripts/start-router.sh for the launch wrapper that exports it.
const WORKER_STACK_SIZE: usize = 256 * 1024 * 1024;

fn main() -> ApplicationResult<()> {
    #[allow(clippy::expect_used)]
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(WORKER_STACK_SIZE)
        .build()
        .expect("Failed to build tokio runtime");

    runtime.block_on(async_main())
}

async fn async_main() -> ApplicationResult<()> {
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
