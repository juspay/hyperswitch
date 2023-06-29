use async_trait::async_trait;
use common_utils::errors::CustomResult;

use crate::{errors};
pub use storage_models::process_tracker as storage;

// macro_rules! runners {
//     ($($body:tt),*) => {
//         as_item! {
//             #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, EnumString)]
//             #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
//             #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
//             pub enum PTRunner {
//                 $($body),*
//             }
//         }

//         $( as_item! {
//             pub struct $body;
//         } )*


//         pub fn runner_from_task<T>(task: &storage::ProcessTracker) -> Result<Option<Box<dyn ProcessTrackerWorkflow<T>>>, errors::ProcessTrackerError> where T : Sync + Send + Clone, PaymentsSyncWorkflow: ProcessTrackerWorkflow<T> {
//             let runner = task.runner.clone().get_required_value("runner")?;
//             let runner: Option<PTRunner> = runner.parse_enum("PTRunner").ok();
//             Ok(match runner {
//                 $( Some( PTRunner::$body ) => {
//                     Some(Box::new($body))
//                 } ,)*
//                 None => {
//                     None
//                 }
//             })

//         }
//     };
// }
// macro_rules! as_item {
//     ($i:item) => {
//         $i
//     };
// }

// runners! {
//     PaymentsSyncWorkflow
// }

pub type WorkflowSelectorFn<T> =
    fn(
        &storage::ProcessTracker,
    ) -> Result<Option<Box<dyn ProcessTrackerWorkflow<T>>>, errors::ProcessTrackerError>;

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
    async fn success_handler<'a>(
        &'a self,
        _state: &'a T,
        _process: storage::ProcessTracker,
    ) {
    }
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
