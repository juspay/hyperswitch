use drainer::{
    errors::DrainerResult, logger::logger, services, settings, start_drainer, start_web_server,
};
use router_env::tracing::Instrument;

#[tokio::main]
async fn main() -> DrainerResult<()> {
    // Get configuration
    let cmd_line = <settings::CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = settings::Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate drainer configuration");

    let state = settings::AppState::new(conf.clone()).await;

    let store = std::sync::Arc::new(services::Store::new(&state.conf, false).await);

    #[cfg(feature = "vergen")]
    println!("Starting drainer (Version: {})", router_env::git_tag!());

    let _guard = router_env::setup(
        &conf.log,
        router_env::service_name!(),
        [router_env::service_name!()],
    );

    #[allow(clippy::expect_used)]
    let web_server = Box::pin(start_web_server(state.conf.as_ref().clone(), store.clone()))
        .await
        .expect("Failed to create the server");

    tokio::spawn(
        async move {
            let _ = web_server.await;
            logger::error!("The health check probe stopped working!");
        }
        .in_current_span(),
    );

    logger::debug!(startup_config=?conf);
    logger::info!("Drainer started [{:?}] [{:?}]", conf.drainer, conf.log);

    start_drainer(store.clone(), conf.drainer).await?;

    Ok(())
}
