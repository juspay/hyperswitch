use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
};
#[cfg(feature = "migrate_data_from_legacy_to_basilisk_hs")]
use router::{
    core::payment_methods::cards::migrate_data_from_legacy_to_basilisk_hs, routes, services::Store,
};
#[cfg(feature = "migrate_data_from_legacy_to_basilisk_hs")]
use tokio::sync::oneshot;

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

    #[cfg(feature = "migrate_data_from_legacy_to_basilisk_hs")]
    #[allow(clippy::expect_used)]
    {
        let (tx, _rx) = oneshot::channel();
        let state = routes::AppState::new(conf.clone(), tx).await;
        
        // Creating a replication database connection, so that we can do bulk read from replica database without affecting the primary database
        let (tx, _rx) = oneshot::channel();
        let store = Store::new(&conf, true, tx).await;
        let conn = &store
            .replica_pool
            .get()
            .await
            .expect("PG connection not established");

        // Read all the payment methods from the database (~1000 records)
        // decided to remove reading all merchant accounts from the database which is more than 3000 records.
        let payment_methods =
            storage_models::payment_method::PaymentMethod::find_all_payment_methods(
                conn
            )
            .await
            .expect("Failed to fetch payment methods from db");

        // Iterate over all the payment methods and migrate the data from legacy to basilisk
        for payment_method in payment_methods.iter() {
            let card_reference = payment_method
                .token
                .clone()
                .expect("card reference not present");
            let _ = migrate_data_from_legacy_to_basilisk_hs(
                &state,
                &payment_method.customer_id,
                payment_method.merchant_id.as_str(),
                card_reference.as_str(),
                "m0010", // locker id is same for all the merchant accounts who has saved cards. 
            )
            .await;
        }
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
