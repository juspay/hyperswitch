//! Disabled execution contracts. Candidate JSON cannot manufacture a server or verifier capability.
use std::{collections::BTreeSet, future::Future};

use serde::Serialize;

use super::{authority::TargetRegistration, journal::RestrictedJson, types::*, Digest, Name};

pub type ApplyResult<T> = error_stack::Result<T, ApplyError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyCode {
    InvalidInput,
    LimitExceeded,
    UnsupportedServerContract,
    TargetMismatch,
    NotAuthoritative,
    ConcurrentChange,
    LeaseLost,
    ReadIncomplete,
    JournalUnavailable,
    RequestRejected,
    CommitUnknown,
    VerificationFailed,
    Cancelled,
}
#[derive(Debug, thiserror::Error)]
#[error("required-fields apply stopped: {code:?}")]
pub struct ApplyError {
    pub code: ApplyCode,
}
pub(super) fn fail(code: ApplyCode) -> error_stack::Report<ApplyError> {
    ApplyError { code }.into()
}
pub(super) fn checked<T, E>(value: Result<T, E>) -> ApplyResult<T> {
    value.map_err(|_| fail(ApplyCode::InvalidInput))
}
pub(super) fn hash<T: Serialize>(value: &T) -> ApplyResult<Digest> {
    checked(super::raw_json::fingerprint(
        "apply-v1",
        &checked(serde_json::to_vec(value))?,
    ))
}

