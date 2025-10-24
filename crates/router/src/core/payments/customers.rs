pub use hyperswitch_domain_models::customer::update_connector_customer_in_customers;
use hyperswitch_interfaces::api::ConnectorSpecifications;
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments,
    },
    logger,
    routes::{metrics, SessionState},
    services,
    types::{self, api, domain},
};

#[instrument(skip_all)]
pub async fn create_connector_customer<F: Clone, T: Clone>(
    state: &SessionState,
    connector: &api::ConnectorData,
    router_data: &types::RouterData<F, T, types::PaymentsResponseData>,
    customer_request_data: types::ConnectorCustomerData,
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

    let resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &customer_router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
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
pub fn should_call_connector_create_customer<'a>(
    connector: &api::ConnectorData,
    customer: &'a Option<domain::Customer>,
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    connector_label: &str,
) -> (bool, Option<&'a str>) {
    // Check if create customer is required for the connector
    let connector_needs_customer = connector
        .connector
        .should_call_connector_customer(payment_attempt);
    let connector_customer_details = customer
        .as_ref()
        .and_then(|customer| customer.get_connector_customer_id(connector_label));
    if connector_needs_customer {
        let should_call_connector = connector_customer_details.is_none();
        (should_call_connector, connector_customer_details)
    } else {
        // Populates connector_customer_id if it is present after data migration
        // For connector which does not have create connector customer flow
        (false, connector_customer_details)
    }
}

#[cfg(feature = "v2")]
pub fn should_call_connector_create_customer<'a>(
    connector: &api::ConnectorData,
    customer: &'a Option<domain::Customer>,
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
) -> (bool, Option<&'a str>) {
    // Check if create customer is required for the connector
    match merchant_connector_account {
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(_) => {
            let connector_needs_customer = connector
                .connector
                .should_call_connector_customer(payment_attempt);

            if connector_needs_customer {
                let connector_customer_details = customer
                    .as_ref()
                    .and_then(|cust| cust.get_connector_customer_id(merchant_connector_account));
                let should_call_connector = connector_customer_details.is_none();
                (should_call_connector, connector_customer_details)
            } else {
                (false, None)
            }
        }

        // TODO: Construct connector_customer for MerchantConnectorDetails if required by connector.
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            todo!("Handle connector_customer construction for MerchantConnectorDetails");
        }
    }
}
