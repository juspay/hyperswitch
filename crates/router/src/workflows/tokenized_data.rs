use scheduler::consumer::workflows::ProcessTrackerWorkflow;

use crate::{
    core::payment_methods::vault, errors, logger::error, routes::AppState, types::storage,
};

pub struct DeleteTokenizeDataWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for DeleteTokenizeDataWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(vault::start_tokenize_data_workflow(state, &process).await?)
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
