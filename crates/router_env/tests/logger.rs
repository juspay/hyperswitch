#![allow(clippy::unwrap_used)]

mod test_module;

use router_env::TelemetryGuard;

use self::test_module::some_module::*;

fn logger() -> &'static TelemetryGuard {
    use once_cell::sync::OnceCell;

    static INSTANCE: OnceCell<TelemetryGuard> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let config = router_env::Config::new().unwrap();

        router_env::setup(&config.log, "router_env_test", [])
    })
}

#[tokio::test]
async fn basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logger();

    fn_with_colon(13).await;

    Ok(())
}
