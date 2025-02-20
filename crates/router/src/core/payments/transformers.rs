use std::{fmt::Debug, marker::PhantomData, str::FromStr};

use api_models::payments::{
    Address, ConnectorMandateReferenceId, CustomerDetails, CustomerDetailsResponse, FrmMessage,
    RequestSurchargeDetails,
};
use common_enums::{Currency, RequestIncrementalAuthorization};
use common_utils::{
    consts::X_HS_LATENCY,
    fp_utils, pii,
    types::{
        self as common_utils_type, AmountConvertor, MinorUnit, StringMajorUnit,
        StringMajorUnitForConnector,
    },
};
use diesel_models::{
    ephemeral_key,
    payment_attempt::ConnectorMandateReferenceId as DieselConnectorMandateReferenceId,
};
use error_stack::{report, ResultExt};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::ApiModelToDieselModelConvertor;
use hyperswitch_domain_models::{payments::payment_intent::CustomerData, router_request_types};
#[cfg(feature = "v2")]
use masking::PeekInterface;
use masking::{ExposeInterface, Maskable, Secret};
use router_env::{instrument, tracing};

use super::{flows::Feature, types::AuthenticationData, OperationSessionGetters, PaymentData};
use crate::{
    configs::settings::ConnectorRequestReferenceIdConfig,
    connector::{Helcim, Nexinets},
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments::{self, helpers},
        utils as core_utils,
    },
    headers::X_PAYMENT_CONFIRM_SOURCE,
    routes::{metrics, SessionState},
    services::{self, RedirectForm},
    types::{
        self,
        api::{self, ConnectorTransactionId},
        domain,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignInto, ForeignTryFrom},
        MultipleCaptureRequestData,
    },
    utils::{OptionExt, ValueExt},
};

#[cfg(feature = "v2")]
pub async fn construct_router_data_to_update_calculated_tax<'a, F, T>(
    state: &'a SessionState,
    payment_data: PaymentData<F>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
) -> RouterResult<types::RouterData<F, T, types::PaymentsResponseData>>
where
    T: TryFrom<PaymentAdditionalData<'a, F>>,
    types::RouterData<F, T, types::PaymentsResponseData>: Feature<F, T>,
    F: Clone,
    error_stack::Report<errors::ApiErrorResponse>:
        From<<T as TryFrom<PaymentAdditionalData<'a, F>>>::Error>,
{
    todo!()
}

