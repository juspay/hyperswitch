use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
};

// fn print_type_of<T>(_: &T) {
//     println!("{}", std::any::type_name::<T>())
// }

#[actix_web::main]
async fn main() -> ApplicationResult<()> {
    // get commandline config before initializing config
    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[cfg(feature = "openapi")]
    {
        use router::configs::settings::Subcommand;
        if let Some(Subcommand::GenerateOpenapiSpec) = cmd_line.subcommand {
            let file_path = "openapi/openapi_spec.json";
            #[allow(clippy::expect_used)]
            std::fs::write(
                file_path,
                <router::openapi::ApiDoc as utoipa::OpenApi>::openapi()
                    .to_pretty_json()
                    .expect("Failed to serialize OpenAPI specification as JSON"),
            )
            .expect("Failed to write OpenAPI specification to file");
            println!("Successfully saved OpenAPI specification file at '{file_path}'");
            // let mut json_schema = std::fs::read_to_string(file_path).unwrap();
            // let spec: oas3::OpenApiV3Spec = serde_json::from_str(json_schema.as_str()).expect("OpenApiV3 Deserialized");
            return Ok(());
        }
    }

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate router configuration");

    let _guard = logger::setup(&conf.log);

    #[cfg(feature = "pii-encryption-script")]
    {
        let store =
            router::services::Store::new(&conf, false, tokio::sync::oneshot::channel().0).await;

        // ^-------- KMS decryption of the master key is a fallible and the server will panic in
        // the above mentioned line

        router::scripts::pii_encryption::test_2_step_encryption(&store).await;

        #[allow(clippy::expect_used)]
        router::scripts::pii_encryption::encrypt_merchant_account_fields(&store)
            .await
            .expect("Failed while encrypting merchant account");

        crate::logger::error!("Done with everything");
    }

    logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);

    #[allow(clippy::expect_used)]
    let server = router::start_server(conf)
        .await
        .expect("Failed to create the server");
    let _ = server.await;

    Err(ApplicationError::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Server shut down",
    )))
}
