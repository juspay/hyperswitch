use async_trait::async_trait;
use common_utils::ext_traits::ValueExt;
use error_stack::{self, ResultExt};

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        mandate,
        payments::{self, access_token, tokenization, transformers, PaymentData},
    },
    logger,
    routes::{metrics, AppState},
    services,
    types::{self, api, storage},
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
        customer: &Option<storage::Customer>,
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
            customer,
        )
        .await
    }
}
#[async_trait]
impl Feature<api::Authorize, types::PaymentsAuthorizeData> for types::PaymentsAuthorizeRouterData {
    async fn decide_flows<'a>(
        mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<Self> {
        let resp = self
            .decide_flow(
                state,
                connector,
                customer,
                Some(true),
                call_connector_action,
                merchant_account,
            )
            .await;

        metrics::PAYMENT_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics

        resp
    }

    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

    async fn add_payment_method_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        tokenization_action: &payments::TokenizationAction,
    ) -> RouterResult<Option<String>> {
        tokenization::add_payment_method_token(
            state,
            connector,
            tokenization_action,
            self,
            types::PaymentMethodTokenizationData::try_from(self.request.to_owned())?,
        )
        .await
    }

    async fn create_connector_customer<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        customer: &Option<storage::Customer>,
    ) -> RouterResult<(Option<String>, Option<storage::CustomerUpdate>)> {
        create_connector_customer(state, connector, customer, self).await
    }
}

impl types::PaymentsAuthorizeRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b mut self,
        state: &'a AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<storage::Customer>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<Self> {
        match confirm {
            Some(true) => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::Authorize,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();
                connector_integration
                    .execute_pretasks(self, state)
                    .await
                    .map_err(|error| error.to_payment_failed_response())?;
                logger::debug!(completed_pre_tasks=?true);
                if self.should_proceed_with_authorize() {
                    self.decide_authentication_type();
                    logger::debug!(auth_type=?self.auth_type);
                    let resp = services::execute_connector_processing_step(
                        state,
                        connector_integration,
                        self,
                        call_connector_action,
                    )
                    .await
                    .map_err(|error| error.to_payment_failed_response())?;

                    let pm_id = tokenization::save_payment_method(
                        state,
                        connector,
                        resp.to_owned(),
                        maybe_customer,
                        merchant_account,
                    )
                    .await?;

                    Ok(mandate::mandate_procedure(state, resp, maybe_customer, pm_id).await?)
                } else {
                    Ok(self.clone())
                }
            }
            _ => Ok(self.clone()),
        }
    }

    fn decide_authentication_type(&mut self) {
        if self.auth_type == storage_models::enums::AuthenticationType::ThreeDs
            && !self.request.enrolled_for_3ds
        {
            self.auth_type = storage_models::enums::AuthenticationType::NoThreeDs
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

pub enum Action {
    Update,
    Insert,
    Skip,
}

impl mandate::MandateBehaviour for types::PaymentsAuthorizeData {
    fn get_amount(&self) -> i64 {
        self.amount
    }
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }
    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }
    fn get_setup_future_usage(&self) -> Option<storage_models::enums::FutureUsage> {
        self.setup_future_usage
    }
    fn get_setup_mandate_details(&self) -> Option<&api_models::payments::MandateData> {
        self.setup_mandate_details.as_ref()
    }

    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>) {
        self.mandate_id = new_mandate_id;
    }
}

pub async fn update_connector_customer_in_customers(
    connector: &api::ConnectorData,
    connector_customer_map: Option<serde_json::Map<String, serde_json::Value>>,
    connector_cust_id: &Option<String>,
) -> RouterResult<Option<storage::CustomerUpdate>> {
    let mut connector_customer = match connector_customer_map {
        Some(cc) => cc,
        None => serde_json::Map::new(),
    };
    connector_cust_id.clone().map(|cc| {
        connector_customer.insert(
            connector.connector_name.to_string(),
            serde_json::Value::String(cc),
        )
    });
    Ok(Some(storage::CustomerUpdate::ConnectorCustomer {
        connector_customer: Some(serde_json::Value::Object(connector_customer)),
    }))
}