#[cfg(feature = "v1")]
pub async fn construct_router_data_to_update_calculated_tax<'a, F, T>(
    state: &'a SessionState,
    payment_data: PaymentData<F>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
) -> RouterResult<types::RouterData<F, T, types::PaymentsResponseData>>
where
    T: TryFrom<PaymentAdditionalData<'a, F>>,
    types::RouterData<F, T, types::PaymentsResponseData>: Feature<F, T>,
    F: Clone,
    error_stack::Report<errors::ApiErrorResponse>:
        From<<T as TryFrom<PaymentAdditionalData<'a, F>>>::Error>,
{
    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    let test_mode = merchant_connector_account.is_test_mode_on();

    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let additional_data = PaymentAdditionalData {
        router_base_url: state.base_url.clone(),
        connector_name: connector_id.to_string(),
        payment_data: payment_data.clone(),
        state,
        customer_data: customer,
    };

    let connector_mandate_request_reference_id = payment_data
        .payment_attempt
        .connector_mandate_detail
        .as_ref()
        .and_then(|detail| detail.get_connector_mandate_request_reference_id());

    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        customer_id: None,
        connector: connector_id.to_owned(),
        payment_id: payment_data
            .payment_attempt
            .payment_id
            .get_string_repr()
            .to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: payment_data.payment_attempt.get_id().to_owned(),
        status: payment_data.payment_attempt.status,
        payment_method: diesel_models::enums::PaymentMethod::default(),
        connector_auth_type: auth_type,
        description: None,
        address: payment_data.address.clone(),
        auth_type: payment_data
            .payment_attempt
            .authentication_type
            .unwrap_or_default(),
        connector_meta_data: None,
        connector_wallets_details: None,
        request: T::try_from(additional_data)?,
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_status: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        connector_request_reference_id: core_utils::get_connector_request_reference_id(
            &state.conf,
            merchant_account.get_id(),
            &payment_data.payment_attempt,
        ),
        preprocessing_id: None,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };
    Ok(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data_for_authorize<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentConfirmData<api::Authorize>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    _merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<types::PaymentsAuthorizeRouterData> {
    use masking::ExposeOptionInterface;

    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    // TODO: Take Globalid and convert to connector reference id
    let customer_id = customer
        .to_owned()
        .map(|customer| common_utils::id_type::CustomerId::try_from(customer.id.clone()))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Invalid global customer generated, not able to convert to reference id",
        )?;

    let connector_customer_id = customer.as_ref().and_then(|customer| {
        customer
            .get_connector_customer_id(&merchant_connector_account.get_id())
            .map(String::from)
    });

    let payment_method = payment_data.payment_attempt.payment_method_type;

    let router_base_url = &state.base_url;
    let attempt = &payment_data.payment_attempt;

    let complete_authorize_url = Some(helpers::create_complete_authorize_url(
        router_base_url,
        attempt,
        connector_id,
    ));

    let webhook_url = Some(helpers::create_webhook_url(
        router_base_url,
        &attempt.merchant_id,
        merchant_connector_account.get_id().get_string_repr(),
    ));

    let router_return_url = payment_data
        .payment_intent
        .create_finish_redirection_url(router_base_url, &merchant_account.publishable_key)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to construct finish redirection url")?
        .to_string();

    let connector_request_reference_id = payment_data
        .payment_intent
        .merchant_reference_id
        .map(|id| id.get_string_repr().to_owned())
        .unwrap_or(payment_data.payment_attempt.id.get_string_repr().to_owned());

    let email = customer
        .as_ref()
        .and_then(|customer| customer.email.clone())
        .map(pii::Email::from);

    let browser_info = payment_data
        .payment_attempt
        .browser_info
        .clone()
        .map(types::BrowserInformation::from);

    // TODO: few fields are repeated in both routerdata and request
    let request = types::PaymentsAuthorizeData {
        payment_method_data: payment_data
            .payment_method_data
            .get_required_value("payment_method_data")?,
        setup_future_usage: Some(payment_data.payment_intent.setup_future_usage),
        mandate_id: None,
        off_session: None,
        setup_mandate_details: None,
        confirm: true,
        statement_descriptor_suffix: None,
        statement_descriptor: None,
        capture_method: Some(payment_data.payment_intent.capture_method),
        amount: payment_data
            .payment_attempt
            .amount_details
            .get_net_amount()
            .get_amount_as_i64(),
        minor_amount: payment_data.payment_attempt.amount_details.get_net_amount(),
        order_tax_amount: None,
        currency: payment_data.payment_intent.amount_details.currency,
        browser_info,
        email,
        customer_name: None,
        payment_experience: None,
        order_details: None,
        order_category: None,
        session_token: None,
        enrolled_for_3ds: true,
        related_transaction_id: None,
        payment_method_type: Some(payment_data.payment_attempt.payment_method_subtype),
        router_return_url: Some(router_return_url),
        webhook_url,
        complete_authorize_url,
        customer_id: None,
        surcharge_details: None,
        request_incremental_authorization: matches!(
            payment_data
                .payment_intent
                .request_incremental_authorization,
            RequestIncrementalAuthorization::True | RequestIncrementalAuthorization::Default
        ),
        metadata: payment_data.payment_intent.metadata.expose_option(),
        authentication_data: None,
        customer_acceptance: None,
        split_payments: None,
        merchant_order_reference_id: None,
        integrity_object: None,
        shipping_cost: payment_data.payment_intent.amount_details.shipping_cost,
        additional_payment_method_data: None,
    };
    let connector_mandate_request_reference_id = payment_data
        .payment_attempt
        .connector_token_details
        .as_ref()
        .and_then(|detail| detail.get_connector_mandate_request_reference_id());

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        tenant_id: state.tenant.tenant_id.clone(),
        // TODO: evaluate why we need customer id at the connector level. We already have connector customer id.
        customer_id,
        connector: connector_id.to_owned(),
        // TODO: evaluate why we need payment id at the connector level. We already have connector reference id
        payment_id: payment_data
            .payment_attempt
            .payment_id
            .get_string_repr()
            .to_owned(),
        // TODO: evaluate why we need attempt id at the connector level. We already have connector reference id
        attempt_id: payment_data
            .payment_attempt
            .get_id()
            .get_string_repr()
            .to_owned(),
        status: payment_data.payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: payment_data
            .payment_intent
            .description
            .as_ref()
            .map(|description| description.get_string_repr())
            .map(ToOwned::to_owned),
        // TODO: Create unified address
        address: payment_data.payment_address.clone(),
        auth_type: payment_data.payment_attempt.authentication_type,
        connector_meta_data: None,
        connector_wallets_details: None,
        request,
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_status: None,
        payment_method_token: None,
        connector_customer: connector_customer_id,
        recurring_mandate_payment_data: None,
        // TODO: This has to be generated as the reference id based on the connector configuration
        // Some connectros might not accept accept the global id. This has to be done when generating the reference id
        connector_request_reference_id,
        preprocessing_id: payment_data.payment_attempt.preprocessing_step_id,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        // TODO: take this based on the env
        test_mode: Some(true),
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload,
        connector_mandate_request_reference_id,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };

    Ok(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data_for_capture<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentCaptureData<api::Capture>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    _merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<types::PaymentsCaptureRouterData> {
    use masking::ExposeOptionInterface;

    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let customer_id = customer
        .to_owned()
        .map(|customer| common_utils::id_type::CustomerId::try_from(customer.id.clone()))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Invalid global customer generated, not able to convert to reference id",
        )?;

    let payment_method = payment_data.payment_attempt.payment_method_type;

    let connector_request_reference_id = payment_data
        .payment_intent
        .merchant_reference_id
        .map(|id| id.get_string_repr().to_owned())
        .unwrap_or(payment_data.payment_attempt.id.get_string_repr().to_owned());

    let connector_mandate_request_reference_id = payment_data
        .payment_attempt
        .connector_token_details
        .as_ref()
        .and_then(|detail| detail.get_connector_mandate_request_reference_id());

    let connector = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector_id,
        api::GetToken::Connector,
        payment_data.payment_attempt.merchant_connector_id.clone(),
    )?;

    let amount_to_capture = payment_data
        .payment_attempt
        .amount_details
        .get_amount_to_capture()
        .unwrap_or(payment_data.payment_attempt.amount_details.get_net_amount());

    let amount = payment_data.payment_attempt.amount_details.get_net_amount();
    let request = types::PaymentsCaptureData {
        capture_method: Some(payment_data.payment_intent.capture_method),
        amount_to_capture: amount_to_capture.get_amount_as_i64(), // This should be removed once we start moving to connector module
        minor_amount_to_capture: amount_to_capture,
        currency: payment_data.payment_intent.amount_details.currency,
        connector_transaction_id: connector
            .connector
            .connector_transaction_id(payment_data.payment_attempt.clone())?
            .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
        payment_amount: amount.get_amount_as_i64(), // This should be removed once we start moving to connector module
        minor_payment_amount: amount,
        connector_meta: payment_data
            .payment_attempt
            .connector_metadata
            .clone()
            .expose_option(),
        // TODO: add multiple capture data
        multiple_capture_data: None,
        // TODO: why do we need browser info during capture?
        browser_info: None,
        metadata: payment_data.payment_intent.metadata.expose_option(),
        integrity_object: None,
    };

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        // TODO: evaluate why we need customer id at the connector level. We already have connector customer id.
        customer_id,
        connector: connector_id.to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        // TODO: evaluate why we need payment id at the connector level. We already have connector reference id
        payment_id: payment_data
            .payment_attempt
            .payment_id
            .get_string_repr()
            .to_owned(),
        // TODO: evaluate why we need attempt id at the connector level. We already have connector reference id
        attempt_id: payment_data
            .payment_attempt
            .get_id()
            .get_string_repr()
            .to_owned(),
        status: payment_data.payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: payment_data
            .payment_intent
            .description
            .as_ref()
            .map(|description| description.get_string_repr())
            .map(ToOwned::to_owned),
        // TODO: Create unified address
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: payment_data.payment_attempt.authentication_type,
        connector_meta_data: None,
        connector_wallets_details: None,
        request,
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_status: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        // TODO: This has to be generated as the reference id based on the connector configuration
        // Some connectros might not accept accept the global id. This has to be done when generating the reference id
        connector_request_reference_id,
        preprocessing_id: payment_data.payment_attempt.preprocessing_step_id,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        // TODO: take this based on the env
        test_mode: Some(true),
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload,
        connector_mandate_request_reference_id,
        psd2_sca_exemption_type: None,
        authentication_id: None,
    };

    Ok(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_router_data_for_psync<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentStatusData<api::PSync>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    _merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<types::PaymentsSyncRouterData> {
    use masking::ExposeOptionInterface;

    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    // TODO: Take Globalid / CustomerReferenceId and convert to connector reference id
    let customer_id = None;

    let payment_intent = payment_data.payment_intent;

    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let attempt = &payment_data
        .payment_attempt
        .get_required_value("attempt")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Payment Attempt is not available in payment data")?;

    let connector_request_reference_id = payment_intent
        .merchant_reference_id
        .map(|id| id.get_string_repr().to_owned())
        .unwrap_or(attempt.id.get_string_repr().to_owned());

    let request = types::PaymentsSyncData {
        amount: attempt.amount_details.get_net_amount(),
        integrity_object: None,
        mandate_id: None,
        connector_transaction_id: match attempt.get_connector_payment_id() {
            Some(connector_txn_id) => {
                types::ResponseId::ConnectorTransactionId(connector_txn_id.to_owned())
            }
            None => types::ResponseId::NoResponseId,
        },
        encoded_data: attempt.encoded_data.clone().expose_option(),
        capture_method: Some(payment_intent.capture_method),
        connector_meta: attempt.connector_metadata.clone().expose_option(),
        sync_type: types::SyncRequestType::SinglePaymentSync,
        payment_method_type: Some(attempt.payment_method_subtype),
        currency: payment_intent.amount_details.currency,
        // TODO: Get the charges object from feature metadata
        split_payments: None,
        payment_experience: None,
    };

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        // TODO: evaluate why we need customer id at the connector level. We already have connector customer id.
        customer_id,
        tenant_id: state.tenant.tenant_id.clone(),
        connector: connector_id.to_owned(),
        // TODO: evaluate why we need payment id at the connector level. We already have connector reference id
        payment_id: payment_intent.id.get_string_repr().to_owned(),
        // TODO: evaluate why we need attempt id at the connector level. We already have connector reference id
        attempt_id: attempt.get_id().get_string_repr().to_owned(),
        status: attempt.status,
        payment_method: attempt.payment_method_type,
        connector_auth_type: auth_type,
        description: payment_intent
            .description
            .as_ref()
            .map(|description| description.get_string_repr())
            .map(ToOwned::to_owned),
        // TODO: Create unified address
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: attempt.authentication_type,
        connector_meta_data: None,
        connector_wallets_details: None,
        request,
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_status: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        // TODO: This has to be generated as the reference id based on the connector configuration
        // Some connectros might not accept accept the global id. This has to be done when generating the reference id
        connector_request_reference_id,
        preprocessing_id: attempt.preprocessing_step_id.clone(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        // TODO: take this based on the env
        test_mode: Some(true),
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };

    Ok(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data_for_sdk_session<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentIntentData<api::Session>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    _merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<types::PaymentsSessionRouterData> {
    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    // TODO: Take Globalid and convert to connector reference id
    let customer_id = customer
        .to_owned()
        .map(|customer| common_utils::id_type::CustomerId::try_from(customer.id.clone()))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Invalid global customer generated, not able to convert to reference id",
        )?;
    let email = customer
        .as_ref()
        .and_then(|customer| customer.email.clone())
        .map(pii::Email::from);
    let order_details = payment_data
        .payment_intent
        .order_details
        .clone()
        .map(|order_details| {
            order_details
                .into_iter()
                .map(|order_detail| order_detail.expose())
                .collect()
        });
    let required_amount_type = StringMajorUnitForConnector;

    let apple_pay_amount = required_amount_type
        .convert(
            payment_data.payment_intent.amount_details.order_amount,
            payment_data.payment_intent.amount_details.currency,
        )
        .change_context(errors::ApiErrorResponse::PreconditionFailed {
            message: "Failed to convert amount to string major unit for applePay".to_string(),
        })?;

    let apple_pay_recurring_details = payment_data
        .payment_intent
        .feature_metadata
        .and_then(|feature_metadata| feature_metadata.apple_pay_recurring_details)
        .map(|apple_pay_recurring_details| {
            ForeignInto::foreign_into((apple_pay_recurring_details, apple_pay_amount))
        });

    // TODO: few fields are repeated in both routerdata and request
    let request = types::PaymentsSessionData {
        amount: payment_data
            .payment_intent
            .amount_details
            .order_amount
            .get_amount_as_i64(),
        currency: payment_data.payment_intent.amount_details.currency,
        country: payment_data
            .payment_intent
            .billing_address
            .and_then(|billing_address| {
                billing_address
                    .get_inner()
                    .address
                    .as_ref()
                    .and_then(|address| address.country)
            }),
        // TODO: populate surcharge here
        surcharge_details: None,
        order_details,
        email,
        minor_amount: payment_data.payment_intent.amount_details.order_amount,
        apple_pay_recurring_details,
    };

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        // TODO: evaluate why we need customer id at the connector level. We already have connector customer id.
        customer_id,
        connector: connector_id.to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        // TODO: evaluate why we need payment id at the connector level. We already have connector reference id
        payment_id: payment_data.payment_intent.id.get_string_repr().to_owned(),
        // TODO: evaluate why we need attempt id at the connector level. We already have connector reference id
        attempt_id: "".to_string(),
        status: enums::AttemptStatus::Started,
        payment_method: enums::PaymentMethod::Wallet,
        connector_auth_type: auth_type,
        description: payment_data
            .payment_intent
            .description
            .as_ref()
            .map(|description| description.get_string_repr())
            .map(ToOwned::to_owned),
        // TODO: Create unified address
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: payment_data
            .payment_intent
            .authentication_type
            .unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: None,
        request,
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_status: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        // TODO: This has to be generated as the reference id based on the connector configuration
        // Some connectros might not accept accept the global id. This has to be done when generating the reference id
        connector_request_reference_id: "".to_string(),
        preprocessing_id: None,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        // TODO: take this based on the env
        test_mode: Some(true),
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload,
        connector_mandate_request_reference_id: None,
        psd2_sca_exemption_type: None,
        authentication_id: None,
    };

    Ok(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data_for_setup_mandate<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentConfirmData<api::SetupMandate>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    _merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<types::SetupMandateRouterData> {
    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    // TODO: Take Globalid and convert to connector reference id
    let customer_id = customer
        .to_owned()
        .map(|customer| common_utils::id_type::CustomerId::try_from(customer.id.clone()))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Invalid global customer generated, not able to convert to reference id",
        )?;

    let connector_customer_id = customer.as_ref().and_then(|customer| {
        customer
            .get_connector_customer_id(&merchant_connector_account.get_id())
            .map(String::from)
    });

    let payment_method = payment_data.payment_attempt.payment_method_type;

    let router_base_url = &state.base_url;
    let attempt = &payment_data.payment_attempt;

    let complete_authorize_url = Some(helpers::create_complete_authorize_url(
        router_base_url,
        attempt,
        connector_id,
    ));

    let webhook_url = Some(helpers::create_webhook_url(
        router_base_url,
        &attempt.merchant_id,
        merchant_connector_account.get_id().get_string_repr(),
    ));

    let router_return_url = payment_data
        .payment_intent
        .create_finish_redirection_url(router_base_url, &merchant_account.publishable_key)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to construct finish redirection url")?
        .to_string();

    let connector_request_reference_id = payment_data
        .payment_intent
        .merchant_reference_id
        .map(|id| id.get_string_repr().to_owned())
        .unwrap_or(payment_data.payment_attempt.id.get_string_repr().to_owned());

    let email = customer
        .as_ref()
        .and_then(|customer| customer.email.clone())
        .map(pii::Email::from);

    let browser_info = payment_data
        .payment_attempt
        .browser_info
        .clone()
        .map(types::BrowserInformation::from);

    // TODO: few fields are repeated in both routerdata and request
    let request = types::SetupMandateRequestData {
        currency: payment_data.payment_intent.amount_details.currency,
        payment_method_data: payment_data
            .payment_method_data
            .get_required_value("payment_method_data")?,
        amount: Some(
            payment_data
                .payment_attempt
                .amount_details
                .get_net_amount()
                .get_amount_as_i64(),
        ),
        confirm: true,
        statement_descriptor_suffix: None,
        customer_acceptance: None,
        mandate_id: None,
        setup_future_usage: Some(payment_data.payment_intent.setup_future_usage),
        off_session: None,
        setup_mandate_details: None,
        router_return_url: Some(router_return_url.clone()),
        webhook_url,
        browser_info,
        email,
        customer_name: None,
        return_url: Some(router_return_url),
        payment_method_type: Some(payment_data.payment_attempt.payment_method_subtype),
        request_incremental_authorization: matches!(
            payment_data
                .payment_intent
                .request_incremental_authorization,
            RequestIncrementalAuthorization::True | RequestIncrementalAuthorization::Default
        ),
        metadata: payment_data.payment_intent.metadata,
        minor_amount: Some(payment_data.payment_attempt.amount_details.get_net_amount()),
        shipping_cost: payment_data.payment_intent.amount_details.shipping_cost,
    };
    let connector_mandate_request_reference_id = payment_data
        .payment_attempt
        .connector_token_details
        .as_ref()
        .and_then(|detail| detail.get_connector_mandate_request_reference_id());

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        tenant_id: state.tenant.tenant_id.clone(),
        // TODO: evaluate why we need customer id at the connector level. We already have connector customer id.
        customer_id,
        connector: connector_id.to_owned(),
        // TODO: evaluate why we need payment id at the connector level. We already have connector reference id
        payment_id: payment_data
            .payment_attempt
            .payment_id
            .get_string_repr()
            .to_owned(),
        // TODO: evaluate why we need attempt id at the connector level. We already have connector reference id
        attempt_id: payment_data
            .payment_attempt
            .get_id()
            .get_string_repr()
            .to_owned(),
        status: payment_data.payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: payment_data
            .payment_intent
            .description
            .as_ref()
            .map(|description| description.get_string_repr())
            .map(ToOwned::to_owned),
        // TODO: Create unified address
        address: payment_data.payment_address.clone(),
        auth_type: payment_data.payment_attempt.authentication_type,
        connector_meta_data: None,
        connector_wallets_details: None,
        request,
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_status: None,
        payment_method_token: None,
        connector_customer: connector_customer_id,
        recurring_mandate_payment_data: None,
        // TODO: This has to be generated as the reference id based on the connector configuration
        // Some connectros might not accept accept the global id. This has to be done when generating the reference id
        connector_request_reference_id,
        preprocessing_id: payment_data.payment_attempt.preprocessing_step_id,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        // TODO: take this based on the env
        test_mode: Some(true),
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload,
        connector_mandate_request_reference_id,
        authentication_id: None,
        psd2_sca_exemption_type: None,
    };

    Ok(router_data)
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data<'a, F, T>(
    state: &'a SessionState,
    payment_data: PaymentData<F>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<types::RouterData<F, T, types::PaymentsResponseData>>
where
    T: TryFrom<PaymentAdditionalData<'a, F>>,
    types::RouterData<F, T, types::PaymentsResponseData>: Feature<F, T>,
    F: Clone,
    error_stack::Report<errors::ApiErrorResponse>:
        From<<T as TryFrom<PaymentAdditionalData<'a, F>>>::Error>,
{
    let (payment_method, router_data);

    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    let test_mode = merchant_connector_account.is_test_mode_on();

    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    payment_method = payment_data
        .payment_attempt
        .payment_method
        .or(payment_data.payment_attempt.payment_method)
        .get_required_value("payment_method_type")?;

    let resource_id = match payment_data
        .payment_attempt
        .get_connector_payment_id()
        .map(ToString::to_string)
    {
        Some(id) => types::ResponseId::ConnectorTransactionId(id),
        None => types::ResponseId::NoResponseId,
    };

    // [#44]: why should response be filled during request
    let response = Ok(types::PaymentsResponseData::TransactionResponse {
        resource_id,
        redirection_data: Box::new(None),
        mandate_reference: Box::new(None),
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: None,
        incremental_authorization_allowed: None,
        charges: None,
    });

    let additional_data = PaymentAdditionalData {
        router_base_url: state.base_url.clone(),
        connector_name: connector_id.to_string(),
        payment_data: payment_data.clone(),
        state,
        customer_data: customer,
    };

    let customer_id = customer.to_owned().map(|customer| customer.customer_id);

    let supported_connector = &state
        .conf
        .multiple_api_version_supported_connectors
        .supported_connectors;
    let connector_enum = api_models::enums::Connector::from_str(connector_id)
        .change_context(errors::ConnectorError::InvalidConnectorName)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_id:?}"))?;

    let connector_api_version = if supported_connector.contains(&connector_enum) {
        state
            .store
            .find_config_by_key(&format!("connector_api_version_{connector_id}"))
            .await
            .map(|value| value.config)
            .ok()
    } else {
        None
    };

    let apple_pay_flow = payments::decide_apple_pay_flow(
        state,
        payment_data.payment_attempt.payment_method_type,
        Some(merchant_connector_account),
    );

    let unified_address = if let Some(payment_method_info) =
        payment_data.payment_method_info.clone()
    {
        let payment_method_billing = payment_method_info
            .payment_method_billing_address
            .map(|decrypted_data| decrypted_data.into_inner().expose())
            .map(|decrypted_value| decrypted_value.parse_value("payment_method_billing_address"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to parse payment_method_billing_address")?;
        payment_data
            .address
            .clone()
            .unify_with_payment_data_billing(payment_method_billing)
    } else {
        payment_data.address
    };
    let connector_mandate_request_reference_id = payment_data
        .payment_attempt
        .connector_mandate_detail
        .as_ref()
        .and_then(|detail| detail.get_connector_mandate_request_reference_id());

    crate::logger::debug!("unified address details {:?}", unified_address);

    router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.get_id().clone(),
        customer_id,
        tenant_id: state.tenant.tenant_id.clone(),
        connector: connector_id.to_owned(),
        payment_id: payment_data
            .payment_attempt
            .payment_id
            .get_string_repr()
            .to_owned(),
        attempt_id: payment_data.payment_attempt.attempt_id.clone(),
        status: payment_data.payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: payment_data.payment_intent.description.clone(),
        address: unified_address,
        auth_type: payment_data
            .payment_attempt
            .authentication_type
            .unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        request: T::try_from(additional_data)?,
        response,
        amount_captured: payment_data
            .payment_intent
            .amount_captured
            .map(|amt| amt.get_amount_as_i64()),
        minor_amount_captured: payment_data.payment_intent.amount_captured,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_status: payment_data.payment_method_info.map(|info| info.status),
        payment_method_token: payment_data
            .pm_token
            .map(|token| types::PaymentMethodToken::Token(Secret::new(token))),
        connector_customer: payment_data.connector_customer_id,
        recurring_mandate_payment_data: payment_data.recurring_mandate_payment_data,
        connector_request_reference_id: core_utils::get_connector_request_reference_id(
            &state.conf,
            merchant_account.get_id(),
            &payment_data.payment_attempt,
        ),
        preprocessing_id: payment_data.payment_attempt.preprocessing_step_id,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        payment_method_balance: None,
        connector_api_version,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: merchant_recipient_data.map(|data| {
            api_models::admin::AdditionalMerchantData::foreign_from(
                types::AdditionalMerchantData::OpenBankingRecipientData(data),
            )
        }),
        header_payload,
        connector_mandate_request_reference_id,
        authentication_id: None,
        psd2_sca_exemption_type: payment_data.payment_intent.psd2_sca_exemption_type,
    };

    Ok(router_data)
}

pub trait ToResponse<F, D, Op>
where
    Self: Sized,
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        data: D,
        customer: Option<domain::Customer>,
        auth_flow: services::AuthFlow,
        base_url: &str,
        operation: Op,
        connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self>;

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        data: D,
        customer: Option<domain::Customer>,
        base_url: &str,
        operation: Op,
        connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResponse<Self>;
}

/// Generate a response from the given Data. This should be implemented on a payment data object
pub trait GenerateResponse<Response>
where
    Self: Sized,
{
    #[cfg(feature = "v2")]
    fn generate_response(
        self,
        state: &SessionState,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResponse<Response>;
}

#[cfg(feature = "v2")]
impl<F> GenerateResponse<api_models::payments::PaymentsCaptureResponse>
    for hyperswitch_domain_models::payments::PaymentCaptureData<F>
where
    F: Clone,
{
    fn generate_response(
        self,
        state: &SessionState,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResponse<api_models::payments::PaymentsCaptureResponse> {
        let payment_intent = &self.payment_intent;
        let payment_attempt = &self.payment_attempt;

        let amount = api_models::payments::PaymentAmountDetailsResponse::foreign_from((
            &payment_intent.amount_details,
            &payment_attempt.amount_details,
        ));

        let response = api_models::payments::PaymentsCaptureResponse {
            id: payment_intent.id.clone(),
            amount,
            status: payment_intent.status,
        };

        Ok(services::ApplicationResponse::JsonWithHeaders((
            response,
            vec![],
        )))
    }
}

#[cfg(feature = "v1")]
impl<F, Op, D> ToResponse<F, D, Op> for api::PaymentsResponse
where
    F: Clone,
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        payment_data: D,
        customer: Option<domain::Customer>,
        auth_flow: services::AuthFlow,
        base_url: &str,
        operation: Op,
        connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self> {
        let captures = payment_data
            .get_multiple_capture_data()
            .and_then(|multiple_capture_data| {
                multiple_capture_data
                    .expand_captures
                    .and_then(|should_expand| {
                        should_expand.then_some(
                            multiple_capture_data
                                .get_all_captures()
                                .into_iter()
                                .cloned()
                                .collect(),
                        )
                    })
            });

        payments_to_payments_response(
            payment_data,
            captures,
            customer,
            auth_flow,
            base_url,
            &operation,
            connector_request_reference_id_config,
            connector_http_status_code,
            external_latency,
            is_latency_header_enabled,
        )
    }
}

#[cfg(feature = "v1")]
impl<F, Op, D> ToResponse<F, D, Op> for api::PaymentsSessionResponse
where
    F: Clone,
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        payment_data: D,
        _customer: Option<domain::Customer>,
        _auth_flow: services::AuthFlow,
        _base_url: &str,
        _operation: Op,
        _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self> {
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                session_token: payment_data.get_sessions_token(),
                payment_id: payment_data.get_payment_attempt().payment_id.clone(),
                client_secret: payment_data
                    .get_payment_intent()
                    .client_secret
                    .clone()
                    .get_required_value("client_secret")?
                    .into(),
            },
            vec![],
        )))
    }
}

