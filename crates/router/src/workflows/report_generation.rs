use scheduler::consumer::workflows::ProcessTrackerWorkflow;

use crate::{
   errors, logger::error, routes::AppState, types::storage,
};

pub struct ReportingWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for ReportingWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a AppState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        //todo
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
