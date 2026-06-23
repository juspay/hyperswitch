use error_stack::ResultExt;
use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
    routes::metrics,
};

fn main() -> ApplicationResult<()> {
    // RUST_MIN_STACK applies to every thread spawned without an explicit stack
    // size — including actix-web worker threads (spawned by actix-rt).  The
    // default 2 MB overflows when the payments-retry path fires while outbound
    // connector calls are routed through an HTTP CONNECT proxy (MITM recording
    // mode).  Set this before the tokio runtime or actix workers are created.
    // Safe to set unconditionally: it only raises the floor, never lowers it.
    if std::env::var("RUST_MIN_STACK").is_err() {
        // 4 MB — only override if the caller has not already set a value.
        std::env::set_var("RUST_MIN_STACK", "4194304");
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(4 * 1024 * 1024)
        .build()
        .expect("Failed to build tokio runtime")
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