/// Every field is filled by the reviewed registry, not a user-supplied capability document.
/// Limits/rollback statuses/audit fields are exact-server claims; the production registry is empty.
pub struct CertifiedMutationTarget {
    pub(super) registration: TargetRegistration,
    pub(super) id: Digest,
    pub(super) read_contract: Name,
    pub(super) server_image: Digest,
    pub(super) server_source: super::Revision,
    pub(super) scope: EvidenceScope,
    pub(super) max_prerequisites: usize,
    pub(super) max_context_actions: usize,
    pub(super) max_body: usize,
    pub(super) request_ms: u64,
    pub(super) rollback_statuses: BTreeSet<u16>,
    pub(super) context_audit: BTreeSet<String>,
    pub(super) default_audit: BTreeSet<String>,
    pub(super) dimension_audit: BTreeSet<String>,
}
impl CertifiedMutationTarget {
    pub fn from_registry(_id: &Name) -> ApplyResult<Self> {
        Err(fail(ApplyCode::UnsupportedServerContract))
    }
    pub(super) fn validate(&self) -> ApplyResult<()> {
        checked(self.registration.validate())?;
        if self.id != self.registration.server_contract
            || self.max_prerequisites == 0
            || self.max_prerequisites > 32
            || self.max_context_actions == 0
            || self.max_context_actions > 1000
            || self.max_body == 0
            || self.max_body > 512 * 1024
            || self.request_ms == 0
            || self.request_ms > 10_000
            || self
                .rollback_statuses
                .iter()
                .any(|s| ![400, 404, 409, 422].contains(s))
        {
            return Err(fail(ApplyCode::UnsupportedServerContract));
        }
        // Never mask values, schemas, descriptions, IDs or arbitrary metadata as audit fields.
        for set in [
            &self.context_audit,
            &self.default_audit,
            &self.dimension_audit,
        ] {
            if set.iter().any(|s| {
                ![
                    "created_at",
                    "last_modified_at",
                    "created_by",
                    "last_modified_by",
                    "change_reason",
                ]
                .contains(&s.as_str())
            }) {
                return Err(fail(ApplyCode::UnsupportedServerContract));
            }
        }
        Ok(())
    }
    pub(super) fn inventory(&self, value: &CompleteInventory) -> ApplyResult<()> {
        let proof = value.proof();
        if proof.target != self.registration.target
            || proof.evidence.contract_id != self.read_contract
            || proof.evidence.server_image != self.server_image
            || proof.evidence.server_source != self.server_source
            || proof.evidence.scope != self.scope
            || proof.evidence.client_version != "0.119.0"
            || proof.identical_full_passes != 2
            || proof.version.is_none()
        {
            return Err(fail(ApplyCode::TargetMismatch));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ApplyOutcome {
    NoChangeVerified,
    NoChangeUnverified,
    Refused,
    RejectedNoChange,
    AppliedVerified,
    PartialApplied,
    CommitUnknown,
    AppliedUnverified,
}
#[derive(Debug, Serialize)]
pub struct ApplyReport {
    pub schema_version: u16,
    pub target: Digest,
    pub plan: Digest,
    pub outcome: ApplyOutcome,
    pub exit_code: u8,
    pub dispatched: usize,
    pub committed: Vec<Digest>,
    pub rejected: Vec<Digest>,
    pub unknown: Vec<Digest>,
    pub reason: Option<ApplyCode>,
    pub recovery_artifact: Option<Digest>,
    pub verification: Option<Digest>,
    pub hosted_apply: &'static str,
}
pub(super) struct OutcomeFacts {
    pub planned: usize,
    pub dispatched: usize,
    pub committed: usize,
    pub rejected: usize,
    pub unknown: usize,
    pub preflight: bool,
    pub verified: bool,
    pub reason: Option<ApplyCode>,
}
pub(super) fn classify(facts: OutcomeFacts) -> (ApplyOutcome, u8) {
    let OutcomeFacts {
        planned,
        dispatched,
        committed,
        rejected,
        unknown,
        preflight,
        verified,
        reason,
    } = facts;
    use ApplyOutcome as O;
    if unknown > 0 {
        return (O::CommitUnknown, 5);
    }
    if planned > 0 && committed == planned {
        return if verified {
            (O::AppliedVerified, 0)
        } else {
            (O::AppliedUnverified, 6)
        };
    }
    if committed > 0 {
        return (O::PartialApplied, 4);
    }
    if dispatched > 0 {
        return if rejected == dispatched {
            (O::RejectedNoChange, 7)
        } else {
            (O::CommitUnknown, 5)
        };
    }
    if planned == 0 && preflight {
        return if verified {
            (O::NoChangeVerified, 0)
        } else {
            (O::NoChangeUnverified, 6)
        };
    }
    (
        O::Refused,
        if matches!(
            reason,
            Some(
                ApplyCode::TargetMismatch
                    | ApplyCode::NotAuthoritative
                    | ApplyCode::ConcurrentChange
                    | ApplyCode::LeaseLost
            )
        ) {
            3
        } else {
            2
        },
    )
}

/// A failed read restarts discovery, never resumes a partial inventory or mixes pages.
pub trait ApplyReader: Sync {
    fn complete(&self) -> impl Future<Output = ApplyResult<CompleteInventory>> + Send;
}
impl ApplyReader for super::administration::DirectAdminReader {
    async fn complete(&self) -> ApplyResult<CompleteInventory> {
        self.inventory(InventoryLimits::default())
            .await
            .map_err(|_| fail(ApplyCode::ReadIncomplete))
    }
}

#[derive(Clone, Serialize)]
pub struct VerificationBinding {
    pub target: TargetIdentity,
    pub plan: Digest,
    pub inventory: Digest,
    pub release: Digest,
}
/// No Deserialize or public successful constructor. Step 11 must validate real V1–V4 receipts.
pub struct VerifiedServing {
    pub(super) binding: Digest,
    pub(super) scope: EvidenceScope,
    pub(super) evidence: RestrictedJson,
}
pub trait ApplyVerifier: Sync {
    fn verify(
        &self,
        binding: &VerificationBinding,
    ) -> impl Future<Output = ApplyResult<VerifiedServing>> + Send;
}
pub struct UnavailableVerification;
impl ApplyVerifier for UnavailableVerification {
    async fn verify(&self, _: &VerificationBinding) -> ApplyResult<VerifiedServing> {
        Err(fail(ApplyCode::VerificationFailed))
    }
}
