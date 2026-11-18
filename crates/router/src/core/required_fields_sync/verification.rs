//! Read-only V1–V4 orchestration. A receipt is evidence only under a reviewed executor contract.
//! No target is registered yet: JSON, a healthy pod, and a successful resolver cannot grant it.
use std::{collections::BTreeMap, future::Future, time::Duration};

use serde::Serialize;
use sha2::{Digest as _, Sha256};

use super::{
    apply_contract::{
        self as ac, ApplyResult, ApplyVerifier, VerificationBinding, VerifiedServing,
    },
    journal::{RestrictedJson, SecretGuard},
    raw_json::Json,
    types::{CompleteInventory, EvidenceScope, TargetIdentity},
    Digest, Name,
};

fn failed<T>() -> ApplyResult<T> {
    Err(ac::fail(ac::ApplyCode::VerificationFailed))
}
fn check(value: bool) -> ApplyResult<()> {
    if value {
        Ok(())
    } else {
        failed()
    }
}
pub(super) fn bytes_hash(bytes: &[u8]) -> ApplyResult<Digest> {
    ac::checked(format!("sha256:{:x}", Sha256::digest(bytes)).try_into())
}
fn json(bytes: &[u8]) -> ApplyResult<Json> {
    check(!bytes.is_empty() && bytes.len() <= 4 * 1024 * 1024)?;
    ac::checked(Json::parse(bytes))
}
fn canonical(bytes: &[u8]) -> ApplyResult<Digest> {
    bytes_hash(&ac::checked(json(bytes)?.canonical())?)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkRoute {
    Payment,
    Profile,
}

/// Inputs are provisioned by the caller's certified read-only adapter, never created here.
/// Hash includes the entire request dimensions/profile/payment inputs, excluding authentication.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ProbeKey {
    pub pod_uid: Name,
    pub reader: Digest,
    pub route: SdkRoute,
    pub input: Digest,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Pod {
    pub uid: Name,
    pub reader: Digest,
    pub build: Digest,
    pub effective_inputs: Digest,
}
/// Issued by the platform adapter after complete discovery under the Step 9 traffic gate.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Population {
    pub target: TargetIdentity,
    pub release: Digest,
    pub membership: Digest,
    pub pods: Vec<Pod>,
    pub observed_ms: u64,
    pub valid_until_ms: u64,
    pub complete: bool,
}
pub struct ExportRead {
    pub inventory: CompleteInventory,
    pub latest: Vec<u8>,
    pub versioned: Vec<u8>,
    pub requested_version: Name,
}
pub struct PodRead {
    pub key: ProbeKey,
    /// Transport/platform evidence, not a self-reported response/version header.
    pub addressed_pod: Name,
    pub status: u16,
    pub body: Vec<u8>,
}
#[derive(Clone, Serialize)]
pub struct BrowserBinding {
    pub request: Digest,
    pub phase: BrowserPhase,
    pub run: Digest,
    pub target: Digest,
    pub plan: Digest,
    pub response: Digest,
    pub input: Digest,
    pub cases: Digest,
    pub artifacts: Digest,
    pub reader: Digest,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserPhase {
    ProjectedBefore,
    ProjectedAfter,
    PostApply,
}
/// Adapter must run the pinned package and verify its Step 1 nested report and exact case set.
/// Neither this struct nor its hashes are accepted without the private registration below.
pub struct BrowserRead {
    pub binding: BrowserBinding,
    pub report: RestrictedJson,
    pub(super) components_passed: bool,
    pub(super) checkout_passed: bool,
    pub(super) retained_snapshot_passed: bool,
    pub(super) assets_passed: bool,
    pub(super) credentials_absent: bool,
    pub(super) confirm_intercepted: bool,
}
pub struct BackupRead {
    pub target: TargetIdentity,
    pub reader: Digest,
    pub version: Name,
    pub content: Vec<u8>,
    pub published_ms: u64,
    pub publication: Digest,
    /// Receipt from the actual provider parser/new startup, separately from last-good memory.
    pub startup: super::verification_provider::StartupReceipt,
}
/// Read-only transport seam. Implementations are capabilities supplied by the reviewed registry;
/// there are no mutation/seed operations and no target discovery through environment variables.
pub trait VerificationReads: Sync {
    fn now_ms(&self) -> u64;
    fn population(&self) -> impl Future<Output = ApplyResult<Population>> + Send;
    fn persisted(&self) -> impl Future<Output = ApplyResult<ExportRead>> + Send;
    fn pod(&self, key: &ProbeKey) -> impl Future<Output = ApplyResult<PodRead>> + Send;
    fn browser(
        &self,
        binding: &BrowserBinding,
        response: &[u8],
    ) -> impl Future<Output = ApplyResult<BrowserRead>> + Send;
    fn backup(&self, reader: &Digest) -> impl Future<Output = ApplyResult<BackupRead>> + Send;
}
#[derive(Clone)]
pub struct ExpectedProbe {
    pub request: Digest,
    pub key: ProbeKey,
    pub response: Vec<u8>,
    pub cases: Digest,
    pub artifacts: Digest,
}
/// Constructible only by a future evidence-reviewed target/executor registration.
pub struct VerificationContract {
    pub(super) scope: EvidenceScope,
    pub(super) run: Digest,
    pub(super) binding: VerificationBinding,
    pub(super) population: Population,
    pub(super) probes: Vec<ExpectedProbe>,
    pub(super) read_contract: Name,
    pub(super) deadline_ms: u64,
    pub(super) publication_after_ms: u64,
    pub(super) guard: SecretGuard,
}
impl VerificationContract {
    pub fn from_registry(_: &Name) -> ApplyResult<Self> {
        failed()
    }
}
pub struct ServingVerifier<R> {
    contract: VerificationContract,
    reads: R,
}
impl<R: VerificationReads> ServingVerifier<R> {
    pub fn new(contract: VerificationContract, reads: R) -> Self {
        Self { contract, reads }
    }
    async fn bounded<T>(&self, future: impl Future<Output = ApplyResult<T>>) -> ApplyResult<T> {
        let remaining = self
            .contract
            .deadline_ms
            .saturating_sub(self.reads.now_ms());
        check(remaining > 0 && remaining <= 40 * 60 * 1000)?;
        let value = tokio::time::timeout(Duration::from_millis(remaining), future)
            .await
            .map_err(|_| ac::fail(ac::ApplyCode::VerificationFailed))??;
        check(self.reads.now_ms() < self.contract.deadline_ms)?;
        Ok(value)
    }
    fn population(&self, observed: &Population) -> ApplyResult<()> {
        let expected = &self.contract.population;
        check(
            observed.complete
                && expected.complete
                && observed.target == self.contract.binding.target
                && observed.release == self.contract.binding.release
                && observed.target == expected.target
                && observed.release == expected.release
                && observed.membership == expected.membership
                && observed.pods == expected.pods
                && observed.observed_ms <= self.reads.now_ms()
                && observed.valid_until_ms > self.reads.now_ms()
                && !observed.pods.is_empty()
                && observed.pods.len() <= 256,
        )?;
        let mut pods = BTreeMap::new();
        for pod in &observed.pods {
            check(pods.insert(&pod.uid, pod).is_none())?;
            for route in [SdkRoute::Payment, SdkRoute::Profile] {
                check(self.contract.probes.iter().any(|p| {
                    p.key.pod_uid == pod.uid && p.key.reader == pod.reader && p.key.route == route
                }))?;
            }
        }
        check(!self.contract.probes.is_empty() && self.contract.probes.len() <= 1024)?;
        let mut unique = std::collections::BTreeSet::new();
        let mut input_bytes = 0usize;
        for probe in &self.contract.probes {
            check(probe.response.len() <= 262_144)?;
            input_bytes = input_bytes
                .checked_add(probe.response.len())
                .ok_or_else(|| ac::fail(ac::ApplyCode::VerificationFailed))?;
            check(input_bytes <= 32 * 1024 * 1024)?;
            check(
                pods.get(&probe.key.pod_uid)
                    .is_some_and(|p| p.reader == probe.key.reader)
                    && unique.insert(ac::checked(serde_json::to_vec(&ac::hash(&probe.key)?))?),
            )?;
        }
        Ok(())
    }
    /// Same exact-input runner for projected preflight and post-apply. No Rust/JS resolver result
    /// can substitute for the component, complete checkout, asset, and retained-session receipts.
    pub async fn preflight(
        &self,
        phase: BrowserPhase,
        probes: &[ExpectedProbe],
    ) -> ApplyResult<Vec<Digest>> {
        check(phase != BrowserPhase::PostApply && !probes.is_empty() && probes.len() <= 1024)?;
        check(probes.len() == self.contract.probes.len())?;
        for (p, registered) in probes.iter().zip(&self.contract.probes) {
            check(
                p.key == registered.key
                    && p.request == registered.request
                    && p.cases == registered.cases
                    && p.artifacts == registered.artifacts,
            )?;
        }
        let mut results = Vec::new();
        for p in probes {
            results.push(self.browser(phase, p, &p.response).await?);
        }
        Ok(results)
    }
    async fn browser(
        &self,
        phase: BrowserPhase,
        p: &ExpectedProbe,
        body: &[u8],
    ) -> ApplyResult<Digest> {
        let binding = BrowserBinding {
            request: p.request.clone(),
            phase,
            run: self.contract.run.clone(),
            target: ac::hash(&self.contract.binding.target)?,
            plan: self.contract.binding.plan.clone(),
            response: bytes_hash(body)?,
            input: p.key.input.clone(),
            cases: p.cases.clone(),
            artifacts: p.artifacts.clone(),
            reader: p.key.reader.clone(),
        };
        let got = self.bounded(self.reads.browser(&binding, body)).await?;
        check(
            ac::hash(&got.binding)? == ac::hash(&binding)?
                && got.components_passed
                && got.checkout_passed
                && got.retained_snapshot_passed
                && got.assets_passed
                && got.credentials_absent
                && got.confirm_intercepted,
        )?;
        bytes_hash(got.report.bytes())
    }
}
impl<R: VerificationReads> ApplyVerifier for ServingVerifier<R> {
    async fn verify(&self, binding: &VerificationBinding) -> ApplyResult<VerifiedServing> {
        check(ac::hash(binding)? == ac::hash(&self.contract.binding)?)?;
        let first = self.bounded(self.reads.population()).await?;
        self.population(&first)?;
        let persisted = self.bounded(self.reads.persisted()).await?;
        let proof = persisted.inventory.proof();
        check(
            proof.target == binding.target
                && proof.inventory_hash == binding.inventory
                && proof.evidence.scope == self.contract.scope
                && proof.evidence.contract_id == self.contract.read_contract
                && proof.identical_full_passes == 2
                && proof.version.as_ref() == Some(&persisted.requested_version),
        )?;
        let exported = verify_exports(&persisted)?;
        let mut pod_receipts = Vec::new();
        for expected in &self.contract.probes {
            let actual = self.bounded(self.reads.pod(&expected.key)).await?;
            check(
                actual.key == expected.key
                    && actual.addressed_pod == expected.key.pod_uid
                    && actual.status == 200
                    && canonical(&actual.body)? == canonical(&expected.response)?,
            )?;
            let browser = self
                .browser(BrowserPhase::PostApply, expected, &actual.body)
                .await?;
            pod_receipts.push((ac::hash(&actual.key)?, bytes_hash(&actual.body)?, browser));
        }
        let readers: BTreeMap<_, _> = first
            .pods
            .iter()
            .map(|p| Ok((ac::checked(serde_json::to_vec(&p.reader))?, &p.reader)))
            .collect::<ApplyResult<_>>()?;
        let mut backup_receipts = Vec::new();
        for reader in readers.into_values() {
            let b = self.bounded(self.reads.backup(reader)).await?;
            check(
                b.target == binding.target
                    && &b.reader == reader
                    && b.version == persisted.requested_version
                    && b.published_ms >= self.contract.publication_after_ms
                    && b.published_ms <= self.reads.now_ms()
                    && b.startup.content == bytes_hash(&b.content)?
                    && b.startup.config == exported,
            )?;
            backup_receipts.push((reader, b.publication, b.startup.content));
        }
        // Scaling/replacement/intent drift during slow browsers or backup checks invalidates all levels.
        self.population(&self.bounded(self.reads.population()).await?)?;
        let bytes = ac::checked(serde_json::to_vec(&serde_json::json!({"schema_version":1,
            "binding":ac::hash(binding)?, "v1":exported, "v2_v3":pod_receipts,
            "v4":backup_receipts, "population":first.membership,
            "scope":self.contract.scope, "hosted_apply":"disabled"})))?;
        Ok(VerifiedServing {
            binding: ac::hash(binding)?,
            scope: self.contract.scope,
            evidence: ac::checked(RestrictedJson::parse(&bytes, &self.contract.guard))?,
        })
    }
}

/// Compare cached/latest and explicit immutable-version exports against direct full inventory.
/// Unknown transformations are refused. Full raw-row preservation is separately bound by inventory hash.
pub fn verify_exports(read: &ExportRead) -> ApplyResult<Digest> {
    let latest = json(&read.latest)?;
    let versioned = json(&read.versioned)?;
    check(ac::checked(latest.canonical())? == ac::checked(versioned.canonical())?)?;
    let head = read
        .inventory
        .version_observations()
        .first()
        .ok_or_else(|| ac::fail(ac::ApplyCode::VerificationFailed))?;
    let data = ac::checked(ac::checked(head.body.get("data"))?.array())?;
    let version = data
        .first()
        .ok_or_else(|| ac::fail(ac::ApplyCode::VerificationFailed))?;
    check(
        ac::checked(ac::checked(version.get("id"))?.string())? == read.requested_version.as_str()
            && ac::checked(ac::checked(version.get("config"))?.canonical())?
                == ac::checked(latest.canonical())?,
    )?;
    let defaults = ac::checked(ac::checked(latest.get("default_configs"))?.object())?;
    check(defaults.len() == read.inventory.defaults().len())?;
    for (name, row) in read.inventory.defaults() {
        check(
            ac::checked(defaults.get(name.as_str()).ok_or(()))
                .and_then(|x| ac::checked(x.canonical()))?
                == ac::checked(ac::checked(row.raw.get("value"))?.canonical())?,
        )?;
    }
    let contexts = ac::checked(ac::checked(latest.get("contexts"))?.array())?;
    let overrides = ac::checked(ac::checked(latest.get("overrides"))?.object())?;
    check(contexts.len() == read.inventory.contexts().len())?;
    let mut ids = std::collections::BTreeSet::new();
    let mut references = std::collections::BTreeSet::new();
    let ordered = ac::checked(super::impact::order(
        &ac::checked(super::impact::rows(&read.inventory))?,
        &read.inventory,
    ))?;
    for (context, expected) in contexts.iter().zip(ordered) {
        let id = ac::checked(ac::checked(context.get("id"))?.string())?;
        check(id == expected.id && ids.insert(id.clone()))?;
        let row = read
            .inventory
            .contexts()
            .iter()
            .find(|(k, _)| k.as_str() == id)
            .map(|(_, v)| v)
            .ok_or_else(|| ac::fail(ac::ApplyCode::VerificationFailed))?;
        check(
            ac::checked(ac::checked(context.get("condition"))?.canonical())?
                == ac::checked(ac::checked(row.raw.get("value"))?.canonical())?,
        )?;
        let refs = ac::checked(ac::checked(context.get("override_with_keys"))?.array())?;
        check(refs.len() == 1)?;
        let reference = ac::checked(refs.first().ok_or(()))?;
        let reference = ac::checked(reference.string())?;
        check(reference == row.projection().override_id.as_str())?;
        references.insert(reference.clone());
        let values = ac::checked(overrides.get(&reference).ok_or(()))?;
        check(
            ac::checked(values.canonical())?
                == ac::checked(ac::checked(row.raw.get("override"))?.canonical())?,
        )?;
    }
    check(references.len() == overrides.len())?;
    let dimensions = ac::checked(ac::checked(latest.get("dimensions"))?.object())?;
    check(dimensions.len() == read.inventory.dimensions().len())?;
    for (name, row) in read.inventory.dimensions() {
        let dim = ac::checked(dimensions.get(name.as_str()).ok_or(()))?;
        for field in [
            "schema",
            "position",
            "dimension_type",
            "dependency_graph",
            "value_compute_function_name",
        ] {
            // superposition_types 0.119.0 DimensionInfo explicitly omits a None compute
            // function during serialization. No other absent metadata is normalized.
            if field == "value_compute_function_name"
                && ac::checked(row.raw.get(field))?.null()
                && ac::checked(dim.optional(field))?.is_none()
            {
                continue;
            }
            check(
                ac::checked(ac::checked(dim.get(field))?.canonical())?
                    == ac::checked(ac::checked(row.raw.get(field))?.canonical())?,
            )?;
        }
    }
    canonical(&read.latest)
}
