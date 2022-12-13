use drainer::{errors::DrainerError, start_drainer};
use router::configs::settings;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), DrainerError> {
    // Get configuration
    let cmd_line = settings::CmdLineConf::from_args();
    let conf = settings::Settings::with_config_path(cmd_line.config_path)
        .map_err(|e| DrainerError::ConfigParsingError(e.to_string()))?;

    let store = router::services::Store::new(&conf, false).await;
    let store = std::sync::Arc::new(store);

    let number_of_drainers = conf.drainer.num_partitions;
    let max_read_count = conf.drainer.max_read_count;

    start_drainer(store, number_of_drainers, max_read_count).await?;

    Ok(())
}
