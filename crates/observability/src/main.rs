//! The standalone `observability` binary.
//!
//! No signal handling: unlike `drainer`, which must interrupt a loop actix knows nothing about,
//! everything this service does happens inside a request, and actix's own graceful shutdown
//! already stops accepting and drains in-flight requests on `SIGTERM`.

use observability::{
    errors::{self, ObservabilityResult},
    settings, start_server,
    state::AppState,
};

#[actix_web::main]
async fn main() -> ObservabilityResult<()> {
    let cmd_line = <settings::CmdLineConf as clap::Parser>::parse();

    #[allow(clippy::expect_used)]
    let conf = settings::Settings::with_config_path(cmd_line.config_path)
        .expect("Unable to construct application configuration");

    // Before anything is bound or connected: a bad configuration should be a failure to start,
    // not a failure on the first alert.
    #[allow(clippy::expect_used)]
    conf.validate()
        .expect("Failed to validate observability configuration");

    let state = AppState::new(conf).await;

    #[allow(clippy::print_stdout)] // The logger has not yet been initialized
    #[cfg(feature = "vergen")]
    {
        println!(
            "Starting observability (Version: {})",
            router_env::git_tag!()
        );
    }

    let _guard = router_env::setup(
        &state.conf.log,
        router_env::service_name!(),
        [router_env::service_name!(), "actix_server"],
    );

    observability::logger::info!(
        "Observability started on {}:{}",
        state.conf.server.host,
        state.conf.server.port
    );

    let server = Box::pin(start_server(state)).await?;
    server.await.map_err(errors::ConfigurationError::from)?;

    Ok(())
}
