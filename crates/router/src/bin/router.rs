use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
};

#[tokio::main]
/// Asynchronous function that represents the main entry point of the application. It parses the command line configuration, initializes the application configuration, validates the router configuration, starts the router, sets up the environment, and logs the application start. It also handles errors related to creating and shutting down the server.
async fn main() -> ApplicationResult<()> {
    // get commandline config before initializing config
    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate router configuration");

    #[cfg(feature = "vergen")]
    println!("Starting router (Version: {})", router_env::git_tag!());

    let _guard = router_env::setup(
        &conf.log,
        router_env::service_name!(),
        [router_env::service_name!(), "actix_server"],
    );

    logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);

    #[allow(clippy::expect_used)]
    let server = Box::pin(router::start_server(conf))
        .await
        .expect("Failed to create the server");
    let _ = server.await;

    Err(ApplicationError::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Server shut down",
    )))
}
