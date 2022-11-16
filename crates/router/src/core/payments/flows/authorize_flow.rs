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
        PaymentsRequestData, PaymentsResponseData, PaymentsRouterData,
    },
    utils,
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::Authorize,
        types::PaymentsRequestData,
        types::PaymentsResponseData,
    > for PaymentData<api::Authorize>
{
    async fn construct_r_d<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<
        types::RouterData<api::Authorize, types::PaymentsRequestData, types::PaymentsResponseData>,
    > {
        let output = transformers::construct_payment_router_data::<
            api::Authorize,
            types::PaymentsRequestData,
        >(state, self.clone(), connector_id, merchant_account)
        .await?;
        Ok(output.1)
    }
}

#[async_trait]
impl Feature<api::Authorize, types::PaymentsRequestData>
    for types::RouterData<api::Authorize, types::PaymentsRequestData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: api::ConnectorData,
        customer: &Option<api::CustomerResponse>,
        payment_data: PaymentData<api::Authorize>,
        call_connector_action: payments::CallConnectorAction,
    ) -> (RouterResult<Self>, PaymentData<api::Authorize>)
    where
        dyn api::Connector: services::ConnectorIntegration<
            api::Authorize,
            types::PaymentsRequestData,
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

impl PaymentsRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: api::ConnectorData,
        maybe_customer: &Option<api::CustomerResponse>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<PaymentsRouterData>
    where
        dyn api::Connector + Sync: services::ConnectorIntegration<
            api::Authorize,
            PaymentsRequestData,
            PaymentsResponseData,
        >,
    {
        match confirm {
            Some(true) => {
                let connector_integration: services::BoxedConnectorIntegration<
                    api::Authorize,
                    PaymentsRequestData,
                    PaymentsResponseData,
                > = connector.connector.get_connector_integration();
                let mut resp = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    self,
                    call_connector_action,
                )
                .await
                .map_err(|error| error.to_payment_failed_response())?;
                match &self.request.mandate_id {
                    Some(mandate_id) => {
                        let mandate = state
                            .store
                            .find_mandate_by_merchant_id_mandate_id(
                                resp.merchant_id.as_ref(),
                                mandate_id,
                            )
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
                            if let Some(new_mandate_data) =
                                self.generate_mandate(maybe_customer, payment_method_id)
                            {
                                resp.request.mandate_id = Some(new_mandate_data.mandate_id.clone());
                                state.store.insert_mandate(new_mandate_data).await.map_err(
                                    |err| {
                                        err.to_duplicate_response(
                                            errors::ApiErrorResponse::DuplicateRefundRequest,
                                        )
                                    },
                                )?;
                            };
                        }
                    }
                }

                Ok(resp)
            }
            _ => Ok(self.clone()),
        }
    }

    fn generate_mandate(
        &self,
        customer: &Option<api::CustomerResponse>,
        payment_method_id: String,
    ) -> Option<storage::MandateNew> {
        match (self.request.setup_mandate_details.clone(), customer) {
            (Some(data), Some(cus)) => {
                let mandate_id = utils::generate_id(consts::ID_LENGTH, "man");
                Some(storage::MandateNew {
                    mandate_id,
                    customer_id: cus.customer_id.clone(),
                    merchant_id: self.merchant_id.clone(),
                    payment_method_id,
                    mandate_status: enums::MandateStatus::Active,
                    mandate_type: enums::MandateType::MultiUse,
                    customer_ip_address: data.customer_acceptance.get_ip_address().map(Secret::new),
                    customer_user_agent: data.customer_acceptance.get_user_agent(),
                    customer_accepted_at: Some(data.customer_acceptance.get_accepted_at()),
                    ..Default::default() // network_transaction_id: Option<String>,
                                         // previous_transaction_id: Option<String>,
                                         // created_at: Option<PrimitiveDateTime>,
                })
            }
            (_, _) => None,
        }
    }
}
