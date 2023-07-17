use async_trait::async_trait;
use common_utils::errors::CustomResult;
pub use diesel_models::process_tracker as storage;

use crate::errors;

pub type WorkflowSelectorFn =
    fn(&storage::ProcessTracker) -> Result<(), errors::ProcessTrackerError>;

#[async_trait]
pub trait ProcessTrackerWorkflows<T>: Send + Sync {
    // The core execution of the workflow
    async fn trigger_workflow<'a>(
        &'a self,
        _state: &'a T,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Err(errors::ProcessTrackerError::NotImplemented)?
    }
}

#[async_trait]
pub trait ProcessTrackerWorkflow<T>: Send + Sync {
    // The core execution of the workflow
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a T,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Err(errors::ProcessTrackerError::NotImplemented)?
    }
    // Callback function after successful execution of the `execute_workflow`
    async fn success_handler<'a>(&'a self, _state: &'a T, _process: storage::ProcessTracker) {}
    // Callback function after error received from `execute_workflow`
    async fn error_handler<'a>(
        &'a self,
        _state: &'a T,
        _process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> CustomResult<(), errors::ProcessTrackerError> {
        Err(errors::ProcessTrackerError::NotImplemented)?
    }
}

// #[cfg(test)]
// mod workflow_tests {
//     #![allow(clippy::unwrap_used)]
//     use common_utils::ext_traits::StringExt;

//     use super::PTRunner;

//     #[test]
//     fn test_enum_to_string() {
//         let string_format = "PAYMENTS_SYNC_WORKFLOW".to_string();
//         let enum_format: PTRunner = string_format.parse_enum("PTRunner").unwrap();
//         assert_eq!(enum_format, PTRunner::PaymentsSyncWorkflow)
//     }
// }
