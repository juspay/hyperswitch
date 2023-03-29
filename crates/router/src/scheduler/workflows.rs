use async_trait::async_trait;
use router_env::{instrument, tracing};
use serde::{Deserialize, Serialize};
use strum::EnumString;

use crate::{core::errors, routes::AppState, scheduler::consumer, types::storage};
pub mod payment_sync;
pub mod refund_router;
pub mod tokenized_data;

macro_rules! runners {
    ($($body:tt),*) => {
        as_item! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, EnumString)]
            #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
            #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
            pub enum PTRunner {
                $($body),*
            }
        }

        $( as_item! {
            pub struct $body;
        } )*

        #[instrument(skip(state))]
        pub async fn perform_workflow_execution<'a>(state: &AppState, process: storage::ProcessTracker, runner: PTRunner)
        where
        {
            match runner {
                $( PTRunner::$body => {
                    let flow = &$body;
                    consumer::run_executor(state, process, flow).await

                } ,)*
            }
        }
    };
}
macro_rules! as_item {
    ($i:item) => {
        $i
    };
}

runners! {
    PaymentsSyncWorkflow,
    RefundWorkflowRouter,
    DeleteTokenizeDataWorkflow
}

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
