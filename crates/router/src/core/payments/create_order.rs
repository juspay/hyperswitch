use std::fmt::Debug;

use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use hyperswitch_domain_models::router_request_types::PaymentsAuthorizeData;

use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        payments,
    },
    routes::{metrics, SessionState},
    services::{self, logger},
    types::{self, api, domain},
};

pub async fn create_order_at_connector<F: Clone, T: Debug + Clone>(
    router_data: types::RouterData<F, T, types::PaymentsResponseData>,
    state: &SessionState,
    connector: &api::ConnectorData,
    should_continue_payment: bool,
) -> RouterResult<types::CreateOrderResult> {
    if connector
        .connector_name
        .supports_create_order(router_data.payment_method) && should_continue_payment
    {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::CreateOrder,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let request_data = types::CreateOrderRequestData::try_from(router_data.request.clone())?;

        let response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
            Err(types::ErrorResponse::default());

        let createorder_router_data =
            payments::helpers::router_data_type_conversion::<_, api::CreateOrder, _, _, _, _>(
                router_data.clone(),
                request_data,
                response_data,
            );

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &createorder_router_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_payment_failed_response()?;

        let create_order_resp = resp.response.map(|res| {
            if let types::PaymentsResponseData::PaymentsCreateOrderResponse { order_id } = res {
                Some(order_id)
            } else {
                None
            }
        });

        Ok(types::CreateOrderResult {
            create_order_result: create_order_resp,
            is_create_order_performed: true,
            connector_response: resp.connector_response.clone(),
        })
    } else {
        Ok(types::CreateOrderResult {
            create_order_result: Ok(None),
            is_create_order_performed: false,
            connector_response: None,
        })
    }
}

pub fn update_router_data_with_create_order_result<F, T>(
    create_order_result: types::CreateOrderResult,
    router_data: &mut types::RouterData<F, T, types::PaymentsResponseData>,
    should_continue_further: bool,
) -> bool {
    if create_order_result.is_create_order_performed {
        match create_order_result.create_order_result {
            Ok(order_id) => {
                router_data.request.order_id = order_id;
                router_data.response = Ok(types::PaymentsResponseData::PaymentsCreateOrderResponse {
                    order_id: order_id.unwrap(),
                });
                true
            }
            Err(err) => {
                router_data.response = Err(err.clone());
                false
            }
        }
    } else {
        should_continue_further
    }
}

