use std::marker::PhantomData;

use api_models::merchant_connector_webhook_management::{
    ConnectorWebhookRegisterRequest as ApiConnectorWebhookRegisterRequest,
    RegisterConnectorWebhookResponse, Scope, ScopeIdentifier, ScopeType, WebhookRegistrationResult,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    connector_endpoints::Connectors,
    router_request_types::merchant_connector_webhook_management::ConnectorWebhookRegisterRequest,
};
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

fn is_webhook_auto_configuration_supported_from_toml(
    connector_name: types::Connector,
) -> Option<bool> {
    connector_configs::connector::ConnectorConfig::get_connector_config(connector_name)
        .ok()
        .flatten()
        .and_then(|toml| toml.connector_webhook_register_details)
        .map(|details| details.webhook_auto_configuration_supported)
}

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
    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let (payment_method, payment_method_type) = match &webhook_register_request.scope {
        ScopeIdentifier::PaymentMethodType(pmt) => {
            let pm = common_enums::PaymentMethod::from(*pmt);
            (pm, Some(*pmt))
        }
        _ => (common_enums::PaymentMethod::default(), None),
    };

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
        payment_method,
        payment_method_type,
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
        request: webhook_register_request,
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

#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectorWebhookRegistrationEntry {
    NotSpecific,
    PaymentMethodType {
        value: common_enums::PaymentMethodType,
    },
    EventType {
        value: common_enums::EventType,
    },
}

#[cfg(feature = "v1")]
pub fn construct_connector_webhook_registration_details(
    register_webhook_response: &ConnectorWebhookRegisterResponse,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    connector_webhook_register_data: &ConnectorWebhookRegisterData,
) -> RouterResult<domain::MerchantConnectorAccountUpdate> {
    if let Some(connector_webhook_id) = register_webhook_response.connector_webhook_id.clone() {
        let mut connector_webhook_registration_details = merchant_connector_account
            .get_connector_webhook_registration_details()
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

        let map = connector_webhook_registration_details
            .as_object_mut()
            .ok_or(errors::ApiErrorResponse::InternalServerError)?;

        let entry_value = match &connector_webhook_register_data.scope {
            ScopeIdentifier::NotSpecific => {
                serde_json::to_value(ConnectorWebhookRegistrationEntry::NotSpecific)
            }
            ScopeIdentifier::PaymentMethodType(pmt) => {
                serde_json::to_value(ConnectorWebhookRegistrationEntry::PaymentMethodType {
                    value: *pmt,
                })
            }
            ScopeIdentifier::EventType(evt) => {
                serde_json::to_value(ConnectorWebhookRegistrationEntry::EventType { value: *evt })
            }
        }
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

        map.insert(connector_webhook_id, entry_value);

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
    webhook_register_request: ApiConnectorWebhookRegisterRequest,
    connectors: &Connectors,
) -> RouterResult<()> {
    let config = connector_data.connector.get_api_webhook_config();

    let is_supported =
        is_webhook_auto_configuration_supported_from_toml(connector_data.connector_name)
            .unwrap_or(config.is_webhook_auto_configuration_supported);

    if !is_supported {
        return Err(errors::ApiErrorResponse::FlowNotSupported {
            flow: "Webhook Registration".to_string(),
            connector: connector_data.connector_name.to_string(),
        }
        .into());
    }

    let plan = connector_data
        .connector
        .get_webhook_registration_plan(&webhook_register_request.scope, connectors);

    if plan.is_empty() {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Webhook registration is not supported for the requested scope".to_string(),
        }
        .into());
    }

    Ok(())
}

pub fn determine_scope_type(scope: &Scope) -> ScopeType {
    match scope {
        Scope::NotSpecific => ScopeType::NotSpecific,
        Scope::PaymentMethodTypes(_) => ScopeType::PaymentMethodType,
        Scope::EventTypes(_) => ScopeType::EventType,
        _ => ScopeType::NotSpecific,
    }
}

pub fn extract_requested_identifiers(scope: &Scope) -> Vec<ScopeIdentifier> {
    match scope {
        Scope::NotSpecific => vec![ScopeIdentifier::NotSpecific],
        Scope::PaymentMethodTypes(pmts) => pmts
            .iter()
            .map(|pmt| ScopeIdentifier::PaymentMethodType(*pmt))
            .collect(),
        Scope::EventTypes(evts) => evts
            .iter()
            .map(|evt| ScopeIdentifier::EventType(*evt))
            .collect(),
        _ => vec![ScopeIdentifier::NotSpecific],
    }
}

#[cfg(feature = "v1")]
pub fn construct_connector_webhook_registration_response(
    results: Vec<WebhookRegistrationResult>,
    scope_type: ScopeType,
    requested: Vec<ScopeIdentifier>,
) -> RouterResult<RegisterConnectorWebhookResponse> {
    Ok(RegisterConnectorWebhookResponse {
        scope_type,
        requested,
        results,
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
