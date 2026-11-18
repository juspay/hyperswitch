//! Explicitly owned, synthetic disk handoff. Read-only child has no original executor memory.
//! This does not certify object-store encryption, retention, IAM or a deployed recovery image.
use std::{
    fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use super::*;

fn root() -> PathBuf {
    let root = PathBuf::from(
        std::env::var("RF_RECOVERY_FIXTURE_ROOT").expect("explicit disposable fixture required"),
    );
    assert!(root.is_absolute());
    assert_eq!(root.canonicalize().unwrap(), root);
    assert!(!fs::symlink_metadata(&root)
        .unwrap()
        .file_type()
        .is_symlink());
    let owner: Value = serde_json::from_slice(&fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(owner["purpose"], "Step 12 isolated recovery checks");
    assert_eq!(owner["root"], root.to_str().unwrap());
    assert_eq!(owner["cleanup_scope"], root.to_str().unwrap());
    assert_eq!(owner["owner"].as_str().unwrap().len(), 36);
    assert!(owner["hosted_target"].is_null());
    root
}
fn write(path: &Path, value: &Value) {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(&serde_json::to_vec(value).unwrap()).unwrap();
    file.sync_all().unwrap();
}
fn read(path: &Path) -> Value {
    let meta = fs::symlink_metadata(path).unwrap();
    assert!(meta.is_file() && !meta.file_type().is_symlink());
    assert!(meta.len() < 160 * 1024 * 1024);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
struct Reader(Value);
impl j::JournalReader for Reader {
    async fn read(&self, key: &str, max: usize) -> j::JournalResult<Option<j::StoredObject>> {
        let Some(value) = self.0.get(key) else {
            return Ok(None);
        };
        let bytes: Vec<u8> = serde_json::from_value(value["bytes"].clone()).unwrap();
        if bytes.len() > max {
            return Err(j::JournalError {
                code: j::JournalCode::LimitExceeded,
            }
            .into());
        }
        Ok(Some(j::StoredObject {
            bytes,
            version: serde_json::from_value(value["version"].clone()).unwrap(),
            retention: serde_json::from_value(value["retention"].clone()).unwrap(),
        }))
    }
}
#[tokio::test]
#[ignore = "requires registered RF_RECOVERY_FIXTURE_ROOT; launches only a read-only synthetic child"]
async fn independent_recovery_after_executor_disappears() {
    let owned = root();
    let mut allocated = None;
    for n in 0..1000 {
        let child = owned.join(format!("recovery-handoff-{n}"));
        if fs::create_dir(&child).is_ok() {
            allocated = Some(child);
            break;
        }
    }
    let child = allocated.expect("bounded disposable directory allocation");
    write(
        &child.join("owner.json"),
        &v!({"parent":owned,"purpose":"Step 12 synthetic recovery child","cleanup_scope":child}),
    );
    let old = original(Shape::TwoContexts, Fault::ReadAfter, StorageFault::None).await;
    write(
        &child.join("journal.json"),
        &old.store.inner.fixture_objects(),
    );
    write(&child.join("coordinator.json"), &v!(old.anchor));
    write(
        &child.join("current.json"),
        &v!({"contexts":old.state.contexts,"defaults":old.state.defaults,"dimensions":old.state.dimensions,
        "experiments":old.state.experiments,"groups":old.state.groups,"workspace":old.state.workspace}),
    );
    drop(old);
    let mut process = Command::new(std::env::current_exe().unwrap())
        .env_clear()
        .env("RF_RECOVERY_FIXTURE_ROOT", &owned)
        .env("RF_RECOVERY_CHILD", child.file_name().unwrap())
        .args([
            "plan::tests::apply_tests::recovery_tests::fixture::independent_reader_child",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut status = None;
    for _ in 0..100 {
        if let Some(result) = process.try_wait().unwrap() {
            status = Some(result);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if status.is_none() {
        process.kill().unwrap();
        process.wait().unwrap();
    }
    assert!(
        status.is_some_and(|s| s.success()),
        "independent recovery reader failed or exceeded deadline"
    );
    let report = read(&child.join("result.json"));
    assert_eq!(report["requests"][0]["state"], "after");
    assert_eq!(report["hosted_apply"], "disabled");
    assert_eq!(child.parent(), Some(owned.as_path()));
    let marker = read(&child.join("owner.json"));
    assert_eq!(marker["cleanup_scope"], child.to_str().unwrap());
    fs::remove_dir_all(child).unwrap();
}
#[tokio::test]
#[ignore = "child helper selected only by registered independent recovery fixture"]
async fn independent_reader_child() {
    let owned = root();
    let name = std::env::var("RF_RECOVERY_CHILD").expect("parent-owned fixture selection");
    assert!(
        name.starts_with("recovery-handoff-")
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    );
    let child = owned.join(name);
    assert_eq!(child.canonicalize().unwrap(), child);
    let marker = read(&child.join("owner.json"));
    assert_eq!(marker["parent"], owned.to_str().unwrap());
    assert_eq!(marker["cleanup_scope"], child.to_str().unwrap());
    let reader = Reader(read(&child.join("journal.json")));
    let anchor: j::UnresolvedRunRecord =
        serde_json::from_value(read(&child.join("coordinator.json"))).unwrap();
    let raw = read(&child.join("current.json"));
    let mut fixture = Fixture::new();
    fixture.contexts = serde_json::from_value(raw["contexts"].clone()).unwrap();
    fixture.defaults = serde_json::from_value(raw["defaults"].clone()).unwrap();
    fixture.dimensions = serde_json::from_value(raw["dimensions"].clone()).unwrap();
    fixture.experiments = serde_json::from_value(raw["experiments"].clone()).unwrap();
    fixture.groups = serde_json::from_value(raw["groups"].clone()).unwrap();
    fixture.workspace = raw["workspace"].clone();
    let current = complete(&fixture).await;
    let cert = j::tests::certificate();
    let target = case(Shape::TwoContexts).await.target;
    let observed = RecoverySource::load(&reader, &cert, &anchor.locator, Some(&anchor))
        .await
        .unwrap()
        .observe(&current, &target, &j::SecretGuard::default())
        .unwrap();
    assert_eq!(observed.report().requests[0].state, RequestState::After);
    assert!(!observed.report().requests[0].durable_prior_resolution);
    write(&child.join("result.json"), &v!(observed.report()));
}
