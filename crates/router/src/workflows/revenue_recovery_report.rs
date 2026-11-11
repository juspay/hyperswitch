//! The recovery report workflow, which records a transaction the merchant reported on a profile
//! whose retries it triggers, decides the next retry of its invoice, and tells the merchant by
//! webhook.

use common_utils::errors::CustomResult;
#[cfg(feature = "v2")]
use common_utils::ext_traits::ValueExt;
#[cfg(feature = "v2")]
use router_env::logger;
#[cfg(feature = "v1")]
use scheduler::consumer;
use scheduler::{consumer::workflows::ProcessTrackerWorkflow, errors};

#[cfg(feature = "v2")]
use crate::{core::revenue_recovery::external_triggers, types::storage::revenue_recovery as pcr};
use crate::{routes::SessionState, types::storage};

pub struct RevenueRecoveryReportWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for RevenueRecoveryReportWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = match process
            .tracking_data
            .clone()
            .parse_value::<pcr::RecoveryReportTrackingData>("RecoveryReportTrackingData")
        {
            Ok(tracking_data) => tracking_data,
            // Data that cannot be read now cannot be read on a later run either.
            Err(error) => {
                logger::error!(
                    ?error,
                    process_id = %process.id,
                    "The recovery report's tracking data cannot be read"
                );
                return external_triggers::finish_failed_report(state, &process).await;
            }
        };
        Box::pin(external_triggers::process_reported_transaction(
            state,
            &process,
            tracking_data,
        ))
        .await
    }

    #[cfg(feature = "v1")]
    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }

    /// The workflow returns an error only for a temporary one, so the report runs again.
    #[cfg(feature = "v2")]
    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> CustomResult<(), errors::ProcessTrackerError> {
        external_triggers::retry_report(state, process, error).await
    }
}
