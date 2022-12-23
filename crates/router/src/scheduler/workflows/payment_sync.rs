use router_env::logger;

use super::{PaymentsSyncWorkflow, ProcessTrackerWorkflow};
use crate::{
    core::payments::{self as payment_flows, operations},
    db::{get_and_deserialize_key, StorageInterface},
    errors,
    routes::AppState,
    scheduler::{consumer, process_data, utils},
    types::{
        api,
        storage::{self, enums, ProcessTrackerExt},
    },
    utils::{OptionExt, ValueExt},
};

#[async_trait::async_trait]
impl ProcessTrackerWorkflow for PaymentsSyncWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db: &dyn StorageInterface = &*state.store;
        let tracking_data: api::PaymentsRetrieveRequest = process
            .tracking_data
            .clone()
            .parse_value("PaymentsRetrieveRequest")?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(
                tracking_data
                    .merchant_id
                    .as_ref()
                    .get_required_value("merchant_id")?,
            )
            .await?;

        let (payment_data, _, _) = payment_flows::payments_operation_core::<api::PSync, _, _, _>(
            state,
            merchant_account.clone(),
            operations::PaymentStatus,
            tracking_data.clone(),
            None,
            payment_flows::CallConnectorAction::Trigger,
        )
        .await?;

        let terminal_status = vec![
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
                    .finish_with_status(db, format!("COMPLETED_BY_PT_{}", id))
                    .await?
            }
            _ => {
                let connector = payment_data
                    .payment_attempt
                    .connector
                    .clone()
                    .ok_or(errors::ProcessTrackerError::MissingRequiredField)?;
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
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::some_error_handler(state, process, error).await
    }
}

pub async fn get_sync_process_schedule_time(
    db: &dyn StorageInterface,
    connector: &str,
    merchant_id: &str,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let redis_mapping: errors::CustomResult<process_data::ConnectorPTMapping, errors::RedisError> =
        get_and_deserialize_key(
            db,
            &format!("pt_mapping_{}", connector),
            "ConnectorPTMapping",
        )
        .await;
    let mapping = match redis_mapping {
        Ok(x) => x,
        Err(err) => {
            logger::error!("Redis Mapping Error: {}", err);
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
) -> Result<(), errors::ProcessTrackerError> {
    let schedule_time =
        get_sync_process_schedule_time(db, &connector, &merchant_id, pt.retry_count).await?;

    match schedule_time {
        Some(s_time) => pt.retry(db, s_time).await,
        None => {
            pt.finish_with_status(db, "RETRIES_EXCEEDED".to_string())
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
