use std::marker::PhantomData;

use api_models::merchant_connector_webhook_management::{
    ConnectorWebhookRegisterRequest, RegisterConnectorWebhookResponse,
};
use common_utils::ext_traits::{Encode, ValueExt};
use error_stack::ResultExt;
use hyperswitch_interfaces::api::ConnectorSpecifications;
use hyperswitch_masking::{ExposeInterface, Secret};
use router_env::tracing::{self, instrument};

use crate::{
    consts,
    core::{errors::RouterResult, payments::helpers},
    errors, types,
    types::{
        api::ConnectorData, domain,
        ConnectorWebhookGenerateSecretRequest as ConnectorWebhookGenerateSecretData,
        ConnectorWebhookGenerateSecretResponse, ConnectorWebhookGenerateSecretRouterData,
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
    let merchant_connector_id = merchant_connector_account
        .merchant_connector_id
        .get_string_repr();
    let request = ConnectorWebhookRegisterData {
        webhook_url: helpers::create_webhook_url(
            &state.base_url,
            &merchant_connector_account.merchant_id,
            merchant_connector_id,
        ),
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
        payout_id: None,
        customer_document_details: None,
        feature_data: None,
        sender_payment_instrument_id: None,
    })
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_generate_secret_router_data<'a>(
    state: &'a SessionState,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    connector_webhook_id: String,
) -> RouterResult<ConnectorWebhookGenerateSecretRouterData> {
    let request = ConnectorWebhookGenerateSecretData {
        connector_webhook_id,
    };

    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get connector auth details for HMAC generation")?;

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
        payout_id: None,
        customer_document_details: None,
        feature_data: None,
        sender_payment_instrument_id: None,
    })
}

#[cfg(feature = "v1")]
pub fn construct_connector_webhook_registration_details(
    register_webhook_response: &ConnectorWebhookRegisterResponse,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    connector_webhook_register_data: &ConnectorWebhookRegisterData,
    generated_secret: Option<Secret<String>>,
) -> RouterResult<domain::MerchantConnectorAccountUpdate> {
    let connector_webhook_registration_details = if let Some(connector_webhook_id) =
        register_webhook_response.connector_webhook_id.clone()
    {
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

        Some(connector_webhook_registration_details)
    } else {
        None
    };

    let connector_webhook_details = generated_secret
        .map(|secret| {
            build_connector_webhook_details_with_secret(merchant_connector_account, secret)
        })
        .transpose()?;

    Ok(
        domain::MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
            connector_webhook_registration_details,
            connector_webhook_details,
        },
    )
}

#[cfg(feature = "v1")]
fn build_connector_webhook_details_with_secret(
    merchant_connector_account: &domain::MerchantConnectorAccount,
    secret: Secret<String>,
) -> RouterResult<common_utils::pii::SecretSerdeValue> {
    let existing_additional_secret = merchant_connector_account
        .connector_webhook_details
        .as_ref()
        .and_then(|details| {
            details
                .clone()
                .expose()
                .parse_value::<api_models::admin::MerchantConnectorWebhookDetails>(
                    "MerchantConnectorWebhookDetails",
                )
                .inspect_err(|err| {
                    router_env::logger::warn!(
                        ?err,
                        "Failed to parse existing MerchantConnectorWebhookDetails; \
                         dropping additional_secret while persisting generated secret"
                    );
                })
                .ok()
                .and_then(|parsed| parsed.additional_secret)
        });

    let merged = api_models::admin::MerchantConnectorWebhookDetails {
        merchant_secret: secret,
        additional_secret: existing_additional_secret,
    };

    let serialized = merged
        .encode_to_value()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize MerchantConnectorWebhookDetails with generated secret")?;

    Ok(Secret::new(serialized))
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
            common_enums::ConnectorWebhookEventType::AllEvents => {
                matches!(
                    config.config_type,
                    Some(
                        common_types::connector_webhook_configuration::WebhookConfigType::AllEvents
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
    generate_secret_response: Option<&ConnectorWebhookGenerateSecretResponse>,
) -> RouterResult<RegisterConnectorWebhookResponse> {
    let (secret_generation_status, secret_error) = generate_secret_response
        .map(|resp| {
            let secret_error = (resp.error_code.is_some() || resp.error_message.is_some()).then(
                || api_models::merchant_connector_webhook_management::ConnectorErrorDetails {
                    code: resp.error_code.clone(),
                    message: resp.error_message.clone(),
                },
            );
            (Some(resp.status), secret_error)
        })
        .unwrap_or((None, None));

    let connector_error = (register_webhook_response.error_code.is_some()
        || register_webhook_response.error_message.is_some())
    .then(
        || api_models::merchant_connector_webhook_management::ConnectorErrorDetails {
            code: register_webhook_response.error_code.clone(),
            message: register_webhook_response.error_message.clone(),
        },
    );

    Ok(RegisterConnectorWebhookResponse {
        event_type: connector_webhook_register_data.event_type,
        connector_webhook_id: register_webhook_response.connector_webhook_id.clone(),
        webhook_registration_status: register_webhook_response.status,
        connector_error,
        secret_generation_status,
        secret_error,
    })
}
#[cfg(feature = "v1")]
pub fn get_connector_webhook_list_response(
    register_webhook_response: &Option<serde_json::Value>,
) -> RouterResult<Vec<api_models::merchant_connector_webhook_management::ConnectorWebhookResponse>>
{
    use std::collections::HashMap;

    let webhook_map: HashMap<String, domain::ConnectorWebhookData> = match register_webhook_response
    {
        Some(webhook_response) => serde_json::from_value(webhook_response.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
        None => HashMap::new(),
    };

    let webhooks = webhook_map
        .into_iter()
        .map(|(connector_webhook_id, webhook_data)| {
            api_models::merchant_connector_webhook_management::ConnectorWebhookResponse {
                event_type: webhook_data.event_type,
                connector_webhook_id,
            }
        })
        .collect();

    Ok(webhooks)
}
