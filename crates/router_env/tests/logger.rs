#![allow(clippy::unwrap_used)]

use router_env as env;
mod test_module;
use env::TelemetryGuard;
use test_module::some_module::*;

fn logger() -> &'static TelemetryGuard {
    use once_cell::sync::OnceCell;

    static INSTANCE: OnceCell<TelemetryGuard> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let config = env::Config::new().unwrap();

        env::logger::setup(
            &config.log,
            env::service_name!(),
            vec![env::service_name!()],
        )
        .unwrap()
    })
}

#[tokio::test]
async fn basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logger();

    fn_with_colon(13).await;

    Ok(())
}
