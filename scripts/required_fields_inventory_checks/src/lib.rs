//! Isolated compile/test boundary importing production code, never replacement implementations.
//! This does not compile router Settings/AppState or certify any HTTP server/target.

#[path = "../../../crates/payment_methods/src/configs/required_fields_renderer/artifacts.rs"]
pub mod foundations;
pub use foundations::{Digest, Name, Revision};

#[path = "../../../crates/router/src/core/required_fields_sync/administration.rs"]
pub mod administration;
#[path = "../../../crates/router/src/core/required_fields_sync/inventory.rs"]
pub mod inventory;
#[path = "../../../crates/router/src/core/required_fields_sync/raw_json.rs"]
mod raw_json;
#[path = "../../../crates/router/src/core/required_fields_sync/types.rs"]
pub mod types;

#[cfg(test)]
mod sdk_wire;
