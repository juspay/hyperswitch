use scheduler::consumer::workflows::ProcessTrackerWorkflow;

use crate::{
    core::refunds as refund_flow, errors, logger::error, routes::AppState, types::storage,
};

pub struct RefundWorkflowRouter;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for RefundWorkflowRouter {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(Box::pin(refund_flow::start_refund_workflow(state, &process)).await?)
    }

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
