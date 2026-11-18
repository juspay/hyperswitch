//! Opt-in owned filesystem fixture. Local fsync/create_new are exercised, not WORM, encryption,
//! retention enforcement, cross-principal authorization or hosted object-store certification.

use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
};

use super::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiskObject {
    bytes: Vec<u8>,
    version: Name,
    retention: Retention,
    hash: Digest,
}
struct DiskReader {
    root: PathBuf,
}
struct DiskWriter {
    reader: DiskReader,
}
fn owned_root() -> PathBuf {
    let root = PathBuf::from(
        std::env::var("RF_JOURNAL_FIXTURE_ROOT")
            .expect("explicit owned fixture registration required"),
    );
    assert!(root.is_absolute());
    assert_eq!(root, root.canonicalize().unwrap());
    assert!(!fs::symlink_metadata(&root)
        .unwrap()
        .file_type()
        .is_symlink());
    let owner: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(owner["purpose"], "Step 8 isolated journal checks");
    assert_eq!(owner["root"], root.to_str().unwrap());
    assert_eq!(owner["cleanup_scope"], root.to_str().unwrap());
    assert!(owner["owner"].as_str().is_some_and(|s| s.len() == 36));
    root
}
fn allocate(root: &Path) -> PathBuf {
    for sequence in 0..1000 {
        let child = root.join(format!("journal-store-{sequence}"));
        match fs::create_dir(&child) {
            Ok(()) => {
                fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
                return child;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => panic!("owned child allocation failed"),
        }
    }
    panic!("owned allocation limit")
}
impl DiskReader {
    fn file(&self, key: &str) -> JournalResult<PathBuf> {
        Ok(self.root.join(component(&key)?))
    }
}
impl JournalReader for DiskReader {
    async fn read(&self, key: &str, max_bytes: usize) -> JournalResult<Option<StoredObject>> {
        let path = self.file(key)?;
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(fail(JournalCode::AccessDenied)),
        };
        let bound = max_bytes
            .checked_mul(4)
            .and_then(|n| n.checked_add(4096))
            .ok_or_else(|| fail(JournalCode::LimitExceeded))?;
        if !meta.is_file()
            || meta.file_type().is_symlink()
            || meta.len() > u64::try_from(bound).unwrap()
            || meta.permissions().mode() & 0o077 != 0
        {
            return Err(fail(JournalCode::AccessDenied));
        }
        let file = fs::File::open(path).map_err(|_| fail(JournalCode::AccessDenied))?;
        let mut bytes = Vec::new();
        file.take(u64::try_from(bound).unwrap() + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| fail(JournalCode::AccessDenied))?;
        if bytes.len() > bound {
            return Err(fail(JournalCode::LimitExceeded));
        }
        let object: DiskObject = decode(&bytes)?;
        if object.bytes.len() > max_bytes || checksum(&object.bytes)? != object.hash {
            return Err(fail(JournalCode::CorruptArtifact));
        }
        Ok(Some(StoredObject {
            bytes: object.bytes,
            version: object.version,
            retention: object.retention,
        }))
    }
}
impl JournalReader for DiskWriter {
    async fn read(&self, key: &str, max_bytes: usize) -> JournalResult<Option<StoredObject>> {
        self.reader.read(key, max_bytes).await
    }
}
impl ImmutableJournalStore for DiskWriter {
    async fn create(&self, key: &str, bytes: &[u8], retention: &Retention) -> JournalResult<Name> {
        let hash = checksum(bytes)?;
        let version = name(&component(&hash)?);
        let object = DiskObject {
            bytes: bytes.to_vec(),
            version: version.clone(),
            retention: retention.clone(),
            hash,
        };
        let encoded = encode(&object)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.reader.file(key)?)
            .map_err(|error| {
                fail(if error.kind() == std::io::ErrorKind::AlreadyExists {
                    JournalCode::Exists
                } else {
                    JournalCode::AccessDenied
                })
            })?;
        file.write_all(&encoded)
            .and_then(|()| file.sync_all())
            .map_err(|_| fail(JournalCode::UnknownStorageOutcome))?;
        fs::File::open(&self.reader.root)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| fail(JournalCode::UnknownStorageOutcome))?;
        Ok(version)
    }
}
#[tokio::test]
#[ignore = "requires explicitly registered owned RF_JOURNAL_FIXTURE_ROOT"]
async fn independent_process_recovery_and_filesystem_tamper() {
    let owned = owned_root();
    let root = allocate(&owned);
    let cert = certificate();
    let guard = SecretGuard::default();
    let store = DiskWriter {
        reader: DiskReader { root: root.clone() },
    };
    let (mut writer, _) =
        RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
            .await
            .unwrap();
    writer.record_intent(0).await.unwrap();
    let anchor = writer.unresolved();
    drop(writer);
    drop(store);
    // No locator/receipt/snapshot/report is passed to the child; it reconstructs from target/run.
    let child = Command::new(std::env::current_exe().unwrap())
        .env_clear()
        .env("RF_JOURNAL_FIXTURE_ROOT", &owned)
        .env("RF_JOURNAL_CHILD_STORE", root.file_name().unwrap())
        .args([
            "journal::tests::fixture::independent_reader_child",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "independent child failed: {}",
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(String::from_utf8_lossy(&child.stdout).contains("independent recovery verified"));
    let reader = DiskReader { root: root.clone() };
    let key = event_key(&anchor.locator, 0).unwrap();
    let writer = DiskWriter {
        reader: DiskReader { root: root.clone() },
    };
    assert_eq!(
        code(writer.create(&key, b"overwrite", &retention()).await),
        JournalCode::Exists
    );
    // The OS owner can bypass the adapter: demonstrate the missing WORM/delete guarantee.
    // Tampering/deletion is performed only on this test's child and detected, never certified.
    let path = reader.file(&key).unwrap();
    let original = fs::read(&path).unwrap();
    fs::write(&path, b"truncated fixture object").unwrap();
    assert_eq!(
        code(recover_run(&reader, &cert, &target(), &hash("run"), Some(&anchor)).await),
        JournalCode::CorruptArtifact
    );
    fs::write(&path, original).unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(
        code(recover_run(&reader, &cert, &target(), &hash("run"), Some(&anchor)).await),
        JournalCode::IncompleteHistory
    );
    assert_eq!(root.parent(), Some(owned.as_path()));
    fs::remove_dir_all(root).unwrap();
}
#[tokio::test]
#[ignore = "child helper; selected by the independent process fixture only"]
async fn independent_reader_child() {
    let owned = owned_root();
    let child = std::env::var("RF_JOURNAL_CHILD_STORE").expect("parent fixture selection required");
    assert!(
        child.starts_with("journal-store-")
            && child
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    );
    let root = owned.join(child);
    assert_eq!(root.canonicalize().unwrap(), root);
    let reader = DiskReader { root };
    let recovered = recover_run(&reader, &certificate(), &target(), &hash("run"), None)
        .await
        .unwrap();
    assert_eq!(
        recovered.snapshot.requests()[0].body(),
        request(vec![0]).body()
    );
    assert_eq!(
        recovered.first_adoption.contents().bytes(),
        snapshot().contents().bytes()
    );
    assert_eq!(recovered.unresolved.pending_requests.len(), 1);
    assert_eq!(recovered.unresolved.pending_create_ownership.len(), 1);
    assert!(recovered.unresolved.blocks_new_execution);
    println!("independent recovery verified");
}
