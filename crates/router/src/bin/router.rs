use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{BachError, BachResult},
    logger,
};
use structopt::StructOpt;

#[actix_web::main]
async fn main() -> BachResult<()> {
    // get commandline config before initializing config
    let cmd_line = CmdLineConf::from_args();
    let conf = Settings::with_config_path(cmd_line.config_path).unwrap();
    let _guard = logger::setup(&conf.log)?;

    logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);

    let (server, mut state) = router::start_server(conf)
        .await
        .expect("Failed to create the server");

    let _ = server.await;

    state.store.close().await;

    Err(BachError::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Server shut down",
    )))
}
