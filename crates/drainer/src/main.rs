use drainer::{errors::DrainerResult, logger::logger, services, settings, start_drainer};

#[tokio::main]
/// Asynchronous function that initializes the drainer application. It first retrieves the configuration from the command line and constructs the application configuration. Then, it validates the drainer configuration and creates a new instance of the Store service. If the "vergen" feature is enabled, it prints the drainer version. It sets up the environment and logs the startup configuration. Finally, it starts the drainer and returns a Result indicating success or failure.
async fn main() -> DrainerResult<()> {
    // Get configuration
    let cmd_line = <settings::CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = settings::Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate drainer configuration");

    let store = services::Store::new(&conf, false).await;
    let store = std::sync::Arc::new(store);

    #[cfg(feature = "vergen")]
    println!("Starting drainer (Version: {})", router_env::git_tag!());

    let _guard = router_env::setup(
        &conf.log,
        router_env::service_name!(),
        [router_env::service_name!()],
    );

    logger::debug!(startup_config=?conf);
    logger::info!("Drainer started [{:?}] [{:?}]", conf.drainer, conf.log);

    start_drainer(store.clone(), conf.drainer).await?;

    Ok(())
}
