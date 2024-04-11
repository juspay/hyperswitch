use api_models::enums::{PaymentMethod, PaymentMethodType};
use async_trait::async_trait;
use error_stack::ResultExt;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    configs::settings,
    core::{
        errors::{self, ConnectorErrorExt, RouterResult, StorageErrorExt},
        mandate,
        payment_methods::cards,
        payments::{
            self, access_token, customers, helpers, tokenization, transformers, PaymentData,
        },
    },
    routes::AppState,
    services,
    types::{self, api, domain},
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for PaymentData<api::SetupMandate>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::SetupMandateRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::SetupMandate,
            types::SetupMandateRequestData,
        >(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
            merchant_connector_account,
        ))
        .await
    }
}

#[async_trait]
impl Feature<api::SetupMandate, types::SetupMandateRequestData> for types::SetupMandateRouterData {
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &domain::MerchantAccount,
        connector_request: Option<services::Request>,
        key_store: &domain::MerchantKeyStore,
        profile_id: Option<String>,
    ) -> RouterResult<Self> {
        if let Some(mandate_id) = self
            .request
            .setup_mandate_details
            .as_ref()
            .and_then(|mandate_data| mandate_data.update_mandate_id.clone())
        {
            Box::pin(self.update_mandate_flow(
                state,
                merchant_account,
                mandate_id,
                connector,
                key_store,
                call_connector_action,
                &state.conf.mandates.update_mandate_supported,
                connector_request,
                maybe_customer,
                profile_id,
            ))
            .await
        } else {
            let connector_integration: services::BoxedConnectorIntegration<
                '_,
                api::SetupMandate,
                types::SetupMandateRequestData,
                types::PaymentsResponseData,
            > = connector.connector.get_connector_integration();

            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                &self,
                call_connector_action.clone(),
                connector_request,
            )
            .await
            .to_setup_mandate_failed_response()?;

            Ok(resp)
        }
    }

    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

    async fn add_payment_method_token<'a>(
        &mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        tokenization_action: &payments::TokenizationAction,
    ) -> RouterResult<Option<String>> {
        let request = self.request.clone();
        tokenization::add_payment_method_token(
            state,
            connector,
            tokenization_action,
            self,
            types::PaymentMethodTokenizationData::try_from(request)?,
        )
        .await
    }

    async fn create_connector_customer<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<String>> {
        customers::create_connector_customer(
            state,
            connector,
            self,
            types::ConnectorCustomerData::try_from(self.request.to_owned())?,
        )
        .await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::SetupMandate,
                    types::SetupMandateRequestData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                Ok((
                    connector_integration
                        .build_request(self, &state.conf.connectors)
                        .to_payment_failed_response()?,
                    true,
                ))
            }
            _ => Ok((None, true)),
        }
    }
}

impl TryFrom<types::SetupMandateRequestData> for types::ConnectorCustomerData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(data: types::SetupMandateRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.email,
            payment_method_data: data.payment_method_data,
            description: None,
            phone: None,
            name: None,
            preprocessing_id: None,
        })
    }
}