type CreateCustomerCheck = (
    bool,
    Option<String>,
    Option<serde_json::Map<String, serde_json::Value>>,
);
pub fn should_call_connector_create_customer(
    state: &AppState,
    connector: &api::ConnectorData,
    customer: &Option<storage::Customer>,
) -> RouterResult<CreateCustomerCheck> {
    let connector_name = connector.connector_name.to_string();
    //Check if create customer is required for the connector
    let connector_customer_filter = state
        .conf
        .connector_customer
        .connector_list
        .contains(&connector.connector_name);
    if connector_customer_filter {
        match customer {
            Some(customer) => match &customer.connector_customer {
                Some(connector_customer) => {
                    let connector_customer_map: serde_json::Map<String, serde_json::Value> =
                        connector_customer
                            .clone()
                            .parse_value("Map<String, Value>")
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to deserialize Value to CustomerConnector")?;
                    let value = connector_customer_map.get(&connector_name); //Check if customer already created for this customer and for this connector
                    Ok((
                        value.is_none(),
                        value.and_then(|val| val.as_str().map(|cust| cust.to_string())),
                        Some(connector_customer_map),
                    ))
                }
                None => Ok((true, None, None)),
            },
            None => Ok((false, None, None)),
        }
    } else {
        Ok((false, None, None))
    }
}

pub async fn create_connector_customer<F: Clone>(
    state: &AppState,
    connector: &api::ConnectorData,
    customer: &Option<storage::Customer>,
    router_data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
) -> RouterResult<(Option<String>, Option<storage::CustomerUpdate>)> {
    let (is_eligible, connector_customer_id, connector_customer_map) =
        should_call_connector_create_customer(state, connector, customer)?;

    if is_eligible {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::CreateConnectorCustomer,
            types::ConnectorCustomerData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let customer_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
            Err(types::ErrorResponse::default());

        let customer_request_data =
            types::ConnectorCustomerData::try_from(router_data.request.to_owned())?;

        let customer_router_data = payments::helpers::router_data_type_conversion::<
            _,
            api::CreateConnectorCustomer,
            _,
            _,
            _,
            _,
        >(
            router_data.clone(),
            customer_request_data,
            customer_response_data,
        );

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &customer_router_data,
            payments::CallConnectorAction::Trigger,
        )
        .await
        .map_err(|error| error.to_payment_failed_response())?;

        let connector_customer_id = match resp.response {
            Ok(response) => match response {
                types::PaymentsResponseData::ConnectorCustomerResponse {
                    connector_customer_id,
                } => Some(connector_customer_id),
                _ => None,
            },
            Err(err) => {
                logger::debug!(payment_method_tokenization_error=?err);
                None
            }
        };
        let update_customer = update_connector_customer_in_customers(
            connector,
            connector_customer_map,
            &connector_customer_id,
        )
        .await?;
        Ok((connector_customer_id, update_customer))
    } else {
        Ok((connector_customer_id, None))
    }
}

impl TryFrom<types::PaymentsAuthorizeData> for types::ConnectorCustomerData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(data: types::PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.email,
            description: None,
            phone: None,
            name: None,
        })
    }
}

pub async fn add_payment_method_token<F: Clone>(
    state: &AppState,
    connector: &api::ConnectorData,
    tokenization_action: &payments::TokenizationAction,
    router_data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
) -> RouterResult<Option<String>> {
    match tokenization_action {
        payments::TokenizationAction::TokenizeInConnector => {
            let connector_integration: services::BoxedConnectorIntegration<
                '_,
                api::PaymentMethodToken,
                types::PaymentMethodTokenizationData,
                types::PaymentsResponseData,
            > = connector.connector.get_connector_integration();

            let pm_token_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
                Err(types::ErrorResponse::default());

            let pm_token_request_data =
                types::PaymentMethodTokenizationData::try_from(router_data.request.to_owned())?;

            let pm_token_router_data = payments::helpers::router_data_type_conversion::<
                _,
                api::PaymentMethodToken,
                _,
                _,
                _,
                _,
            >(
                router_data.clone(),
                pm_token_request_data,
                pm_token_response_data,
            );
            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                &pm_token_router_data,
                payments::CallConnectorAction::Trigger,
            )
            .await
            .map_err(|error| error.to_payment_failed_response())?;

            let pm_token = match resp.response {
                Ok(response) => match response {
                    types::PaymentsResponseData::TokenizationResponse { token } => Some(token),
                    _ => None,
                },
                Err(err) => {
                    logger::debug!(payment_method_tokenization_error=?err);
                    None
                }
            };
            Ok(pm_token)
        }
        _ => Ok(None),
    }
}

impl TryFrom<types::PaymentsAuthorizeData> for types::PaymentMethodTokenizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(data: types::PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
        })
    }
}
