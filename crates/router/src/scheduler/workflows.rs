use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum::EnumString;

use crate::{
    core::errors,
    routes::AppState,
    types::storage,
    utils::{OptionExt, StringExt},
};
#[cfg(feature = "email")]
pub mod api_key_expiry;

pub mod payment_sync;
pub mod refund_router;
pub mod tokenized_data;

macro_rules! runners {
    ($(#[$attr:meta] $body:tt),*) => {
        as_item! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, EnumString)]
            #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
            #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
            pub enum PTRunner {
                $(#[$attr] $body),*
            }
        }

        $( as_item! {
            #[$attr]
            pub struct $body;
        } )*


        pub fn runner_from_task(task: &storage::ProcessTracker) -> Result<Option<Box<dyn ProcessTrackerWorkflow>>, errors::ProcessTrackerError> {
            let runner = task.runner.clone().get_required_value("runner")?;
            let runner: Option<PTRunner> = runner.parse_enum("PTRunner").ok();
            Ok(match runner {
                $( #[$attr] Some( PTRunner::$body ) => {
                    Some(Box::new($body))
                } ,)*
                None => {
                    None
                }
            })

        }
    };
}
macro_rules! as_item {
    ($i:item) => {
        $i
    };
}

runners! {
    #[cfg(all())] PaymentsSyncWorkflow,
    #[cfg(all())] RefundWorkflowRouter,
    #[cfg(all())] DeleteTokenizeDataWorkflow,
    #[cfg(feature = "email")] ApiKeyExpiryWorkflow
}

pub type WorkflowSelectorFn =
    fn(
        &storage::ProcessTracker,
    ) -> Result<Option<Box<dyn ProcessTrackerWorkflow>>, errors::ProcessTrackerError>;

#[async_trait]
pub trait ProcessTrackerWorkflow: Send + Sync {
    // The core execution of the workflow
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a AppState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Err(errors::ProcessTrackerError::NotImplemented)?
    }
    // Callback function after successful execution of the `execute_workflow`
    async fn success_handler<'a>(
        &'a self,
        _state: &'a AppState,
        _process: storage::ProcessTracker,
    ) {
    }
    // Callback function after error received from `execute_workflow`
    async fn error_handler<'a>(
        &'a self,
        _state: &'a AppState,
        _process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        Err(errors::ProcessTrackerError::NotImplemented)?
    }
}

#[cfg(test)]
mod workflow_tests {
    #![allow(clippy::unwrap_used)]
    use super::PTRunner;
    use crate::utils::StringExt;

    #[test]
    fn test_enum_to_string() {
        let string_format = "PAYMENTS_SYNC_WORKFLOW".to_string();
        let enum_format: PTRunner = string_format.parse_enum("PTRunner").unwrap();
        assert_eq!(enum_format, PTRunner::PaymentsSyncWorkflow)
    }
}
