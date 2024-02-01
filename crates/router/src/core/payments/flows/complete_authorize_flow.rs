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
        /// Asynchronously constructs router data for a specific type of payment authorization.
    ///
    /// This method takes in various parameters such as the application state, connector ID, merchant account,
    /// key store, customer information, and merchant connector account type, and uses them to construct
    /// router data for a complete authorization payment. It then returns a `RouterResult` containing the
    /// complete authorize API, complete authorize data types, and payments response data types.
    ///
    /// # Arguments
    ///
    /// * `state` - The application state
    /// * `connector_id` - The connector ID
    /// * `merchant_account` - The merchant account information
    /// * `key_store` - The key store for the merchant
    /// * `customer` - An optional customer information
    /// * `merchant_connector_account` - The type of merchant connector account
    ///
    /// # Returns
    ///
    /// A `RouterResult` containing the complete authorize API, complete authorize data types, and payments response data types.
    ///
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
        /// Asynchronously decides the flows for processing a payment by executing the connector integration 
    /// and handling the response to return a RouterResult containing the payment response data.
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

        /// Asynchronously adds an access token to the specified merchant account using the provided state, connector data, and merchant account information.
    ///
    /// # Arguments
    ///
    /// * `state` - The application state containing necessary dependencies and configurations.
    /// * `connector` - The connector data needed to communicate with external APIs.
    /// * `merchant_account` - The merchant account to which the access token will be added.
    ///
    /// # Returns
    ///
    /// The result of adding the access token, which includes information about the success or failure of the operation.
    ///
    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

        /// Asynchronously adds a payment method token. If the connector is Payme, it calls the `add_payment_method_token` method from the `tokenization` module passing the provided `state`, `connector`, `TokenizationAction`, `self`, and `PaymentMethodTokenizationData`. Otherwise, it returns `Ok(None)`.
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

        /// Asynchronously builds a flow-specific connector request based on the provided connector data and action.
    /// 
    /// # Arguments
    /// 
    /// * `state` - The application state.
    /// * `connector` - The connector data for which the request is to be built.
    /// * `call_connector_action` - The action to be performed on the connector.
    /// 
    /// # Returns
    /// 
    /// A tuple containing an optional service request and a boolean indicating whether the request was successfully built.
    ///
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

        /// Asynchronously performs preprocessing steps on the current state and connector data.
    ///
    /// This method takes in the current application state and connector data, and then passes them to the
    /// complete_authorize_preprocessing_steps function with the current object reference and a boolean value of true.
    /// It then awaits the result of the preprocessing steps and returns the updated state.
    ///
    /// # Arguments
    ///
    /// * `state` - The reference to the current application state
    /// * `connector` - The reference to the connector data
    ///
    /// # Returns
    ///
    /// The result of the preprocessing steps, wrapped in a RouterResult
    async fn preprocessing_steps<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        complete_authorize_preprocessing_steps(state, &self, true, connector).await
    }
}

/// Asynchronously completes the authorization preprocessing steps based on the provided data.
///
/// # Arguments
///
/// * `state` - The application state
/// * `router_data` - The router data containing the authorization and payment response data
/// * `confirm` - A boolean indicating whether to confirm the preprocessing steps
/// * `connector` - The connector data for the authorization
///
/// # Returns
///
/// Returns a `RouterResult` containing the updated router data after completing the authorization preprocessing steps.
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

        /// Attempts to create a new instance of the struct by converting from the provided `CompleteAuthorizeData`. 
    /// If successful, returns a `Result` containing the newly created instance. 
    ///
    /// # Arguments
    /// * `data` - The `CompleteAuthorizeData` to convert from.
    ///
    /// # Returns
    /// * If successful, returns a `Result` containing the newly created instance of the struct.
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
