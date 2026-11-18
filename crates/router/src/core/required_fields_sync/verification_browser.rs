//! Strict reader for W's bounded Step 11 envelope containing unchanged Step 1 stage reports.
//! Receipt parsing is not executor attestation; the production registry remains closed.
use std::collections::BTreeMap;

use serde::Deserialize;

use super::{
    apply_contract as ac,
    journal::{RestrictedJson, SecretGuard},
    raw_json::Json,
    types::EvidenceScope,
    verification::{bytes_hash, BrowserBinding, BrowserPhase, BrowserRead},
    Digest,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Execution {
    engine: String,
    artifact_sha256: String,
    runtime: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultRow {
    id: String,
    status: String,
    detail: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    schema_version: u16,
    contract_id: String,
    pack_sha256: String,
    provenance: serde_json::Value,
    stage: String,
    execution: Execution,
    results: Vec<ResultRow>,
    status: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    phase: String,
    response_sha256: String,
    input_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema_version: u16,
    scope: String,
    request_sha256: String,
    run: String,
    target: String,
    plan: String,
    reader: String,
    cases_sha256: String,
    artifacts_sha256: String,
    bindings: Vec<Binding>,
    reports: Vec<Report>,
    parent_pack_sha256: String,
    regressions: Vec<Report>,
    observations: BTreeMap<String, String>,
    status: String,
    platform_credential_isolation: String,
}
fn same(value: &str, expected: &Digest) -> ac::ApplyResult<()> {
    let digest: Digest = ac::checked(format!("sha256:{value}").try_into())?;
    require(&digest == expected)
}
fn require(value: bool) -> ac::ApplyResult<()> {
    if value {
        Ok(())
    } else {
        Err(ac::fail(ac::ApplyCode::VerificationFailed))
    }
}
impl BrowserRead {
    /// The current companion is local synthetic evidence. No report can assert platform credential
    /// isolation for itself. A future registered executor must supply independent mount/access proof.
    pub fn from_companion(
        binding: BrowserBinding,
        bytes: &[u8],
        scope: EvidenceScope,
        guard: &SecretGuard,
    ) -> ac::ApplyResult<Self> {
        require(bytes.len() <= 128 * 1024)?;
        ac::checked(Json::parse(bytes))?; // reject duplicate keys before typed deserialization
        let e: Envelope = ac::checked(serde_json::from_slice(bytes))?;
        // The current package supports exactly this pinned Step 1 corpus. A different/old/mobile
        // reader requires its own reviewed package and case contract, never an empty case set.
        let baseline_bytes =
            include_bytes!("../../../../../docs/required_fields_sync_contracts/sdk-v1/cases.json");
        let baseline: serde_json::Value = ac::checked(serde_json::from_slice(baseline_bytes))?;
        let provenance = ac::checked(baseline.get("provenance").ok_or(()))?;
        same(&e.parent_pack_sha256, &bytes_hash(baseline_bytes)?)?;
        require(e.regressions.len() == 3)?;
        for (report, stage) in
            e.regressions
                .iter()
                .zip(["browser-components", "checkout", "js-resolver"])
        {
            require(
                report.schema_version == 1
                    && report.contract_id == "required-fields-sdk-v1"
                    && report.stage == stage
                    && report.execution.engine == stage
                    && report.status == "passed"
                    && !report.execution.runtime.is_empty()
                    && report.execution.runtime.len() <= 128
                    && &report.provenance == provenance,
            )?;
            same(&report.pack_sha256, &bytes_hash(baseline_bytes)?)?;
            let artifact = if stage == "js-resolver" {
                let hash = ac::checked(
                    provenance
                        .get("files")
                        .and_then(|v| v.get("shared-code/sdk-utils/superposition/superposition.js"))
                        .and_then(serde_json::Value::as_str)
                        .ok_or(()),
                )?;
                ac::checked(format!("sha256:{hash}").try_into())?
            } else {
                binding.artifacts.clone()
            };
            same(&report.execution.artifact_sha256, &artifact)?;
            let mut expected = std::collections::BTreeSet::new();
            let field = if stage == "js-resolver" {
                "resolver_cases"
            } else {
                "browser_case_ids"
            };
            for value in ac::checked(
                baseline
                    .get(field)
                    .and_then(serde_json::Value::as_array)
                    .ok_or(()),
            )? {
                let id = ac::checked(
                    if stage == "js-resolver" {
                        value.get("id").and_then(serde_json::Value::as_str)
                    } else {
                        value.as_str()
                    }
                    .ok_or(()),
                )?;
                if stage == "js-resolver"
                    || id.starts_with(if stage == "checkout" { "E-" } else { "B-" })
                {
                    expected.insert(id.to_owned());
                }
            }
            require(!expected.is_empty() && report.results.len() == expected.len())?;
            for result in &report.results {
                require(
                    result.status == "passed"
                        && result.detail.len() <= 512
                        && expected.remove(&result.id),
                )?;
            }
            require(expected.is_empty())?;
        }
        let synthetic = match scope {
            #[cfg(test)]
            EvidenceScope::SyntheticFixture => true,
            EvidenceScope::CertifiedServer => false,
        };
        require(
            synthetic
                && e.scope == "synthetic_local"
                && e.platform_credential_isolation == "pending"
                && e.schema_version == 1
                && e.status == "passed"
                && e.bindings.len() == 3
                && e.reports.len() == 2,
        )?;
        for (got, want) in [
            (&e.request_sha256, &binding.request),
            (&e.run, &binding.run),
            (&e.target, &binding.target),
            (&e.plan, &binding.plan),
            (&e.reader, &binding.reader),
            (&e.cases_sha256, &binding.cases),
            (&e.artifacts_sha256, &binding.artifacts),
        ] {
            same(got, want)?;
        }
        let phases = ["projected_before", "projected_after", "post_apply"];
        for (got, phase) in e.bindings.iter().zip(phases) {
            require(got.phase == phase)?;
        }
        let index = match binding.phase {
            BrowserPhase::ProjectedBefore => 0,
            BrowserPhase::ProjectedAfter => 1,
            BrowserPhase::PostApply => 2,
        };
        let selected = e
            .bindings
            .get(index)
            .ok_or_else(|| ac::fail(ac::ApplyCode::VerificationFailed))?;
        same(&selected.response_sha256, &binding.response)?;
        same(&selected.input_sha256, &binding.input)?;
        let mut ids = std::collections::BTreeSet::new();
        for (report, stage) in e.reports.iter().zip(["browser-components", "checkout"]) {
            require(
                report.schema_version == 1
                    && report.contract_id == "required-fields-sdk-v1"
                    && report.stage == stage
                    && report.execution.engine == stage
                    && report.execution.runtime.len() <= 128
                    && !report.execution.runtime.is_empty()
                    && report.status == "passed",
            )?;
            same(&report.pack_sha256, &binding.cases)?;
            same(&report.execution.artifact_sha256, &binding.artifacts)?;
            require(
                bytes_hash(&ac::checked(
                    ac::checked(Json::parse(&ac::checked(serde_json::to_vec(
                        &report.provenance,
                    ))?))?
                    .canonical(),
                )?)? == binding.reader,
            )?;
            let prefix = if stage == "checkout" { "E" } else { "B" };
            let mut expected: std::collections::BTreeSet<_> =
                (0..3).map(|i| format!("{prefix}-response-{i}")).collect();
            if stage == "checkout" {
                expected.insert("E-retained".into());
            }
            require(report.results.len() == expected.len())?;
            for result in &report.results {
                require(
                    result.status == "passed"
                        && result.detail.len() <= 512
                        && expected.remove(&result.id)
                        && ids.insert(result.id.clone()),
                )?;
            }
            require(expected.is_empty())?;
        }
        require(
            ids.len() == e.observations.len()
                && ids.iter().all(|id| e.observations.contains_key(id)),
        )?;
        for hash in e.observations.values() {
            let _: Digest = ac::checked(format!("sha256:{hash}").try_into())?;
        }
        Ok(Self {
            binding,
            report: ac::checked(RestrictedJson::parse(bytes, guard))?,
            components_passed: true,
            checkout_passed: true,
            retained_snapshot_passed: true,
            assets_passed: true,
            credentials_absent: true,
            confirm_intercepted: true,
        })
    }
}
