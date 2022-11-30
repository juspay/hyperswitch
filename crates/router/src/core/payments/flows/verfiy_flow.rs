use async_trait::async_trait;
use error_stack::ResultExt;
use masking::Secret;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, RouterResult, StorageErrorExt},
        payments::{self, helpers, transformers, PaymentData},
    },
    db::mandate::IMandate,
    routes::AppState,
    services,
    types::{
        self, api,
        storage::{self, enums},
    },
    utils,
};

#[async_trait]
impl ConstructFlowSpecificData<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for PaymentData<api::Verify>
{
    async fn construct_r_d<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::VerifyRouterData> {
        let (_, router_data) = transformers::construct_payment_router_data::<
            api::Verify,
            types::VerifyRequestData,
        >(state, self.clone(), connector_id, merchant_account)
        .await?;

        Ok(router_data)
    }
}

#[async_trait]
impl Feature<api::Verify, types::VerifyRequestData> for types::VerifyRouterData {
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: api::ConnectorData,
        customer: &Option<api::CustomerResponse>,
        payment_data: PaymentData<api::Verify>,
        call_connector_action: payments::CallConnectorAction,
    ) -> (RouterResult<Self>, PaymentData<api::Verify>)
    where
        dyn api::Connector: services::ConnectorIntegration<
            api::Verify,
            types::VerifyRequestData,
            types::PaymentsResponseData,
        >,
    {
        let resp = self
            .decide_flow(
                state,
                connector,
                customer,
                Some(true),
                call_connector_action,
            )
            .await;

        (resp, payment_data)
    }
}

impl types::VerifyRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: api::ConnectorData,
        maybe_customer: &Option<api::CustomerResponse>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<Self>
    where
        dyn api::Connector + Sync: services::ConnectorIntegration<
            api::Verify,
            types::VerifyRequestData,
            types::PaymentsResponseData,
        >,
    {
        match confirm {
            Some(true) => {
                let connector_integration: services::BoxedConnectorIntegration<
                    api::Verify,
                    types::VerifyRequestData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();
                let mut resp = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    self,
                    call_connector_action,
                )
                .await
                .map_err(|err| err.to_verify_failed_response())?;

                match &self.request.mandate_id {
                    Some(mandate_id) => {
                        let mandate = state
                            .store
                            .find_mandate_by_merchant_id_mandate_id(&resp.merchant_id, mandate_id)
                            .await
                            .change_context(errors::ApiErrorResponse::MandateNotFound)?;
                        resp.payment_method_id = Some(mandate.payment_method_id);
                    }
                    None => {
                        if self.request.setup_future_usage.is_some() {
                            let payment_method_id = helpers::call_payment_method(
                                state,
                                &self.merchant_id,
                                Some(&self.request.payment_method_data),
                                Some(self.payment_method),
                                maybe_customer,
                            )
                            .await?
                            .payment_method_id;

                            resp.payment_method_id = Some(payment_method_id.clone());
                            if let Some(new_mandate_data) = generate_mandate(
                                self.merchant_id.clone(),
                                self.request.setup_mandate_details.clone(),
                                maybe_customer,
                                payment_method_id,
                            ) {
                                resp.request.mandate_id = Some(new_mandate_data.mandate_id.clone());
                                state.store.insert_mandate(new_mandate_data).await.map_err(
                                    |err| {
                                        err.to_duplicate_response(
                                            errors::ApiErrorResponse::DuplicateMandate,
                                        )
                                    },
                                )?;
                            }
                        }
                    }
                }
                Ok(resp)
            }
            _ => Ok(self.clone()),
        }
    }
}

fn generate_mandate(
    merchant_id: String,
    setup_mandate_details: Option<api::MandateData>,
    customer: &Option<api::CustomerResponse>,
    payment_method_id: String,
) -> Option<storage::MandateNew> {
    match (setup_mandate_details, customer) {
        (Some(data), Some(cus)) => {
            let mandate_id = utils::generate_id(consts::ID_LENGTH, "man");
            Some(storage::MandateNew {
                mandate_id,
                customer_id: cus.customer_id.clone(),
                merchant_id,
                payment_method_id,
                mandate_status: enums::MandateStatus::Active,
                mandate_type: enums::MandateType::MultiUse,
                customer_ip_address: data.customer_acceptance.get_ip_address().map(Secret::new),
                customer_user_agent: data.customer_acceptance.get_user_agent(),
                customer_accepted_at: Some(data.customer_acceptance.get_accepted_at()),
                ..Default::default()
            })
        }
        (_, _) => None,
    }
}
