//! Required-fields executor. Production execution remains unregistered and disabled.

pub mod administration;
pub mod apply;
pub mod apply_contract;
pub mod apply_observation;
pub mod attestation;
pub mod authority;
pub mod command;
pub mod command_runtime;
pub mod coordination;
pub mod impact;
pub mod inventory;
pub mod journal;
mod journal_snapshot;
pub mod kubernetes_coordination;
pub mod mutation;
pub mod operation;
pub mod operation_executor;
pub mod ownership;
pub mod plan;
pub mod planner_types;
pub mod preflight;
mod raw_json;
pub mod recovery;
mod recovery_data;
pub mod seed;
mod seed_command;
pub mod types;
pub mod verification;
pub mod verification_browser;
pub mod verification_http;
pub mod verification_provider;

use std::path::PathBuf;

use error_stack::ResultExt;
use hyperswitch_interfaces::secrets_interface::secret_state::SecuredSecret;
use payment_methods::configs::required_fields_renderer::{self as renderer, *};
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings::Settings,
    core::errors::{ApplicationError, ApplicationResult},
};

/// Re-run the pure renderer before exposing inputs to the planner. Public bundle fields cannot
/// smuggle an unreviewed desired map or a different catalog under a caller-supplied hash.
pub fn verified_desired(
    bundle: &RenderedBundle,
) -> planner_types::PlanResult<planner_types::VerifiedDesired> {
    let verified = render(RenderInputs {
        effective: &bundle.inputs,
        policy: &bundle.policy,
        fields: &bundle.catalog,
        exclusions: &bundle.exclusions,
        build: &bundle.desired.input_manifest.build,
    })
    .map_err(|_| planner_types::refuse(planner_types::PlanCode::InvalidInput))?;
    let hash = verified
        .hash()
        .map_err(|_| planner_types::refuse(planner_types::PlanCode::InvalidInput))?;
    if bundle
        .hash()
        .map_err(|_| planner_types::refuse(planner_types::PlanCode::InvalidInput))?
        != hash
    {
        return Err(planner_types::refuse(
            planner_types::PlanCode::BindingMismatch,
        ));
    }
    let build = planner_types::json(&verified.desired.input_manifest.build)?;
    let availability = raw_json::Json::members(&std::collections::BTreeMap::from([
        (
            "router_feature_keys".to_owned(),
            planner_types::checked(build.get("router_feature_keys"))?.clone(),
        ),
        (
            "payment_methods_feature_keys".to_owned(),
            planner_types::checked(build.get("payment_methods_feature_keys"))?.clone(),
        ),
    ]));
    Ok(planner_types::VerifiedDesired {
        desired: verified.desired.clone(),
        catalog: verified.catalog,
        bundle_hash: hash,
        release_hash: planner_types::hash(planner_types::checked(
            planner_types::json(&verified.desired.input_manifest.build)?.get("release"),
        )?)?,
        source_availability_hash: planner_types::hash(&planner_types::checked(availability)?)?,
        excluded_connectors: verified
            .exclusions
            .sources
            .into_iter()
            .map(|v| v.source.connector)
            .collect(),
    })
}

/// Uses the ordinary file/default/ROUTER__ precedence without validation/decryption or AppState.
pub fn load_effective_inputs(config_path: Option<PathBuf>) -> ApplicationResult<EffectiveInputs> {
    Settings::<SecuredSecret>::with_config_path(config_path)?
        .required_fields_inputs()
        .change_context(ApplicationError::ConfigurationError)
}

pub fn build_identity(release: ReleaseIdentity) -> InputResult<BuildIdentity> {
    BuildIdentity::from_feature_keys(
        release,
        env!("REQUIRED_FIELDS_FEATURE_KEYS"),
        renderer::compiled_feature_keys(),
    )
}

/// Identity required by a future plan, not an executable request or authority grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanBinding {
    pub schema_version: u16,
    pub target_id: Name,
    pub org: Name,
    pub workspace: Name,
    pub instance_generation: u64,
    pub authoritative_release: Digest,
    pub intent_generation: u64,
    pub bundle_hash: Digest,
    pub inventory_hash: Digest,
    pub ownership_hash: Digest,
    pub server_contract_id: Name,
    pub sdk_verification_manifest_hash: Digest,
}

/// Step 3 can report input identity only; no successful apply/verification outcome exists here.
#[derive(Debug, Clone, Serialize)]
pub struct FoundationReport {
    pub schema_version: u16,
    pub input_manifest: InputManifest,
    pub input_manifest_hash: Digest,
    pub hosted_apply: HostedApply,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedApply {
    #[default]
    Disabled,
}

impl FoundationReport {
    pub fn new(input_manifest: InputManifest) -> InputResult<Self> {
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            input_manifest_hash: canonical_hash(HashDomain::InputManifest, &input_manifest)?,
            input_manifest,
            hosted_apply: HostedApply::Disabled,
        })
    }
}

#[cfg(test)]
#[path = "required_fields_sync/tests.rs"]
mod tests;
