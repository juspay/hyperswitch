use async_trait::async_trait;
use error_stack;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, helpers, tokenization, transformers, PaymentData,
        },
    },
    logger,
    routes::{metrics, AppState},
    services,
    types::{self, api, domain},
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentData<api::Authorize>
{
        /// Asynchronously constructs router data for payment authorization using the given state, connector ID, merchant account, key store, customer, and merchant connector account. Returns a RouterResult containing the constructed router data for payment authorization.
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
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(transformers::construct_payment_router_data::<
            api::Authorize,
            types::PaymentsAuthorizeData,
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
impl Feature<api::Authorize, types::PaymentsAuthorizeData> for types::PaymentsAuthorizeRouterData {
        /// Asynchronously decides the flows for payment processing based on various conditions including the payment method, authentication type, and mandate details.
    async fn decide_flows<'a>(
        mut self,
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
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        connector
            .connector
            .validate_capture_method(self.request.capture_method)
            .to_payment_failed_response()?;

        if self.should_proceed_with_authorize() {
            self.decide_authentication_type();
            logger::debug!(auth_type=?self.auth_type);
            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                &self,
                call_connector_action,
                connector_request,
            )
            .await
            .to_payment_failed_response()?;

            metrics::PAYMENT_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics

            let is_mandate = resp.request.setup_mandate_details.is_some();

            if is_mandate {
                let payment_method_id = Box::pin(tokenization::save_payment_method(
                    state,
                    connector,
                    resp.to_owned(),
                    maybe_customer,
                    merchant_account,
                    self.request.payment_method_type,
                    key_store,
                ))
                .await?;
                Ok(mandate::mandate_procedure(
                    state,
                    resp,
                    maybe_customer,
                    payment_method_id,
                    connector.merchant_connector_id.clone(),
                )
                .await?)
            } else {
                let connector = connector.clone();
                let response = resp.clone();
                let maybe_customer = maybe_customer.clone();
                let merchant_account = merchant_account.clone();
                let key_store = key_store.clone();
                let state = state.clone();

                logger::info!("Initiating async call to save_payment_method in locker");

                tokio::spawn(async move {
                    logger::info!("Starting async call to save_payment_method in locker");

                    let result = Box::pin(tokenization::save_payment_method(
                        &state,
                        &connector,
                        response,
                        &maybe_customer,
                        &merchant_account,
                        self.request.payment_method_type,
                        &key_store,
                    ))
                    .await;

                    if let Err(err) = result {
                        logger::error!("Asynchronously saving card in locker failed : {:?}", err);
                    }
                });

                Ok(resp)
            }
        } else {
            Ok(self.clone())
        }
    }

        /// Asynchronously adds an access token to a merchant account using the provided state, connector data, and merchant account. Returns a RouterResult containing the result of adding the access token.
    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

        /// Asynchronously adds a payment method token using the provided connector data and tokenization action.
    /// 
    /// # Arguments
    /// 
    /// * `state` - The state of the application
    /// * `connector` - The connector data for the payment method
    /// * `tokenization_action` - The tokenization action to be performed
    /// 
    /// # Returns
    /// 
    /// An optional string representing the result of adding the payment method token
    /// 
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

        /// This method performs preprocessing steps by authorizing the current state, the current object, and the given connector data. It then awaits the authorization result and returns the modified state.
    async fn preprocessing_steps<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        authorize_preprocessing_steps(state, &self, true, connector).await
    }

        /// Asynchronously creates a connector customer using the provided `AppState` and `ConnectorData`.
    /// Returns a `RouterResult` containing an optional `String` representing the created customer.
    async fn create_connector_customer<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<String>> {
        customers::create_connector_customer(
            state,
            connector,
            self,
            types::ConnectorCustomerData::try_from(self)?,
        )
        .await
    }

        /// Asynchronously builds a specific connector request for the flow, based on the provided state, connector data, and the call connector action. 
    /// This method executes pre-tasks for the connector integration, adds metrics for execution of pre-tasks, and logs the completion of pre-tasks. 
    /// It then decides the authentication type, builds the request using the connector integration, and returns the result along with a boolean indicating whether the request was successfully built or not. 
    /// If the call connector action is not trigger, it returns None and true.
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
                    api::Authorize,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                connector_integration
                    .execute_pretasks(self, state)
                    .await
                    .to_payment_failed_response()?;

                metrics::EXECUTE_PRETASK_COUNT.add(
                    &metrics::CONTEXT,
                    1,
                    &[
                        metrics::request::add_attributes(
                            "connector",
                            connector.connector_name.to_string(),
                        ),
                        metrics::request::add_attributes("flow", format!("{:?}", api::Authorize)),
                    ],
                );

                logger::debug!(completed_pre_tasks=?true);

                if self.should_proceed_with_authorize() {
                    self.decide_authentication_type();
                    logger::debug!(auth_type=?self.auth_type);

                    Ok((
                        connector_integration
                            .build_request(self, &state.conf.connectors)
                            .to_payment_failed_response()?,
                        true,
                    ))
                } else {
                    Ok((None, false))
                }
            }
            _ => Ok((None, true)),
        }
    }
}

