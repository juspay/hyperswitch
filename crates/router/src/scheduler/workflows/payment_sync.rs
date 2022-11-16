use std::sync;

use super::{PaymentsSyncWorkflow, ProcessTrackerWorkflow};
use crate::{
    core::payments::{self as payment_flows, operations},
    db::Db,
    errors,
    routes::AppState,
    scheduler::{consumer, process_data, utils as pt_utils},
    services::redis,
    types::{
        api,
        storage::{self, enums},
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
        let redis_conn = state.store.redis_conn.clone();
        let db: &dyn Db = &state.store;
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
            payment_flows::CallConnectorAction::Trigger,
        )
        .await?;

        let terminal_status = vec![
            enums::AttemptStatus::JuspayDeclined,
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
                retry_sync_task(
                    db,
                    redis_conn,
                    payment_data.payment_attempt.connector,
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
    connector: &str,
    merchant_id: &str,
    redis: sync::Arc<redis::RedisConnectionPool>,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let mapping: process_data::ConnectorPTMapping = redis
        .get_and_deserialize_key(connector, "ConnectorPTMapping")
        .await?;
    let time_delta = get_sync_schedule_time(mapping, merchant_id, retry_count + 1);

    Ok(pt_utils::get_time_from_delta(time_delta))
}

pub fn get_sync_schedule_time(
    mapping: process_data::ConnectorPTMapping,
    merchant_name: &str,
    retry_count: i32,
) -> Option<i32> {
    let mapping = match mapping.custom_merchant_mapping.get(merchant_name) {
        Some(map) => map.clone(),
        None => mapping.default_mapping,
    };

    if retry_count == 0 {
        Some(mapping.start_after)
    } else {
        get_delay(
            retry_count,
            mapping.count.iter().zip(mapping.frequency.iter()),
        )
    }
}

fn get_delay<'a>(
    retry_count: i32,
    mut array: impl Iterator<Item = (&'a i32, &'a i32)>,
) -> Option<i32> {
    match array.next() {
        Some(ele) => {
            let v = retry_count - ele.0;
            if v <= 0 {
                Some(*ele.1)
            } else {
                get_delay(v, array)
            }
        }
        None => None,
    }
}

pub async fn retry_sync_task(
    db: &dyn Db,
    redis_conn: sync::Arc<redis::RedisConnectionPool>,
    connector: String,
    merchant_id: String,
    pt: storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let schedule_time =
        get_sync_process_schedule_time(&connector, &merchant_id, redis_conn, pt.retry_count)
            .await?;

    match schedule_time {
        Some(s_time) => pt.retry(db, s_time).await,
        None => {
            pt.finish_with_status(db, "RETRIES_EXCEEDED".to_string())
                .await
        }
    }
}
