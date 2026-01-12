use std::marker::PhantomData;

use api_models::merchant_connector_webhook_management::{
    ConnectorWebhookRegisterRequest, RegisterConnectorWebhookResponse,
};
use error_stack::ResultExt;
use hyperswitch_interfaces::api::ConnectorSpecifications;
use router_env::tracing::{self, instrument};

use crate::{
    consts,
    core::errors::RouterResult,
    errors, types,
    types::{
        api::ConnectorData, domain,
        ConnectorWebhookRegisterRequest as ConnectorWebhookRegisterData,
        ConnectorWebhookRegisterResponse, ConnectorWebhookRegisterRouterData, ErrorResponse,
    },
    SessionState,
};

#[cfg(feature = "v2")]
pub async fn construct_webhook_register_router_data(
    _state: &SessionState,
    _merchant_connector_account: domain::MerchantConnectorAccount,
    _webhook_register_request: ConnectorWebhookRegisterRequest,
) -> RouterResult<types::ConnectorWebhookRegisterRouterData> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_webhook_register_router_data<'a>(
    state: &'a SessionState,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    webhook_register_request: ConnectorWebhookRegisterRequest,
) -> RouterResult<ConnectorWebhookRegisterRouterData> {
    let merchant_id = merchant_connector_account.merchant_id.get_string_repr();
    let merchant_connector_id = merchant_connector_account
        .merchant_connector_id
        .get_string_repr();
    let router_base_url = state.base_url.clone();
    let request = ConnectorWebhookRegisterData {
        webhook_url: format!("{router_base_url}/webhooks/{merchant_id}/{merchant_connector_id}"),
        event_type: webhook_register_request.event_type,
    };

    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_connector_account.merchant_id.clone(),
        customer_id: None,
        connector_customer: None,
        connector: merchant_connector_account.connector_name.clone(),
        payment_id: consts::IRRELEVANT_PAYMENT_INTENT_ID.to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: consts::IRRELEVANT_PAYMENT_ATTEMPT_ID.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method: common_enums::PaymentMethod::default(),
        payment_method_type: None,
        connector_auth_type: auth_type,
        description: None,
        address: types::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata().clone(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request,
        response: Err(ErrorResponse::default()),
        connector_request_reference_id: consts::IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID
            .to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: None,
        refund_id: None,
        payment_method_status: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
    })
}

#[cfg(feature = "v1")]
pub fn construct_connector_webhook_registration_details(
    register_webhook_response: &ConnectorWebhookRegisterResponse,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    connector_webhook_register_data: &ConnectorWebhookRegisterData,
) -> RouterResult<domain::MerchantConnectorAccountUpdate> {
    if let Some(connector_webhook_id) = register_webhook_response.connector_webhook_id.clone() {
        let webhook_event = connector_webhook_register_data.event_type;

        let connector_webhook_value = serde_json::to_value(domain::ConnectorWebhookData {
            event_type: webhook_event,
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let mut connector_webhook_registration_details = merchant_connector_account
            .get_connector_webhook_registration_details()
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

        let map = connector_webhook_registration_details
            .as_object_mut()
            .ok_or(errors::ApiErrorResponse::InternalServerError)?;

        map.insert(connector_webhook_id, connector_webhook_value);

        Ok(
            domain::MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
                connector_webhook_registration_details: Some(
                    connector_webhook_registration_details,
                ),
            },
        )
    } else {
        Ok(
            domain::MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
                connector_webhook_registration_details: None,
            },
        )
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn validate_webhook_registration_request(
    connector_data: &ConnectorData,
    webhook_register_request: ConnectorWebhookRegisterRequest,
) -> RouterResult<()> {
    let config = connector_data.connector.get_api_webhook_config();

    if !config.is_webhook_auto_configuration_supported {
        Err(errors::ApiErrorResponse::FlowNotSupported {
            flow: "Webhook Registration".to_string(),
            connector: connector_data.connector_name.to_string(),
        }
        .into())
    } else {
        let is_supported = match webhook_register_request.event_type {
            common_enums::ConnectorWebhookEventType::Standard => {
                matches!(
                    config.config_type,
                    Some(
                        common_types::connector_webhook_configuration::WebhookConfigType::Standard
                    )
                )
            }

            common_enums::ConnectorWebhookEventType::SpecificEvent(event) => {
                matches!(
                    config.config_type,
                    Some(common_types::connector_webhook_configuration::WebhookConfigType::CustomEvents(
                        ref supported_events
                    )) if supported_events.contains(&event)
                )
            }
        };

        if !is_supported {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Webhook registration is not supported for the specified event type"
                    .to_string(),
            }
            .into());
        }

        Ok(())
    }
}

#[cfg(feature = "v1")]
pub fn construct_connector_webhook_registration_response(
    register_webhook_response: &ConnectorWebhookRegisterResponse,
    connector_webhook_register_data: &ConnectorWebhookRegisterData,
) -> RouterResult<RegisterConnectorWebhookResponse> {
    Ok(RegisterConnectorWebhookResponse {
        event_type: connector_webhook_register_data.event_type,
        connector_webhook_id: register_webhook_response.connector_webhook_id.clone(),
        webhook_registration_status: register_webhook_response.status,
        error_code: register_webhook_response.error_code.clone(),
        error_message: register_webhook_response.error_message.clone(),
    })
}
