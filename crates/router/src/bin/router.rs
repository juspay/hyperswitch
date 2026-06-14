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

    let _guard = router_env::setup(
        &conf.log,
        router_env::service_name!(),
        [router_env::service_name!(), "actix_server"],
    )
    .change_context(ApplicationError::ConfigurationError)?;

    logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);

    #[cfg(feature = "deja")]
    {
        // Compose the Kafka recording sink (DEJA_MODE=record) BEFORE the
        // getter below, which otherwise locks in an env-derived hook via its
        // OnceLock. No-op for the replay/none paths.
        router::deja_boot::install(&conf.events).await;
        if let Some(hook) = deja::global_runtime_hook_from_env() {
            router_env::tracing::info!(
                target: "deja",
                mode = hook.variant_name(),
                recording_run_id = ?std::env::var("DEJA_RECORDING_RUN_ID")
                    .ok()
                    .or_else(|| std::env::var("DEJA_RUN_ID").ok()),
                "deja runtime hook initialized"
            );
        } else {
            router_env::tracing::debug!(
                target: "deja",
                "deja disabled (no DEJA_MODE)"
            );
        }
    }

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
