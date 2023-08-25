use common_utils::ext_traits::ValueExt;
use diesel_models::enums::{self as storage_enums};

use super::{ProcessTrackerWorkflow,AutoRefundWorkflow};
use crate::{
    errors, logger::error, routes::AppState, types::storage,
};

#[async_trait::async_trait]
impl ProcessTrackerWorkflow for AutoRefundWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(refund_flow::start_refund_workflow(state, &process).await?)
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
