use drainer::{errors::DrainerResult, services, settings, start_drainer};
use structopt::StructOpt;

#[tokio::main]
async fn main() -> DrainerResult<()> {
    // Get configuration
    let cmd_line = settings::CmdLineConf::from_args();

    #[allow(clippy::expect_used)]
    let conf = settings::Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");

    let store = services::Store::new(&conf, false).await;
    let store = std::sync::Arc::new(store);

    let number_of_streams = store.config.drainer_num_partitions;
    let max_read_count = conf.drainer.max_read_count;

    start_drainer(store, number_of_streams, max_read_count).await?;

    Ok(())
}
