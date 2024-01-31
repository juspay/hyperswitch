use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::{metrics, AppState},
    services,
    types::{self, api, domain},
    utils::OptionExt,
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentData<api::CompleteAuthorize>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<
        types::RouterData<
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(transformers::construct_payment_router_data::<
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
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
impl Feature<api::CompleteAuthorize, types::CompleteAuthorizeData>
    for types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    >
{
    async fn decide_flows<'a>(
        mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        _customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _merchant_account: &domain::MerchantAccount,
        connector_request: Option<services::Request>,
        _key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action,
            connector_request,
        )
        .await
        .to_payment_failed_response()?;

        Ok(resp)
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
        _tokenization_action: &payments::TokenizationAction,
    ) -> RouterResult<Option<String>> {
        // TODO: remove this and handle it in core
        if matches!(connector.connector_name, types::Connector::Payme) {
            let request = self.request.clone();
            payments::tokenization::add_payment_method_token(
                state,
                connector,
                &payments::TokenizationAction::TokenizeInConnector,
                self,
                types::PaymentMethodTokenizationData::try_from(request)?,
            )
            .await
        } else {
            Ok(None)
        }
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        let request = match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::CompleteAuthorize,
                    types::CompleteAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                connector_integration
                    .build_request(self, &state.conf.connectors)
                    .to_payment_failed_response()?
            }
            _ => None,
        };

        Ok((request, true))
    }

    async fn preprocessing_steps<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        complete_authorize_preprocessing_steps(state, &self, true, connector).await
    }
}

pub async fn complete_authorize_preprocessing_steps<F: Clone>(
    state: &AppState,
    router_data: &types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
    confirm: bool,
    connector: &api::ConnectorData,
) -> RouterResult<types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>> {
    if confirm {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let preprocessing_request_data =
            types::PaymentsPreProcessingData::try_from(router_data.request.to_owned())?;

        let preprocessing_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
            Err(types::ErrorResponse::default());

        let preprocessing_router_data =
            payments::helpers::router_data_type_conversion::<_, api::PreProcessing, _, _, _, _>(
                router_data.clone(),
                preprocessing_request_data,
                preprocessing_response_data,
            );

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &preprocessing_router_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await
        .to_payment_failed_response()?;

        metrics::PREPROCESSING_STEPS_COUNT.add(
            &metrics::CONTEXT,
            1,
            &[
                metrics::request::add_attributes("connector", connector.connector_name.to_string()),
                metrics::request::add_attributes(
                    "payment_method",
                    router_data.payment_method.to_string(),
                ),
            ],
        );

        let mut router_data_request = router_data.request.to_owned();

        if let Ok(types::PaymentsResponseData::TransactionResponse {
            connector_metadata, ..
        }) = &resp.response
        {
            router_data_request.connector_meta = connector_metadata.to_owned();
        };

        let authorize_router_data =
            payments::helpers::router_data_type_conversion::<_, F, _, _, _, _>(
                resp.clone(),
                router_data_request,
                resp.response,
            );

        Ok(authorize_router_data)
    } else {
        Ok(router_data.clone())
    }
}

impl TryFrom<types::CompleteAuthorizeData> for types::PaymentMethodTokenizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(data: types::CompleteAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data
                .payment_method_data
                .get_required_value("payment_method_data")?,
            browser_info: data.browser_info,
            currency: data.currency,
            amount: Some(data.amount),
        })
    }
}
