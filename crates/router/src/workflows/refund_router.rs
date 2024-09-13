use scheduler::consumer::workflows::ProcessTrackerWorkflow;

use crate::{
    core::refunds as refund_flow, errors, logger::error, routes::SessionState, types::storage,
};

pub struct RefundWorkflowRouter;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for RefundWorkflowRouter {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(Box::pin(refund_flow::start_refund_workflow(state, &process)).await?)
    }

    async fn error_handler<'a>(
        &'a self,
        _state: &'a SessionState,
        process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        error!(%process.id, "Failed while executing workflow");
        Ok(())
    }
}
