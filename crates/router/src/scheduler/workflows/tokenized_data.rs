use super::{DeleteTokenizeDataWorkflow, ProcessTrackerWorkflow};
#[cfg(feature = "basilisk")]
use crate::core::payment_methods::vault;
use crate::{errors, logger::error, routes::AppState, types::storage};

#[async_trait::async_trait]
impl ProcessTrackerWorkflow for DeleteTokenizeDataWorkflow {
    #[cfg(feature = "basilisk")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(vault::start_tokenize_data_workflow(state, &process).await?)
    }

    #[cfg(not(feature = "basilisk"))]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a AppState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(())
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
