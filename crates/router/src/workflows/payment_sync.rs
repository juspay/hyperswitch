use common_utils::ext_traits::{OptionExt, StringExt, ValueExt};
use error_stack::ResultExt;
use router_env::logger;
use scheduler::{
    consumer::{self, types::process_data, workflows::ProcessTrackerWorkflow},
    db::process_tracker::ProcessTrackerExt,
    errors as sch_errors, utils, SchedulerAppState,
};

use crate::{
    core::payments::{self as payment_flows, operations},
    db::StorageInterface,
    errors,
    routes::AppState,
    services,
    types::{
        api,
        handler::Oss,
        storage::{self, enums},
    },
};

pub struct PaymentsSyncWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for PaymentsSyncWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        let db: &dyn StorageInterface = &*state.store;
        let tracking_data: api::PaymentsRetrieveRequest = process
            .tracking_data
            .clone()
            .parse_value("PaymentsRetrieveRequest")?;

        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                tracking_data
                    .merchant_id
                    .as_ref()
                    .get_required_value("merchant_id")?,
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(
                tracking_data
                    .merchant_id
                    .as_ref()
                    .get_required_value("merchant_id")?,
                &key_store,
            )
            .await?;

        let (payment_data, _, _, _) =
            payment_flows::payments_operation_core::<api::PSync, _, _, _, Oss>(
                state,
                merchant_account.clone(),
                key_store,
                operations::PaymentStatus,
                tracking_data.clone(),
                payment_flows::CallConnectorAction::Trigger,
                services::AuthFlow::Client,
                api::HeaderPayload::default(),
            )
            .await?;

        let terminal_status = [
            enums::AttemptStatus::RouterDeclined,
            enums::AttemptStatus::Charged,
            enums::AttemptStatus::AutoRefunded,
            enums::AttemptStatus::Voided,
            enums::AttemptStatus::VoidFailed,
            enums::AttemptStatus::CaptureFailed,
            enums::AttemptStatus::Failure,
        ];
        match &payment_data.payment_attempt.status {
            status if terminal_status.contains(status) => {
                let id = process.id.clone();
                process
                    .finish_with_status(
                        state.get_db().as_scheduler(),
                        format!("COMPLETED_BY_PT_{id}"),
                    )
                    .await?
            }
            _ => {
                let connector = payment_data
                    .payment_attempt
                    .connector
                    .ok_or(sch_errors::ProcessTrackerError::MissingRequiredField)?;

                retry_sync_task(
                    db,
                    connector,
                    payment_data.payment_attempt.merchant_id,
                    process,
                )
                .await?
            }
        };
        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
        error: sch_errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), sch_errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

pub async fn get_sync_process_schedule_time(
    db: &dyn StorageInterface,
    connector: &str,
    merchant_id: &str,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let mapping: common_utils::errors::CustomResult<
        process_data::ConnectorPTMapping,
        errors::StorageError,
    > = db
        .find_config_by_key(&format!("pt_mapping_{connector}"))
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("ConnectorPTMapping")
                .change_context(errors::StorageError::DeserializationFailed)
        });
    let mapping = match mapping {
        Ok(x) => x,
        Err(err) => {
            logger::info!("Redis Mapping Error: {}", err);
            process_data::ConnectorPTMapping::default()
        }
    };
    let time_delta = utils::get_schedule_time(mapping, merchant_id, retry_count + 1);

    Ok(utils::get_time_from_delta(time_delta))
}

pub async fn retry_sync_task(
    db: &dyn StorageInterface,
    connector: String,
    merchant_id: String,
    pt: storage::ProcessTracker,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let schedule_time =
        get_sync_process_schedule_time(db, &connector, &merchant_id, pt.retry_count).await?;

    match schedule_time {
        Some(s_time) => pt.retry(db.as_scheduler(), s_time).await,
        None => {
            pt.finish_with_status(db.as_scheduler(), "RETRIES_EXCEEDED".to_string())
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_get_default_schedule_time() {
        let schedule_time_delta =
            utils::get_schedule_time(process_data::ConnectorPTMapping::default(), "-", 0).unwrap();
        let first_retry_time_delta =
            utils::get_schedule_time(process_data::ConnectorPTMapping::default(), "-", 1).unwrap();
        let cpt_default = process_data::ConnectorPTMapping::default().default_mapping;
        assert_eq!(
            vec![schedule_time_delta, first_retry_time_delta],
            vec![cpt_default.start_after, cpt_default.frequency[0]]
        );
    }
}
