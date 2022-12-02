use structopt::StructOpt;

use drainer::{errors::DrainerError, start_drainer};
use router::configs::settings;

#[tokio::main]
async fn main() -> Result<(), DrainerError> {
    // Get configuration
    let cmd_line = settings::CmdLineConf::from_args();
    let conf = settings::Settings::with_config_path(cmd_line.config_path)
        .map_err(|e| DrainerError::ConfigParsingError(e.to_string()))?;

    let store = router::services::Store::new(&conf).await;
    let store = std::sync::Arc::new(store);

    let number_of_drainers = conf.drainer.num_partitions;

    start_drainer(&store,&number_of_drainers,&200).await?;
    Ok(())
}
