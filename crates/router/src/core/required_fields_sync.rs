//! Foundation boundaries only. No inventory, plan construction, CLI dispatch or mutation adapter.

use std::path::PathBuf;

use error_stack::ResultExt;
use hyperswitch_interfaces::secrets_interface::secret_state::SecuredSecret;
use payment_methods::configs::required_fields_renderer::{self as renderer, *};
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings::Settings,
    core::errors::{ApplicationError, ApplicationResult},
};

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
