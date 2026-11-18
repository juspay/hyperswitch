//! Immutable recovery records. No HTTP dispatch, lease, restoration or production store adapter.
//!
//! A receipt establishes only a journal precondition. It cannot authorize a server write.
//! Store registration requires independently certified Q6 semantics; the registry is empty.

use std::{collections::BTreeMap, fmt, future::Future, time::Duration};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

pub use super::journal_snapshot::{Capture, PreparedSnapshot, RecoveryIdentity};
#[path = "journal_coordination.rs"]
mod coordination_bridge;
pub use coordination_bridge::JournalCheckpoint;

use super::{raw_json::Json, types::TargetIdentity, Digest, Name};

pub type JournalResult<T> = error_stack::Result<T, JournalError>;
pub const MIN_RETENTION_SECONDS: u64 = 90 * 24 * 60 * 60;
pub(super) const MAX_SNAPSHOT: usize = 64 * 1024 * 1024;
pub(super) const MAX_EVENT: usize = 4 * 1024 * 1024;
const MAX_EVENTS: u16 = 128;
pub(super) const MAX_REQUESTS: usize = 32;
const MAX_BODY: usize = 512 * 1024;
pub(super) const MAX_RECOVERY_BYTES: usize = 160 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalCode {
    CertificationPending,
    InvalidInput,
    BindingMismatch,
    CredentialMaterial,
    LimitExceeded,
    MissingGuarantee,
    Exists,
    AccessDenied,
    UnknownStorageOutcome,
    CorruptArtifact,
    MissingArtifact,
    IncompleteHistory,
    InvalidTransition,
}
#[derive(Debug, thiserror::Error)]
#[error("required-fields journal refused: {code:?}")]
pub struct JournalError {
    pub code: JournalCode,
}
pub(super) fn fail(code: JournalCode) -> error_stack::Report<JournalError> {
    JournalError { code }.into()
}
pub(super) fn encode<T: Serialize>(value: &T) -> JournalResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| fail(JournalCode::InvalidInput))
}
fn decode<T: DeserializeOwned>(bytes: &[u8]) -> JournalResult<T> {
    Json::parse(bytes).map_err(|_| fail(JournalCode::CorruptArtifact))?;
    serde_json::from_slice(bytes).map_err(|_| fail(JournalCode::CorruptArtifact))
}
pub(super) fn checksum(bytes: &[u8]) -> JournalResult<Digest> {
    super::raw_json::fingerprint("journal-v1", bytes).map_err(|_| fail(JournalCode::InvalidInput))
}
fn component<T: Serialize>(value: &T) -> JournalResult<String> {
    Ok(format!("{:x}", Sha256::digest(encode(value)?)))
}

