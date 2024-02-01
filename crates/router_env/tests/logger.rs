#![allow(clippy::unwrap_used)]

mod test_module;

use router_env::TelemetryGuard;

use self::test_module::some_module::*;

/// This method retrieves a static instance of TelemetryGuard using OnceCell. If the instance does not exist, it creates a new one by initializing the TelemetryGuard with the logging configuration from router_env. The TelemetryGuard is then returned.
fn logger() -> &'static TelemetryGuard {
    use once_cell::sync::OnceCell;

    static INSTANCE: OnceCell<TelemetryGuard> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let config = router_env::Config::new().unwrap();

        router_env::setup(&config.log, "router_env_test", [])
    })
}

#[tokio::test]
/// This async function performs a basic operation by calling the `logger` function and then awaiting the result of calling the `fn_with_colon` function with the value `13`. It returns a `Result` with an `Ok` value if the operation is successful, otherwise it returns an error.
async fn basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logger();

    fn_with_colon(13).await;

    Ok(())
}
