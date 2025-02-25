#![allow(clippy::unwrap_used)]

mod test_module;

use ::config::ConfigError;
use router_env::TelemetryGuard;

use self::test_module::fn_with_colon;

fn logger() -> error_stack::Result<&'static TelemetryGuard, ConfigError> {
    use once_cell::sync::OnceCell;

    static INSTANCE: OnceCell<TelemetryGuard> = OnceCell::new();
    Ok(INSTANCE.get_or_init(|| {
        let config = router_env::Config::new().unwrap();

        router_env::setup(&config.log, "router_env_test", []).unwrap()
    }))
}

#[tokio::test]
async fn basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logger()?;

    fn_with_colon(13).await;

    Ok(())
}
