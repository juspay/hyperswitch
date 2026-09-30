//! Exercise subscriber re-entrancy in a child so a regression fails instead of hanging CI.
use std::{process::Command, thread, time::{Duration, Instant}};

use router_env::{FormattingLayer, StorageSubscription};
use tracing_subscriber::prelude::*;

#[test]
fn tracing_callbacks_do_not_deadlock() {
    const CHILD: &str = "ROUTER_ENV_REENTRANCY_CASE";
    if let Ok(case) = std::env::var(CHILD) {
        let subscriber = tracing_subscriber::registry()
            .with(tracing_opentelemetry::layer())
            .with(StorageSubscription)
            .with(FormattingLayer::new(
                "test", std::io::sink, serde_json::ser::CompactFormatter,
            ).unwrap());
        // A global subscriber reproduces nested events; no exporter/network is needed.
        tracing::subscriber::set_global_default(subscriber).unwrap();
        let span = tracing::info_span!("payment", merchant_id = "test", level = tracing::field::Empty);
        let _entered = span.enter();
        match case.as_str() {
            "duplicate" => tracing::info!(merchant_id = "test", "payment event"),
            "reserved" => { span.record("level", "test"); }
            _ => panic!("unknown test case"),
        }
        return;
    }

    for case in ["duplicate", "reserved"] {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tracing_callbacks_do_not_deadlock", "--nocapture"])
            .env(CHILD, case)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "{case}: child failed: {status}");
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("{case}: tracing callback deadlocked");
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