#[cfg(feature = "v2")]
impl<F, Op, D> ToResponse<F, D, Op> for api::PaymentsSessionResponse
where
    F: Clone,
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        payment_data: D,
        _customer: Option<domain::Customer>,
        _base_url: &str,
        _operation: Op,
        _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
        _merchant_account: &domain::MerchantAccount,
    ) -> RouterResponse<Self> {
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                session_token: payment_data.get_sessions_token(),
                payment_id: payment_data.get_payment_intent().id.clone(),
            },
            vec![],
        )))
    }
}

#[cfg(feature = "v1")]
impl<F, Op, D> ToResponse<F, D, Op> for api::PaymentsDynamicTaxCalculationResponse
where
    F: Clone,
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        payment_data: D,
        _customer: Option<domain::Customer>,
        _auth_flow: services::AuthFlow,
        _base_url: &str,
        _operation: Op,
        _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self> {
        let mut amount = payment_data.get_payment_intent().amount;
        let shipping_cost = payment_data.get_payment_intent().shipping_cost;
        if let Some(shipping_cost) = shipping_cost {
            amount = amount + shipping_cost;
        }
        let order_tax_amount = payment_data
            .get_payment_intent()
            .tax_details
            .clone()
            .and_then(|tax| {
                tax.payment_method_type
                    .map(|a| a.order_tax_amount)
                    .or_else(|| tax.default.map(|a| a.order_tax_amount))
            });
        if let Some(tax_amount) = order_tax_amount {
            amount = amount + tax_amount;
        }

        let currency = payment_data
            .get_payment_attempt()
            .currency
            .get_required_value("currency")?;

        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                net_amount: amount,
                payment_id: payment_data.get_payment_attempt().payment_id.clone(),
                order_tax_amount,
                shipping_cost,
                display_amount: api_models::payments::DisplayAmountOnSdk::foreign_try_from((
                    amount,
                    shipping_cost,
                    order_tax_amount,
                    currency,
                ))?,
            },
            vec![],
        )))
    }
}

#[cfg(feature = "v2")]
impl<F, Op, D> ToResponse<F, D, Op> for api::PaymentsIntentResponse
where
    F: Clone,
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        payment_data: D,
        _customer: Option<domain::Customer>,
        _base_url: &str,
        operation: Op,
        _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
        _merchant_account: &domain::MerchantAccount,
    ) -> RouterResponse<Self> {
        let payment_intent = payment_data.get_payment_intent();
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                id: payment_intent.id.clone(),
                profile_id: payment_intent.profile_id.clone(),
                status: payment_intent.status,
                amount_details: api_models::payments::AmountDetailsResponse::foreign_from(
                    payment_intent.amount_details.clone(),
                ),
                client_secret: payment_intent.client_secret.clone(),
                merchant_reference_id: payment_intent.merchant_reference_id.clone(),
                routing_algorithm_id: payment_intent.routing_algorithm_id.clone(),
                capture_method: payment_intent.capture_method,
                authentication_type: payment_intent.authentication_type,
                billing: payment_intent
                    .billing_address
                    .clone()
                    .map(|billing| billing.into_inner())
                    .map(From::from),
                shipping: payment_intent
                    .shipping_address
                    .clone()
                    .map(|shipping| shipping.into_inner())
                    .map(From::from),
                customer_id: payment_intent.customer_id.clone(),
                customer_present: payment_intent.customer_present.clone(),
                description: payment_intent.description.clone(),
                return_url: payment_intent.return_url.clone(),
                setup_future_usage: payment_intent.setup_future_usage,
                apply_mit_exemption: payment_intent.apply_mit_exemption.clone(),
                statement_descriptor: payment_intent.statement_descriptor.clone(),
                order_details: payment_intent.order_details.clone().map(|order_details| {
                    order_details
                        .into_iter()
                        .map(|order_detail| order_detail.expose().convert_back())
                        .collect()
                }),
                allowed_payment_method_types: payment_intent.allowed_payment_method_types.clone(),
                metadata: payment_intent.metadata.clone(),
                connector_metadata: payment_intent.connector_metadata.clone(),
                feature_metadata: payment_intent
                    .feature_metadata
                    .clone()
                    .map(|feature_metadata| feature_metadata.convert_back()),
                payment_link_enabled: payment_intent.enable_payment_link.clone(),
                payment_link_config: payment_intent
                    .payment_link_config
                    .clone()
                    .map(ForeignFrom::foreign_from),
                request_incremental_authorization: payment_intent.request_incremental_authorization,
                expires_on: payment_intent.session_expiry,
                frm_metadata: payment_intent.frm_metadata.clone(),
                request_external_three_ds_authentication: payment_intent
                    .request_external_three_ds_authentication
                    .clone(),
            },
            vec![],
        )))
    }
}

#[cfg(feature = "v2")]
impl<F> GenerateResponse<api_models::payments::PaymentsConfirmIntentResponse>
    for hyperswitch_domain_models::payments::PaymentConfirmData<F>
where
    F: Clone,
{
    fn generate_response(
        self,
        state: &SessionState,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResponse<api_models::payments::PaymentsConfirmIntentResponse> {
        let payment_intent = self.payment_intent;
        let payment_attempt = self.payment_attempt;

        let amount = api_models::payments::PaymentAmountDetailsResponse::foreign_from((
            &payment_intent.amount_details,
            &payment_attempt.amount_details,
        ));

        let connector = payment_attempt
            .connector
            .clone()
            .get_required_value("connector")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector is none when constructing response")?;

        let merchant_connector_id = payment_attempt
            .merchant_connector_id
            .clone()
            .get_required_value("merchant_connector_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Merchant connector id is none when constructing response")?;

        let error = payment_attempt
            .error
            .clone()
            .map(api_models::payments::ErrorDetails::foreign_from);

        let payment_address = self.payment_address;

        let payment_method_data =
            Some(api_models::payments::PaymentMethodDataResponseWithBilling {
                payment_method_data: None,
                billing: payment_address
                    .get_request_payment_method_billing()
                    .cloned()
                    .map(From::from),
            });

        // TODO: Add support for other next actions, currently only supporting redirect to url
        let redirect_to_url = payment_intent.create_start_redirection_url(
            &state.base_url,
            merchant_account.publishable_key.clone(),
        )?;

        let next_action = payment_attempt
            .redirection_data
            .as_ref()
            .map(|_| api_models::payments::NextActionData::RedirectToUrl { redirect_to_url });

        let connector_token_details = payment_attempt
            .connector_token_details
            .and_then(Option::<api_models::payments::ConnectorTokenDetails>::foreign_from);

        let response = api_models::payments::PaymentsConfirmIntentResponse {
            id: payment_intent.id.clone(),
            status: payment_intent.status,
            amount,
            customer_id: payment_intent.customer_id.clone(),
            connector,
            client_secret: payment_intent.client_secret.clone(),
            created: payment_intent.created_at,
            payment_method_data,
            payment_method_type: payment_attempt.payment_method_type,
            payment_method_subtype: payment_attempt.payment_method_subtype,
            next_action,
            connector_transaction_id: payment_attempt.connector_payment_id.clone(),
            connector_reference_id: None,
            connector_token_details,
            merchant_connector_id,
            browser_info: None,
            error,
            authentication_type: payment_intent.authentication_type,
            applied_authentication_type: payment_attempt.authentication_type,
        };

        Ok(services::ApplicationResponse::JsonWithHeaders((
            response,
            vec![],
        )))
    }
}

#[cfg(feature = "v2")]
impl<F> GenerateResponse<api_models::payments::PaymentsRetrieveResponse>
    for hyperswitch_domain_models::payments::PaymentStatusData<F>
where
    F: Clone,
{
    fn generate_response(
        self,
        state: &SessionState,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResponse<api_models::payments::PaymentsRetrieveResponse> {
        let payment_intent = self.payment_intent;
        let optional_payment_attempt = self.payment_attempt.as_ref();

        let amount = api_models::payments::PaymentAmountDetailsResponse::foreign_from((
            &payment_intent.amount_details,
            optional_payment_attempt.map(|payment_attempt| &payment_attempt.amount_details),
        ));

        let connector =
            optional_payment_attempt.and_then(|payment_attempt| payment_attempt.connector.clone());

        let merchant_connector_id = optional_payment_attempt
            .and_then(|payment_attempt| payment_attempt.merchant_connector_id.clone());

        let error = optional_payment_attempt
            .and_then(|payment_attempt| payment_attempt.error.clone())
            .map(api_models::payments::ErrorDetails::foreign_from);
        let attempts = self.attempts.as_ref().map(|attempts| {
            attempts
                .iter()
                .map(api_models::payments::PaymentAttemptResponse::foreign_from)
                .collect()
        });

        let payment_method_data =
            Some(api_models::payments::PaymentMethodDataResponseWithBilling {
                payment_method_data: None,
                billing: self
                    .payment_address
                    .get_request_payment_method_billing()
                    .cloned()
                    .map(From::from),
            });

        let response = api_models::payments::PaymentsRetrieveResponse {
            id: payment_intent.id.clone(),
            status: payment_intent.status,
            amount,
            customer_id: payment_intent.customer_id.clone(),
            connector,
            billing: self
                .payment_address
                .get_payment_billing()
                .cloned()
                .map(From::from),
            shipping: self.payment_address.get_shipping().cloned().map(From::from),
            client_secret: payment_intent.client_secret.clone(),
            created: payment_intent.created_at,
            payment_method_data,
            payment_method_type: self
                .payment_attempt
                .as_ref()
                .map(|attempt| attempt.payment_method_type),
            payment_method_subtype: self
                .payment_attempt
                .as_ref()
                .map(|attempt| attempt.payment_method_subtype),
            connector_transaction_id: self
                .payment_attempt
                .as_ref()
                .and_then(|attempt| attempt.connector_payment_id.clone()),
            connector_reference_id: None,
            merchant_connector_id,
            browser_info: None,
            error,
            attempts,
        };

        Ok(services::ApplicationResponse::JsonWithHeaders((
            response,
            vec![],
        )))
    }
}

#[cfg(feature = "v1")]
impl<F, Op, D> ToResponse<F, D, Op> for api::PaymentsPostSessionTokensResponse
where
    F: Clone,
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    fn generate_response(
        payment_data: D,
        _customer: Option<domain::Customer>,
        _auth_flow: services::AuthFlow,
        _base_url: &str,
        _operation: Op,
        _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self> {
        let papal_sdk_next_action =
            paypal_sdk_next_steps_check(payment_data.get_payment_attempt().clone())?;
        let next_action = papal_sdk_next_action.map(|paypal_next_action_data| {
            api_models::payments::NextActionData::InvokeSdkClient {
                next_action_data: paypal_next_action_data,
            }
        });
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                payment_id: payment_data.get_payment_intent().payment_id.clone(),
                next_action,
                status: payment_data.get_payment_intent().status,
            },
            vec![],
        )))
    }
}

impl ForeignTryFrom<(MinorUnit, Option<MinorUnit>, Option<MinorUnit>, Currency)>
    for api_models::payments::DisplayAmountOnSdk
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        (net_amount, shipping_cost, order_tax_amount, currency): (
            MinorUnit,
            Option<MinorUnit>,
            Option<MinorUnit>,
            Currency,
        ),
    ) -> Result<Self, Self::Error> {
        let major_unit_convertor = StringMajorUnitForConnector;

        let sdk_net_amount = major_unit_convertor
            .convert(net_amount, currency)
            .change_context(errors::ApiErrorResponse::PreconditionFailed {
                message: "Failed to convert net_amount to base unit".to_string(),
            })
            .attach_printable("Failed to convert net_amount to string major unit")?;

        let sdk_shipping_cost = shipping_cost
            .map(|cost| {
                major_unit_convertor
                    .convert(cost, currency)
                    .change_context(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Failed to convert shipping_cost to base unit".to_string(),
                    })
                    .attach_printable("Failed to convert shipping_cost to string major unit")
            })
            .transpose()?;

        let sdk_order_tax_amount = order_tax_amount
            .map(|cost| {
                major_unit_convertor
                    .convert(cost, currency)
                    .change_context(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Failed to convert order_tax_amount to base unit".to_string(),
                    })
                    .attach_printable("Failed to convert order_tax_amount to string major unit")
            })
            .transpose()?;
        Ok(Self {
            net_amount: sdk_net_amount,
            shipping_cost: sdk_shipping_cost,
            order_tax_amount: sdk_order_tax_amount,
        })
    }
}

