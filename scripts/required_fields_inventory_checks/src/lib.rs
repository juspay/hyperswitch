//! Isolated compile/test boundary importing production code, never replacement implementations.
//! This does not compile router Settings/AppState or certify any HTTP server/target.

#[path = "../../../crates/payment_methods/src/configs/required_fields_renderer/artifacts.rs"]
pub mod foundations;
pub use foundations::*;

#[path = "../../../crates/router/src/configs/cli.rs"]
pub mod cli;
pub mod configs {
    pub use crate::cli;
}
#[cfg(feature = "required_fields_sync")]
#[path = "../../../crates/router/src/core/required_fields_sync/command.rs"]
#[allow(
    dead_code,
    reason = "production Settings/reader adapter is compiled by the actual router build, not this isolated boundary"
)]
pub mod command;

#[cfg(feature = "required_fields_sync")]
#[path = "../../../crates/router/src/core/required_fields_sync/seed.rs"]
pub mod seed;
#[cfg(feature = "required_fields_sync")]
#[path = "../../../crates/router/src/core/required_fields_sync/seed_command.rs"]
#[allow(
    dead_code,
    reason = "actual binary dispatch is checked in the router build"
)]
pub mod seed_command;

#[cfg(feature = "required_fields_sync")]
#[path = "../../../crates/router/src/core/required_fields_sync/attestation.rs"]
pub mod attestation;
#[cfg(feature = "required_fields_sync")]
#[path = "../../../crates/router/src/core/required_fields_sync/operation.rs"]
#[allow(
    dead_code,
    reason = "actual binary adapter is checked in the router build"
)]
pub mod operation;
#[cfg(feature = "required_fields_sync")]
#[path = "../../../crates/router/src/core/required_fields_sync/operation_executor.rs"]
#[allow(
    dead_code,
    reason = "process observation is called by the actual router runtime"
)]
pub mod operation_executor;

#[path = "../../../crates/router/src/core/required_fields_sync/administration.rs"]
pub mod administration;
#[path = "../../../crates/router/src/core/required_fields_sync/apply.rs"]
pub mod apply;
#[path = "../../../crates/router/src/core/required_fields_sync/apply_contract.rs"]
pub mod apply_contract;
#[path = "../../../crates/router/src/core/required_fields_sync/apply_observation.rs"]
pub mod apply_observation;
#[path = "../../../crates/router/src/core/required_fields_sync/authority.rs"]
pub mod authority;
#[path = "../../../crates/router/src/core/required_fields_sync/coordination.rs"]
pub mod coordination;
#[path = "../../../crates/router/src/core/required_fields_sync/inventory.rs"]
pub mod inventory;
#[path = "../../../crates/router/src/core/required_fields_sync/journal.rs"]
pub mod journal;
#[path = "../../../crates/router/src/core/required_fields_sync/journal_snapshot.rs"]
mod journal_snapshot;
#[path = "../../../crates/router/src/core/required_fields_sync/kubernetes_coordination.rs"]
pub mod kubernetes_coordination;
#[path = "../../../crates/router/src/core/required_fields_sync/mutation.rs"]
pub mod mutation;
#[path = "../../../crates/router/src/core/required_fields_sync/raw_json.rs"]
mod raw_json;
#[path = "../../../crates/router/src/core/required_fields_sync/types.rs"]
pub mod types;

#[path = "../../../crates/router/src/core/required_fields_sync/impact.rs"]
pub mod impact;
#[path = "../../../crates/router/src/core/required_fields_sync/ownership.rs"]
pub mod ownership;
#[path = "../../../crates/router/src/core/required_fields_sync/plan.rs"]
pub mod plan;
#[path = "../../../crates/router/src/core/required_fields_sync/planner_types.rs"]
pub mod planner_types;
#[path = "../../../crates/router/src/core/required_fields_sync/preflight.rs"]
pub mod preflight;
#[path = "../../../crates/router/src/core/required_fields_sync/recovery.rs"]
pub mod recovery;
#[path = "../../../crates/router/src/core/required_fields_sync/recovery_data.rs"]
mod recovery_data;

#[cfg(test)]
mod mutation_wire;
#[cfg(test)]
mod sdk_wire;

#[path = "../../../crates/router/src/core/required_fields_sync/verification.rs"]
pub mod verification;
#[path = "../../../crates/router/src/core/required_fields_sync/verification_provider.rs"]
pub mod verification_provider;

#[path = "../../../crates/router/src/core/required_fields_sync/verification_browser.rs"]
pub mod verification_browser;

#[path = "../../../crates/router/src/core/required_fields_sync/verification_http.rs"]
pub mod verification_http;
