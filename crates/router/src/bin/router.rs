use error_stack::ResultExt;
use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
    routes::metrics,
};

#[cfg(feature = "profiling")]
use pyroscope::PyroscopeAgent;

#[cfg(feature = "profiling")]
fn setup_profiling(config: &router::configs::settings::ProfilingConfig) {
    if !config.enabled {
        logger::info!("Profiling is disabled");
        return;
    }

    logger::info!("Initializing Pyroscope profiler...");

    let profiling_agent = PyroscopeAgent::builder(
        &config.server_address,
        &config.application_name,
    ).build().expect("Failed to build Pyroscope agent");

    profiling_agent.start().expect("Failed to start Pyroscope agent");
    logger::info!("Pyroscope profiler initialized successfully");
}

#[tokio::main]
async fn main() -> ApplicationResult<()> {
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
        [router_env::service_name!(), "actix_server"],
    )
    .change_context(ApplicationError::ConfigurationError)?;

    logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);

    #[cfg(feature = "profiling")]
    setup_profiling(&conf.profiling);

    // Spawn a thread for collecting metrics at fixed intervals
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
}
