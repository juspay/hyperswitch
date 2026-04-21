use error_stack::ResultExt;
use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
    routes::metrics,
};

fn main() -> ApplicationResult<()> {
    let multi_threaded_rt = || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Multithreaded Tokio runtime could not be created.")
    };
    <::actix_web::rt::System>::with_tokio_rt(multi_threaded_rt).block_on(async move {
        {
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
        }
    })
}
