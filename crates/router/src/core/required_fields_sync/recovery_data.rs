//! Restricted durable evidence decoding; never manufactures a complete live inventory.
use std::collections::BTreeMap;

use serde::de::DeserializeOwned;

use super::{
    apply_contract::{checked, fail, ApplyCode, ApplyResult, CertifiedMutationTarget},
    journal::{PreparedSnapshot, RestrictedJson, SecretGuard},
    planner_types::{ActionSummary, PlannedAction},
    raw_json::Json,
    types::TargetIdentity,
    Digest,
};

pub(super) fn decode<T: DeserializeOwned>(value: &Json) -> ApplyResult<T> {
    checked(serde_json::from_str(value.raw()))
}
pub(super) fn raw(value: &Json) -> ApplyResult<Json> {
    checked(Json::parse(checked(value.string())?.as_bytes()))
}
pub(super) fn same(a: Option<&Json>, b: Option<&Json>) -> ApplyResult<bool> {
    Ok(a.map(|v| checked(v.canonical())).transpose()?
        == b.map(|v| checked(v.canonical())).transpose()?)
}
pub(super) struct Images {
    pub contexts: BTreeMap<Vec<u8>, Json>,
    pub defaults: BTreeMap<String, Json>,
    pub dimensions: BTreeMap<String, Json>,
    pub experiments: BTreeMap<String, Json>,
    pub groups: BTreeMap<String, Json>,
    pub workspace: Json,
    pub proof: Json,
}
impl Images {
    pub fn parse(images: &RestrictedJson) -> ApplyResult<Self> {
        let root = checked(Json::parse(images.bytes()))?;
        let mut contexts = BTreeMap::new();
        for value in checked(checked(root.get("contexts"))?.array())? {
            let row = raw(value)?;
            let key = checked(checked(row.get("value"))?.canonical())?;
            if contexts.insert(key, row).is_some() {
                return Err(fail(ApplyCode::ConcurrentChange));
            }
        }
        let rows = |label: &str, identity: &str| -> ApplyResult<BTreeMap<String, Json>> {
            let mut result = BTreeMap::new();
            for value in checked(checked(root.get(label))?.array())? {
                let row = raw(value)?;
                let key = checked(checked(row.get(identity))?.string())?;
                if result.insert(key, row).is_some() {
                    return Err(fail(ApplyCode::ConcurrentChange));
                }
            }
            Ok(result)
        };
        Ok(Self {
            contexts,
            defaults: rows("defaults", "key")?,
            dimensions: rows("dimensions", "dimension")?,
            experiments: rows("experiments", "id")?,
            groups: rows("experiment_groups", "id")?,
            workspace: raw(checked(root.get("workspace"))?)?,
            proof: checked(root.get("proof"))?.clone(),
        })
    }
    pub fn validate(
        &self,
        target: &CertifiedMutationTarget,
        inventory: &Digest,
    ) -> ApplyResult<()> {
        let evidence = checked(self.proof.get("evidence"))?;
        if decode::<TargetIdentity>(checked(self.proof.get("target"))?)?
            != target.registration.target
            || decode::<Digest>(checked(self.proof.get("inventory_hash"))?)? != *inventory
            || decode::<super::Name>(checked(evidence.get("contract_id"))?)? != target.read_contract
            || decode::<Digest>(checked(evidence.get("server_image"))?)? != target.server_image
            || decode::<super::Revision>(checked(evidence.get("server_source"))?)?
                != target.server_source
            || checked(checked(evidence.get("scope"))?.canonical())?
                != checked(checked(super::planner_types::json(&target.scope))?.canonical())?
            || checked(checked(self.proof.get("identical_full_passes"))?.unsigned())? != 2
        {
            return Err(fail(ApplyCode::TargetMismatch));
        }
        Ok(())
    }
}
pub(super) fn before(
    snapshot: &PreparedSnapshot,
    guard: &SecretGuard,
) -> ApplyResult<RestrictedJson> {
    let contents = checked(Json::parse(snapshot.contents().bytes()))?;
    let mut rows = checked(checked(contents.get("rows"))?.object())?.clone();
    rows.insert(
        "workspace".into(),
        checked(contents.get("workspace"))?.clone(),
    );
    rows.insert("proof".into(), checked(contents.get("proof"))?.clone());
    checked(RestrictedJson::parse(
        checked(Json::members(&rows))?.raw().as_bytes(),
        guard,
    ))
}
pub(super) fn actions(snapshot: &PreparedSnapshot) -> ApplyResult<Vec<PlannedAction>> {
    let contents = checked(Json::parse(snapshot.contents().bytes()))?;
    checked(checked(contents.get("actions"))?.array())?
        .iter()
        .map(|row| {
            let summary: ActionSummary = decode(checked(row.get("summary"))?)?;
            let image = |name: &str| -> ApplyResult<Option<Json>> {
                let v = checked(row.get(name))?;
                if v.null() {
                    Ok(None)
                } else {
                    raw(v).map(Some)
                }
            };
            let action = PlannedAction {
                kind: summary.kind.clone(),
                server_id: decode(checked(row.get("server_id"))?)?,
                condition: decode(checked(row.get("condition"))?)?,
                before: image("before")?,
                after: image("after")?,
                preservation_hash: summary.preserved_hash.clone(),
                ownership_intent: summary.ownership_intent.clone(),
            };
            if super::planner_types::digest(&checked(action.summary())?)
                .map_err(|_| fail(ApplyCode::InvalidInput))?
                != checked(super::planner_types::digest(&summary))?
            {
                return Err(fail(ApplyCode::InvalidInput));
            }
            Ok(action)
        })
        .collect()
}
