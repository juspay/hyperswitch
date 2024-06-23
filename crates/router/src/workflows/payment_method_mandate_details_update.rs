use error_stack::ResultExt;
use scheduler::workflows::ProcessTrackerWorkflow;

use crate::{errors, routes::SessionState, types::storage};

pub struct PaymentMethodMandateDetailsUpdateWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodMandateDetailsUpdateWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(())
    }
}
