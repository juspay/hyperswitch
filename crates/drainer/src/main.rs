use drainer::{errors::DrainerResult, services, settings, start_drainer};

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

    let store = services::Store::new(&conf, false).await;
    let store = std::sync::Arc::new(store);

    let number_of_streams = store.config.drainer_num_partitions;
    let max_read_count = conf.drainer.max_read_count;

    start_drainer(store, number_of_streams, max_read_count).await?;

    Ok(())
}
