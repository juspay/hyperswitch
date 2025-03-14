use scheduler::consumer::workflows::ProcessTrackerWorkflow;

#[cfg(feature = "v1")]
use crate::core::refunds as refund_flow;
use crate::{errors, logger::error, routes::SessionState, types::storage};

pub struct RefundWorkflowRouter;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for RefundWorkflowRouter {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(Box::pin(refund_flow::start_refund_workflow(state, &process)).await?)
    }
    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        todo!()
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
