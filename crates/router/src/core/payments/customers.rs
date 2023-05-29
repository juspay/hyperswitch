use common_utils::ext_traits::ValueExt;
use error_stack::{self, ResultExt};

use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        payments,
    },
    logger,
    routes::{metrics, AppState},
    services,
    types::{self, api, storage},
};

pub async fn create_connector_customer<F: Clone, T: Clone>(
    state: &AppState,
    connector: &api::ConnectorData,
    router_data: &types::RouterData<F, T, types::PaymentsResponseData>,
    customer_request_data: types::ConnectorCustomerData,
    connector_customer_map: Option<serde_json::Map<String, serde_json::Value>>,
) -> RouterResult<(Option<String>, Option<storage::CustomerUpdate>)> {
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
    )
    .await
    .map_err(|error| error.to_payment_failed_response())?;

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
