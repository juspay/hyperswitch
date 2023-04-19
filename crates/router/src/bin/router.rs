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

    #[cfg(feature = "pii-encryption-script")]
    {
        use router::{configs::settings::Subcommand, db::StorageInterface};
        if let Some(Subcommand::EncryptDatabase) = cmd_line.subcommand {
            let store = router::services::Store::new(&conf, false).await;

            router::scripts::pii_encryption::crate_merchant_key_store(&store)
                .await
                .expect("Failed while generating merchant key store");

            router::scripts::pii_encryption::encrypt_merchant_account_fields(&store)
                .await
                .expect("Failed while encrypting merchant account");

            router::scripts::pii_encryption::encrypt_merchant_connector_account_fields(&store)
                .await
                .expect("Failed while encrypting merchant connector account");

            router::scripts::pii_encryption::encrypt_customer_fields(&store)
                .await
                .expect("Failed while encrypting customer");

            router::scripts::pii_encryption::encrypt_address_fields(&store)
                .await
                .expect("Failed while encrypting address");

            store.close().await;
            return Ok(());
        }
    }

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