/// Caller supplies known ephemeral credentials for rejection, not persistence. Also rejects
/// credential-shaped JSON keys/values. Unknown secret classification remains a Q6 input gate.
/// Never redact an image in place: reject it, because redaction would destroy restoration data.
#[derive(Default)]
pub struct SecretGuard {
    forbidden: Vec<String>,
}
impl SecretGuard {
    pub fn new(forbidden: Vec<String>) -> JournalResult<Self> {
        if forbidden.len() > 64 || forbidden.iter().any(|s| s.len() < 8 || s.len() > 8192) {
            return Err(fail(JournalCode::InvalidInput));
        }
        Ok(Self { forbidden })
    }
    fn check_text(&self, text: &str) -> JournalResult<()> {
        let lower = text.to_ascii_lowercase();
        if self.forbidden.iter().any(|s| text.contains(s))
            || [
                "bearer ",
                "basic ",
                "-----begin ",
                "x-amz-signature=",
                "x-goog-signature=",
            ]
            .iter()
            .any(|pattern| lower.contains(pattern))
        {
            return Err(fail(JournalCode::CredentialMaterial));
        }
        Ok(())
    }
    fn inspect(&self, value: &serde_json::Value, depth: usize) -> JournalResult<()> {
        if depth > 32 {
            return Err(fail(JournalCode::LimitExceeded));
        }
        match value {
            serde_json::Value::Object(values) => {
                for (key, value) in values {
                    let key = key.to_ascii_lowercase().replace('-', "_");
                    if [
                        "authorization",
                        "proxy_authorization",
                        "cookie",
                        "set_cookie",
                        "password",
                        "secret",
                        "token",
                        "api_key",
                        "private_key",
                        "credentials",
                    ]
                    .iter()
                    .any(|part| key == *part || key.ends_with(&format!("_{part}")))
                    {
                        return Err(fail(JournalCode::CredentialMaterial));
                    }
                    self.check_text(&key)?;
                    self.inspect(value, depth + 1)?;
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    self.inspect(value, depth + 1)?;
                }
            }
            serde_json::Value::String(value) => {
                self.check_text(value)?;
                // Raw nested JSON is stored as a string to retain numeric lexemes/whitespace.
                if value.trim_start().starts_with(['{', '[']) {
                    if let Ok(nested) = serde_json::from_str(value) {
                        self.inspect(&nested, depth + 1)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn check(&self, bytes: &[u8]) -> JournalResult<()> {
        let value = serde_json::from_slice(bytes).map_err(|_| fail(JournalCode::InvalidInput))?;
        self.inspect(&value, 0)
    }
}

/// Restricted exact UTF-8 JSON; Serialize retains its original bytes as a string.
#[derive(Clone, Serialize)]
#[serde(transparent)]
pub struct RestrictedJson(String);
impl<'de> Deserialize<'de> for RestrictedJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(raw.as_bytes(), &SecretGuard::default())
            .map_err(|_| serde::de::Error::custom("invalid restricted JSON"))
    }
}
impl fmt::Debug for RestrictedJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RestrictedJson")
            .field("bytes", &self.0.len())
            .finish()
    }
}
impl RestrictedJson {
    pub fn parse(bytes: &[u8], guard: &SecretGuard) -> JournalResult<Self> {
        if bytes.is_empty() || bytes.len() > MAX_SNAPSHOT {
            return Err(fail(JournalCode::LimitExceeded));
        }
        Json::parse(bytes).map_err(|_| fail(JournalCode::InvalidInput))?;
        guard.check(bytes)?;
        String::from_utf8(bytes.to_vec())
            .map(Self)
            .map_err(|_| fail(JournalCode::InvalidInput))
    }
    pub fn bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
    pub fn hash(&self) -> JournalResult<Digest> {
        checksum(self.bytes())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Post,
    Put,
    Patch,
    Delete,
}

/// Exact application-level request. Authentication/transport framing is never journaled.
/// A future certified adapter must consume these frozen bytes without reconstructing a body.
/// Construction does NOT certify that these bytes implement the actions (Step 2/10 gate).
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedRequest {
    method: Method,
    path: String,
    headers: BTreeMap<String, String>,
    body: RestrictedJson,
    pub(super) action_indices: Vec<usize>,
    wire_contract: Digest,
}
impl PreparedRequest {
    pub fn new(
        method: Method,
        path: String,
        headers: BTreeMap<String, String>,
        body: RestrictedJson,
        action_indices: Vec<usize>,
        wire_contract: Digest,
    ) -> JournalResult<Self> {
        let request = Self {
            method,
            path,
            headers,
            body,
            action_indices,
            wire_contract,
        };
        request.validate()?;
        Ok(request)
    }
    pub(super) fn validate(&self) -> JournalResult<()> {
        if !self.path.starts_with('/')
            || self.path.starts_with("//")
            || self.path.contains("..")
            || self.path.len() > 256
            || !self
                .path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
            || self.body.bytes().len() > MAX_BODY
            || self.action_indices.is_empty()
            || self.headers.len() != 1
            || self.headers.get("content-type").map(String::as_str) != Some("application/json")
        {
            return Err(fail(JournalCode::InvalidInput));
        }
        Ok(())
    }
    pub fn method(&self) -> Method {
        self.method
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
    }
    pub fn body(&self) -> &[u8] {
        self.body.bytes()
    }
    pub(super) fn wire_contract(&self) -> &Digest {
        &self.wire_contract
    }
    pub fn hash(&self) -> JournalResult<Digest> {
        checksum(&encode(self)?)
    }
}

/// All artifacts start held. Releasing holds/expiry requires later approved lifecycle work;
/// this module has no delete, unhold or mutable retention API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retention {
    pub created_at_unix: u64,
    pub retain_until_unix: u64,
    pub rollback_window_seconds: u64,
    pub protected_hold: bool,
}
impl Retention {
    pub(super) fn validate(&self) -> JournalResult<()> {
        if !self.protected_hold
            || self
                .retain_until_unix
                .checked_sub(self.created_at_unix)
                .is_none_or(|duration| {
                    duration < MIN_RETENTION_SECONDS.max(self.rollback_window_seconds)
                })
        {
            return Err(fail(JournalCode::MissingGuarantee));
        }
        Ok(())
    }
}

/// Required adapter semantics, not assertions supplied by a command-line user.
#[derive(Debug, Clone)]
pub struct StorageRequirements {
    pub conditional_create: bool,
    pub immutable_versions: bool,
    pub strong_complete_reads: bool,
    pub deny_overwrite_delete: bool,
    pub encrypted_restricted_access: bool,
    pub access_logged: bool,
    pub protected_retention: bool,
    pub independent_recovery_access: bool,
}
impl StorageRequirements {
    pub fn validate(&self) -> JournalResult<()> {
        if !self.conditional_create
            || !self.immutable_versions
            || !self.strong_complete_reads
            || !self.deny_overwrite_delete
            || !self.encrypted_restricted_access
            || !self.access_logged
            || !self.protected_retention
            || !self.independent_recovery_access
        {
            return Err(fail(JournalCode::MissingGuarantee));
        }
        Ok(())
    }
}
/// Unforgeable outside this module. No existing S3/file adapter currently satisfies registration.
pub struct CertifiedJournalStore {
    id: Digest,
    synthetic: bool,
}
impl CertifiedJournalStore {
    pub fn contract_id(&self) -> &Digest {
        &self.id
    }
    pub fn from_registry(_id: &Name) -> JournalResult<Self> {
        Err(fail(JournalCode::CertificationPending))
    }
}
/// Adapters must bound allocation before reading, distinguish absence from denial, preserve the
/// immutable version, and never perform a hidden overwrite/retry of an unknown create.
pub struct StoredObject {
    pub bytes: Vec<u8>,
    pub version: Name,
    pub retention: Retention,
}
pub trait JournalReader: Sync {
    fn read(
        &self,
        key: &str,
        max_bytes: usize,
    ) -> impl Future<Output = JournalResult<Option<StoredObject>>> + Send;
}
pub trait ImmutableJournalStore: JournalReader {
    fn create(
        &self,
        key: &str,
        bytes: &[u8],
        retention: &Retention,
    ) -> impl Future<Output = JournalResult<Name>> + Send;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReference {
    key: String,
    version: Name,
    hash: Digest,
    bytes: usize,
    retention: Retention,
}
impl ArtifactReference {
    pub fn hash(&self) -> &Digest {
        &self.hash
    }
    pub(super) fn retention(&self) -> &Retention {
        &self.retention
    }
    pub(super) fn bytes(&self) -> usize {
        self.bytes
    }
}
/// Not deserializable: only successful create + exact read-back can issue a new receipt.
#[derive(Debug)]
pub struct DurableReceipt {
    reference: ArtifactReference,
    store: Digest,
    synthetic: bool,
}
impl DurableReceipt {
    pub fn reference(&self) -> &ArtifactReference {
        &self.reference
    }
    pub fn is_certified(&self) -> bool {
        !self.synthetic
    }
    pub fn store_contract(&self) -> &Digest {
        &self.store
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunLocator {
    target: TargetIdentity,
    run: Digest,
    store: Digest,
    prepared: ArtifactReference,
}
impl RunLocator {
    pub fn run(&self) -> &Digest {
        &self.run
    }
    pub fn artifact(&self) -> &Digest {
        &self.prepared.hash
    }
}

/// Immutable recovery decision in the successor's namespace. Contains exact observations,
/// source anchors and exclusion evidence; it cannot replace any original intent or baseline.
pub(super) async fn persist_recovery<S: ImmutableJournalStore>(
    store: &S,
    certificate: &CertifiedJournalStore,
    target: &TargetIdentity,
    run: &Digest,
    contents: &RestrictedJson,
    retention: &Retention,
) -> JournalResult<ArtifactReference> {
    Ok(persist(
        store,
        certificate,
        format!("{}/recovery", prefix(target, run)?),
        contents.bytes(),
        retention,
        MAX_SNAPSHOT,
    )
    .await?
    .reference)
}
/// Immutable input to the future Step 9 active-run record. Never means safe takeover/retry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnresolvedRunRecord {
    pub locator: RunLocator,
    pub last_sequence: Option<u16>,
    pub last_event_hash: Option<Digest>,
    pub pending_requests: Vec<Digest>,
    pub pending_create_ownership: Vec<Digest>,
    pub first_adoption: ArtifactReference,
    pub blocks_new_execution: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation {
    /// HTTP success alone does not prove commit; post-commit errors/timeouts stay unresolved.
    Transport {
        request: Digest,
        status: Option<u16>,
        response_body_hash: Option<Digest>,
        version: Option<Name>,
        request_id: Option<Name>,
        outcome: TransportOutcome,
    },
    /// Complete direct-state images, retained for later three-way restore/verification.
    DirectState {
        request: Digest,
        inventory_hash: Digest,
        images: RestrictedJson,
        evidence: Digest,
    },
    Verification {
        request: Digest,
        evidence: RestrictedJson,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportOutcome {
    Response,
    Timeout,
    ConnectionLost,
    ExecutorLost,
    Unknown,
}
impl Observation {
    fn request(&self) -> &Digest {
        match self {
            Self::Transport { request, .. }
            | Self::DirectState { request, .. }
            | Self::Verification { request, .. } => request,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum EventBody {
    Intent {
        ordinal: usize,
        request: PreparedRequest,
    },
    Observation {
        observation: Observation,
    },
    Resolution {
        resolution: RequestResolution,
    },
    RunReport {
        report: RestrictedJson,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestResolution {
    pub request: Digest,
    pub disposition: super::apply_observation::RequestDisposition,
    pub inventory: Digest,
    pub images: RestrictedJson,
    pub contract: Digest,
}
/// Only a read-back-verified journal append can issue this coordinator transition capability.
pub struct ResolvedIntent {
    receipt: DurableReceipt,
    request: Digest,
}
impl ResolvedIntent {
    pub fn request_hash(&self) -> &Digest {
        &self.request
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    schema_version: u16,
    prepared_hash: Digest,
    sequence: u16,
    previous_hash: Digest,
    body: EventBody,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedRecord {
    schema_version: u16,
    store: Digest,
    run: Digest,
    baseline: ArtifactReference,
    snapshot: super::journal_snapshot::SnapshotData,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema_version: u16,
    locator: RunLocator,
}

fn prefix(target: &TargetIdentity, run: &Digest) -> JournalResult<String> {
    Ok(format!(
        "rf-journal-v1/{}/{}",
        component(target)?,
        component(run)?
    ))
}
fn event_key(locator: &RunLocator, sequence: u16) -> JournalResult<String> {
    Ok(format!(
        "{}/event-{sequence:04}",
        prefix(&locator.target, &locator.run)?
    ))
}
async fn read<R: JournalReader>(
    reader: &R,
    key: &str,
    limit: usize,
) -> JournalResult<Option<StoredObject>> {
    let object = tokio::time::timeout(Duration::from_secs(5), reader.read(key, limit))
        .await
        .map_err(|_| fail(JournalCode::UnknownStorageOutcome))?
        .map_err(|error| fail(error.current_context().code))?;
    if let Some(object) = &object {
        if object.bytes.len() > limit {
            return Err(fail(JournalCode::LimitExceeded));
        }
        object.retention.validate()?;
    }
    Ok(object)
}
fn verify(object: &StoredObject, reference: &ArtifactReference) -> JournalResult<()> {
    if object.version != reference.version
        || object.bytes.len() != reference.bytes
        || checksum(&object.bytes)? != reference.hash
        || object.retention != reference.retention
    {
        return Err(fail(JournalCode::CorruptArtifact));
    }
    Ok(())
}
async fn referenced<R: JournalReader>(
    reader: &R,
    reference: &ArtifactReference,
    limit: usize,
) -> JournalResult<StoredObject> {
    if reference.bytes > limit {
        return Err(fail(JournalCode::LimitExceeded));
    }
    let object = read(reader, &reference.key, limit)
        .await?
        .ok_or_else(|| fail(JournalCode::MissingArtifact))?;
    verify(&object, reference)?;
    Ok(object)
}
async fn persist<S: ImmutableJournalStore>(
    store: &S,
    certificate: &CertifiedJournalStore,
    key: String,
    bytes: &[u8],
    retention: &Retention,
    limit: usize,
) -> JournalResult<DurableReceipt> {
    if bytes.len() > limit {
        return Err(fail(JournalCode::LimitExceeded));
    }
    retention.validate()?;
    let version =
        tokio::time::timeout(Duration::from_secs(5), store.create(&key, bytes, retention))
            .await
            .map_err(|_| fail(JournalCode::UnknownStorageOutcome))?
            .map_err(|error| fail(error.current_context().code))?;
    let reference = ArtifactReference {
        key,
        version,
        hash: checksum(bytes)?,
        bytes: bytes.len(),
        retention: retention.clone(),
    };
    referenced(store, &reference, limit).await?;
    Ok(DurableReceipt {
        reference,
        store: certificate.id.clone(),
        synthetic: certificate.synthetic,
    })
}

/// Append-only writer. Error/unknown write poisons the handle: recover with an independent reader.
/// There is deliberately no constructor from recovery, dispatch method, or resumption shortcut.
pub struct RunJournal<'a, S> {
    store: &'a S,
    certificate: &'a CertifiedJournalStore,
    guard: &'a SecretGuard,
    locator: RunLocator,
    baseline: ArtifactReference,
    requests: Vec<PreparedRequest>,
    pending_create_ownership: Vec<Digest>,
    next_request: usize,
    next_sequence: u16,
    previous_hash: Digest,
    pending_requests: Vec<Digest>,
    poisoned: bool,
    bytes: usize,
    recovery_identity: RecoveryIdentity,
    prepared_contents: RestrictedJson,
}
pub struct RecordedIntent {
    receipt: DurableReceipt,
    request: PreparedRequest,
}
impl RecordedIntent {
    pub fn receipt(&self) -> &DurableReceipt {
        &self.receipt
    }
    pub fn request(&self) -> &PreparedRequest {
        &self.request
    }
}
impl<'a, S: ImmutableJournalStore> RunJournal<'a, S> {
    /// None explicitly means first adoption: an existing baseline is a conflict, never replaced.
    /// Later runs must supply the exact previously recovered baseline reference.
    pub async fn persist_prepared(
        store: &'a S,
        certificate: &'a CertifiedJournalStore,
        guard: &'a SecretGuard,
        run: Digest,
        snapshot: PreparedSnapshot,
        baseline: Option<&ArtifactReference>,
    ) -> JournalResult<(Self, DurableReceipt)> {
        snapshot.validate()?;
        let snapshot_bytes = encode(&snapshot)?;
        guard.check(&snapshot_bytes)?;
        let target = snapshot.data.identity.target.clone();
        let recovery_bytes = if let Some(reference) = &snapshot.data.recovery {
            if baseline.is_none()
                || reference.key != format!("{}/recovery", prefix(&target, &run)?)
                || reference.retention != snapshot.data.retention
            {
                return Err(fail(JournalCode::BindingMismatch));
            }
            referenced(store, reference, MAX_SNAPSHOT)
                .await?
                .bytes
                .len()
        } else {
            0
        };
        let baseline_key = format!("rf-journal-v1/{}/first-adoption", component(&target)?);
        let baseline = if let Some(reference) = baseline {
            if reference.key != baseline_key {
                return Err(fail(JournalCode::BindingMismatch));
            }
            let original = referenced(store, reference, MAX_SNAPSHOT).await?;
            let original: super::journal_snapshot::SnapshotData = decode(&original.bytes)?;
            original.validate()?;
            if original.identity.target != target {
                return Err(fail(JournalCode::BindingMismatch));
            }
            reference.clone()
        } else {
            persist(
                store,
                certificate,
                baseline_key,
                &snapshot_bytes,
                &snapshot.data.retention,
                MAX_SNAPSHOT,
            )
            .await?
            .reference
        };
        let requests = snapshot.data.requests.clone();
        let recovery_identity = snapshot.data.identity.clone();
        let prepared_contents = snapshot.data.contents.clone();
        let pending_create_ownership = snapshot.data.pending_create_ownership.clone();
        let retention = snapshot.data.retention.clone();
        let record = PreparedRecord {
            schema_version: 1,
            store: certificate.id.clone(),
            run: run.clone(),
            baseline: baseline.clone(),
            snapshot: snapshot.data,
        };
        let prepared = persist(
            store,
            certificate,
            format!("{}/prepared", prefix(&target, &run)?),
            &encode(&record)?,
            &retention,
            MAX_SNAPSHOT,
        )
        .await?;
        let locator = RunLocator {
            target,
            run,
            store: certificate.id.clone(),
            prepared: prepared.reference.clone(),
        };
        let marker = Marker {
            schema_version: 1,
            locator: locator.clone(),
        };
        let marker_bytes = encode(&marker)?;
        let bytes = baseline.bytes + prepared.reference.bytes + marker_bytes.len() + recovery_bytes;
        if bytes > MAX_RECOVERY_BYTES {
            return Err(fail(JournalCode::LimitExceeded));
        }
        persist(
            store,
            certificate,
            format!("{}/unresolved", prefix(&locator.target, &locator.run)?),
            &marker_bytes,
            &retention,
            MAX_EVENT,
        )
        .await?;
        Ok((
            Self {
                store,
                certificate,
                guard,
                previous_hash: locator.prepared.hash.clone(),
                locator,
                baseline,
                requests,
                pending_create_ownership,
                next_request: 0,
                next_sequence: 0,
                pending_requests: vec![],
                poisoned: false,
                bytes,
                recovery_identity,
                prepared_contents,
            },
            prepared,
        ))
    }
    pub fn unresolved(&self) -> UnresolvedRunRecord {
        UnresolvedRunRecord {
            locator: self.locator.clone(),
            last_sequence: self.next_sequence.checked_sub(1),
            last_event_hash: (self.next_sequence > 0).then(|| self.previous_hash.clone()),
            pending_requests: self.pending_requests.clone(),
            pending_create_ownership: self.pending_create_ownership.clone(),
            first_adoption: self.baseline.clone(),
            blocks_new_execution: true,
        }
    }
    async fn append(&mut self, body: EventBody) -> JournalResult<DurableReceipt> {
        if self.poisoned {
            return Err(fail(JournalCode::UnknownStorageOutcome));
        }
        if self.next_sequence >= MAX_EVENTS {
            return Err(fail(JournalCode::LimitExceeded));
        }
        let event = Event {
            schema_version: 2,
            prepared_hash: self.locator.prepared.hash.clone(),
            sequence: self.next_sequence,
            previous_hash: self.previous_hash.clone(),
            body,
        };
        let bytes = encode(&event)?;
        self.guard.check(&bytes)?;
        let total = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| fail(JournalCode::LimitExceeded))?;
        if total > MAX_RECOVERY_BYTES {
            return Err(fail(JournalCode::LimitExceeded));
        }
        self.poisoned = true;
        let receipt = persist(
            self.store,
            self.certificate,
            event_key(&self.locator, self.next_sequence)?,
            &bytes,
            &self.locator.prepared.retention,
            MAX_EVENT,
        )
        .await?;
        self.previous_hash = receipt.reference.hash.clone();
        self.next_sequence += 1;
        self.bytes = total;
        self.poisoned = false;
        Ok(receipt)
    }
    pub async fn record_intent(&mut self, ordinal: usize) -> JournalResult<RecordedIntent> {
        if ordinal != self.next_request {
            return Err(fail(JournalCode::InvalidTransition));
        }
        let request = self
            .requests
            .get(ordinal)
            .ok_or_else(|| fail(JournalCode::InvalidTransition))?
            .clone();
        let receipt = self
            .append(EventBody::Intent {
                ordinal,
                request: request.clone(),
            })
            .await?;
        self.next_request += 1;
        self.pending_requests.push(request.hash()?);
        Ok(RecordedIntent { receipt, request })
    }
    pub async fn record_observation(
        &mut self,
        observation: Observation,
    ) -> JournalResult<DurableReceipt> {
        validate_observation(&observation, &self.pending_requests)?;
        self.append(EventBody::Observation { observation }).await
    }
    pub async fn record_resolution(
        &mut self,
        evidence: &super::apply_observation::ObservedResolution,
    ) -> JournalResult<ResolvedIntent> {
        if !self.pending_requests.contains(&evidence.request)
            || !self.requests.iter().any(|r| {
                r.hash().is_ok_and(|h| h == evidence.request)
                    && r.wire_contract() == &evidence.contract
            })
        {
            return Err(fail(JournalCode::InvalidTransition));
        }
        let resolution = RequestResolution {
            request: evidence.request.clone(),
            disposition: evidence.disposition,
            inventory: evidence.inventory.clone(),
            images: evidence.images.clone(),
            contract: evidence.contract.clone(),
        };
        let receipt = self.append(EventBody::Resolution { resolution }).await?;
        self.pending_requests.retain(|r| r != &evidence.request);
        Ok(ResolvedIntent {
            receipt,
            request: evidence.request.clone(),
        })
    }
    pub async fn record_run_report(
        &mut self,
        report: RestrictedJson,
    ) -> JournalResult<DurableReceipt> {
        self.append(EventBody::RunReport { report }).await
    }
    pub fn artifact_hash(&self) -> &Digest {
        &self.locator.prepared.hash
    }
}
fn validate_observation(observation: &Observation, pending: &[Digest]) -> JournalResult<()> {
    if !pending.contains(observation.request()) {
        return Err(fail(JournalCode::InvalidTransition));
    }
    if let Observation::Transport {
        status,
        response_body_hash,
        outcome,
        ..
    } = observation
    {
        if status.is_some_and(|s| !(100..=599).contains(&s))
            || (matches!(outcome, TransportOutcome::Response)
                && (status.is_none() || response_body_hash.is_none()))
            || (!matches!(outcome, TransportOutcome::Response)
                && (status.is_some() || response_body_hash.is_some()))
        {
            return Err(fail(JournalCode::InvalidInput));
        }
    }
    Ok(())
}

/// Restricted recovery result; no Debug/Serialize that could leak rows or exact request data.
pub struct RecoveredRun {
    pub snapshot: PreparedSnapshot,
    pub first_adoption: PreparedSnapshot,
    pub observations: Vec<Observation>,
    pub resolutions: Vec<RequestResolution>,
    pub run_reports: Vec<RestrictedJson>,
    pub unresolved: UnresolvedRunRecord,
    store: Digest,
    synthetic: bool,
    pub(super) intended_requests: usize,
}
#[derive(Debug, Serialize)]
pub struct JournalReport {
    pub artifact: Digest,
    pub store_contract: Digest,
    pub event_count: u16,
    pub pending_requests: usize,
    pub pending_creates: usize,
    pub scope: &'static str,
    pub hosted_apply: &'static str,
    pub blocks_new_execution: bool,
}
impl RecoveredRun {
    pub fn report(&self) -> JournalReport {
        JournalReport {
            artifact: self.unresolved.locator.prepared.hash.clone(),
            store_contract: self.store.clone(),
            event_count: self.unresolved.last_sequence.map_or(0, |n| n + 1),
            pending_requests: self.unresolved.pending_requests.len(),
            pending_creates: self.unresolved.pending_create_ownership.len(),
            scope: if self.synthetic {
                "synthetic_fixture"
            } else {
                "certified_storage_only"
            },
            hosted_apply: "disabled",
            blocks_new_execution: true,
        }
    }
}

/// Scan all bounded slots (including overflow sentinel), not a possibly incomplete object listing.
/// The optional last durable anchor detects rollback below the Step 9 persisted head. Immutable
/// storage/complete read certification is indispensable when the original executor disappears.
pub async fn recover<R: JournalReader>(
    reader: &R,
    certificate: &CertifiedJournalStore,
    locator: &RunLocator,
    anchor: Option<&UnresolvedRunRecord>,
) -> JournalResult<RecoveredRun> {
    tokio::time::timeout(
        Duration::from_secs(30),
        recover_inner(reader, certificate, locator, anchor),
    )
    .await
    .map_err(|_| fail(JournalCode::UnknownStorageOutcome))?
}
/// Discover the immutable prepared record by registered target/run identity alone. This path
/// needs no executor memory, stdout, final report or original receipt. Storage authenticity and
/// complete reads remain certification requirements, not a hash's claim of provenance.
pub async fn recover_run<R: JournalReader>(
    reader: &R,
    certificate: &CertifiedJournalStore,
    target: &TargetIdentity,
    run: &Digest,
    anchor: Option<&UnresolvedRunRecord>,
) -> JournalResult<RecoveredRun> {
    let key = format!("{}/prepared", prefix(target, run)?);
    let object = read(reader, &key, MAX_SNAPSHOT)
        .await?
        .ok_or_else(|| fail(JournalCode::MissingArtifact))?;
    let locator = RunLocator {
        target: target.clone(),
        run: run.clone(),
        store: certificate.id.clone(),
        prepared: ArtifactReference {
            key,
            version: object.version,
            hash: checksum(&object.bytes)?,
            bytes: object.bytes.len(),
            retention: object.retention,
        },
    };
    recover(reader, certificate, &locator, anchor).await
}
async fn recover_inner<R: JournalReader>(
    reader: &R,
    certificate: &CertifiedJournalStore,
    locator: &RunLocator,
    anchor: Option<&UnresolvedRunRecord>,
) -> JournalResult<RecoveredRun> {
    let run_prefix = prefix(&locator.target, &locator.run)?;
    if locator.store != certificate.id || locator.prepared.key != format!("{run_prefix}/prepared") {
        return Err(fail(JournalCode::BindingMismatch));
    }
    let prepared = referenced(reader, &locator.prepared, MAX_SNAPSHOT).await?;
    let record: PreparedRecord = decode(&prepared.bytes)?;
    record.snapshot.validate()?;
    if record.schema_version != 1
        || record.store != certificate.id
        || record.run != locator.run
        || record.snapshot.identity.target != locator.target
        || record.snapshot.retention != prepared.retention
        || record.baseline.key
            != format!(
                "rf-journal-v1/{}/first-adoption",
                component(&locator.target)?
            )
    {
        return Err(fail(JournalCode::BindingMismatch));
    }
    let original = referenced(reader, &record.baseline, MAX_SNAPSHOT).await?;
    let recovery_bytes = if let Some(reference) = &record.snapshot.recovery {
        if reference.key != format!("{run_prefix}/recovery")
            || reference.retention != record.snapshot.retention
        {
            return Err(fail(JournalCode::BindingMismatch));
        }
        let decision = referenced(reader, reference, MAX_SNAPSHOT).await?;
        RestrictedJson::parse(&decision.bytes, &SecretGuard::default())?;
        decision.bytes.len()
    } else {
        0
    };
    let first_adoption: super::journal_snapshot::SnapshotData = decode(&original.bytes)?;
    first_adoption.validate()?;
    if first_adoption.identity.target != locator.target {
        return Err(fail(JournalCode::BindingMismatch));
    }
    let marker = read(reader, &format!("{run_prefix}/unresolved"), MAX_EVENT)
        .await?
        .ok_or_else(|| fail(JournalCode::MissingArtifact))?;
    let marker_value: Marker = decode(&marker.bytes)?;
    if marker_value.schema_version != 1
        || encode(&marker_value.locator)? != encode(locator)?
        || marker.retention != prepared.retention
    {
        return Err(fail(JournalCode::BindingMismatch));
    }
    let mut total_bytes =
        prepared.bytes.len() + original.bytes.len() + marker.bytes.len() + recovery_bytes;
    if total_bytes > MAX_RECOVERY_BYTES {
        return Err(fail(JournalCode::LimitExceeded));
    }
    let mut previous = locator.prepared.hash.clone();
    let mut pending = Vec::new();
    let mut observations = Vec::new();
    let mut resolutions = Vec::new();
    let mut run_reports = Vec::new();
    let mut next_ordinal = 0;
    let mut tail = None;
    let mut missing = false;
    let mut anchor_found =
        anchor.is_none_or(|a| a.last_sequence.is_none() && a.last_event_hash.is_none());
    if let Some(anchor) = anchor {
        if encode(&anchor.locator)? != encode(locator)?
            || anchor.first_adoption != record.baseline
            || !anchor.blocks_new_execution
            || anchor.pending_create_ownership != record.snapshot.pending_create_ownership
            || anchor.last_sequence.is_some() != anchor.last_event_hash.is_some()
            || (anchor.last_sequence.is_none() && !anchor.pending_requests.is_empty())
        {
            return Err(fail(JournalCode::BindingMismatch));
        }
    }
    for sequence in 0..=MAX_EVENTS {
        let Some(object) = read(reader, &event_key(locator, sequence)?, MAX_EVENT).await? else {
            missing = true;
            continue;
        };
        if missing || sequence == MAX_EVENTS {
            return Err(fail(JournalCode::IncompleteHistory));
        }
        total_bytes = total_bytes
            .checked_add(object.bytes.len())
            .ok_or_else(|| fail(JournalCode::LimitExceeded))?;
        if total_bytes > MAX_RECOVERY_BYTES {
            return Err(fail(JournalCode::LimitExceeded));
        }
        let event: Event = decode(&object.bytes)?;
        if ![1, 2].contains(&event.schema_version)
            || event.sequence != sequence
            || event.previous_hash != previous
            || event.prepared_hash != locator.prepared.hash
            || object.retention != prepared.retention
        {
            return Err(fail(JournalCode::CorruptArtifact));
        }
        previous = checksum(&object.bytes)?;
        match event.body {
            EventBody::Intent { ordinal, request } => {
                let expected = record
                    .snapshot
                    .requests
                    .get(ordinal)
                    .ok_or_else(|| fail(JournalCode::InvalidTransition))?;
                if ordinal != next_ordinal || encode(&request)? != encode(expected)? {
                    return Err(fail(JournalCode::BindingMismatch));
                }
                pending.push(request.hash()?);
                next_ordinal += 1;
            }
            EventBody::Observation { observation } => {
                validate_observation(&observation, &pending)?;
                observations.push(observation);
            }
            EventBody::Resolution { resolution } => {
                if event.schema_version != 2
                    || !pending.contains(&resolution.request)
                    || !record.snapshot.requests.iter().any(|r| {
                        r.hash().is_ok_and(|h| h == resolution.request)
                            && r.wire_contract() == &resolution.contract
                    })
                {
                    return Err(fail(JournalCode::InvalidTransition));
                }
                pending.retain(|request| request != &resolution.request);
                resolutions.push(resolution);
            }
            EventBody::RunReport { report } => {
                if event.schema_version != 2 {
                    return Err(fail(JournalCode::InvalidTransition));
                }
                run_reports.push(report);
            }
        }
        if let Some(anchor) = anchor.filter(|a| a.last_sequence == Some(sequence)) {
            if anchor.last_event_hash.as_ref() != Some(&previous)
                || anchor.pending_requests != pending
            {
                return Err(fail(JournalCode::CorruptArtifact));
            }
            anchor_found = true;
        }
        tail = Some(sequence);
    }
    if !anchor_found {
        return Err(fail(JournalCode::IncompleteHistory));
    }
    let unresolved = UnresolvedRunRecord {
        locator: locator.clone(),
        last_sequence: tail,
        last_event_hash: tail.map(|_| previous),
        pending_requests: pending,
        pending_create_ownership: record.snapshot.pending_create_ownership.clone(),
        first_adoption: record.baseline,
        blocks_new_execution: true,
    };
    Ok(RecoveredRun {
        snapshot: PreparedSnapshot {
            data: record.snapshot,
        },
        first_adoption: PreparedSnapshot {
            data: first_adoption,
        },
        observations,
        resolutions,
        run_reports,
        unresolved,
        store: certificate.id.clone(),
        synthetic: certificate.synthetic,
        intended_requests: next_ordinal,
    })
}

#[cfg(test)]
#[path = "journal_tests.rs"]
pub(super) mod tests;
