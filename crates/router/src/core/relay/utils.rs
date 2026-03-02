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

    let connector_name = &connector_account.get_connector_name_as_string();

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

    let relay_refund_data = relay_record
        .request_data
        .clone()
        .get_required_value("refund relay data")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain relay data to construct relay refund data")?
        .get_refund_data()?;

    let relay_id_string = relay_record.id.get_string_repr().to_string();

    let router_data = hyperswitch_domain_models::router_data::RouterData {
        flow: std::marker::PhantomData,
        merchant_id: merchant_id.clone(),
        customer_id: None,
        tenant_id: state.tenant.tenant_id.clone(),
        connector: connector_name.to_string(),
        payment_id: IRRELEVANT_PAYMENT_INTENT_ID.to_string(),
        attempt_id: IRRELEVANT_PAYMENT_ATTEMPT_ID.to_string(),
        status: common_enums::AttemptStatus::Pending,
        payment_method: common_enums::PaymentMethod::default(),
        payment_method_type: None,
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
            refund_connector_metadata: None,
            reason: relay_refund_data.reason,
            connector_refund_id: relay_record.connector_reference_id.clone(),
            browser_info: None,
            split_refunds: None,
            integrity_object: None,
            refund_status: common_enums::RefundStatus::from(relay_record.status),
            merchant_account_id: None,
            merchant_config_currency: None,
            capture_method: None,
            additional_payment_method_data: None,
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
        payout_id: None,
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
        customer_document_details: None,
    };

    Ok(router_data)
}

