use common_utils::ext_traits::ValueExt;
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use router_env::metrics::add_attributes;
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

use crate::{
    core::{
        mandate::utils,
        payments::{helpers, CallConnectorAction},
    },
    db::StorageInterface,
    errors,
    routes::{metrics, SessionState},
    services,
    types::{
        self,
        api::{ConnectorData, GetToken},
        storage,
    },
};

pub struct PaymentMethodMandateDetailsRevokeWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodMandateDetailsRevokeWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<storage::PaymentMethodMandateRevokeTrackingData>(
            "PaymentMethodMandateRevokeTrackingData",
        )?;

        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                &tracking_data.merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let retry_count = process.retry_count;
        let merchant_account = db
            .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
            .await?;

        let connector_name = tracking_data.connector.to_string();
        let merchant_connector_account = helpers::get_merchant_connector_account(
            state,
            &tracking_data.merchant_id,
            None,
            &key_store,
            &tracking_data.profile_id,
            &connector_name,
            Some(&tracking_data.merchant_connector_id),
        )
        .await?;
        let connector_data = ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector_name,
            GetToken::Connector,
            Some(tracking_data.merchant_connector_id.clone()),
        )?;
        let connector_integration: services::BoxedMandateRevokeConnectorIntegrationInterface<
            types::api::MandateRevoke,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
        > = connector_data.connector.get_connector_integration();

        let router_data = utils::construct_mandate_revoke_router_data(
            merchant_connector_account,
            &merchant_account,
            tracking_data.customer_id.clone(),
            &connector_name,
            Some(tracking_data.connector_mandate_id),
            None,
        )
        .await?;

        let response = services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            CallConnectorAction::Trigger,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
        match response.response {
            Ok(_mandate) => {
                metrics::CONNECTOR_MANDATE_REVOKE_SUCCEDED.add(
                    &metrics::CONTEXT,
                    1,
                    &add_attributes([("flow", "")]),
                );
                db.as_scheduler()
                    .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
                    .await?;
            }
            Err(err) => {
                metrics::CONNECTOR_MANDATE_REVOKE_FAILED.add(
                    &metrics::CONTEXT,
                    1,
                    &add_attributes([(
                        "flow",
                        format!(
                            "connector_name = {:?}, error_code = {:?}",
                            &connector_name, err.code
                        ),
                    )]),
                );
                // if not implemented end the task in the PT
                if err.code == "IR_00" {
                    db.as_scheduler()
                        .finish_process_with_business_status(
                            process,
                            business_status::NOT_IMPLEMENTED_FOR_THE_CONNECTOR,
                        )
                        .await?;
                } else {
                    // if connector err re-schedule task
                    metrics::TASKS_RESET_COUNT.add(
                        &metrics::CONTEXT,
                        1,
                        &add_attributes([("flow", "ConnectorMandateRevokeFlow")]),
                    );
                    connector_reschedule_task(db, retry_count, process).await?;
                }
            }
        };

        Ok(())
    }
}

async fn connector_reschedule_task(
    db: &dyn StorageInterface,
    retry_count: i32,
    process: storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let mapping = process_data::PaymentMethodMandateRevokePTMapping::default();
    let time_delta = if retry_count == 0 {
        Some(mapping.default.start_after)
    } else {
        pt_utils::get_delay(retry_count + 1, &mapping.default.frequencies)
    };
    let schedule_time = pt_utils::get_time_from_delta(time_delta);

    match schedule_time {
        Some(s_time) => db
            .as_scheduler()
            .retry_process(process, s_time)
            .await
            .map_err(Into::<errors::ProcessTrackerError>::into)?,
        None => db
            .as_scheduler()
            .finish_process_with_business_status(process, business_status::RETRIES_EXCEEDED)
            .await
            .map_err(Into::<errors::ProcessTrackerError>::into)?,
    };
    Ok(())
}
