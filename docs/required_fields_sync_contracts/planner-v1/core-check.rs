//! Copied as tests/planner.rs into the owned Step 1 Rust probe before offline execution.
//! Uses actual core 0.119.0; it is not evidence of any server image or HTTP behavior.
use std::io::Write as _;

use serde_json::{json, Value};
use superposition_core::{eval_config, helpers::hash, Config, MergeStrategy};

#[test]
fn actual_core_resolves_bound_synthetic_candidates_and_exposes_identity_collisions() {
    let root = std::path::PathBuf::from(std::env::var("RF_STEP6_FIXTURE_ROOT").unwrap());
    let owner: Value =
        serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert!(root.is_absolute());
    assert_eq!(owner["root"], json!(root));
    assert_eq!(owner["cleanup_scope"], json!(root));
    assert_eq!(owner["purpose"], "Step 6 offline synthetic planner checks");
    // Independently specified: current core identity sorts arrays, but resolving preserves order.
    assert_eq!(
        hash(&json!({"options":["B","A"]})),
        hash(&json!({"options":["A","B"]}))
    );
    assert_eq!(
        hash(&json!({"options":["A,B"]})),
        hash(&json!({"options":["A","B"]}))
    );
    assert_ne!(
        hash(&json!({"options":["A","A"]})),
        hash(&json!({"options":["A"]}))
    );
    let bytes = std::fs::read(root.join("planner-cases.json")).unwrap();
    assert!(bytes.len() <= 1024 * 1024);
    let cases: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cases["scope"], "synthetic_fixture");
    let mut results = Vec::new();
    for profile in cases["profiles"].as_array().unwrap() {
        let request = &profile["request"];
        let mut observations = Vec::new();
        for context in request["contexts"].as_array().unwrap() {
            let mut phases = Vec::new();
            for phase in ["before", "after"] {
                let config: Config = serde_json::from_value(request[phase].clone()).unwrap();
                phases.push(Value::Object(eval_config(
                    config,
                    serde_json::from_value(context.clone()).unwrap(),
                    MergeStrategy::MERGE,
                    None,
                    None,
                )));
            }
            observations.push(phases);
        }
        results.push(json!({"id":profile["id"],"observations":observations}));
    }
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join("core-observations.json"))
        .unwrap();
    file.write_all(&serde_json::to_vec(&results).unwrap())
        .unwrap();
}
