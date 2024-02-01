use scheduler::consumer::workflows::ProcessTrackerWorkflow;

use crate::{
    core::payment_methods::vault, errors, logger::error, routes::AppState, types::storage,
};

pub struct DeleteTokenizeDataWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for DeleteTokenizeDataWorkflow {
        /// Asynchronously executes a workflow by starting the tokenization process for the given state and process.
    /// 
    /// # Arguments
    /// 
    /// * `state` - The application state to be used in the workflow execution.
    /// * `process` - The storage process tracker for the workflow.
    /// 
    /// # Returns
    /// 
    /// * `Result<(), errors::ProcessTrackerError>` - A result indicating success or an error of type ProcessTrackerError.
    /// 
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(vault::start_tokenize_data_workflow(state, &process).await?)
    }

        /// This method handles errors that occur during the execution of a workflow. It logs the process ID
    /// that failed and the reason for the failure, and then returns an Ok result.
    async fn error_handler<'a>(
        &'a self,
        _state: &'a AppState,
        process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        error!(%process.id, "Failed while executing workflow");
        Ok(())
    }
}
