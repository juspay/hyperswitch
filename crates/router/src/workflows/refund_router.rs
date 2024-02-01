use scheduler::consumer::workflows::ProcessTrackerWorkflow;

use crate::{
    core::refunds as refund_flow, errors, logger::error, routes::AppState, types::storage,
};

pub struct RefundWorkflowRouter;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for RefundWorkflowRouter {
        /// Asynchronously executes a workflow using the provided state and process tracker.
    ///
    /// # Arguments
    ///
    /// * `state` - The state of the application.
    /// * `process` - The process tracker for the workflow.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating success or a `ProcessTrackerError` if an error occurs during the execution.
    ///
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(Box::pin(refund_flow::start_refund_workflow(state, &process)).await?)
    }

    /// Asynchronously handles errors that occur during the execution of a workflow. It logs the process ID
    /// and a failure message using the error! macro, and then returns a custom result indicating success.
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
