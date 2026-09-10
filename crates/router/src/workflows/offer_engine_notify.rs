use scheduler::consumer::workflows::ProcessTrackerWorkflow;

use crate::{
    core::offer_engine::notify, errors, logger::error, routes::SessionState, types::storage,
};

pub struct OfferEngineNotifyWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for OfferEngineNotifyWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Box::pin(notify::execute_notification(state, process)).await
    }

    async fn error_handler<'a>(
        &'a self,
        _state: &'a SessionState,
        process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        error!(%process.id, "Failed while executing offer engine notify workflow");
        Ok(())
    }
}
