use std::fs::File;

use router::{
    configs::settings::{CmdLineConf, Settings, Subcommand},
    core::errors::{ApplicationError, ApplicationResult},
    logger,
};

#[actix_web::main]
async fn main() -> ApplicationResult<()> {
    // get commandline config before initializing config
    let cmd_line = <CmdLineConf as clap::Parser>::parse();
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

    #[allow(clippy::expect_used)]
    let conf = Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");
    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate router configuration");

    let _guard = logger::setup(&conf.log)?;

    let _prof_guard = pprof::ProfilerGuardBuilder::default().frequency(1000).blocklist(&["libc", "libgcc", "pthread", "vdso"]).build().unwrap();

    logger::info!("Application started [{:?}] [{:?}]", conf.server, conf.log);

    #[allow(clippy::expect_used)]
    let (server, mut state) = router::start_server(conf)
        .await
        .expect("Failed to create the server");

    let _ = server.await;

    state.store.close().await;

    if let Ok(report) = _prof_guard.report().build() {
        let file = File::create("flamegraph.svg").unwrap();
        let mut options = pprof::flamegraph::Options::default();
        options.image_width = Some(2500);
        report.flamegraph_with_options(file, &mut options).unwrap();
    };

    Err(ApplicationError::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Server shut down",
    )))
}
