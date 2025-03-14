use scheduler::consumer::workflows::ProcessTrackerWorkflow;

#[cfg(feature = "v1")]
use crate::core::payment_methods::vault;
use crate::{errors, logger::error, routes::SessionState, types::storage};

pub struct DeleteTokenizeDataWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for DeleteTokenizeDataWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(vault::start_tokenize_data_workflow(state, &process).await?)
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