#[allow(clippy::too_many_arguments)]
impl types::SetupMandateRouterData {
    async fn update_mandate_flow(
        self,
        state: &AppState,
        merchant_account: &domain::MerchantAccount,
        mandate_id: String,
        connector: &api::ConnectorData,
        key_store: &domain::MerchantKeyStore,
        call_connector_action: payments::CallConnectorAction,
        supported_connectors_for_update_mandate: &settings::SupportedPaymentMethodsForMandate,
        connector_request: Option<services::Request>,
        _maybe_customer: &Option<domain::Customer>,
        _profile_id: Option<String>,
    ) -> RouterResult<Self> {
        let payment_method_type = self.request.payment_method_type;

        let payment_method = self.request.payment_method_data.get_payment_method();
        let supported_connectors_config = payment_method.zip(payment_method_type).map_or_else(
            || {
                if payment_method == Some(PaymentMethod::Card) {
                    cards::filter_pm_based_on_update_mandate_support_for_connector(
                        supported_connectors_for_update_mandate,
                        &PaymentMethod::Card,
                        &PaymentMethodType::Credit,
                        connector.connector_name,
                    ) && cards::filter_pm_based_on_update_mandate_support_for_connector(
                        supported_connectors_for_update_mandate,
                        &PaymentMethod::Card,
                        &PaymentMethodType::Debit,
                        connector.connector_name,
                    )
                } else {
                    false
                }
            },
            |(pm, pmt)| {
                cards::filter_pm_based_on_update_mandate_support_for_connector(
                    supported_connectors_for_update_mandate,
                    &pm,
                    &pmt,
                    connector.connector_name,
                )
            },
        );
        if supported_connectors_config {
            let connector_integration: services::BoxedConnectorIntegration<
                '_,
                api::SetupMandate,
                types::SetupMandateRequestData,
                types::PaymentsResponseData,
            > = connector.connector.get_connector_integration();

            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                &self,
                call_connector_action.clone(),
                connector_request,
            )
            .await
            .to_setup_mandate_failed_response()?;
            let mandate = state
                .store
                .find_mandate_by_merchant_id_mandate_id(&merchant_account.merchant_id, &mandate_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;

            let profile_id = mandate::helpers::get_profile_id_for_mandate(
                state,
                merchant_account,
                mandate.clone(),
            )
            .await?;
            match resp.response {
                Ok(types::PaymentsResponseData::TransactionResponse { .. }) => {
                    let connector_integration: services::BoxedConnectorIntegration<
                        '_,
                        types::api::MandateRevoke,
                        types::MandateRevokeRequestData,
                        types::MandateRevokeResponseData,
                    > = connector.connector.get_connector_integration();
                    let merchant_connector_account = helpers::get_merchant_connector_account(
                        state,
                        &merchant_account.merchant_id,
                        None,
                        key_store,
                        &profile_id,
                        &mandate.connector,
                        mandate.merchant_connector_id.as_ref(),
                    )
                    .await?;

                    let router_data = mandate::utils::construct_mandate_revoke_router_data(
                        merchant_connector_account,
                        merchant_account,
                        mandate.clone(),
                    )
                    .await?;

                    let _response = services::execute_connector_processing_step(
                        state,
                        connector_integration,
                        &router_data,
                        call_connector_action,
                        None,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;
                    // TODO:Add the revoke mandate task to process tracker
                    mandate::update_mandate_procedure(
                        state,
                        resp,
                        mandate,
                        &merchant_account.merchant_id,
                        Some(profile_id),
                    )
                    .await
                }
                Ok(_) => Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unexpected response received")?,
                Err(_) => Ok(resp),
            }
        } else {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: format!(
                    "Update Mandate flow not implemented for the connector {:?}",
                    connector.connector_name
                ),
            }
            .into())
        }
    }
}

impl mandate::MandateBehaviour for types::SetupMandateRequestData {
    fn get_amount(&self) -> i64 {
        0
    }

    fn get_setup_future_usage(&self) -> Option<diesel_models::enums::FutureUsage> {
        self.setup_future_usage
    }

    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }

    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>) {
        self.mandate_id = new_mandate_id;
    }

    fn get_payment_method_data(&self) -> domain::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }

    fn get_setup_mandate_details(&self) -> Option<&data_models::mandates::MandateData> {
        self.setup_mandate_details.as_ref()
    }
    fn get_customer_acceptance(&self) -> Option<api_models::payments::CustomerAcceptance> {
        self.customer_acceptance.clone().map(From::from)
    }
}

impl TryFrom<types::SetupMandateRequestData> for types::PaymentMethodTokenizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(data: types::SetupMandateRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            browser_info: None,
            currency: data.currency,
            amount: data.amount,
        })
    }
}