pub async fn construct_relay_capture_router_data(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    connector_account: &domain::MerchantConnectorAccount,
    relay_record: &hyperswitch_domain_models::relay::Relay,
) -> RouterResult<types::PaymentsCaptureRouterData> {
    let connector_auth_type = connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let connector_name = &connector_account.get_connector_name_as_string();

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

    let relay_capture_data = relay_record
        .request_data
        .clone()
        .get_required_value("capture relay data")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain relay data to construct relay capture data")?
        .get_capture_data()?;

    let relay_id_string = relay_record.id.get_string_repr().to_string();

    let router_data = hyperswitch_domain_models::router_data::RouterData {
        flow: std::marker::PhantomData,
        merchant_id: merchant_id.clone(),
        customer_id: None,
        tenant_id: state.tenant.tenant_id.clone(),
        connector: connector_name.to_string(),
        payment_id: IRRELEVANT_PAYMENT_INTENT_ID.to_string(),
        attempt_id: IRRELEVANT_PAYMENT_ATTEMPT_ID.to_string(),
        status: common_enums::AttemptStatus::Pending,
        payment_method: common_enums::PaymentMethod::default(),
        payment_method_type: None,
        connector_auth_type,
        description: None,
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: connector_account.metadata.clone(),
        connector_wallets_details: None,
        amount_captured: None,
        payment_method_status: None,
        minor_amount_captured: None,
        request: hyperswitch_domain_models::router_request_types::PaymentsCaptureData {
            amount_to_capture: relay_capture_data.amount_to_capture.get_amount_as_i64(),
            currency: relay_capture_data.currency,
            connector_transaction_id: relay_record.connector_resource_id.clone(),
            payment_amount: relay_capture_data.authorized_amount.get_amount_as_i64(),
            multiple_capture_data: Some(
                // for relay, each manual multiple capture is a separate entity i.e not related
                hyperswitch_domain_models::router_request_types::MultipleCaptureRequestData {
                    capture_sequence: 1,
                    capture_reference: relay_id_string.clone(),
                },
            ),
            connector_meta: None,
            browser_info: None,
            metadata: None,
            capture_method: None,
            split_payments: None,
            minor_payment_amount: relay_capture_data.authorized_amount,
            minor_amount_to_capture: relay_capture_data.amount_to_capture,
            integrity_object: None,
            webhook_url,
            merchant_order_reference_id: None,
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
        refund_id: None,
        dispute_id: None,
        payout_id: None,
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
        customer_document_details: None,
    };

    Ok(router_data)
}

pub async fn construct_relay_incremental_authorization_router_data(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    connector_account: &domain::MerchantConnectorAccount,
    relay_record: &hyperswitch_domain_models::relay::Relay,
) -> RouterResult<types::PaymentsIncrementalAuthorizationRouterData> {
    let connector_auth_type = connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let connector_name = &connector_account.get_connector_name_as_string();

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

    let relay_incremental_authorization_data = relay_record
        .request_data
        .clone()
        .get_required_value("incremental authorization relay data")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Failed to obtain relay data to construct relay incremental authorization data",
        )?
        .get_incremental_authorization_data()?;

    let relay_id_string = relay_record.id.get_string_repr().to_string();

    let router_data = hyperswitch_domain_models::router_data::RouterData {
        flow: std::marker::PhantomData,
        merchant_id: merchant_id.clone(),
        customer_id: None,
        tenant_id: state.tenant.tenant_id.clone(),
        connector: connector_name.to_string(),
        payment_id: IRRELEVANT_PAYMENT_INTENT_ID.to_string(),
        attempt_id: IRRELEVANT_PAYMENT_ATTEMPT_ID.to_string(),
        status: common_enums::AttemptStatus::Pending,
        payment_method: common_enums::PaymentMethod::default(),
        payment_method_type: None,
        connector_auth_type,
        description: None,
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: connector_account.metadata.clone(),
        connector_wallets_details: None,
        amount_captured: None,
        payment_method_status: None,
        minor_amount_captured: None,
        request:
            hyperswitch_domain_models::router_request_types::PaymentsIncrementalAuthorizationData {
                total_amount: relay_incremental_authorization_data
                    .total_amount
                    .get_amount_as_i64(),
                additional_amount: relay_incremental_authorization_data
                    .additional_amount
                    .get_amount_as_i64(),
                currency: relay_incremental_authorization_data.currency,
                reason: None,
                connector_transaction_id: relay_record.connector_resource_id.clone(),
                connector_meta: None,
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
        refund_id: None,
        dispute_id: None,
        payout_id: None,
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
        customer_document_details: None,
    };

    Ok(router_data)
}

pub async fn construct_relay_void_router_data(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    connector_account: &domain::MerchantConnectorAccount,
    relay_record: &hyperswitch_domain_models::relay::Relay,
) -> RouterResult<types::PaymentsCancelRouterData> {
    let connector_auth_type = connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let connector_name = &connector_account.get_connector_name_as_string();

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

    let relay_void_data = relay_record
        .request_data
        .clone()
        .get_required_value("void relay data")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain relay data to construct relay void data")?
        .get_void_data()?;

    let relay_id_string = relay_record.id.get_string_repr().to_string();

    let router_data = hyperswitch_domain_models::router_data::RouterData {
        flow: std::marker::PhantomData,
        merchant_id: merchant_id.clone(),
        customer_id: None,
        tenant_id: state.tenant.tenant_id.clone(),
        connector: connector_name.to_string(),
        payment_id: IRRELEVANT_PAYMENT_INTENT_ID.to_string(),
        attempt_id: IRRELEVANT_PAYMENT_ATTEMPT_ID.to_string(),
        status: common_enums::AttemptStatus::Pending,
        payment_method: common_enums::PaymentMethod::default(),
        payment_method_type: None,
        connector_auth_type,
        description: None,
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: connector_account.metadata.clone(),
        connector_wallets_details: None,
        amount_captured: None,
        payment_method_status: None,
        minor_amount_captured: None,
        request: hyperswitch_domain_models::router_request_types::PaymentsCancelData {
            amount: relay_void_data
                .amount
                .map(|value| value.get_amount_as_i64()),
            currency: relay_void_data.currency,
            connector_transaction_id: relay_record.connector_resource_id.clone(),
            cancellation_reason: relay_void_data.cancellation_reason,
            connector_meta: None,
            browser_info: None,
            metadata: None,
            minor_amount: relay_void_data.amount,
            webhook_url,
            capture_method: None,
            split_payments: None,
            merchant_order_reference_id: None,
            feature_metadata: None,
            payment_method_type: None,
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
        refund_id: None,
        dispute_id: None,
        payout_id: None,
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
        customer_document_details: None,
    };

    Ok(router_data)
}

pub async fn construct_relay_payments_retrieve_router_data(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    connector_account: &domain::MerchantConnectorAccount,
    relay_record: &hyperswitch_domain_models::relay::Relay,
    capture_method_type: Option<hyperswitch_interfaces::api::CaptureSyncMethod>,
) -> RouterResult<types::PaymentsSyncRouterData> {
    let connector_auth_type = connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let connector_name = &connector_account.get_connector_name_as_string();

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

    let relay_capture_data = relay_record
        .request_data
        .clone()
        .get_required_value("capture relay data")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain relay data to construct relay capture data")?
        .get_capture_data()?;

    let connector_transaction_id = match capture_method_type {
        Some(hyperswitch_interfaces::api::CaptureSyncMethod::Bulk) => {
            hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(
                relay_record.connector_resource_id.clone(),
            )
        }
        Some(hyperswitch_interfaces::api::CaptureSyncMethod::Individual) | None => {
            hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(
                relay_record
                    .connector_reference_id
                    .clone()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Missing connector_reference_id")?,
            )
        }
    };

    let relay_id_string = relay_record.id.get_string_repr().to_string();

    let router_data = hyperswitch_domain_models::router_data::RouterData {
        flow: std::marker::PhantomData,
        merchant_id: merchant_id.clone(),
        customer_id: None,
        tenant_id: state.tenant.tenant_id.clone(),
        connector: connector_name.to_string(),
        payment_id: IRRELEVANT_PAYMENT_INTENT_ID.to_string(),
        attempt_id: IRRELEVANT_PAYMENT_ATTEMPT_ID.to_string(),
        status: common_enums::AttemptStatus::Pending,
        payment_method: common_enums::PaymentMethod::default(),
        payment_method_type: None,
        connector_auth_type,
        description: None,
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: connector_account.metadata.clone(),
        connector_wallets_details: None,
        amount_captured: None,
        payment_method_status: None,
        minor_amount_captured: None,
        request: hyperswitch_domain_models::router_request_types::PaymentsSyncData {
            currency: relay_capture_data.currency,
            connector_transaction_id,
            connector_meta: None,
            capture_method: None,
            split_payments: None,
            integrity_object: None,
            encoded_data: None,
            sync_type:
                hyperswitch_domain_models::router_request_types::SyncRequestType::SinglePaymentSync,
            mandate_id: None,
            payment_method_type: None,
            payment_experience: None,
            amount: relay_capture_data.amount_to_capture,
            connector_reference_id: None,
            setup_future_usage: None,
            feature_metadata: None,
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
        refund_id: None,
        dispute_id: None,
        payout_id: None,
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
        customer_document_details: None,
    };

    Ok(router_data)
}
