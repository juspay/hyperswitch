//! Nonsecret observations, never proof of a running image or deployment authority by themselves.
use serde::{Deserialize, Serialize};

use super::{Digest, Name};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Serving never executes synchronization, regardless of this opt-in executor policy.
    pub mode: super::operation::Mode,
    /// Optional immutable profile for desired-state observation from this pod's actual inputs.
    pub attestation_profile: Option<std::path::PathBuf>,
}

#[derive(Serialize)]
pub struct ReleaseObservation {
    pub schema_version: u16,
    pub router_features: Vec<Name>,
    pub payment_methods_features: Vec<Name>,
    pub effective_settings: Digest,
    pub retained_rules: Digest,
    pub profile: Option<Digest>,
    pub build: Option<Digest>,
    pub input_manifest: Option<Digest>,
    pub desired: Option<Digest>,
    pub bundle: Option<Digest>,
    pub desired_state: &'static str,
    pub image_provenance: &'static str,
    pub application_rollout: &'static str,
    pub synchronization: &'static str,
    pub verification: &'static str,
}

/// Exact feature closure is observed from Cargo, not supplied by configuration or a version tag.
pub fn features(value: &str) -> super::command::CommandResult<Vec<Name>> {
    let values = value
        .split(',')
        .map(|v| v.to_owned().try_into())
        .collect::<Result<Vec<Name>, _>>()
        .map_err(|_| {
            super::command::fail(
                super::command::CommandCode::UnsupportedBuild,
                super::command::Phase::Input,
            )
        })?;
    if values.windows(2).any(|v| v.first() >= v.get(1))
        || !values.iter().any(|v| v.as_str() == "V1")
        || !values.iter().any(|v| v.as_str() == "REQUIRED_FIELDS_SYNC")
        || values.iter().any(|v| v.as_str() == "V2")
    {
        return Err(super::command::fail(
            super::command::CommandCode::UnsupportedBuild,
            super::command::Phase::Input,
        ));
    }
    Ok(values)
}
