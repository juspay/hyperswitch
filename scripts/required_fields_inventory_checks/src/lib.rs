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

#[path = "../../../crates/router/src/core/required_fields_sync/administration.rs"]
pub mod administration;
#[path = "../../../crates/router/src/core/required_fields_sync/inventory.rs"]
pub mod inventory;
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

#[cfg(test)]
mod sdk_wire;
