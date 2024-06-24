use crate::{
    core::{
        payment_methods::helpers as pm_helpers,
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
use std::collections::HashSet;
use router_env::tracing::instrument
use error_stack::ResultExt;
use scheduler::workflows::ProcessTrackerWorkflow;

pub struct PaymentMethodMandateDetailsUpdateWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodMandateDetailsUpdateWorkflow {
    #[instrument(skip_all)]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<storage::PaymentMethodMandateUpdateTrackingData>(
            "PaymentMethodMandateUpdateTrackingData",
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

        let hm = HashSet::new();
        let mca_ids = tracking_data.list_mca_ids;
        for (mca_id, mandate_details) in mca_ids {
            let connector_name = mandate_details.connector_variant.to_string();
            let merchant_connector_account = helpers::get_merchant_connector_account(
                state,
                &tracking_data.merchant_id,
                None,
                &key_store,
                &mandate_details.profile_id,
                &connector_name,
                Some(&mca_id),
            )
            .await?;
            let connector_data = ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &connector_name,
                GetToken::Connector,
                Some(mca_id),
            )?;
            let connector_integration: services::BoxedConnectorIntegration<
                '_,
                types::api::UpdateMandateDetails,
                types::MandateDetailsUpdateData,
                types::MandateDetailsUpdateResponeData,
            > = connector_data.connector.get_connector_integration();

            let router_data = pm_helpers::construct_mandate_update_router_data(
                merchant_connector_account,
                &merchant_account,
                mandate_details,
                tracking_data.card_updation_obj.clone(),
                tracking_data.customer_id.clone(),
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
                Ok(mandate) => {
                    hm.insert(mandate_details.connector_mandate_id);
                    continue;
                }
                Err(_) => {
                    // make the retru count as +1
                    // finish after the retru cpunt is over
                    // finish after the updation task has been overrirden
                    // finisha after the deletion of the card takes place
                }
            };
        }

        Ok(())
    }
}

#[instrument(skip_all)]
pub(crate) async fn get_mandate_update_retry_for_specific_connector(retry)->{

}
