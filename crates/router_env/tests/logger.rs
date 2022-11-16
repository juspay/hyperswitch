use std::{path::PathBuf, time::SystemTime};

use router_env as env;
mod test_module;
use env::{workspace_path, TelemetryGuard};
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

pub fn last_modified_log() -> Option<PathBuf> {
    std::fs::read_dir(workspace_path().join("logs"))
        .ok()?
        .flatten()
        .max_by_key(|entry| {
            entry
                .metadata()
                .and_then(|entry| entry.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
        .map(|n| n.path())
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn extra_fields() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::fs::read_dir(workspace_path().join("logs"))?;

    logger();

    env::log!(
        env::Level::WARN,
        tag = ?env::Tag::ApiIncomingRequest,
        category = ?env::Category::Api,
        flow = "some_flow",
        session_id = "some_session",
        answer = 13,
        message2 = "yyy",
        message = "Experiment",
    );

    let path = last_modified_log().unwrap();
    let file = std::fs::read_to_string(path).unwrap();

    file.lines()
        .flat_map(serde_json::from_str::<serde_json::Map<String, serde_json::Value>>)
        .find(|obj| {
            obj.get("extra")
                == Some(&serde_json::json!({
                    "answer": 13,
                    "message2": "yyy"
                }))
        })
        .unwrap();

    Ok(())
}