#[cfg(feature = "v1")]
impl<F, Op, D> ToResponse<F, D, Op> for api::VerifyResponse
where
    F: Clone,
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        _data: D,
        _customer: Option<domain::Customer>,
        _auth_flow: services::AuthFlow,
        _base_url: &str,
        _operation: Op,
        _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self> {
        todo!()
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        payment_data: D,
        customer: Option<domain::Customer>,
        _auth_flow: services::AuthFlow,
        _base_url: &str,
        _operation: Op,
        _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self> {
        let additional_payment_method_data: Option<api_models::payments::AdditionalPaymentData> =
            payment_data
                .get_payment_attempt()
                .payment_method_data
                .clone()
                .map(|data| data.parse_value("payment_method_data"))
                .transpose()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "payment_method_data",
                })?;
        let payment_method_data_response =
            additional_payment_method_data.map(api::PaymentMethodDataResponse::from);
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                verify_id: Some(payment_data.get_payment_intent().payment_id.clone()),
                merchant_id: Some(payment_data.get_payment_intent().merchant_id.clone()),
                client_secret: payment_data
                    .get_payment_intent()
                    .client_secret
                    .clone()
                    .map(Secret::new),
                customer_id: customer.as_ref().map(|x| x.customer_id.clone()),
                email: customer
                    .as_ref()
                    .and_then(|cus| cus.email.as_ref().map(|s| s.to_owned())),
                name: customer
                    .as_ref()
                    .and_then(|cus| cus.name.as_ref().map(|s| s.to_owned())),
                phone: customer
                    .as_ref()
                    .and_then(|cus| cus.phone.as_ref().map(|s| s.to_owned())),
                mandate_id: payment_data
                    .get_mandate_id()
                    .and_then(|mandate_ids| mandate_ids.mandate_id.clone()),
                payment_method: payment_data.get_payment_attempt().payment_method,
                payment_method_data: payment_method_data_response,
                payment_token: payment_data.get_token().map(ToString::to_string),
                error_code: payment_data.get_payment_attempt().clone().error_code,
                error_message: payment_data.get_payment_attempt().clone().error_message,
            },
            vec![],
        )))
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[instrument(skip_all)]
// try to use router data here so that already validated things , we don't want to repeat the validations.
// Add internal value not found and external value not found so that we can give 500 / Internal server error for internal value not found
#[allow(clippy::too_many_arguments)]
pub fn payments_to_payments_response<Op, F: Clone, D>(
    _payment_data: D,
    _captures: Option<Vec<storage::Capture>>,
    _customer: Option<domain::Customer>,
    _auth_flow: services::AuthFlow,
    _base_url: &str,
    _operation: &Op,
    _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
    _connector_http_status_code: Option<u16>,
    _external_latency: Option<u128>,
    _is_latency_header_enabled: Option<bool>,
) -> RouterResponse<api_models::payments::PaymentsRetrieveResponse>
where
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    todo!()
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument(skip_all)]
// try to use router data here so that already validated things , we don't want to repeat the validations.
// Add internal value not found and external value not found so that we can give 500 / Internal server error for internal value not found
#[allow(clippy::too_many_arguments)]
pub fn payments_to_payments_response<Op, F: Clone, D>(
    payment_data: D,
    captures: Option<Vec<storage::Capture>>,
    customer: Option<domain::Customer>,
    _auth_flow: services::AuthFlow,
    base_url: &str,
    operation: &Op,
    connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
    connector_http_status_code: Option<u16>,
    external_latency: Option<u128>,
    _is_latency_header_enabled: Option<bool>,
) -> RouterResponse<api::PaymentsResponse>
where
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    use std::ops::Not;

    let payment_attempt = payment_data.get_payment_attempt().clone();
    let payment_intent = payment_data.get_payment_intent().clone();
    let payment_link_data = payment_data.get_payment_link_data();

    let currency = payment_attempt
        .currency
        .as_ref()
        .get_required_value("currency")?;
    let amount = currency
        .to_currency_base_unit(
            payment_attempt
                .net_amount
                .get_total_amount()
                .get_amount_as_i64(),
        )
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "amount",
        })?;
    let mandate_id = payment_attempt.mandate_id.clone();

    let refunds_response = payment_data.get_refunds().is_empty().not().then(|| {
        payment_data
            .get_refunds()
            .into_iter()
            .map(ForeignInto::foreign_into)
            .collect()
    });

    let disputes_response = payment_data.get_disputes().is_empty().not().then(|| {
        payment_data
            .get_disputes()
            .into_iter()
            .map(ForeignInto::foreign_into)
            .collect()
    });

    let incremental_authorizations_response =
        payment_data.get_authorizations().is_empty().not().then(|| {
            payment_data
                .get_authorizations()
                .into_iter()
                .map(ForeignInto::foreign_into)
                .collect()
        });

    let external_authentication_details = payment_data
        .get_authentication()
        .map(ForeignInto::foreign_into);

    let attempts_response = payment_data.get_attempts().map(|attempts| {
        attempts
            .into_iter()
            .map(ForeignInto::foreign_into)
            .collect()
    });

    let captures_response = captures.map(|captures| {
        captures
            .into_iter()
            .map(ForeignInto::foreign_into)
            .collect()
    });

    let merchant_id = payment_attempt.merchant_id.to_owned();
    let payment_method_type = payment_attempt
        .payment_method_type
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or("".to_owned());
    let payment_method = payment_attempt
        .payment_method
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or("".to_owned());
    let additional_payment_method_data: Option<api_models::payments::AdditionalPaymentData> =
        payment_attempt
            .payment_method_data
            .clone()
            .and_then(|data| match data {
                serde_json::Value::Null => None, // This is to handle the case when the payment_method_data is null
                _ => Some(data.parse_value("AdditionalPaymentData")),
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse the AdditionalPaymentData from payment_attempt.payment_method_data")?;

    let surcharge_details =
        payment_attempt
            .net_amount
            .get_surcharge_amount()
            .map(|surcharge_amount| RequestSurchargeDetails {
                surcharge_amount,
                tax_amount: payment_attempt.net_amount.get_tax_on_surcharge(),
            });
    let merchant_decision = payment_intent.merchant_decision.to_owned();
    let frm_message = payment_data.get_frm_message().map(FrmMessage::foreign_from);

    let payment_method_data =
        additional_payment_method_data.map(api::PaymentMethodDataResponse::from);

    let payment_method_data_response = (payment_method_data.is_some()
        || payment_data
            .get_address()
            .get_request_payment_method_billing()
            .is_some())
    .then_some(api_models::payments::PaymentMethodDataResponseWithBilling {
        payment_method_data,
        billing: payment_data
            .get_address()
            .get_request_payment_method_billing()
            .cloned()
            .map(From::from),
    });

    let mut headers = connector_http_status_code
        .map(|status_code| {
            vec![(
                "connector_http_status_code".to_string(),
                Maskable::new_normal(status_code.to_string()),
            )]
        })
        .unwrap_or_default();
    if let Some(payment_confirm_source) = payment_intent.payment_confirm_source {
        headers.push((
            X_PAYMENT_CONFIRM_SOURCE.to_string(),
            Maskable::new_normal(payment_confirm_source.to_string()),
        ))
    }

    // For the case when we don't have Customer data directly stored in Payment intent
    let customer_table_response: Option<CustomerDetailsResponse> =
        customer.as_ref().map(ForeignInto::foreign_into);

    // If we have customer data in Payment Intent and if the customer is not deleted, We are populating the Retrieve response from the
    // same. If the customer is deleted then we use the customer table to populate customer details
    let customer_details_response =
        if let Some(customer_details_raw) = payment_intent.customer_details.clone() {
            let customer_details_encrypted =
                serde_json::from_value::<CustomerData>(customer_details_raw.into_inner().expose());
            if let Ok(customer_details_encrypted_data) = customer_details_encrypted {
                Some(CustomerDetailsResponse {
                    id: customer_table_response
                        .as_ref()
                        .and_then(|customer_data| customer_data.id.clone()),
                    name: customer_table_response
                        .as_ref()
                        .and_then(|customer_data| customer_data.name.clone())
                        .or(customer_details_encrypted_data
                            .name
                            .or(customer.as_ref().and_then(|customer| {
                                customer.name.as_ref().map(|name| name.clone().into_inner())
                            }))),
                    email: customer_table_response
                        .as_ref()
                        .and_then(|customer_data| customer_data.email.clone())
                        .or(customer_details_encrypted_data.email.or(customer
                            .as_ref()
                            .and_then(|customer| customer.email.clone().map(pii::Email::from)))),
                    phone: customer_table_response
                        .as_ref()
                        .and_then(|customer_data| customer_data.phone.clone())
                        .or(customer_details_encrypted_data
                            .phone
                            .or(customer.as_ref().and_then(|customer| {
                                customer
                                    .phone
                                    .as_ref()
                                    .map(|phone| phone.clone().into_inner())
                            }))),
                    phone_country_code: customer_table_response
                        .as_ref()
                        .and_then(|customer_data| customer_data.phone_country_code.clone())
                        .or(customer_details_encrypted_data
                            .phone_country_code
                            .or(customer
                                .as_ref()
                                .and_then(|customer| customer.phone_country_code.clone()))),
                })
            } else {
                customer_table_response
            }
        } else {
            customer_table_response
        };

    headers.extend(
        external_latency
            .map(|latency| {
                vec![(
                    X_HS_LATENCY.to_string(),
                    Maskable::new_normal(latency.to_string()),
                )]
            })
            .unwrap_or_default(),
    );

    let output = if payments::is_start_pay(&operation)
        && payment_attempt.authentication_data.is_some()
    {
        let redirection_data = payment_attempt
            .authentication_data
            .clone()
            .get_required_value("redirection_data")?;

        let form: RedirectForm = serde_json::from_value(redirection_data)
            .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;

        services::ApplicationResponse::Form(Box::new(services::RedirectionFormData {
            redirect_form: form,
            payment_method_data: payment_data.get_payment_method_data().cloned(),
            amount,
            currency: currency.to_string(),
        }))
    } else {
        let mut next_action_response = None;

        let bank_transfer_next_steps = bank_transfer_next_steps_check(payment_attempt.clone())?;

        let next_action_voucher = voucher_next_steps_check(payment_attempt.clone())?;

        let next_action_mobile_payment = mobile_payment_next_steps_check(&payment_attempt)?;

        let next_action_containing_qr_code_url = qr_code_next_steps_check(payment_attempt.clone())?;

        let papal_sdk_next_action = paypal_sdk_next_steps_check(payment_attempt.clone())?;

        let next_action_containing_fetch_qr_code_url =
            fetch_qr_code_url_next_steps_check(payment_attempt.clone())?;

        let next_action_containing_wait_screen =
            wait_screen_next_steps_check(payment_attempt.clone())?;

        if payment_intent.status == enums::IntentStatus::RequiresCustomerAction
            || bank_transfer_next_steps.is_some()
            || next_action_voucher.is_some()
            || next_action_containing_qr_code_url.is_some()
            || next_action_containing_wait_screen.is_some()
            || papal_sdk_next_action.is_some()
            || next_action_containing_fetch_qr_code_url.is_some()
            || payment_data.get_authentication().is_some()
        {
            next_action_response = bank_transfer_next_steps
                        .map(|bank_transfer| {
                            api_models::payments::NextActionData::DisplayBankTransferInformation {
                                bank_transfer_steps_and_charges_details: bank_transfer,
                            }
                        })
                        .or(next_action_voucher.map(|voucher_data| {
                            api_models::payments::NextActionData::DisplayVoucherInformation {
                                voucher_details: voucher_data,
                            }
                        }))
                        .or(next_action_mobile_payment.map(|mobile_payment_data| {
                            api_models::payments::NextActionData::CollectOtp {
                                consent_data_required: mobile_payment_data.consent_data_required,
                            }
                        }))
                        .or(next_action_containing_qr_code_url.map(|qr_code_data| {
                            api_models::payments::NextActionData::foreign_from(qr_code_data)
                        }))
                        .or(next_action_containing_fetch_qr_code_url.map(|fetch_qr_code_data| {
                            api_models::payments::NextActionData::FetchQrCodeInformation {
                                qr_code_fetch_url: fetch_qr_code_data.qr_code_fetch_url
                            }
                        }))
                        .or(papal_sdk_next_action.map(|paypal_next_action_data| {
                            api_models::payments::NextActionData::InvokeSdkClient{
                                next_action_data: paypal_next_action_data
                            }
                        }))
                        .or(next_action_containing_wait_screen.map(|wait_screen_data| {
                            api_models::payments::NextActionData::WaitScreenInformation {
                                display_from_timestamp: wait_screen_data.display_from_timestamp,
                                display_to_timestamp: wait_screen_data.display_to_timestamp,
                            }
                        }))
                        .or(payment_attempt.authentication_data.as_ref().map(|_| {
                            api_models::payments::NextActionData::RedirectToUrl {
                                redirect_to_url: helpers::create_startpay_url(
                                    base_url,
                                    &payment_attempt,
                                    &payment_intent,
                                ),
                            }
                        }))
                        .or(match payment_data.get_authentication().as_ref(){
                            Some(authentication) => {
                                if payment_intent.status == common_enums::IntentStatus::RequiresCustomerAction && authentication.cavv.is_none() && authentication.is_separate_authn_required(){
                                    // if preAuthn and separate authentication needed.
                                    let poll_config = payment_data.get_poll_config().unwrap_or_default();
                                    let request_poll_id = core_utils::get_external_authentication_request_poll_id(&payment_intent.payment_id);
                                    let payment_connector_name = payment_attempt.connector
                                        .as_ref()
                                        .get_required_value("connector")?;
                                    Some(api_models::payments::NextActionData::ThreeDsInvoke {
                                        three_ds_data: api_models::payments::ThreeDsData {
                                            three_ds_authentication_url: helpers::create_authentication_url(base_url, &payment_attempt),
                                            three_ds_authorize_url: helpers::create_authorize_url(
                                                base_url,
                                                &payment_attempt,
                                                payment_connector_name,
                                            ),
                                            three_ds_method_details: authentication.three_ds_method_url.as_ref().zip(authentication.three_ds_method_data.as_ref()).map(|(three_ds_method_url,three_ds_method_data )|{
                                                api_models::payments::ThreeDsMethodData::AcsThreeDsMethodData {
                                                    three_ds_method_data_submission: true,
                                                    three_ds_method_data: Some(three_ds_method_data.clone()),
                                                    three_ds_method_url: Some(three_ds_method_url.to_owned()),
                                                }
                                            }).unwrap_or(api_models::payments::ThreeDsMethodData::AcsThreeDsMethodData {
                                                    three_ds_method_data_submission: false,
                                                    three_ds_method_data: None,
                                                    three_ds_method_url: None,
                                            }),
                                            poll_config: api_models::payments::PollConfigResponse {poll_id: request_poll_id, delay_in_secs: poll_config.delay_in_secs, frequency: poll_config.frequency},
                                            message_version: authentication.message_version.as_ref()
                                            .map(|version| version.to_string()),
                                            directory_server_id: authentication.directory_server_id.clone(),
                                        },
                                    })
                                }else{
                                    None
                                }
                            },
                            None => None
                        });
        };

        // next action check for third party sdk session (for ex: Apple pay through trustpay has third party sdk session response)
        if third_party_sdk_session_next_action(&payment_attempt, operation) {
            next_action_response = Some(
                api_models::payments::NextActionData::ThirdPartySdkSessionToken {
                    session_token: payment_data.get_sessions_token().first().cloned(),
                },
            )
        }

        let routed_through = payment_attempt.connector.clone();

        let connector_label = routed_through.as_ref().and_then(|connector_name| {
            core_utils::get_connector_label(
                payment_intent.business_country,
                payment_intent.business_label.as_ref(),
                payment_attempt.business_sub_label.as_ref(),
                connector_name,
            )
        });

        let mandate_data = payment_data.get_setup_mandate().map(|d| api::MandateData {
            customer_acceptance: d
                .customer_acceptance
                .clone()
                .map(|d| api::CustomerAcceptance {
                    acceptance_type: match d.acceptance_type {
                        hyperswitch_domain_models::mandates::AcceptanceType::Online => {
                            api::AcceptanceType::Online
                        }
                        hyperswitch_domain_models::mandates::AcceptanceType::Offline => {
                            api::AcceptanceType::Offline
                        }
                    },
                    accepted_at: d.accepted_at,
                    online: d.online.map(|d| api::OnlineMandate {
                        ip_address: d.ip_address,
                        user_agent: d.user_agent,
                    }),
                }),
            mandate_type: d.mandate_type.clone().map(|d| match d {
                hyperswitch_domain_models::mandates::MandateDataType::MultiUse(Some(i)) => {
                    api::MandateType::MultiUse(Some(api::MandateAmountData {
                        amount: i.amount,
                        currency: i.currency,
                        start_date: i.start_date,
                        end_date: i.end_date,
                        metadata: i.metadata,
                    }))
                }
                hyperswitch_domain_models::mandates::MandateDataType::SingleUse(i) => {
                    api::MandateType::SingleUse(api::payments::MandateAmountData {
                        amount: i.amount,
                        currency: i.currency,
                        start_date: i.start_date,
                        end_date: i.end_date,
                        metadata: i.metadata,
                    })
                }
                hyperswitch_domain_models::mandates::MandateDataType::MultiUse(None) => {
                    api::MandateType::MultiUse(None)
                }
            }),
            update_mandate_id: d.update_mandate_id.clone(),
        });

        let order_tax_amount = payment_data
            .get_payment_attempt()
            .net_amount
            .get_order_tax_amount()
            .or_else(|| {
                payment_data
                    .get_payment_intent()
                    .tax_details
                    .clone()
                    .and_then(|tax| {
                        tax.payment_method_type
                            .map(|a| a.order_tax_amount)
                            .or_else(|| tax.default.map(|a| a.order_tax_amount))
                    })
            });
        let connector_mandate_id = payment_data.get_mandate_id().and_then(|mandate| {
            mandate
                .mandate_reference_id
                .as_ref()
                .and_then(|mandate_ref| match mandate_ref {
                    api_models::payments::MandateReferenceId::ConnectorMandateId(
                        connector_mandate_reference_id,
                    ) => connector_mandate_reference_id.get_connector_mandate_id(),
                    _ => None,
                })
        });

        let connector_transaction_id = payment_attempt
            .get_connector_payment_id()
            .map(ToString::to_string);

        let payments_response = api::PaymentsResponse {
            payment_id: payment_intent.payment_id,
            merchant_id: payment_intent.merchant_id,
            status: payment_intent.status,
            amount: payment_attempt.net_amount.get_order_amount(),
            net_amount: payment_attempt.get_total_amount(),
            amount_capturable: payment_attempt.amount_capturable,
            amount_received: payment_intent.amount_captured,
            connector: routed_through,
            client_secret: payment_intent.client_secret.map(Secret::new),
            created: Some(payment_intent.created_at),
            currency: currency.to_string(),
            customer_id: customer.as_ref().map(|cus| cus.clone().customer_id),
            customer: customer_details_response,
            description: payment_intent.description,
            refunds: refunds_response,
            disputes: disputes_response,
            attempts: attempts_response,
            captures: captures_response,
            mandate_id,
            mandate_data,
            setup_future_usage: payment_intent.setup_future_usage,
            off_session: payment_intent.off_session,
            capture_on: None,
            capture_method: payment_attempt.capture_method,
            payment_method: payment_attempt.payment_method,
            payment_method_data: payment_method_data_response,
            payment_token: payment_attempt.payment_token,
            shipping: payment_data
                .get_address()
                .get_shipping()
                .cloned()
                .map(From::from),
            billing: payment_data
                .get_address()
                .get_payment_billing()
                .cloned()
                .map(From::from),
            order_details: payment_intent.order_details,
            email: customer
                .as_ref()
                .and_then(|cus| cus.email.as_ref().map(|s| s.to_owned())),
            name: customer
                .as_ref()
                .and_then(|cus| cus.name.as_ref().map(|s| s.to_owned())),
            phone: customer
                .as_ref()
                .and_then(|cus| cus.phone.as_ref().map(|s| s.to_owned())),
            return_url: payment_intent.return_url,
            authentication_type: payment_attempt.authentication_type,
            statement_descriptor_name: payment_intent.statement_descriptor_name,
            statement_descriptor_suffix: payment_intent.statement_descriptor_suffix,
            next_action: next_action_response,
            cancellation_reason: payment_attempt.cancellation_reason,
            error_code: payment_attempt.error_code,
            error_message: payment_attempt
                .error_reason
                .or(payment_attempt.error_message),
            unified_code: payment_attempt.unified_code,
            unified_message: payment_attempt.unified_message,
            payment_experience: payment_attempt.payment_experience,
            payment_method_type: payment_attempt.payment_method_type,
            connector_label,
            business_country: payment_intent.business_country,
            business_label: payment_intent.business_label,
            business_sub_label: payment_attempt.business_sub_label,
            allowed_payment_method_types: payment_intent.allowed_payment_method_types,
            ephemeral_key: payment_data
                .get_ephemeral_key()
                .map(ForeignFrom::foreign_from),
            manual_retry_allowed: helpers::is_manual_retry_allowed(
                &payment_intent.status,
                &payment_attempt.status,
                connector_request_reference_id_config,
                &merchant_id,
            ),
            connector_transaction_id,
            frm_message,
            metadata: payment_intent.metadata,
            connector_metadata: payment_intent.connector_metadata,
            feature_metadata: payment_intent.feature_metadata,
            reference_id: payment_attempt.connector_response_reference_id,
            payment_link: payment_link_data,
            profile_id: payment_intent.profile_id,
            surcharge_details,
            attempt_count: payment_intent.attempt_count,
            merchant_decision,
            merchant_connector_id: payment_attempt.merchant_connector_id,
            incremental_authorization_allowed: payment_intent.incremental_authorization_allowed,
            authorization_count: payment_intent.authorization_count,
            incremental_authorizations: incremental_authorizations_response,
            external_authentication_details,
            external_3ds_authentication_attempted: payment_attempt
                .external_three_ds_authentication_attempted,
            expires_on: payment_intent.session_expiry,
            fingerprint: payment_intent.fingerprint_id,
            browser_info: payment_attempt.browser_info,
            payment_method_id: payment_attempt.payment_method_id,
            payment_method_status: payment_data
                .get_payment_method_info()
                .map(|info| info.status),
            updated: Some(payment_intent.modified_at),
            split_payments: payment_attempt.charges,
            frm_metadata: payment_intent.frm_metadata,
            merchant_order_reference_id: payment_intent.merchant_order_reference_id,
            order_tax_amount,
            connector_mandate_id,
            shipping_cost: payment_intent.shipping_cost,
            capture_before: payment_attempt.capture_before,
            extended_authorization_applied: payment_attempt.extended_authorization_applied,
            card_discovery: payment_attempt.card_discovery,
        };

        services::ApplicationResponse::JsonWithHeaders((payments_response, headers))
    };

    metrics::PAYMENT_OPS_COUNT.add(
        1,
        router_env::metric_attributes!(
            ("operation", format!("{:?}", operation)),
            ("merchant", merchant_id.clone()),
            ("payment_method_type", payment_method_type),
            ("payment_method", payment_method),
        ),
    );

    Ok(output)
}

#[cfg(feature = "v1")]
pub fn third_party_sdk_session_next_action<Op>(
    payment_attempt: &storage::PaymentAttempt,
    operation: &Op,
) -> bool
where
    Op: Debug,
{
    // If the operation is confirm, we will send session token response in next action
    if format!("{operation:?}").eq("PaymentConfirm") {
        let condition1 = payment_attempt
            .connector
            .as_ref()
            .map(|connector| {
                matches!(connector.as_str(), "trustpay") || matches!(connector.as_str(), "payme")
            })
            .and_then(|is_connector_supports_third_party_sdk| {
                if is_connector_supports_third_party_sdk {
                    payment_attempt
                        .payment_method
                        .map(|pm| matches!(pm, diesel_models::enums::PaymentMethod::Wallet))
                } else {
                    Some(false)
                }
            })
            .unwrap_or(false);

        // This condition to be triggered for open banking connectors, third party SDK session token will be provided
        let condition2 = payment_attempt
            .connector
            .as_ref()
            .map(|connector| matches!(connector.as_str(), "plaid"))
            .and_then(|is_connector_supports_third_party_sdk| {
                if is_connector_supports_third_party_sdk {
                    payment_attempt
                        .payment_method
                        .map(|pm| matches!(pm, diesel_models::enums::PaymentMethod::OpenBanking))
                        .and_then(|first_match| {
                            payment_attempt
                                .payment_method_type
                                .map(|pmt| {
                                    matches!(
                                        pmt,
                                        diesel_models::enums::PaymentMethodType::OpenBankingPIS
                                    )
                                })
                                .map(|second_match| first_match && second_match)
                        })
                } else {
                    Some(false)
                }
            })
            .unwrap_or(false);

        condition1 || condition2
    } else {
        false
    }
}

pub fn qr_code_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::QrCodeInformation>> {
    let qr_code_steps: Option<Result<api_models::payments::QrCodeInformation, _>> = payment_attempt
        .connector_metadata
        .map(|metadata| metadata.parse_value("QrCodeInformation"));

    let qr_code_instructions = qr_code_steps.transpose().ok().flatten();
    Ok(qr_code_instructions)
}
pub fn paypal_sdk_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::SdkNextActionData>> {
    let paypal_connector_metadata: Option<Result<api_models::payments::SdkNextActionData, _>> =
        payment_attempt.connector_metadata.map(|metadata| {
            metadata.parse_value("SdkNextActionData").map_err(|_| {
                crate::logger::warn!(
                    "SdkNextActionData parsing failed for paypal_connector_metadata"
                )
            })
        });

    let paypal_next_steps = paypal_connector_metadata.transpose().ok().flatten();
    Ok(paypal_next_steps)
}

pub fn fetch_qr_code_url_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::FetchQrCodeInformation>> {
    let qr_code_steps: Option<Result<api_models::payments::FetchQrCodeInformation, _>> =
        payment_attempt
            .connector_metadata
            .map(|metadata| metadata.parse_value("FetchQrCodeInformation"));

    let qr_code_fetch_url = qr_code_steps.transpose().ok().flatten();
    Ok(qr_code_fetch_url)
}

pub fn wait_screen_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::WaitScreenInstructions>> {
    let display_info_with_timer_steps: Option<
        Result<api_models::payments::WaitScreenInstructions, _>,
    > = payment_attempt
        .connector_metadata
        .map(|metadata| metadata.parse_value("WaitScreenInstructions"));

    let display_info_with_timer_instructions =
        display_info_with_timer_steps.transpose().ok().flatten();
    Ok(display_info_with_timer_instructions)
}

#[cfg(feature = "v1")]
impl ForeignFrom<(storage::PaymentIntent, storage::PaymentAttempt)> for api::PaymentsResponse {
    fn foreign_from((pi, pa): (storage::PaymentIntent, storage::PaymentAttempt)) -> Self {
        let connector_transaction_id = pa.get_connector_payment_id().map(ToString::to_string);
        Self {
            payment_id: pi.payment_id,
            merchant_id: pi.merchant_id,
            status: pi.status,
            amount: pi.amount,
            amount_capturable: pa.amount_capturable,
            client_secret: pi.client_secret.map(|s| s.into()),
            created: Some(pi.created_at),
            currency: pi.currency.map(|c| c.to_string()).unwrap_or_default(),
            description: pi.description,
            metadata: pi.metadata,
            order_details: pi.order_details,
            customer_id: pi.customer_id.clone(),
            connector: pa.connector,
            payment_method: pa.payment_method,
            payment_method_type: pa.payment_method_type,
            business_label: pi.business_label,
            business_country: pi.business_country,
            business_sub_label: pa.business_sub_label,
            setup_future_usage: pi.setup_future_usage,
            capture_method: pa.capture_method,
            authentication_type: pa.authentication_type,
            connector_transaction_id,
            attempt_count: pi.attempt_count,
            profile_id: pi.profile_id,
            merchant_connector_id: pa.merchant_connector_id,
            payment_method_data: pa.payment_method_data.and_then(|data| {
                match data.parse_value("PaymentMethodDataResponseWithBilling") {
                    Ok(parsed_data) => Some(parsed_data),
                    Err(e) => {
                        router_env::logger::error!("Failed to parse 'PaymentMethodDataResponseWithBilling' from payment method data. Error: {e:?}");
                        None
                    }
                }
            }),
            merchant_order_reference_id: pi.merchant_order_reference_id,
            customer: pi.customer_details.and_then(|customer_details|
                match customer_details.into_inner().expose().parse_value::<CustomerData>("CustomerData"){
                    Ok(parsed_data) => Some(
                        CustomerDetailsResponse {
                            id: pi.customer_id,
                            name: parsed_data.name,
                            phone: parsed_data.phone,
                            email: parsed_data.email,
                            phone_country_code:parsed_data.phone_country_code
                    }),
                    Err(e) => {
                        router_env::logger::error!("Failed to parse 'CustomerDetailsResponse' from payment method data. Error: {e:?}");
                        None
                    }
                }
            ),
            billing: pi.billing_details.and_then(|billing_details|
                match billing_details.into_inner().expose().parse_value::<Address>("Address") {
                    Ok(parsed_data) => Some(parsed_data),
                    Err(e) => {
                        router_env::logger::error!("Failed to parse 'BillingAddress' from payment method data. Error: {e:?}");
                        None
                    }
                }
            ),
            shipping: pi.shipping_details.and_then(|shipping_details|
                match shipping_details.into_inner().expose().parse_value::<Address>("Address") {
                    Ok(parsed_data) => Some(parsed_data),
                    Err(e) => {
                        router_env::logger::error!("Failed to parse 'ShippingAddress' from payment method data. Error: {e:?}");
                        None
                    }
                }
            ),
            // TODO: fill in details based on requirement
            net_amount: pa.net_amount.get_total_amount(),
            amount_received: None,
            refunds: None,
            disputes: None,
            attempts: None,
            captures: None,
            mandate_id: None,
            mandate_data: None,
            off_session: None,
            capture_on: None,
            payment_token: None,
            email: None,
            name: None,
            phone: None,
            return_url: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            next_action: None,
            cancellation_reason: None,
            error_code: None,
            error_message: None,
            unified_code: None,
            unified_message: None,
            payment_experience: None,
            connector_label: None,
            allowed_payment_method_types: None,
            ephemeral_key: None,
            manual_retry_allowed: None,
            frm_message: None,
            connector_metadata: None,
            feature_metadata: None,
            reference_id: None,
            payment_link: None,
            surcharge_details: None,
            merchant_decision: None,
            incremental_authorization_allowed: None,
            authorization_count: None,
            incremental_authorizations: None,
            external_authentication_details: None,
            external_3ds_authentication_attempted: None,
            expires_on: None,
            fingerprint: None,
            browser_info: None,
            payment_method_id: None,
            payment_method_status: None,
            updated: None,
            split_payments: None,
            frm_metadata: None,
            capture_before: pa.capture_before,
            extended_authorization_applied: pa.extended_authorization_applied,
            order_tax_amount: None,
            connector_mandate_id:None,
            shipping_cost: None,
            card_discovery: pa.card_discovery
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<ephemeral_key::EphemeralKey> for api::ephemeral_key::EphemeralKeyCreateResponse {
    fn foreign_from(from: ephemeral_key::EphemeralKey) -> Self {
        Self {
            customer_id: from.customer_id,
            created_at: from.created_at,
            expires: from.expires,
            secret: from.secret,
        }
    }
}

#[cfg(feature = "v1")]
pub fn bank_transfer_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::BankTransferNextStepsData>> {
    let bank_transfer_next_step = if let Some(diesel_models::enums::PaymentMethod::BankTransfer) =
        payment_attempt.payment_method
    {
        if payment_attempt.payment_method_type != Some(diesel_models::enums::PaymentMethodType::Pix)
        {
            let bank_transfer_next_steps: Option<api_models::payments::BankTransferNextStepsData> =
                payment_attempt
                    .connector_metadata
                    .map(|metadata| {
                        metadata
                            .parse_value("NextStepsRequirements")
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable(
                                "Failed to parse the Value to NextRequirements struct",
                            )
                    })
                    .transpose()?;
            bank_transfer_next_steps
        } else {
            None
        }
    } else {
        None
    };
    Ok(bank_transfer_next_step)
}

#[cfg(feature = "v1")]
pub fn voucher_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::VoucherNextStepData>> {
    let voucher_next_step = if let Some(diesel_models::enums::PaymentMethod::Voucher) =
        payment_attempt.payment_method
    {
        let voucher_next_steps: Option<api_models::payments::VoucherNextStepData> = payment_attempt
            .connector_metadata
            .map(|metadata| {
                metadata
                    .parse_value("NextStepsRequirements")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse the Value to NextRequirements struct")
            })
            .transpose()?;
        voucher_next_steps
    } else {
        None
    };
    Ok(voucher_next_step)
}

#[cfg(feature = "v1")]
pub fn mobile_payment_next_steps_check(
    payment_attempt: &storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::MobilePaymentNextStepData>> {
    let mobile_payment_next_step = if let Some(diesel_models::enums::PaymentMethod::MobilePayment) =
        payment_attempt.payment_method
    {
        let mobile_paymebnt_next_steps: Option<api_models::payments::MobilePaymentNextStepData> =
            payment_attempt
                .connector_metadata
                .clone()
                .map(|metadata| {
                    metadata
                        .parse_value("MobilePaymentNextStepData")
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to parse the Value to NextRequirements struct")
                })
                .transpose()?;
        mobile_paymebnt_next_steps
    } else {
        None
    };
    Ok(mobile_payment_next_step)
}

impl ForeignFrom<api_models::payments::QrCodeInformation> for api_models::payments::NextActionData {
    fn foreign_from(qr_info: api_models::payments::QrCodeInformation) -> Self {
        match qr_info {
            api_models::payments::QrCodeInformation::QrCodeUrl {
                image_data_url,
                qr_code_url,
                display_to_timestamp,
            } => Self::QrCodeInformation {
                image_data_url: Some(image_data_url),
                qr_code_url: Some(qr_code_url),
                display_to_timestamp,
            },
            api_models::payments::QrCodeInformation::QrDataUrl {
                image_data_url,
                display_to_timestamp,
            } => Self::QrCodeInformation {
                image_data_url: Some(image_data_url),
                display_to_timestamp,
                qr_code_url: None,
            },
            api_models::payments::QrCodeInformation::QrCodeImageUrl {
                qr_code_url,
                display_to_timestamp,
            } => Self::QrCodeInformation {
                qr_code_url: Some(qr_code_url),
                image_data_url: None,
                display_to_timestamp,
            },
        }
    }
}

#[derive(Clone)]
pub struct PaymentAdditionalData<'a, F>
where
    F: Clone,
{
    router_base_url: String,
    connector_name: String,
    payment_data: PaymentData<F>,
    state: &'a SessionState,
    customer_data: &'a Option<domain::Customer>,
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsAuthorizeData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(_additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsAuthorizeData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data.clone();
        let router_base_url = &additional_data.router_base_url;
        let connector_name = &additional_data.connector_name;
        let attempt = &payment_data.payment_attempt;
        let browser_info: Option<types::BrowserInformation> = attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        let order_category = additional_data
            .payment_data
            .payment_intent
            .connector_metadata
            .map(|cm| {
                cm.parse_value::<api_models::payments::ConnectorMetadata>("ConnectorMetadata")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed parsing ConnectorMetadata")
            })
            .transpose()?
            .and_then(|cm| cm.noon.and_then(|noon| noon.order_category));

        let order_details = additional_data
            .payment_data
            .payment_intent
            .order_details
            .map(|order_details| {
                order_details
                    .iter()
                    .map(|data| {
                        data.to_owned()
                            .parse_value("OrderDetailsWithAmount")
                            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                                field_name: "OrderDetailsWithAmount",
                            })
                            .attach_printable("Unable to parse OrderDetailsWithAmount")
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;

        let complete_authorize_url = Some(helpers::create_complete_authorize_url(
            router_base_url,
            attempt,
            connector_name,
        ));
        let merchant_connector_account_id_or_connector_name = payment_data
            .payment_attempt
            .merchant_connector_id
            .as_ref()
            .map(|mca_id| mca_id.get_string_repr())
            .unwrap_or(connector_name);

        let webhook_url = Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            merchant_connector_account_id_or_connector_name,
        ));
        let router_return_url = Some(helpers::create_redirect_url(
            router_base_url,
            attempt,
            connector_name,
            payment_data.creds_identifier.as_deref(),
        ));

        let additional_payment_method_data = if payment_data.mandate_id.is_some() {
            let parsed_additional_payment_data: Option<api_models::payments::AdditionalPaymentData> =
                payment_data.payment_attempt
                    .payment_method_data
                    .as_ref().map(|data| data.clone().parse_value("AdditionalPaymentData"))
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse AdditionalPaymentData from payment_data.payment_attempt.payment_method_data")?;
            parsed_additional_payment_data
        } else {
            None
        };
        let payment_method_data = payment_data.payment_method_data.or_else(|| {
            if payment_data.mandate_id.is_some() {
                Some(domain::PaymentMethodData::MandatePayment)
            } else {
                None
            }
        });

        let amount = payment_data.payment_attempt.get_total_amount();

        let customer_name = additional_data
            .customer_data
            .as_ref()
            .and_then(|customer_data| {
                customer_data
                    .name
                    .as_ref()
                    .map(|customer| customer.clone().into_inner())
            });

        let customer_id = additional_data
            .customer_data
            .as_ref()
            .map(|data| data.customer_id.clone());

        let split_payments = payment_data.payment_intent.split_payments.clone();

        let merchant_order_reference_id = payment_data
            .payment_intent
            .merchant_order_reference_id
            .clone();
        let shipping_cost = payment_data.payment_intent.shipping_cost;

        Ok(Self {
            payment_method_data: (payment_method_data.get_required_value("payment_method_data")?),
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            mandate_id: payment_data.mandate_id.clone(),
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            setup_mandate_details: payment_data.setup_mandate.clone(),
            confirm: payment_data.payment_attempt.confirm,
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            statement_descriptor: payment_data.payment_intent.statement_descriptor_name,
            capture_method: payment_data.payment_attempt.capture_method,
            amount: amount.get_amount_as_i64(),
            order_tax_amount: payment_data
                .payment_attempt
                .net_amount
                .get_order_tax_amount(),
            minor_amount: amount,
            currency: payment_data.currency,
            browser_info,
            email: payment_data.email,
            customer_name,
            payment_experience: payment_data.payment_attempt.payment_experience,
            order_details,
            order_category,
            session_token: None,
            enrolled_for_3ds: true,
            related_transaction_id: None,
            payment_method_type: payment_data.payment_attempt.payment_method_type,
            router_return_url,
            webhook_url,
            complete_authorize_url,
            customer_id,
            surcharge_details: payment_data.surcharge_details,
            request_incremental_authorization: matches!(
                payment_data
                    .payment_intent
                    .request_incremental_authorization,
                Some(RequestIncrementalAuthorization::True)
                    | Some(RequestIncrementalAuthorization::Default)
            ),
            metadata: additional_data.payment_data.payment_intent.metadata,
            authentication_data: payment_data
                .authentication
                .as_ref()
                .map(AuthenticationData::foreign_try_from)
                .transpose()?,
            customer_acceptance: payment_data.customer_acceptance,
            split_payments,
            merchant_order_reference_id,
            integrity_object: None,
            additional_payment_method_data,
            shipping_cost,
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsSyncData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsSyncData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let capture_method = payment_data.get_capture_method();
        let amount = payment_data.payment_attempt.get_total_amount();

        let payment_method_type = payment_data
            .payment_attempt
            .get_payment_method_type()
            .to_owned();
        Ok(Self {
            amount,
            integrity_object: None,
            mandate_id: payment_data.mandate_id.clone(),
            connector_transaction_id: match payment_data.payment_attempt.get_connector_payment_id()
            {
                Some(connector_txn_id) => {
                    types::ResponseId::ConnectorTransactionId(connector_txn_id.to_owned())
                }
                None => types::ResponseId::NoResponseId,
            },
            encoded_data: payment_data.payment_attempt.encoded_data,
            capture_method,
            connector_meta: payment_data.payment_attempt.connector_metadata,
            sync_type: match payment_data.multiple_capture_data {
                Some(multiple_capture_data) => types::SyncRequestType::MultipleCaptureSync(
                    multiple_capture_data.get_pending_connector_capture_ids(),
                ),
                None => types::SyncRequestType::SinglePaymentSync,
            },
            payment_method_type,
            currency: payment_data.currency,
            split_payments: payment_data.payment_intent.split_payments,
            payment_experience: payment_data.payment_attempt.payment_experience,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>>
    for types::PaymentsIncrementalAuthorizationData
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        let total_amount = payment_data
            .incremental_authorization_details
            .clone()
            .map(|details| details.total_amount)
            .ok_or(
                report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("missing incremental_authorization_details in payment_data"),
            )?;
        let additional_amount = payment_data
            .incremental_authorization_details
            .clone()
            .map(|details| details.additional_amount)
            .ok_or(
                report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("missing incremental_authorization_details in payment_data"),
            )?;
        Ok(Self {
            total_amount: total_amount.get_amount_as_i64(),
            additional_amount: additional_amount.get_amount_as_i64(),
            reason: payment_data
                .incremental_authorization_details
                .and_then(|details| details.reason),
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_data.payment_attempt.clone())?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
        })
    }
}

impl ConnectorTransactionId for Helcim {
    #[cfg(feature = "v1")]
    fn connector_transaction_id(
        &self,
        payment_attempt: storage::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        if payment_attempt.get_connector_payment_id().is_none() {
            let metadata =
                Self::connector_transaction_id(self, payment_attempt.connector_metadata.as_ref());
            metadata.map_err(|_| errors::ApiErrorResponse::ResourceIdNotFound)
        } else {
            Ok(payment_attempt
                .get_connector_payment_id()
                .map(ToString::to_string))
        }
    }

    #[cfg(feature = "v2")]
    fn connector_transaction_id(
        &self,
        payment_attempt: storage::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        if payment_attempt.get_connector_payment_id().is_none() {
            let metadata = Self::connector_transaction_id(
                self,
                payment_attempt
                    .connector_metadata
                    .as_ref()
                    .map(|connector_metadata| connector_metadata.peek()),
            );
            metadata.map_err(|_| errors::ApiErrorResponse::ResourceIdNotFound)
        } else {
            Ok(payment_attempt
                .get_connector_payment_id()
                .map(ToString::to_string))
        }
    }
}

impl ConnectorTransactionId for Nexinets {
    #[cfg(feature = "v1")]
    fn connector_transaction_id(
        &self,
        payment_attempt: storage::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        let metadata =
            Self::connector_transaction_id(self, payment_attempt.connector_metadata.as_ref());
        metadata.map_err(|_| errors::ApiErrorResponse::ResourceIdNotFound)
    }

    #[cfg(feature = "v2")]
    fn connector_transaction_id(
        &self,
        payment_attempt: storage::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        let metadata = Self::connector_transaction_id(
            self,
            payment_attempt
                .connector_metadata
                .as_ref()
                .map(|connector_metadata| connector_metadata.peek()),
        );
        metadata.map_err(|_| errors::ApiErrorResponse::ResourceIdNotFound)
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsCaptureData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        use masking::ExposeOptionInterface;

        let payment_data = additional_data.payment_data;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        let amount_to_capture = payment_data
            .payment_attempt
            .amount_details
            .get_amount_to_capture()
            .unwrap_or(payment_data.payment_attempt.get_total_amount());

        let amount = payment_data.payment_attempt.get_total_amount();
        Ok(Self {
            capture_method: Some(payment_data.payment_intent.capture_method),
            amount_to_capture: amount_to_capture.get_amount_as_i64(), // This should be removed once we start moving to connector module
            minor_amount_to_capture: amount_to_capture,
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_data.payment_attempt.clone())?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            payment_amount: amount.get_amount_as_i64(), // This should be removed once we start moving to connector module
            minor_payment_amount: amount,
            connector_meta: payment_data
                .payment_attempt
                .connector_metadata
                .expose_option(),
            // TODO: add multiple capture data
            multiple_capture_data: None,
            // TODO: why do we need browser info during capture?
            browser_info: None,
            metadata: payment_data.payment_intent.metadata.expose_option(),
            integrity_object: None,
        })
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsCaptureData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        let amount_to_capture = payment_data
            .payment_attempt
            .amount_to_capture
            .unwrap_or(payment_data.payment_attempt.get_total_amount());
        let browser_info: Option<types::BrowserInformation> = payment_data
            .payment_attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;
        let amount = payment_data.payment_attempt.get_total_amount();
        Ok(Self {
            capture_method: payment_data.get_capture_method(),
            amount_to_capture: amount_to_capture.get_amount_as_i64(), // This should be removed once we start moving to connector module
            minor_amount_to_capture: amount_to_capture,
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_data.payment_attempt.clone())?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            payment_amount: amount.get_amount_as_i64(), // This should be removed once we start moving to connector module
            minor_payment_amount: amount,
            connector_meta: payment_data.payment_attempt.connector_metadata,
            multiple_capture_data: match payment_data.multiple_capture_data {
                Some(multiple_capture_data) => Some(MultipleCaptureRequestData {
                    capture_sequence: multiple_capture_data.get_captures_count()?,
                    capture_reference: multiple_capture_data
                        .get_latest_capture()
                        .capture_id
                        .clone(),
                }),
                None => None,
            },
            browser_info,
            metadata: payment_data.payment_intent.metadata,
            integrity_object: None,
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsCancelData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsCancelData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        let browser_info: Option<types::BrowserInformation> = payment_data
            .payment_attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;
        let amount = payment_data.payment_attempt.get_total_amount();
        Ok(Self {
            amount: Some(amount.get_amount_as_i64()), // This should be removed once we start moving to connector module
            minor_amount: Some(amount),
            currency: Some(payment_data.currency),
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_data.payment_attempt.clone())?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            cancellation_reason: payment_data.payment_attempt.cancellation_reason,
            connector_meta: payment_data.payment_attempt.connector_metadata,
            browser_info,
            metadata: payment_data.payment_intent.metadata,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsApproveData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let amount = payment_data.payment_attempt.get_total_amount();
        Ok(Self {
            amount: Some(amount.get_amount_as_i64()), //need to change after we move to connector module
            currency: Some(payment_data.currency),
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::SdkPaymentsSessionUpdateData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::SdkPaymentsSessionUpdateData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let order_tax_amount = payment_data
            .payment_intent
            .tax_details
            .clone()
            .and_then(|tax| tax.payment_method_type.map(|pmt| pmt.order_tax_amount))
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "order_tax_amount",
            })?;
        let surcharge_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.get_total_surcharge_amount())
            .unwrap_or_default();
        let shipping_cost = payment_data
            .payment_intent
            .shipping_cost
            .unwrap_or_default();
        // net_amount here would include amount, order_tax_amount, surcharge_amount and shipping_cost
        let net_amount = payment_data.payment_intent.amount
            + order_tax_amount
            + shipping_cost
            + surcharge_amount;
        Ok(Self {
            amount: net_amount,
            order_tax_amount,
            currency: payment_data.currency,
            order_amount: payment_data.payment_intent.amount,
            session_id: payment_data.session_id,
            shipping_cost: payment_data.payment_intent.shipping_cost,
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsPostSessionTokensData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsPostSessionTokensData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data.clone();
        let surcharge_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.get_total_surcharge_amount())
            .unwrap_or_default();
        let shipping_cost = payment_data
            .payment_intent
            .shipping_cost
            .unwrap_or_default();
        // amount here would include amount, surcharge_amount and shipping_cost
        let amount = payment_data.payment_intent.amount + shipping_cost + surcharge_amount;
        let merchant_order_reference_id = payment_data
            .payment_intent
            .merchant_order_reference_id
            .clone();
        let router_base_url = &additional_data.router_base_url;
        let connector_name = &additional_data.connector_name;
        let attempt = &payment_data.payment_attempt;
        let router_return_url = Some(helpers::create_redirect_url(
            router_base_url,
            attempt,
            connector_name,
            payment_data.creds_identifier.as_deref(),
        ));
        Ok(Self {
            amount, //need to change after we move to connector module
            order_amount: payment_data.payment_intent.amount,
            currency: payment_data.currency,
            merchant_order_reference_id,
            capture_method: payment_data.payment_attempt.capture_method,
            shipping_cost: payment_data.payment_intent.shipping_cost,
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            router_return_url,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsRejectData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let amount = payment_data.payment_attempt.get_total_amount();
        Ok(Self {
            amount: Some(amount.get_amount_as_i64()), //need to change after we move to connector module
            currency: Some(payment_data.currency),
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsSessionData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data.clone();

        let order_details = additional_data
            .payment_data
            .payment_intent
            .order_details
            .map(|order_details| {
                order_details
                    .iter()
                    .map(|data| data.to_owned().expose())
                    .collect()
            });

        let surcharge_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.get_total_surcharge_amount())
            .unwrap_or_default();

        let amount = payment_data.payment_intent.amount_details.order_amount;

        let shipping_cost = payment_data
            .payment_intent
            .amount_details
            .shipping_cost
            .unwrap_or_default();

        // net_amount here would include amount, surcharge_amount and shipping_cost
        let net_amount = amount + surcharge_amount + shipping_cost;

        let required_amount_type = StringMajorUnitForConnector;

        let apple_pay_amount = required_amount_type
            .convert(net_amount, payment_data.currency)
            .change_context(errors::ApiErrorResponse::PreconditionFailed {
                message: "Failed to convert amount to string major unit for applePay".to_string(),
            })?;

        let apple_pay_recurring_details = payment_data
            .payment_intent
            .feature_metadata
            .and_then(|feature_metadata| feature_metadata.apple_pay_recurring_details)
            .map(|apple_pay_recurring_details| {
                ForeignInto::foreign_into((apple_pay_recurring_details, apple_pay_amount))
            });

        Ok(Self {
            amount: amount.get_amount_as_i64(), //need to change once we move to connector module
            minor_amount: amount,
            currency: payment_data.currency,
            country: payment_data.address.get_payment_method_billing().and_then(
                |billing_address| {
                    billing_address
                        .address
                        .as_ref()
                        .and_then(|address| address.country)
                },
            ),
            order_details,
            surcharge_details: payment_data.surcharge_details,
            email: payment_data.email,
            apple_pay_recurring_details,
        })
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsSessionData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data.clone();

        let order_details = additional_data
            .payment_data
            .payment_intent
            .order_details
            .map(|order_details| {
                order_details
                    .iter()
                    .map(|data| {
                        data.to_owned()
                            .parse_value("OrderDetailsWithAmount")
                            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                                field_name: "OrderDetailsWithAmount",
                            })
                            .attach_printable("Unable to parse OrderDetailsWithAmount")
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;

        let surcharge_amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.get_total_surcharge_amount())
            .unwrap_or_default();

        let amount = payment_data.payment_intent.amount;

        let shipping_cost = payment_data
            .payment_intent
            .shipping_cost
            .unwrap_or_default();

        // net_amount here would include amount, surcharge_amount and shipping_cost
        let net_amount = amount + surcharge_amount + shipping_cost;

        let required_amount_type = StringMajorUnitForConnector;

        let apple_pay_amount = required_amount_type
            .convert(net_amount, payment_data.currency)
            .change_context(errors::ApiErrorResponse::PreconditionFailed {
                message: "Failed to convert amount to string major unit for applePay".to_string(),
            })?;

        let apple_pay_recurring_details = payment_data
            .payment_intent
            .feature_metadata
            .map(|feature_metadata| {
                feature_metadata
                    .parse_value::<diesel_models::types::FeatureMetadata>("FeatureMetadata")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed parsing FeatureMetadata")
            })
            .transpose()?
            .and_then(|feature_metadata| feature_metadata.apple_pay_recurring_details)
            .map(|apple_pay_recurring_details| {
                ForeignFrom::foreign_from((apple_pay_recurring_details, apple_pay_amount))
            });

        Ok(Self {
            amount: net_amount.get_amount_as_i64(), //need to change once we move to connector module
            minor_amount: amount,
            currency: payment_data.currency,
            country: payment_data.address.get_payment_method_billing().and_then(
                |billing_address| {
                    billing_address
                        .address
                        .as_ref()
                        .and_then(|address| address.country)
                },
            ),
            order_details,
            email: payment_data.email,
            surcharge_details: payment_data.surcharge_details,
            apple_pay_recurring_details,
        })
    }
}

impl
    ForeignFrom<(
        diesel_models::types::ApplePayRecurringDetails,
        StringMajorUnit,
    )> for api_models::payments::ApplePayRecurringPaymentRequest
{
    fn foreign_from(
        (apple_pay_recurring_details, net_amount): (
            diesel_models::types::ApplePayRecurringDetails,
            StringMajorUnit,
        ),
    ) -> Self {
        Self {
            payment_description: apple_pay_recurring_details.payment_description,
            regular_billing: api_models::payments::ApplePayRegularBillingRequest {
                amount: net_amount,
                label: apple_pay_recurring_details.regular_billing.label,
                payment_timing: api_models::payments::ApplePayPaymentTiming::Recurring,
                recurring_payment_start_date: apple_pay_recurring_details
                    .regular_billing
                    .recurring_payment_start_date,
                recurring_payment_end_date: apple_pay_recurring_details
                    .regular_billing
                    .recurring_payment_end_date,
                recurring_payment_interval_unit: apple_pay_recurring_details
                    .regular_billing
                    .recurring_payment_interval_unit
                    .map(ForeignFrom::foreign_from),
                recurring_payment_interval_count: apple_pay_recurring_details
                    .regular_billing
                    .recurring_payment_interval_count,
            },
            billing_agreement: apple_pay_recurring_details.billing_agreement,
            management_u_r_l: apple_pay_recurring_details.management_url,
        }
    }
}

impl ForeignFrom<diesel_models::types::ApplePayRecurringDetails>
    for api_models::payments::ApplePayRecurringDetails
{
    fn foreign_from(
        apple_pay_recurring_details: diesel_models::types::ApplePayRecurringDetails,
    ) -> Self {
        Self {
            payment_description: apple_pay_recurring_details.payment_description,
            regular_billing: ForeignFrom::foreign_from(apple_pay_recurring_details.regular_billing),
            billing_agreement: apple_pay_recurring_details.billing_agreement,
            management_url: apple_pay_recurring_details.management_url,
        }
    }
}

impl ForeignFrom<diesel_models::types::ApplePayRegularBillingDetails>
    for api_models::payments::ApplePayRegularBillingDetails
{
    fn foreign_from(
        apple_pay_regular_billing: diesel_models::types::ApplePayRegularBillingDetails,
    ) -> Self {
        Self {
            label: apple_pay_regular_billing.label,
            recurring_payment_start_date: apple_pay_regular_billing.recurring_payment_start_date,
            recurring_payment_end_date: apple_pay_regular_billing.recurring_payment_end_date,
            recurring_payment_interval_unit: apple_pay_regular_billing
                .recurring_payment_interval_unit
                .map(ForeignFrom::foreign_from),
            recurring_payment_interval_count: apple_pay_regular_billing
                .recurring_payment_interval_count,
        }
    }
}

impl ForeignFrom<diesel_models::types::RecurringPaymentIntervalUnit>
    for api_models::payments::RecurringPaymentIntervalUnit
{
    fn foreign_from(
        apple_pay_recurring_payment_interval_unit: diesel_models::types::RecurringPaymentIntervalUnit,
    ) -> Self {
        match apple_pay_recurring_payment_interval_unit {
            diesel_models::types::RecurringPaymentIntervalUnit::Day => Self::Day,
            diesel_models::types::RecurringPaymentIntervalUnit::Month => Self::Month,
            diesel_models::types::RecurringPaymentIntervalUnit::Year => Self::Year,
            diesel_models::types::RecurringPaymentIntervalUnit::Hour => Self::Hour,
            diesel_models::types::RecurringPaymentIntervalUnit::Minute => Self::Minute,
        }
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::SetupMandateRequestData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let router_base_url = &additional_data.router_base_url;
        let connector_name = &additional_data.connector_name;
        let attempt = &payment_data.payment_attempt;
        let router_return_url = Some(helpers::create_redirect_url(
            router_base_url,
            attempt,
            connector_name,
            payment_data.creds_identifier.as_deref(),
        ));
        let browser_info: Option<types::BrowserInformation> = attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        let customer_name = additional_data
            .customer_data
            .as_ref()
            .and_then(|customer_data| {
                customer_data
                    .name
                    .as_ref()
                    .map(|customer| customer.clone().into_inner())
            });
        let amount = payment_data.payment_attempt.get_total_amount();
        let merchant_connector_account_id_or_connector_name = payment_data
            .payment_attempt
            .merchant_connector_id
            .as_ref()
            .map(|mca_id| mca_id.get_string_repr())
            .unwrap_or(connector_name);
        let webhook_url = Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            merchant_connector_account_id_or_connector_name,
        ));

        Ok(Self {
            currency: payment_data.currency,
            confirm: true,
            amount: Some(amount.get_amount_as_i64()), //need to change once we move to connector module
            minor_amount: Some(amount),
            payment_method_data: (payment_data
                .payment_method_data
                .get_required_value("payment_method_data")?),
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            mandate_id: payment_data.mandate_id.clone(),
            setup_mandate_details: payment_data.setup_mandate,
            customer_acceptance: payment_data.customer_acceptance,
            router_return_url,
            email: payment_data.email,
            customer_name,
            return_url: payment_data.payment_intent.return_url,
            browser_info,
            payment_method_type: attempt.payment_method_type,
            request_incremental_authorization: matches!(
                payment_data
                    .payment_intent
                    .request_incremental_authorization,
                Some(RequestIncrementalAuthorization::True)
                    | Some(RequestIncrementalAuthorization::Default)
            ),
            metadata: payment_data.payment_intent.metadata.clone().map(Into::into),
            shipping_cost: payment_data.payment_intent.shipping_cost,
            webhook_url,
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::SetupMandateRequestData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl ForeignTryFrom<types::CaptureSyncResponse> for storage::CaptureUpdate {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        capture_sync_response: types::CaptureSyncResponse,
    ) -> Result<Self, Self::Error> {
        match capture_sync_response {
            types::CaptureSyncResponse::Success {
                resource_id,
                status,
                connector_response_reference_id,
                ..
            } => {
                let (connector_capture_id, processor_capture_data) = match resource_id {
                    types::ResponseId::EncodedData(_) | types::ResponseId::NoResponseId => {
                        (None, None)
                    }
                    types::ResponseId::ConnectorTransactionId(id) => {
                        let (txn_id, txn_data) =
                            common_utils_type::ConnectorTransactionId::form_id_and_data(id);
                        (Some(txn_id), txn_data)
                    }
                };
                Ok(Self::ResponseUpdate {
                    status: enums::CaptureStatus::foreign_try_from(status)?,
                    connector_capture_id,
                    connector_response_reference_id,
                    processor_capture_data,
                })
            }
            types::CaptureSyncResponse::Error {
                code,
                message,
                reason,
                status_code,
                ..
            } => Ok(Self::ErrorUpdate {
                status: match status_code {
                    500..=511 => enums::CaptureStatus::Pending,
                    _ => enums::CaptureStatus::Failed,
                },
                error_code: Some(code),
                error_message: Some(message),
                error_reason: reason,
            }),
        }
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::CompleteAuthorizeData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let router_base_url = &additional_data.router_base_url;
        let connector_name = &additional_data.connector_name;
        let attempt = &payment_data.payment_attempt;
        let browser_info: Option<types::BrowserInformation> = payment_data
            .payment_attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        let redirect_response = payment_data.redirect_response.map(|redirect| {
            types::CompleteAuthorizeRedirectResponse {
                params: redirect.param,
                payload: redirect.json_payload,
            }
        });
        let amount = payment_data.payment_attempt.get_total_amount();
        let complete_authorize_url = Some(helpers::create_complete_authorize_url(
            router_base_url,
            attempt,
            connector_name,
        ));
        Ok(Self {
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            mandate_id: payment_data.mandate_id.clone(),
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            setup_mandate_details: payment_data.setup_mandate.clone(),
            confirm: payment_data.payment_attempt.confirm,
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            capture_method: payment_data.payment_attempt.capture_method,
            amount: amount.get_amount_as_i64(), // need to change once we move to connector module
            minor_amount: amount,
            currency: payment_data.currency,
            browser_info,
            email: payment_data.email,
            payment_method_data: payment_data.payment_method_data.map(From::from),
            connector_transaction_id: payment_data
                .payment_attempt
                .get_connector_payment_id()
                .map(ToString::to_string),
            redirect_response,
            connector_meta: payment_data.payment_attempt.connector_metadata,
            complete_authorize_url,
            metadata: payment_data.payment_intent.metadata,
            customer_acceptance: payment_data.customer_acceptance,
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::CompleteAuthorizeData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsPreProcessingData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsPreProcessingData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let payment_method_data = payment_data.payment_method_data;
        let router_base_url = &additional_data.router_base_url;
        let attempt = &payment_data.payment_attempt;
        let connector_name = &additional_data.connector_name;

        let order_details = payment_data
            .payment_intent
            .order_details
            .map(|order_details| {
                order_details
                    .iter()
                    .map(|data| {
                        data.to_owned()
                            .parse_value("OrderDetailsWithAmount")
                            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                                field_name: "OrderDetailsWithAmount",
                            })
                            .attach_printable("Unable to parse OrderDetailsWithAmount")
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        let merchant_connector_account_id_or_connector_name = payment_data
            .payment_attempt
            .merchant_connector_id
            .as_ref()
            .map(|mca_id| mca_id.get_string_repr())
            .unwrap_or(connector_name);
        let webhook_url = Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            merchant_connector_account_id_or_connector_name,
        ));
        let router_return_url = Some(helpers::create_redirect_url(
            router_base_url,
            attempt,
            connector_name,
            payment_data.creds_identifier.as_deref(),
        ));
        let complete_authorize_url = Some(helpers::create_complete_authorize_url(
            router_base_url,
            attempt,
            connector_name,
        ));
        let browser_info: Option<types::BrowserInformation> = payment_data
            .payment_attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;
        let amount = payment_data.payment_attempt.get_total_amount();

        Ok(Self {
            payment_method_data: payment_method_data.map(From::from),
            email: payment_data.email,
            currency: Some(payment_data.currency),
            amount: Some(amount.get_amount_as_i64()), // need to change this once we move to connector module
            minor_amount: Some(amount),
            payment_method_type: payment_data.payment_attempt.payment_method_type,
            setup_mandate_details: payment_data.setup_mandate,
            capture_method: payment_data.payment_attempt.capture_method,
            order_details,
            router_return_url,
            webhook_url,
            complete_authorize_url,
            browser_info,
            surcharge_details: payment_data.surcharge_details,
            connector_transaction_id: payment_data
                .payment_attempt
                .get_connector_payment_id()
                .map(ToString::to_string),
            redirect_response: None,
            mandate_id: payment_data.mandate_id,
            related_transaction_id: None,
            enrolled_for_3ds: true,
            metadata: payment_data.payment_intent.metadata.map(Secret::new),
        })
    }
}

impl ForeignFrom<payments::FraudCheck> for FrmMessage {
    fn foreign_from(fraud_check: payments::FraudCheck) -> Self {
        Self {
            frm_name: fraud_check.frm_name,
            frm_transaction_id: fraud_check.frm_transaction_id,
            frm_transaction_type: Some(fraud_check.frm_transaction_type.to_string()),
            frm_status: Some(fraud_check.frm_status.to_string()),
            frm_score: fraud_check.frm_score,
            frm_reason: fraud_check.frm_reason,
            frm_error: fraud_check.frm_error,
        }
    }
}

impl ForeignFrom<CustomerDetails> for router_request_types::CustomerDetails {
    fn foreign_from(customer: CustomerDetails) -> Self {
        Self {
            customer_id: Some(customer.id),
            name: customer.name,
            email: customer.email,
            phone: customer.phone,
            phone_country_code: customer.phone_country_code,
        }
    }
}

/// The response amount details in the confirm intent response will have the combined fields from
/// intent amount details and attempt amount details.
#[cfg(feature = "v2")]
impl
    ForeignFrom<(
        &hyperswitch_domain_models::payments::AmountDetails,
        &hyperswitch_domain_models::payments::payment_attempt::AttemptAmountDetails,
    )> for api_models::payments::PaymentAmountDetailsResponse
{
    fn foreign_from(
        (intent_amount_details, attempt_amount_details): (
            &hyperswitch_domain_models::payments::AmountDetails,
            &hyperswitch_domain_models::payments::payment_attempt::AttemptAmountDetails,
        ),
    ) -> Self {
        Self {
            order_amount: intent_amount_details.order_amount,
            currency: intent_amount_details.currency,
            shipping_cost: attempt_amount_details.get_shipping_cost(),
            order_tax_amount: attempt_amount_details.get_order_tax_amount(),
            external_tax_calculation: intent_amount_details.skip_external_tax_calculation,
            surcharge_calculation: intent_amount_details.skip_surcharge_calculation,
            surcharge_amount: attempt_amount_details.get_surcharge_amount(),
            tax_on_surcharge: attempt_amount_details.get_tax_on_surcharge(),
            net_amount: attempt_amount_details.get_net_amount(),
            amount_to_capture: attempt_amount_details.get_amount_to_capture(),
            amount_capturable: attempt_amount_details.get_amount_capturable(),
            amount_captured: intent_amount_details.amount_captured,
        }
    }
}

/// The response amount details in the confirm intent response will have the combined fields from
/// intent amount details and attempt amount details.
#[cfg(feature = "v2")]
impl
    ForeignFrom<(
        &hyperswitch_domain_models::payments::AmountDetails,
        Option<&hyperswitch_domain_models::payments::payment_attempt::AttemptAmountDetails>,
    )> for api_models::payments::PaymentAmountDetailsResponse
{
    fn foreign_from(
        (intent_amount_details, attempt_amount_details): (
            &hyperswitch_domain_models::payments::AmountDetails,
            Option<&hyperswitch_domain_models::payments::payment_attempt::AttemptAmountDetails>,
        ),
    ) -> Self {
        Self {
            order_amount: intent_amount_details.order_amount,
            currency: intent_amount_details.currency,
            shipping_cost: attempt_amount_details
                .and_then(|attempt_amount| attempt_amount.get_shipping_cost())
                .or(intent_amount_details.shipping_cost),
            order_tax_amount: attempt_amount_details
                .and_then(|attempt_amount| attempt_amount.get_order_tax_amount())
                .or(intent_amount_details
                    .tax_details
                    .as_ref()
                    .and_then(|tax_details| tax_details.get_default_tax_amount())),
            external_tax_calculation: intent_amount_details.skip_external_tax_calculation,
            surcharge_calculation: intent_amount_details.skip_surcharge_calculation,
            surcharge_amount: attempt_amount_details
                .and_then(|attempt| attempt.get_surcharge_amount())
                .or(intent_amount_details.surcharge_amount),
            tax_on_surcharge: attempt_amount_details
                .and_then(|attempt| attempt.get_tax_on_surcharge())
                .or(intent_amount_details.tax_on_surcharge),
            net_amount: attempt_amount_details
                .map(|attempt| attempt.get_net_amount())
                .unwrap_or(intent_amount_details.calculate_net_amount()),
            amount_to_capture: attempt_amount_details
                .and_then(|attempt| attempt.get_amount_to_capture()),
            amount_capturable: attempt_amount_details
                .map(|attempt| attempt.get_amount_capturable())
                .unwrap_or(MinorUnit::zero()),
            amount_captured: intent_amount_details.amount_captured,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<&hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt>
    for api_models::payments::PaymentAttemptResponse
{
    fn foreign_from(
        attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> Self {
        Self {
            id: attempt.get_id().to_owned(),
            status: attempt.status,
            amount: api_models::payments::PaymentAttemptAmountDetails::foreign_from(
                &attempt.amount_details,
            ),
            connector: attempt.connector.clone(),
            error: attempt
                .error
                .clone()
                .map(api_models::payments::ErrorDetails::foreign_from),
            authentication_type: attempt.authentication_type,
            created_at: attempt.created_at,
            modified_at: attempt.modified_at,
            cancellation_reason: attempt.cancellation_reason.clone(),
            payment_token: attempt.payment_token.clone(),
            connector_metadata: attempt.connector_metadata.clone(),
            payment_experience: attempt.payment_experience,
            payment_method_type: attempt.payment_method_type,
            connector_reference_id: attempt.connector_response_reference_id.clone(),
            payment_method_subtype: attempt.get_payment_method_type(),
            connector_payment_id: attempt.get_connector_payment_id().map(ToString::to_string),
            payment_method_id: attempt.payment_method_id.clone(),
            client_source: attempt.client_source.clone(),
            client_version: attempt.client_version.clone(),
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<&hyperswitch_domain_models::payments::payment_attempt::AttemptAmountDetails>
    for api_models::payments::PaymentAttemptAmountDetails
{
    fn foreign_from(
        amount: &hyperswitch_domain_models::payments::payment_attempt::AttemptAmountDetails,
    ) -> Self {
        Self {
            net_amount: amount.get_net_amount(),
            amount_to_capture: amount.get_amount_to_capture(),
            surcharge_amount: amount.get_surcharge_amount(),
            tax_on_surcharge: amount.get_tax_on_surcharge(),
            amount_capturable: amount.get_amount_capturable(),
            shipping_cost: amount.get_shipping_cost(),
            order_tax_amount: amount.get_order_tax_amount(),
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<hyperswitch_domain_models::payments::payment_attempt::ErrorDetails>
    for api_models::payments::ErrorDetails
{
    fn foreign_from(
        amount_details: hyperswitch_domain_models::payments::payment_attempt::ErrorDetails,
    ) -> Self {
        let hyperswitch_domain_models::payments::payment_attempt::ErrorDetails {
            code,
            message,
            reason,
            unified_code,
            unified_message,
        } = amount_details;

        Self {
            code,
            message: reason.unwrap_or(message),
            unified_code,
            unified_message,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<hyperswitch_domain_models::payments::AmountDetails>
    for api_models::payments::AmountDetailsResponse
{
    fn foreign_from(amount_details: hyperswitch_domain_models::payments::AmountDetails) -> Self {
        Self {
            order_amount: amount_details.order_amount,
            currency: amount_details.currency,
            shipping_cost: amount_details.shipping_cost,
            order_tax_amount: amount_details.tax_details.and_then(|tax_details| {
                tax_details.default.map(|default| default.order_tax_amount)
            }),
            external_tax_calculation: amount_details.skip_external_tax_calculation,
            surcharge_calculation: amount_details.skip_surcharge_calculation,
            surcharge_amount: amount_details.surcharge_amount,
            tax_on_surcharge: amount_details.tax_on_surcharge,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<api_models::admin::PaymentLinkConfigRequest>
    for diesel_models::PaymentLinkConfigRequestForPayments
{
    fn foreign_from(config: api_models::admin::PaymentLinkConfigRequest) -> Self {
        Self {
            theme: config.theme,
            logo: config.logo,
            seller_name: config.seller_name,
            sdk_layout: config.sdk_layout,
            display_sdk_only: config.display_sdk_only,
            enabled_saved_payment_method: config.enabled_saved_payment_method,
            hide_card_nickname_field: config.hide_card_nickname_field,
            show_card_form_by_default: config.show_card_form_by_default,
            details_layout: config.details_layout,
            transaction_details: config.transaction_details.map(|transaction_details| {
                transaction_details
                    .iter()
                    .map(|details| {
                        diesel_models::PaymentLinkTransactionDetails::foreign_from(details.clone())
                    })
                    .collect()
            }),
            background_image: config.background_image.map(|background_image| {
                diesel_models::business_profile::PaymentLinkBackgroundImageConfig::foreign_from(
                    background_image.clone(),
                )
            }),
            payment_button_text: config.payment_button_text,
            custom_message_for_card_terms: config.custom_message_for_card_terms,
            payment_button_colour: config.payment_button_colour,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<api_models::admin::PaymentLinkTransactionDetails>
    for diesel_models::PaymentLinkTransactionDetails
{
    fn foreign_from(from: api_models::admin::PaymentLinkTransactionDetails) -> Self {
        Self {
            key: from.key,
            value: from.value,
            ui_configuration: from
                .ui_configuration
                .map(diesel_models::TransactionDetailsUiConfiguration::foreign_from),
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<api_models::admin::TransactionDetailsUiConfiguration>
    for diesel_models::TransactionDetailsUiConfiguration
{
    fn foreign_from(from: api_models::admin::TransactionDetailsUiConfiguration) -> Self {
        Self {
            position: from.position,
            is_key_bold: from.is_key_bold,
            is_value_bold: from.is_value_bold,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<diesel_models::PaymentLinkConfigRequestForPayments>
    for api_models::admin::PaymentLinkConfigRequest
{
    fn foreign_from(config: diesel_models::PaymentLinkConfigRequestForPayments) -> Self {
        Self {
            theme: config.theme,
            logo: config.logo,
            seller_name: config.seller_name,
            sdk_layout: config.sdk_layout,
            display_sdk_only: config.display_sdk_only,
            enabled_saved_payment_method: config.enabled_saved_payment_method,
            hide_card_nickname_field: config.hide_card_nickname_field,
            show_card_form_by_default: config.show_card_form_by_default,
            details_layout: config.details_layout,
            transaction_details: config.transaction_details.map(|transaction_details| {
                transaction_details
                    .iter()
                    .map(|details| {
                        api_models::admin::PaymentLinkTransactionDetails::foreign_from(
                            details.clone(),
                        )
                    })
                    .collect()
            }),
            background_image: config.background_image.map(|background_image| {
                api_models::admin::PaymentLinkBackgroundImageConfig::foreign_from(
                    background_image.clone(),
                )
            }),
            payment_button_text: config.payment_button_text,
            custom_message_for_card_terms: config.custom_message_for_card_terms,
            payment_button_colour: config.payment_button_colour,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<diesel_models::PaymentLinkTransactionDetails>
    for api_models::admin::PaymentLinkTransactionDetails
{
    fn foreign_from(from: diesel_models::PaymentLinkTransactionDetails) -> Self {
        Self {
            key: from.key,
            value: from.value,
            ui_configuration: from
                .ui_configuration
                .map(api_models::admin::TransactionDetailsUiConfiguration::foreign_from),
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<diesel_models::TransactionDetailsUiConfiguration>
    for api_models::admin::TransactionDetailsUiConfiguration
{
    fn foreign_from(from: diesel_models::TransactionDetailsUiConfiguration) -> Self {
        Self {
            position: from.position,
            is_key_bold: from.is_key_bold,
            is_value_bold: from.is_value_bold,
        }
    }
}

impl ForeignFrom<DieselConnectorMandateReferenceId> for ConnectorMandateReferenceId {
    fn foreign_from(value: DieselConnectorMandateReferenceId) -> Self {
        Self::new(
            value.connector_mandate_id,
            value.payment_method_id,
            None,
            value.mandate_metadata,
            value.connector_mandate_request_reference_id,
        )
    }
}

impl ForeignFrom<ConnectorMandateReferenceId> for DieselConnectorMandateReferenceId {
    fn foreign_from(value: ConnectorMandateReferenceId) -> Self {
        Self {
            connector_mandate_id: value.get_connector_mandate_id(),
            payment_method_id: value.get_payment_method_id(),
            mandate_metadata: value.get_mandate_metadata(),
            connector_mandate_request_reference_id: value
                .get_connector_mandate_request_reference_id(),
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<diesel_models::ConnectorTokenDetails>
    for Option<api_models::payments::ConnectorTokenDetails>
{
    fn foreign_from(value: diesel_models::ConnectorTokenDetails) -> Self {
        value
            .connector_mandate_id
            .map(|mandate_id| api_models::payments::ConnectorTokenDetails { token: mandate_id })
    }
}

impl ForeignFrom<(Self, Option<&api_models::payments::AdditionalPaymentData>)>
    for Option<enums::PaymentMethodType>
{
    fn foreign_from(req: (Self, Option<&api_models::payments::AdditionalPaymentData>)) -> Self {
        let (payment_method_type, additional_pm_data) = req;
        additional_pm_data
            .and_then(|pm_data| {
                if let api_models::payments::AdditionalPaymentData::Card(card_info) = pm_data {
                    card_info.card_type.as_ref().and_then(|card_type_str| {
                        api_models::enums::PaymentMethodType::from_str(&card_type_str.to_lowercase()).map_err(|err| {
                            crate::logger::error!(
                                "Err - {:?}\nInvalid card_type value found in BIN DB - {:?}",
                                err,
                                card_type_str,
                            );
                        }).ok()
                    })
                } else {
                    None
                }
            })
            .map_or(payment_method_type, |card_type_in_bin_store| {
                if let Some(card_type_in_req) = payment_method_type {
                    if card_type_in_req != card_type_in_bin_store {
                        crate::logger::info!(
                            "Mismatch in card_type\nAPI request - {}; BIN lookup - {}\nOverriding with {}",
                            card_type_in_req, card_type_in_bin_store, card_type_in_bin_store,
                        );
                    }
                }
                Some(card_type_in_bin_store)
            })
    }
}
