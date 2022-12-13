use async_trait::async_trait;
use error_stack::ResultExt;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult, StorageErrorExt},
        payments::{self, helpers, transformers, PaymentData},
    },
    routes::AppState,
    scheduler::metrics,
    services,
    types::{
        self, api,
        storage::{self, enums as storage_enums},
        PaymentsAuthorizeData, PaymentsAuthorizeRouterData, PaymentsResponseData,
    },
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentData<api::Authorize>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<
        types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        transformers::construct_payment_router_data::<api::Authorize, types::PaymentsAuthorizeData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
        )
        .await
    }
}

#[async_trait]
impl Feature<api::Authorize, types::PaymentsAuthorizeData> for types::PaymentsAuthorizeRouterData {
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<Self> {
        let resp = self
            .decide_flow(
                state,
                connector,
                customer,
                Some(true),
                call_connector_action,
                storage_scheme,
            )
            .await;

        metrics::PAYMENT_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics

        resp
    }
}

impl PaymentsAuthorizeRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<storage::Customer>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentsAuthorizeRouterData> {
        match confirm {
            Some(true) => {
                let connector_integration: services::BoxedConnectorIntegration<
                    api::Authorize,
                    PaymentsAuthorizeData,
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
                        let mandate = match mandate.mandate_type {
                            storage_enums::MandateType::SingleUse => state
                                .store
                                .update_mandate_by_merchant_id_mandate_id(
                                    &resp.merchant_id,
                                    mandate_id,
                                    storage::MandateUpdate::StatusUpdate {
                                        mandate_status: storage_enums::MandateStatus::Revoked,
                                    },
                                )
                                .await
                                .change_context(errors::ApiErrorResponse::MandateNotFound),
                            storage_enums::MandateType::MultiUse => state
                                .store
                                .update_mandate_by_merchant_id_mandate_id(
                                    &resp.merchant_id,
                                    mandate_id,
                                    storage::MandateUpdate::CaptureAmountUpdate {
                                        amount_captured: Some(
                                            mandate.amount_captured.unwrap_or(0)
                                                + self.request.amount,
                                        ),
                                    },
                                )
                                .await
                                .change_context(errors::ApiErrorResponse::MandateNotFound),
                        }?;

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
                                None,
                                maybe_customer,
                                payment_method_id,
                                mandate_reference,
                            ) {
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
}
