use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
};

#[actix_web::main]
async fn main() -> ApplicationResult<()> {
    // get commandline config before initializing config
    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[cfg(feature = "openapi")]
    {
        use router::configs::settings::Subcommand;
        if let Some(Subcommand::GenerateOpenapiSpec) = cmd_line.subcommand {
            let file_path = "openapi/generated.json";
            #[allow(clippy::expect_used)]
            std::fs::write(
                file_path,
                <router::openapi::ApiDoc as utoipa::OpenApi>::openapi()
                    .to_pretty_json()
                    .expect("Failed to generate serialize OpenAPI specification as JSON"),
            )
            .expect("Failed to write OpenAPI specification to file");
            println!("Successfully saved OpenAPI specification file at '{file_path}'");
            return Ok(());
        }
    }

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate router configuration");

    let _guard = logger::setup(&conf.log)?;

    logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);

    #[allow(clippy::expect_used)]
    let (server, mut state) = router::start_server(conf)
        .await
        .expect("Failed to create the server");

    let _ = server.await;

    state.store.close().await;

    Err(ApplicationError::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Server shut down",
    )))
}
