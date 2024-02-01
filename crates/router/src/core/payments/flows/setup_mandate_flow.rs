use async_trait::async_trait;
use error_stack::{IntoReport, ResultExt};

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult, StorageErrorExt},
        mandate,
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
        /// Asynchronously constructs router data for setting up a mandate. This method takes various parameters such as the application state, connector ID, merchant account, key store, customer information, and merchant connector account type to create the required setup mandate router data. It then calls the `construct_payment_router_data` method from the `transformers` module to perform the actual data construction and returns the result as a future wrapped in a `RouterResult` enum.
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
        /// Asynchronously decides the flows based on the given parameters and returns a RouterResult.
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &domain::MerchantAccount,
        connector_request: Option<services::Request>,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self> {
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
        let pm_id = Box::pin(tokenization::save_payment_method(
            state,
            connector,
            resp.to_owned(),
            maybe_customer,
            merchant_account,
            self.request.payment_method_type,
            key_store,
        ))
        .await?;

        if let Some(mandate_id) = self
            .request
            .setup_mandate_details
            .as_ref()
            .and_then(|mandate_data| mandate_data.update_mandate_id.clone())
        {
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
                        pm_id,
                    )
                    .await
                }
                Ok(_) => Err(errors::ApiErrorResponse::InternalServerError)
                    .into_report()
                    .attach_printable("Unexpected response received")?,
                Err(_) => Ok(resp),
            }
        } else {
            mandate::mandate_procedure(
                state,
                resp,
                maybe_customer,
                pm_id,
                connector.merchant_connector_id.clone(),
            )
            .await
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

        /// Asynchronously adds a payment method token to the application state using the provided connector data and tokenization action. Returns an optional String representing the result of the operation.
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

        /// Asynchronously creates a connector customer using the provided state and connector data.
    /// Returns an optional string representing the created connector customer.
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

        /// Builds a specific connector request based on the given state, connector data, and call connector action. 
    /// If the call connector action is Trigger, it retrieves the connector integration, builds a request using the integration and connector configuration, and returns it along with a boolean value true. 
    /// If the call connector action is not Trigger, it returns None along with a boolean value true.
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
        /// Attempts to convert the provided SetupMandateRequestData into a new instance of the current type.
    /// If successful, returns the new instance with the provided data.
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
        /// Asynchronously decides the flow of the payment processing based on the confirmation status. If confirmation is true, it executes the connector processing step, saves the payment method, and initiates the mandate procedure. If confirmation is not true, it returns a clone of the current instance.
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<domain::Customer>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self> {
        match confirm {
            Some(true) => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::SetupMandate,
                    types::SetupMandateRequestData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();
                let resp = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    self,
                    call_connector_action,
                    None,
                )
                .await
                .to_setup_mandate_failed_response()?;

                let payment_method_type = self.request.payment_method_type;

                let pm_id = Box::pin(tokenization::save_payment_method(
                    state,
                    connector,
                    resp.to_owned(),
                    maybe_customer,
                    merchant_account,
                    payment_method_type,
                    key_store,
                ))
                .await?;

                Ok(mandate::mandate_procedure(
                    state,
                    resp,
                    maybe_customer,
                    pm_id,
                    connector.merchant_connector_id.clone(),
                )
                .await?)
            }
            _ => Ok(self.clone()),
        }
    }
}

impl mandate::MandateBehaviour for types::SetupMandateRequestData {
        /// This method returns the amount as an i64.
    fn get_amount(&self) -> i64 {
        0
    }

        /// Retrieves the setup future usage for the current object, if it exists.
    fn get_setup_future_usage(&self) -> Option<diesel_models::enums::FutureUsage> {
        self.setup_future_usage
    }

        /// Returns the optional reference to the mandate ID associated with the payment.
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }

        /// Sets the mandate ID for the payment.
    ///
    /// # Arguments
    ///
    /// * `new_mandate_id` - The new mandate ID to be set for the payment.
    ///
    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>) {
        self.mandate_id = new_mandate_id;
    }

        /// This method returns a clone of the payment method data associated with the current object.
    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }

        /// Retrieves the setup mandate details, if available, to be used for further processing.
    fn get_setup_mandate_details(&self) -> Option<&data_models::mandates::MandateData> {
        self.setup_mandate_details.as_ref()
    }
}

impl TryFrom<types::SetupMandateRequestData> for types::PaymentMethodTokenizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

        /// Attempts to create a new instance of the current type from the provided `SetupMandateRequestData`.
    /// Returns a Result with the newly created instance if successful, or an error if the conversion fails.
    fn try_from(data: types::SetupMandateRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            browser_info: None,
            currency: data.currency,
            amount: data.amount,
        })
    }
}
