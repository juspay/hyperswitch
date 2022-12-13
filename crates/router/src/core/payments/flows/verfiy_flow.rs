use async_trait::async_trait;
use error_stack::ResultExt;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult, StorageErrorExt},
        payments::{self, helpers, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{
        self, api,
        storage::{self, enums},
    },
};

#[async_trait]
impl ConstructFlowSpecificData<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for PaymentData<api::Verify>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::VerifyRouterData> {
        transformers::construct_payment_router_data::<api::Verify, types::VerifyRequestData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
        )
        .await
    }
}

#[async_trait]
impl Feature<api::Verify, types::VerifyRequestData> for types::VerifyRouterData {
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<Self> {
        self.decide_flow(
            state,
            connector,
            customer,
            Some(true),
            call_connector_action,
            storage_scheme,
        )
        .await
    }
}

impl types::VerifyRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<storage::Customer>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<Self> {
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
                            let mandate_reference = match resp.response.as_ref().ok() {
                                Some(types::PaymentsResponseData::TransactionResponse {
                                    mandate_reference,
                                    ..
                                }) => mandate_reference.clone(),
                                _ => None,
                            };

                            if let Some(new_mandate_data) = helpers::generate_mandate(
                                self.merchant_id.clone(),
                                self.connector.clone(),
                                self.request.setup_mandate_details.clone(),
                                maybe_customer,
                                payment_method_id,
                                mandate_reference,
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
