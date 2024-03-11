use router::{
    configs::settings::{CmdLineConf, Settings},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
};

#[tokio::main]
async fn main() -> ApplicationResult<()> {
    // get commandline config before initializing config
    let cmd_line = <CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate router configuration");

    #[cfg(feature = "vergen")]
    println!("Starting router (Version: {})", router_env::git_tag!());

    let _guard = router_env::setup(
        &conf.log,
        router_env::service_name!(),
        [router_env::service_name!(), "actix_server"],
    );

    // let (signal_sender, mut signal_receiver) = tokio::sync::mpsc::channel(1);

    // let signal = common_utils::signals::get_allowed_signals().map_err(|error| {
    //     logger::error!(?error, "Signal Handler Error");
    //     ApplicationError::InvalidConfigurationValueError(error.to_string())
    // })?;
    // let handle = signal.handle();
    // let task_handle = tokio::spawn(common_utils::signals::signal_handler(signal, signal_sender));

    logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);

    let body = async {
        #[allow(clippy::expect_used)]
        let (server, background_thread_awaiter) = Box::pin(router::start_server(conf))
            .await
            .expect("Failed to create the server");

        let _ = server.await;
    };

    let background_thread_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body);

    // match signal_receiver.try_recv() {
    //     Ok(()) => {
    //         let _ = background_thread_awaiter.await;

    //         handle.close();
    //         task_handle.await.unwrap();
    //     }
    //     _ => {}
    // }

    Err(ApplicationError::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Server shut down",
    )))
}