impl types::PaymentsAuthorizeRouterData {
        /// Determines the authentication type based on the current auth_type and request.enrolled_for_3ds.
    fn decide_authentication_type(&mut self) {
        if self.auth_type == diesel_models::enums::AuthenticationType::ThreeDs
            && !self.request.enrolled_for_3ds
        {
            self.auth_type = diesel_models::enums::AuthenticationType::NoThreeDs
        }
    }

    /// to decide if we need to proceed with authorize or not, Eg: If any of the pretask returns `redirection_response` then we should not proceed with authorize call
    fn should_proceed_with_authorize(&self) -> bool {
        match &self.response {
            Ok(types::PaymentsResponseData::TransactionResponse {
                redirection_data, ..
            }) => !redirection_data.is_some(),
            _ => true,
        }
    }
}

impl mandate::MandateBehaviour for types::PaymentsAuthorizeData {
        /// Retrieves the amount stored in the object.
    fn get_amount(&self) -> i64 {
        self.amount
    }
        /// This method returns an optional reference to the MandateIds associated with the current object.
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }
        /// This method returns a cloned instance of the payment method data associated with the current object.
    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }
        /// Returns the setup future usage of the current instance, if it exists.
    fn get_setup_future_usage(&self) -> Option<diesel_models::enums::FutureUsage> {
            self.setup_future_usage
        }
        /// This method retrieves the setup mandate details, if available.
    fn get_setup_mandate_details(&self) -> Option<&data_models::mandates::MandateData> {
        self.setup_mandate_details.as_ref()
    }

        /// Sets the mandate ID for the payment.
    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>) {
        self.mandate_id = new_mandate_id;
    }
}

/// Asynchronously authorizes preprocessing steps for payments based on the given parameters. If the confirm flag is true, the method retrieves the connector integration and executes the preprocessing step, updating the metrics accordingly. It then converts the response data into the appropriate router data type and returns it. If the confirm flag is false, it simply returns the input router data. 
pub async fn authorize_preprocessing_steps<F: Clone>(
    state: &AppState,
    router_data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    confirm: bool,
    connector: &api::ConnectorData,
) -> RouterResult<types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>> {
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
                metrics::request::add_attributes(
                    "payment_method_type",
                    router_data
                        .request
                        .payment_method_type
                        .as_ref()
                        .map(|inner| inner.to_string())
                        .unwrap_or("null".to_string()),
                ),
            ],
        );

        let authorize_router_data =
            payments::helpers::router_data_type_conversion::<_, F, _, _, _, _>(
                resp.clone(),
                router_data.request.to_owned(),
                resp.response,
            );

        Ok(authorize_router_data)
    } else {
        Ok(router_data.clone())
    }
}

impl<F> TryFrom<&types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>>
    for types::ConnectorCustomerData
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

        /// Tries to create a new instance of the current struct from the provided RouterData.
    /// If successful, returns the new instance with the specified fields populated with the data from the RouterData.
    fn try_from(
        data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.request.email.clone(),
            payment_method_data: data.request.payment_method_data.clone(),
            description: None,
            phone: None,
            name: data.request.customer_name.clone(),
            preprocessing_id: data.preprocessing_id.clone(),
        })
    }
}

impl TryFrom<types::PaymentsAuthorizeData> for types::PaymentMethodTokenizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

        /// Attempts to create an instance of the current struct from the provided `PaymentsAuthorizeData`.
    /// 
    /// # Arguments
    /// 
    /// * `data` - The `PaymentsAuthorizeData` used to create the instance.
    /// 
    /// # Returns
    /// 
    /// If successful, returns `Ok` with the newly created instance. If an error occurs, returns `Err` with the specific error.
    fn try_from(data: types::PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            browser_info: data.browser_info,
            currency: data.currency,
            amount: Some(data.amount),
        })
    }
}

impl TryFrom<types::PaymentsAuthorizeData> for types::PaymentsPreProcessingData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

        /// Attempts to create a new instance of the current struct from the provided PaymentsAuthorizeData.
    fn try_from(data: types::PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: Some(data.payment_method_data),
            amount: Some(data.amount),
            email: data.email,
            currency: Some(data.currency),
            payment_method_type: data.payment_method_type,
            setup_mandate_details: data.setup_mandate_details,
            capture_method: data.capture_method,
            order_details: data.order_details,
            router_return_url: data.router_return_url,
            webhook_url: data.webhook_url,
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            surcharge_details: data.surcharge_details,
            connector_transaction_id: None,
            redirect_response: None,
        })
    }
}

impl TryFrom<types::CompleteAuthorizeData> for types::PaymentsPreProcessingData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

        /// Attempts to create a new instance of the struct from the provided CompleteAuthorizeData.
    fn try_from(data: types::CompleteAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            amount: Some(data.amount),
            email: data.email,
            currency: Some(data.currency),
            payment_method_type: None,
            setup_mandate_details: data.setup_mandate_details,
            capture_method: data.capture_method,
            order_details: None,
            router_return_url: None,
            webhook_url: None,
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            surcharge_details: None,
            connector_transaction_id: data.connector_transaction_id,
            redirect_response: data.redirect_response,
        })
    }
}
