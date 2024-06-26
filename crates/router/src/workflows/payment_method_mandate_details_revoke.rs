use crate::{
    core::{
        mandate::utils,
        payments::{helpers, CallConnectorAction},
    },
    errors,
    routes::SessionState,
    services,
    types::{
        self,
        api::{ConnectorData, GetToken},
        storage,
    },
};
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use scheduler::workflows::ProcessTrackerWorkflow;

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
            "PaymentMethodMandateDetailsRevokeWorkflow",
        )?;

        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                &tracking_data.merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let _retry_count = process.retry_count;
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
            connector_name,
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
                db.as_scheduler()
                    .finish_process_with_business_status(process, "COMPLETED_BY_PT")
                    .await?;
            }
            Err(_) => {
                // if not implemented end the task in the PT
                // if not supported revoke in the core, not handle  here
                // if genuine err re scgedule task
            }
        };

        Ok(())
    }
}
