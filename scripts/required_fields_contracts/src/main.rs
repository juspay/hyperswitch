use std::{io::Write, path::PathBuf, time::Duration};

use aws_smithy_runtime_api::client::http::SharedHttpConnector;
use clap::Parser;
use required_fields_api_contracts::{
    api::{self, Api},
    contract::Registration,
    refuse, suite,
    transport::LocalConnector,
    Result,
};
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Parser)]
#[command(about = "Opt-in disposable Superposition contract probe; never use hosted credentials")]
struct Args {
    #[arg(long)]
    registration: PathBuf,
    #[arg(long)]
    report: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let bytes =
        std::fs::read(&args.registration).map_err(|_| refuse("registration unavailable"))?;
    if bytes.len() > 16384 {
        return Err(refuse("registration byte cap"));
    }
    let registration: Registration =
        serde_json::from_slice(&bytes).map_err(|_| refuse("invalid registration"))?;
    registration.validate()?;
    let guard = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixture.py");
    let connector = LocalConnector::new(registration.clone(), args.registration, guard)?;
    let trace = connector.trace.clone();
    let shared = SharedHttpConnector::new(connector);
    let api = Api {
        reader: api::client(
            shared.clone(),
            &registration.endpoint,
            2,
            Duration::from_secs(8),
        ),
        writer: api::client(shared, &registration.endpoint, 1, Duration::from_secs(8)),
        trace: trace.clone(),
        org: registration.org.clone(),
        workspace: registration.workspace.clone(),
    };
    // Reserve the report before writes; never overwrite existing evidence.
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.report)
        .map_err(|_| refuse("report must be a new writable file"))?;
    let results = suite::run(&api).await;
    required_fields_api_contracts::contract::validate_results(&results, true)?;
    let failed = results.iter().any(|r| r.status == "failed");
    let has_dispatch = !trace.lock().expect("trace lock").is_empty();
    let report = json!({"schema_version":1,"evidence_kind":if has_dispatch {"disposable_server"} else {"refused_fixture"},"client_version":"0.119.0",
        "client_lock_sha256":format!("{:x}",Sha256::digest(include_bytes!("../Cargo.lock"))),
        "hyperswitch_lock_sha256":format!("{:x}",Sha256::digest(include_bytes!("../../../Cargo.lock"))),
        "registration":registration,"results":results,"requests":*trace.lock().expect("trace lock"),
        "status":if failed {"failed"} else {"pending"},"hosted_apply_enabled":false,
        "uncertified":["CAS/fencing","external writer exclusion","fault-instrumented post-commit/cache/timeout termination","authenticated full-scope reads","server maximum limits"]});
    let data = serde_json::to_vec_pretty(&report).map_err(|_| refuse("report serialization"))?;
    if data.len() > 8 * 1024 * 1024 {
        return Err(refuse("report byte cap"));
    }
    output
        .write_all(&data)
        .map_err(|_| refuse("report write"))?;
    output
        .write_all(b"\n")
        .map_err(|_| refuse("report write"))?;
    output.sync_all().map_err(|_| refuse("report sync"))?;
    Err(refuse(if failed {
        "server contract failed; report saved"
    } else {
        "partial evidence saved; required certification gates remain pending"
    }))
}
