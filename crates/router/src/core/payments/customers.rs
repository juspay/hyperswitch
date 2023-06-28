use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments,
    },
    logger,
    routes::{metrics, AppState},
    services,
    types::{self, api, domain, storage},
};

#[instrument(skip_all)]
pub async fn create_connector_customer<F: Clone, T: Clone>(
    state: &AppState,
    connector: &api::ConnectorData,
    router_data: &types::RouterData<F, T, types::PaymentsResponseData>,
    customer_request_data: types::ConnectorCustomerData,
) -> RouterResult<Option<String>> {
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
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
    )
    .await
    .to_payment_failed_response()?;

    metrics::CONNECTOR_CUSTOMER_CREATE.add(
        &metrics::CONTEXT,
        1,
        &[metrics::request::add_attributes(
            "connector",
            connector.connector_name.to_string(),
        )],
    );

    let connector_customer_id = match resp.response {
        Ok(response) => match response {
            types::PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id,
            } => Some(connector_customer_id),
            _ => None,
        },
        Err(err) => {
            logger::error!(create_connector_customer_error=?err);
            None
        }
    };

    Ok(connector_customer_id)
}

pub fn get_connector_customer_details_if_present<'a>(
    customer: &'a domain::Customer,
    connector_name: &str,
) -> Option<&'a str> {
    customer
        .connector_customer
        .as_ref()
        .and_then(|connector_customer_value| connector_customer_value.get(connector_name))
        .and_then(|connector_customer| connector_customer.as_str())
}

pub fn should_call_connector_create_customer<'a>(
    state: &AppState,
    connector: &api::ConnectorData,
    customer: &'a Option<domain::Customer>,
    connector_label: &str,
) -> (bool, Option<&'a str>) {
    // Check if create customer is required for the connector
    let connector_needs_customer = state
        .conf
        .connector_customer
        .connector_list
        .contains(&connector.connector_name);

    if connector_needs_customer {
        let connector_customer_details = customer.as_ref().and_then(|customer| {
            get_connector_customer_details_if_present(customer, connector_label)
        });
        let should_call_connector = connector_customer_details.is_none();
        (should_call_connector, connector_customer_details)
    } else {
        (false, None)
    }
}

#[instrument]
pub async fn update_connector_customer_in_customers(
    connector_label: &str,
    customer: Option<&domain::Customer>,
    connector_customer_id: &Option<String>,
) -> Option<storage::CustomerUpdate> {
    let connector_customer_map = customer
        .and_then(|customer| customer.connector_customer.as_ref())
        .and_then(|connector_customer| connector_customer.as_object())
        .map(ToOwned::to_owned)
        .unwrap_or(serde_json::Map::new());

    let updated_connector_customer_map =
        connector_customer_id.as_ref().map(|connector_customer_id| {
            let mut connector_customer_map = connector_customer_map;
            let connector_customer_value =
                serde_json::Value::String(connector_customer_id.to_string());
            connector_customer_map.insert(connector_label.to_string(), connector_customer_value);
            connector_customer_map
        });

    updated_connector_customer_map
        .map(serde_json::Value::Object)
        .map(
            |connector_customer_value| storage::CustomerUpdate::ConnectorCustomer {
                connector_customer: Some(connector_customer_value),
            },
        )
}
