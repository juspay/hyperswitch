use std::str::FromStr;

use common_utils::{ext_traits::OptionExt, id_type};
use error_stack::ResultExt;
use hyperswitch_domain_models::{router_data::ErrorResponse, types};

use crate::{
    core::payments,
    db::{
        domain,
        errors::{self, RouterResult},
    },
    routes::SessionState,
};

const IRRELEVANT_PAYMENT_INTENT_ID: &str = "irrelevant_payment_intent_id";

const IRRELEVANT_PAYMENT_ATTEMPT_ID: &str = "irrelevant_payment_attempt_id";

pub async fn construct_relay_refund_router_data<F>(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    connector_account: &domain::MerchantConnectorAccount,
    relay_record: &hyperswitch_domain_models::relay::Relay,
) -> RouterResult<types::RefundsRouterData<F>> {
    let connector_auth_type = connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    #[cfg(feature = "v2")]
    let connector_name = &connector_account.connector_name.to_string();

    #[cfg(feature = "v1")]
    let connector_name = &connector_account.connector_name;

    let webhook_url = Some(payments::helpers::create_webhook_url(
        &state.base_url.clone(),
        merchant_id,
        connector_account.get_id().get_string_repr(),
    ));

    let supported_connector = &state
        .conf
        .multiple_api_version_supported_connectors
        .supported_connectors;

    let connector_enum = api_models::enums::Connector::from_str(connector_name)
        .change_context(errors::ConnectorError::InvalidConnectorName)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_name:?}"))?;

    let connector_api_version = if supported_connector.contains(&connector_enum) {
        state
            .store
            .find_config_by_key(&format!("connector_api_version_{connector_name}"))
            .await
            .map(|value| value.config)
            .ok()
    } else {
        None
    };

    let hyperswitch_domain_models::relay::RelayData::Refund(relay_refund_data) = relay_record
        .request_data
        .clone()
        .get_required_value("refund relay data")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain relay data to construct relay refund data")?;

    let relay_id_string = relay_record.id.get_string_repr().to_string();

    let router_data = hyperswitch_domain_models::router_data::RouterData {
        flow: std::marker::PhantomData,
        merchant_id: merchant_id.clone(),
        customer_id: None,
        tenant_id: state.tenant.tenant_id.clone(),
        connector: connector_name.to_string(),
        payment_id: IRRELEVANT_PAYMENT_INTENT_ID.to_string(),
        attempt_id: IRRELEVANT_PAYMENT_ATTEMPT_ID.to_string(),
        status: common_enums::AttemptStatus::Charged,
        payment_method: common_enums::PaymentMethod::default(),
        connector_auth_type,
        description: None,
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: connector_account.metadata.clone(),
        connector_wallets_details: None,
        amount_captured: None,
        payment_method_status: None,
        minor_amount_captured: None,
        request: hyperswitch_domain_models::router_request_types::RefundsData {
            refund_id: relay_id_string.clone(),
            connector_transaction_id: relay_record.connector_resource_id.clone(),
            refund_amount: relay_refund_data.amount.get_amount_as_i64(),
            minor_refund_amount: relay_refund_data.amount,
            currency: relay_refund_data.currency,
            payment_amount: relay_refund_data.amount.get_amount_as_i64(),
            minor_payment_amount: relay_refund_data.amount,
            webhook_url,
            connector_metadata: None,
            reason: relay_refund_data.reason,
            connector_refund_id: relay_record.connector_reference_id.clone(),
            browser_info: None,
            split_refunds: None,
            integrity_object: None,
            refund_status: common_enums::RefundStatus::from(relay_record.status),
            capture_method: None,
        },

        response: Err(ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id: relay_id_string.clone(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: connector_account.get_connector_test_mode(),
        payment_method_balance: None,
        connector_api_version,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: Some(relay_id_string),
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };

    Ok(router_data)
}
