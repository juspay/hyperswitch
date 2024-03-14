use async_trait::async_trait;
use common_utils::errors::CustomResult;
pub use diesel_models::process_tracker as storage;
use router_env::logger;

use crate::{errors, SchedulerAppState};

pub type WorkflowSelectorFn =
    fn(&storage::ProcessTracker) -> Result<(), errors::ProcessTrackerError>;

#[async_trait]
pub trait ProcessTrackerWorkflows<T>: Send + Sync {
    // The core execution of the workflow
    async fn trigger_workflow<'a>(
        &'a self,
        _state: &'a T,
        _process: storage::ProcessTracker,
    ) -> CustomResult<(), errors::ProcessTrackerError> {
        Err(errors::ProcessTrackerError::NotImplemented)?
    }

    async fn execute_workflow<'a>(
        &'a self,
        operation: Box<dyn ProcessTrackerWorkflow<T>>,
        state: &'a T,
        process: storage::ProcessTracker,
    ) -> CustomResult<(), errors::ProcessTrackerError>
    where
        T: SchedulerAppState,
    {
        let app_state = &state.clone();
        let output = operation.execute_workflow(app_state, process.clone()).await;
        match output {
            Ok(_) => operation.success_handler(app_state, process).await,
            Err(error) => match operation
                .error_handler(app_state, process.clone(), error)
                .await
            {
                Ok(_) => (),
                Err(error) => {
                    logger::error!(
                        ?error,
                        "Failed to handle process tracker workflow execution error"
                    );
                    let status = app_state
                        .get_db()
                        .as_scheduler()
                        .finish_process_with_business_status(process, "GLOBAL_FAILURE".to_string())
                        .await;
                    if let Err(error) = status {
                        logger::error!(?error, "Failed to update process business status");
                    }
                }
            },
        };
        Ok(())
    }
}

#[async_trait]
pub trait ProcessTrackerWorkflow<T>: Send + Sync {
    /// The core execution of the workflow
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a T,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Err(errors::ProcessTrackerError::NotImplemented)?
    }

    /// Callback function after successful execution of the `execute_workflow`
    async fn success_handler<'a>(&'a self, _state: &'a T, _process: storage::ProcessTracker) {}

    /// Callback function after error received from `execute_workflow`
    async fn error_handler<'a>(
        &'a self,
        _state: &'a T,
        _process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> CustomResult<(), errors::ProcessTrackerError> {
        Err(errors::ProcessTrackerError::NotImplemented)?
    }
}
