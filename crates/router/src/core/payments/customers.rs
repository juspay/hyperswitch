use common_utils::pii;
use masking::ExposeOptionInterface;
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments,
    },
    logger,
    routes::{metrics, SessionState},
    services,
    types::{self, api, domain, storage},
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
    )
    .await
    .to_payment_failed_response()?;

    metrics::CONNECTOR_CUSTOMER_CREATE.add(
        1,
        router_env::metric_attributes!(("connector", connector.connector_name.to_string())),
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

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub fn should_call_connector_create_customer<'a>(
    state: &SessionState,
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
        let connector_customer_details = customer
            .as_ref()
            .and_then(|customer| customer.get_connector_customer_id(connector_label));
        let should_call_connector = connector_customer_details.is_none();
        (should_call_connector, connector_customer_details)
    } else {
        (false, None)
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub fn should_call_connector_create_customer<'a>(
    state: &SessionState,
    connector: &api::ConnectorData,
    customer: &'a Option<domain::Customer>,
    merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
) -> (bool, Option<&'a str>) {
    // Check if create customer is required for the connector
    let connector_needs_customer = state
        .conf
        .connector_customer
        .connector_list
        .contains(&connector.connector_name);

    if connector_needs_customer {
        let connector_customer_details = customer
            .as_ref()
            .and_then(|customer| customer.get_connector_customer_id(merchant_connector_id));
        let should_call_connector = connector_customer_details.is_none();
        (should_call_connector, connector_customer_details)
    } else {
        (false, None)
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument]
pub async fn update_connector_customer_in_customers(
    connector_label: &str,
    customer: Option<&domain::Customer>,
    connector_customer_id: Option<String>,
) -> Option<storage::CustomerUpdate> {
    let mut connector_customer_map = customer
        .and_then(|customer| customer.connector_customer.clone().expose_option())
        .and_then(|connector_customer| connector_customer.as_object().cloned())
        .unwrap_or_default();

    let updated_connector_customer_map = connector_customer_id.map(|connector_customer_id| {
        let connector_customer_value = serde_json::Value::String(connector_customer_id);
        connector_customer_map.insert(connector_label.to_string(), connector_customer_value);
        connector_customer_map
    });

    updated_connector_customer_map
        .map(serde_json::Value::Object)
        .map(
            |connector_customer_value| storage::CustomerUpdate::ConnectorCustomer {
                connector_customer: Some(pii::SecretSerdeValue::new(connector_customer_value)),
            },
        )
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[instrument]
pub async fn update_connector_customer_in_customers(
    merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    customer: Option<&domain::Customer>,
    connector_customer_id: Option<String>,
) -> Option<storage::CustomerUpdate> {
    connector_customer_id.map(|connector_customer_id| {
        let mut connector_customer_map = customer
            .and_then(|customer| customer.connector_customer.clone())
            .unwrap_or_default();
        connector_customer_map.insert(merchant_connector_id, connector_customer_id);

        storage::CustomerUpdate::ConnectorCustomer {
            connector_customer: Some(connector_customer_map),
        }
    })
}
