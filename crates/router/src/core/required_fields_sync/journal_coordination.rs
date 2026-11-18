//! Sealed bridge: coordinator inputs come from the successfully persisted journal handle.
use serde::de::DeserializeOwned;

use super::{
    super::{
        authority::{ReleaseIntent, TargetRegistration},
        canonical_hash, HashDomain,
    },
    *,
};

pub struct JournalCheckpoint {
    pub(crate) record: UnresolvedRunRecord,
    identity: RecoveryIdentity,
    contents: RestrictedJson,
    store: Digest,
    synthetic: bool,
}
impl<S: ImmutableJournalStore> RunJournal<'_, S> {
    pub fn checkpoint(&self) -> JournalResult<JournalCheckpoint> {
        if self.poisoned {
            return Err(fail(JournalCode::UnknownStorageOutcome));
        }
        Ok(JournalCheckpoint {
            record: self.unresolved(),
            identity: self.recovery_identity.clone(),
            contents: self.prepared_contents.clone(),
            store: self.certificate.id.clone(),
            synthetic: self.certificate.synthetic,
        })
    }
}
fn parsed<T: DeserializeOwned>(value: &Json) -> JournalResult<T> {
    serde_json::from_str(value.raw()).map_err(|_| fail(JournalCode::BindingMismatch))
}
impl JournalCheckpoint {
    pub(crate) fn resolves(
        &self,
        previous: &UnresolvedRunRecord,
        resolved: &ResolvedIntent,
    ) -> JournalResult<()> {
        let receipt = &resolved.receipt;
        let expected: Vec<_> = previous
            .pending_requests
            .iter()
            .filter(|r| *r != &resolved.request)
            .cloned()
            .collect();
        if encode(&self.record.locator)? != encode(&previous.locator)?
            || self.record.first_adoption != previous.first_adoption
            || !previous.pending_requests.contains(&resolved.request)
            || self.record.pending_requests != expected
            || self.record.last_sequence <= previous.last_sequence
            || receipt.store != self.store
            || receipt.synthetic != self.synthetic
            || self.record.last_event_hash.as_ref() != Some(&receipt.reference.hash)
            || receipt.reference.key
                != event_key(
                    &self.record.locator,
                    self.record
                        .last_sequence
                        .ok_or_else(|| fail(JournalCode::BindingMismatch))?,
                )?
        {
            return Err(fail(JournalCode::BindingMismatch));
        }
        Ok(())
    }
    pub(crate) fn matches(
        &self,
        target: &TargetRegistration,
        run: &Digest,
        intent: &ReleaseIntent,
        plan: &Digest,
        synthetic: bool,
    ) -> JournalResult<()> {
        let bad = || fail(JournalCode::BindingMismatch);
        if self.synthetic != synthetic
            || self.store != target.journal_contract
            || self.record.locator.run != *run
            || self.record.locator.target != target.target
            || self.identity.target != target.target
            || self.identity.endpoint_identity != target.endpoint_identity
            || self.identity.coordinator != target.coordinator.object
            || self.identity.executor_image != intent.executor_image
            || !self.record.blocks_new_execution
        {
            return Err(bad());
        }
        let contents = Json::parse(self.contents.bytes()).map_err(|_| bad())?;
        let report = contents.get("plan").map_err(|_| bad())?;
        let inputs = report.get("inputs").map_err(|_| bad())?;
        let input_intent = inputs.get("intent").map_err(|_| bad())?;
        let bundle_text: String = parsed(contents.get("bundle").map_err(|_| bad())?)?;
        let bundle = Json::parse(bundle_text.as_bytes()).map_err(|_| bad())?;
        let manifest = bundle
            .get("desired")
            .and_then(|d| d.get("input_manifest"))
            .map_err(|_| bad())?;
        let build = manifest.get("build").map_err(|_| bad())?;
        let release = build.get("release").map_err(|_| bad())?;
        let feature_keys: Vec<Name> = parsed(build.get("router_feature_keys").map_err(|_| bad())?)?;
        let artifact: super::super::ArtifactIdentity =
            parsed(release.get("artifact").map_err(|_| bad())?)?;
        let image_matches = match artifact {
            super::super::ArtifactIdentity::Image {
                index_digest,
                platform_digests,
            } => {
                index_digest == intent.executor_image
                    || platform_digests
                        .values()
                        .any(|image| image == &intent.executor_image)
            }
            super::super::ArtifactIdentity::Offline => synthetic,
        };
        let manifest_value: serde_json::Value = parsed(manifest)?;
        if parsed::<Digest>(report.get("plan_hash").map_err(|_| bad())?)? != *plan
            || parsed::<Digest>(inputs.get("bundle_hash").map_err(|_| bad())?)? != intent.bundle
            || parsed::<Digest>(input_intent.get("release_hash").map_err(|_| bad())?)?
                != intent.release
            || parsed::<u64>(input_intent.get("generation").map_err(|_| bad())?)?
                != intent.generation
            || parsed::<Name>(input_intent.get("authority").map_err(|_| bad())?)?
                != intent.authority
            || canonical_hash(HashDomain::InputManifest, &manifest_value).map_err(|_| bad())?
                != intent.input_manifest
            || super::super::planner_types::hash(build).map_err(|_| bad())? != intent.build
            || parsed::<Digest>(manifest.get("mapping_hash").map_err(|_| bad())?)? != intent.policy
            || parsed::<Digest>(manifest.get("catalog_hash").map_err(|_| bad())?)? != intent.catalog
            || parsed::<Digest>(manifest.get("effective_settings_hash").map_err(|_| bad())?)?
                != intent.settings
            || parsed::<super::super::Revision>(
                release.get("application_revision").map_err(|_| bad())?,
            )? != intent.source
            || feature_keys != intent.feature_keys
            || !image_matches
        {
            return Err(bad());
        }
        Ok(())
    }
    pub(crate) fn follows(&self, previous: &UnresolvedRunRecord) -> JournalResult<()> {
        if encode(&self.record.locator)? != encode(&previous.locator)?
            || self.record.first_adoption != previous.first_adoption
            || self.record.last_sequence
                != Some(previous.last_sequence.map_or(0, |sequence| sequence + 1))
            || self.record.pending_requests.len() != previous.pending_requests.len() + 1
            || !self
                .record
                .pending_requests
                .starts_with(&previous.pending_requests)
        {
            return Err(fail(JournalCode::BindingMismatch));
        }
        Ok(())
    }
    pub(crate) fn matches_intent(&self, intent: &RecordedIntent) -> JournalResult<()> {
        let receipt = intent.receipt();
        if receipt.store != self.store
            || receipt.synthetic != self.synthetic
            || self.record.last_event_hash.as_ref() != Some(&receipt.reference.hash)
            || self.record.pending_requests.last() != Some(&intent.request().hash()?)
            || receipt.reference.key
                != event_key(
                    &self.record.locator,
                    self.record
                        .last_sequence
                        .ok_or_else(|| fail(JournalCode::BindingMismatch))?,
                )?
        {
            return Err(fail(JournalCode::BindingMismatch));
        }
        Ok(())
    }
}
