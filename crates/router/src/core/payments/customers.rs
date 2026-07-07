pub use hyperswitch_domain_models::customer::update_connector_customer_in_customers;
use hyperswitch_interfaces::api::{gateway, ConnectorSpecifications};
use hyperswitch_masking::PeekInterface;
use router_env::{instrument, tracing};

#[cfg(feature = "v2")]
use crate::types::domain;
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, gateway::context as gateway_context},
    },
    logger,
    routes::{metrics, SessionState},
    services,
    types::{self, api},
};

#[instrument(skip_all)]
pub async fn create_connector_customer<F: Clone, T: Clone>(
    state: &SessionState,
    connector: &api::ConnectorData,
    router_data: &types::RouterData<F, T, types::PaymentsResponseData>,
    customer_request_data: types::ConnectorCustomerData,
    gateway_context: &gateway_context::RouterGatewayContext,
) -> RouterResult<Option<String>> {
    let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        api::CreateConnectorCustomer,
        types::ConnectorCustomerData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();

    let customer_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
        Err(types::ErrorResponse::default());

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

    let resp = gateway::execute_payment_gateway(
        state,
        connector_integration,
        &customer_router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
        gateway_context.clone(),
    )
    .await
    .to_payment_failed_response()?;

    metrics::CONNECTOR_CUSTOMER_CREATE.add(
        1,
        router_env::metric_attributes!(("connector", connector.connector_name.to_string())),
    );

    let connector_customer_id = match resp.response {
        Ok(response) => match response {
            types::PaymentsResponseData::ConnectorCustomerResponse(customer_data) => {
                Some(customer_data.connector_customer_id)
            }
            _ => None,
        },
        Err(err) => {
            logger::error!(create_connector_customer_error=?err);
            None
        }
    };

    Ok(connector_customer_id)
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ConnectorCustomerAction<'a> {
    CreateCustomer,
    StoreGeneratedCustomerId(String),
    UseExistingCustomer(Option<&'a str>),
}

#[cfg(feature = "v1")]
pub fn should_call_connector_create_customer<'a>(
    connector: &api::ConnectorData,
    connector_customer_map: Option<&'a common_utils::pii::SecretSerdeValue>,
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    connector_label: &str,
) -> ConnectorCustomerAction<'a> {
    // Check if create customer is required for the connector
    let mca_string = payment_attempt
        .merchant_connector_id
        .clone()
        .map(|mca_id| mca_id.get_string_repr().to_string())
        .unwrap_or_else(|| connector_label.to_string());

    let connector_needs_customer = connector
        .connector
        .should_call_connector_customer(payment_attempt);

    let connector_customer_details = connector_customer_map
        .and_then(|connector_customer_map| connector_customer_map.peek().get(mca_string.as_str()))
        .and_then(|connector_customer| connector_customer.as_str());

    match connector_needs_customer {
        hyperswitch_interfaces::api::ConnectorCustomerAction::CallConnectorCustomer => {
            match connector_customer_details {
                Some(existing_customer_id) => {
                    ConnectorCustomerAction::UseExistingCustomer(Some(existing_customer_id))
                }
                None => ConnectorCustomerAction::CreateCustomer,
            }
        }
        hyperswitch_interfaces::api::ConnectorCustomerAction::NoAction => {
            ConnectorCustomerAction::UseExistingCustomer(connector_customer_details)
        }
        hyperswitch_interfaces::api::ConnectorCustomerAction::GeneratedCustomerId(customer_id) => {
            match connector_customer_details {
                Some(existing_customer_id) => {
                    ConnectorCustomerAction::UseExistingCustomer(Some(existing_customer_id))
                }
                None => ConnectorCustomerAction::StoreGeneratedCustomerId(customer_id),
            }
        }
    }
}

#[cfg(feature = "v2")]
pub fn should_call_connector_create_customer<'a>(
    connector: &api::ConnectorData,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
) -> (bool, Option<&'a str>) {
    // Check if create customer is required for the connector
    match merchant_connector_account {
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(_) => {
            let connector_needs_customer = connector.connector.should_call_connector_customer();

            match connector_needs_customer {
                hyperswitch_interfaces::api::ConnectorCustomerAction::CallConnectorCustomer => {
                    let connector_customer_details = customer.as_ref().and_then(|cust| {
                        cust.get_connector_customer_id(merchant_connector_account)
                    });
                    let should_call_connector = connector_customer_details.is_none();
                    (should_call_connector, connector_customer_details)
                }
                _ => (false, None),
            }
        }

        // TODO: Construct connector_customer for MerchantConnectorDetails if required by connector.
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            todo!("Handle connector_customer construction for MerchantConnectorDetails");
        }
    }
}
