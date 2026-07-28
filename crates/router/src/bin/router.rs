use error_stack::ResultExt;
use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
    routes::metrics,
};

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

    #[cfg(feature = "deja")]
    let deja_install_report = {
        // An empty deja broker list inherits the analytics Kafka brokers
        // (shared cluster provisioning, separate producer client).
        let analytics_brokers = match &conf.events.source {
            router::events::EventsSource::Kafka { kafka } => Some(kafka.brokers()),
            _ => None,
        };
        router::deja_boot::install(&conf.deja, analytics_brokers).map_err(|message| {
            error_stack::report!(ApplicationError::ConfigurationError).attach_printable(message)
        })?
    };

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

    #[cfg(feature = "deja")]
    router_env::tracing::info!(
        target: "deja",
        mode = deja_install_report.mode,
        run_id = ?deja_install_report.run_id,
        detail = ?deja_install_report.detail,
        "deja runtime hook initialized"
    );

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
