use std::{fmt::Debug, marker::PhantomData, str::FromStr};

#[cfg(feature = "v2")]
use api_models::enums as api_enums;
#[cfg(feature = "v2")]
use api_models::payments::RevenueRecoveryGetIntentResponse;
use api_models::payments::{
    Address, ConnectorMandateReferenceId, CustomerDetails, CustomerDetailsResponse, FrmMessage,
    MandateIds, NetworkDetails, RequestSurchargeDetails,
};
use common_enums::{Currency, RequestIncrementalAuthorization};
#[cfg(feature = "v1")]
use common_utils::{
    consts::X_HS_LATENCY,
    fp_utils, pii,
    types::{
        self as common_utils_type, AmountConvertor, MinorUnit, StringMajorUnit,
        StringMajorUnitForConnector,
    },
};
#[cfg(feature = "v2")]
use common_utils::{
    ext_traits::Encode,
    fp_utils, pii,
    types::{
        self as common_utils_type, AmountConvertor, MinorUnit, StringMajorUnit,
        StringMajorUnitForConnector,
    },
};
use diesel_models::{
    ephemeral_key,
    payment_attempt::{
        ConnectorMandateReferenceId as DieselConnectorMandateReferenceId,
        NetworkDetails as DieselNetworkDetails,
    },
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{payments::payment_intent::CustomerData, router_request_types};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::{
    router_data_v2::{flow_common_types, RouterDataV2},
    ApiModelToDieselModelConvertor,
};
#[cfg(feature = "v2")]
use hyperswitch_interfaces::api::ConnectorSpecifications;
#[cfg(feature = "v2")]
use hyperswitch_interfaces::connector_integration_interface::RouterDataConversion;
use masking::{ExposeInterface, Maskable, Secret};
#[cfg(feature = "v2")]
use masking::{ExposeOptionInterface, PeekInterface};
use router_env::{instrument, tracing};

use super::{flows::Feature, types::AuthenticationData, OperationSessionGetters, PaymentData};
use crate::{
    configs::settings::ConnectorRequestReferenceIdConfig,
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments::{self, helpers},
        utils as core_utils,
    },
    headers::{X_CONNECTOR_HTTP_STATUS_CODE, X_PAYMENT_CONFIRM_SOURCE},
    routes::{metrics, SessionState},
    services::{self, RedirectForm},
    types::{
        self,
        api::{self, ConnectorTransactionId},
        domain, payment_methods as pm_types,
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
    platform: &domain::Platform,
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
    platform: &domain::Platform,
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
        merchant_id: platform.get_processor().get_account().get_id().clone(),
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
        payment_method_type: payment_data.payment_attempt.payment_method_type,
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
            platform.get_processor().get_account().get_id(),
            &payment_data.payment_intent,
            &payment_data.payment_attempt,
            connector_id,
        )?,
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
        raw_connector_response: None,
        is_payment_id_from_merchant: payment_data.payment_intent.is_payment_id_from_merchant,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
    };
    Ok(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_external_vault_proxy_router_data_v2<'a>(
    state: &'a SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
    payment_data: &hyperswitch_domain_models::payments::PaymentConfirmData<api::ExternalVaultProxy>,
    request: types::ExternalVaultProxyPaymentsData,
    connector_request_reference_id: String,
    connector_customer_id: Option<String>,
    customer_id: Option<common_utils::id_type::CustomerId>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<
    RouterDataV2<
        api::ExternalVaultProxy,
        hyperswitch_domain_models::router_data_v2::ExternalVaultProxyFlowData,
        types::ExternalVaultProxyPaymentsData,
        types::PaymentsResponseData,
    >,
> {
    use hyperswitch_domain_models::router_data_v2::{ExternalVaultProxyFlowData, RouterDataV2};

    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let external_vault_proxy_flow_data = ExternalVaultProxyFlowData {
        merchant_id: merchant_account.get_id().clone(),
        customer_id,
        connector_customer: connector_customer_id,
        payment_id: payment_data
            .payment_attempt
            .payment_id
            .get_string_repr()
            .to_owned(),
        attempt_id: payment_data
            .payment_attempt
            .get_id()
            .get_string_repr()
            .to_owned(),
        status: payment_data.payment_attempt.status,
        payment_method: payment_data.payment_attempt.payment_method_type,
        description: payment_data
            .payment_intent
            .description
            .as_ref()
            .map(|description| description.get_string_repr())
            .map(ToOwned::to_owned),
        address: payment_data.payment_address.clone(),
        auth_type: payment_data.payment_attempt.authentication_type,
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: payment_data.payment_attempt.preprocessing_step_id.clone(),
        payment_method_balance: None,
        connector_api_version: None,
        connector_request_reference_id,
        test_mode: Some(true),
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        connector_response: None,
        payment_method_status: None,
    };

    let router_data_v2 = RouterDataV2 {
        flow: PhantomData,
        tenant_id: state.tenant.tenant_id.clone(),
        resource_common_data: external_vault_proxy_flow_data,
        connector_auth_type: auth_type,
        request,
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
    };

    Ok(router_data_v2)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data_for_authorize<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentConfirmData<api::Authorize>,
    connector_id: &str,
    platform: &domain::Platform,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
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

    let connector_customer_id =
        payment_data.get_connector_customer_id(customer.as_ref(), merchant_connector_account);

    let payment_method = payment_data.payment_attempt.payment_method_type;

    let router_base_url = &state.base_url;
    let attempt = &payment_data.payment_attempt;

    let complete_authorize_url = Some(helpers::create_complete_authorize_url(
        router_base_url,
        attempt,
        connector_id,
        None,
    ));

    let webhook_url = match merchant_connector_account {
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(
            merchant_connector_account,
        ) => Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            merchant_connector_account.get_id().get_string_repr(),
        )),
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            payment_data.webhook_url
        }
    };

    let router_return_url = payment_data
        .payment_intent
        .create_finish_redirection_url(
            router_base_url,
            platform
                .get_processor()
                .get_account()
                .publishable_key
                .as_ref(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to construct finish redirection url")?
        .to_string();

    let connector_request_reference_id = payment_data
        .payment_attempt
        .connector_request_reference_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("connector_request_reference_id not found in payment_attempt")?;

    let email = customer
        .as_ref()
        .and_then(|customer| customer.email.clone())
        .map(pii::Email::from);

    let browser_info = payment_data
        .payment_attempt
        .browser_info
        .clone()
        .map(types::BrowserInformation::from);
    let additional_payment_method_data: Option<api_models::payments::AdditionalPaymentData> =
            payment_data.payment_attempt
                .payment_method_data
                .as_ref().map(|data| data.clone().parse_value("AdditionalPaymentData"))
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse AdditionalPaymentData from payment_data.payment_attempt.payment_method_data")?;

    let connector_metadata = payment_data.payment_intent.connector_metadata.clone();

    let order_category = connector_metadata.as_ref().and_then(|cm| {
        cm.noon
            .as_ref()
            .and_then(|noon| noon.order_category.clone())
    });

    // TODO: few fields are repeated in both routerdata and request
    let request = types::PaymentsAuthorizeData {
        payment_method_data: payment_data
            .payment_method_data
            .get_required_value("payment_method_data")?,
        setup_future_usage: Some(payment_data.payment_intent.setup_future_usage),
        mandate_id: payment_data.mandate_data.clone(),
        off_session: None,
        setup_mandate_details: None,
        confirm: true,
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
        order_category,
        session_token: None,
        enrolled_for_3ds: true,
        related_transaction_id: None,
        payment_method_type: Some(payment_data.payment_attempt.payment_method_subtype),
        router_return_url: Some(router_return_url),
        webhook_url,
        complete_authorize_url,
        customer_id: customer_id.clone(),
        surcharge_details: None,
        request_extended_authorization: None,
        request_incremental_authorization: matches!(
            payment_data
                .payment_intent
                .request_incremental_authorization,
            RequestIncrementalAuthorization::True
        ),
        metadata: payment_data.payment_intent.metadata.expose_option(),
        authentication_data: None,
        customer_acceptance: None,
        split_payments: None,
        merchant_order_reference_id: payment_data
            .payment_intent
            .merchant_reference_id
            .map(|reference_id| reference_id.get_string_repr().to_owned()),
        integrity_object: None,
        shipping_cost: payment_data.payment_intent.amount_details.shipping_cost,
        additional_payment_method_data,
        merchant_account_id: None,
        merchant_config_currency: None,
        connector_testing_data: None,
        order_id: None,
        locale: None,
        mit_category: None,
        tokenization: None,
        payment_channel: None,
        enable_partial_authorization: payment_data.payment_intent.enable_partial_authorization,
        enable_overcapture: None,
        is_stored_credential: None,
        billing_descriptor: None,
        partner_merchant_identifier_details: None,
    };
    let connector_mandate_request_reference_id = payment_data
        .payment_attempt
        .connector_token_details
        .as_ref()
        .and_then(|detail| detail.get_connector_token_request_reference_id());

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
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
        payment_method_type: Some(payment_data.payment_attempt.payment_method_subtype),
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
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: None,
        request,
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
        amount_captured: payment_data
            .payment_intent
            .amount_captured
            .map(|amt| amt.get_amount_as_i64()),
        minor_amount_captured: payment_data.payment_intent.amount_captured,
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
        raw_connector_response: None,
        is_payment_id_from_merchant: payment_data.payment_intent.is_payment_id_from_merchant,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
    };

    Ok(router_data)
}
#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_external_vault_proxy_payment_router_data<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentConfirmData<api::ExternalVaultProxy>,
    connector_id: &str,
    platform: &domain::Platform,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
    _merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<types::ExternalVaultProxyPaymentsRouterData> {
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

    let connector_customer_id =
        payment_data.get_connector_customer_id(customer.as_ref(), merchant_connector_account);

    let payment_method = payment_data.payment_attempt.payment_method_type;

    let router_base_url = &state.base_url;
    let attempt = &payment_data.payment_attempt;

    let complete_authorize_url = Some(helpers::create_complete_authorize_url(
        router_base_url,
        attempt,
        connector_id,
        None,
    ));

    let webhook_url = match merchant_connector_account {
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(
            merchant_connector_account,
        ) => Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            merchant_connector_account.get_id().get_string_repr(),
        )),
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            payment_data.webhook_url.clone()
        }
    };

    let router_return_url = payment_data
        .payment_intent
        .create_finish_redirection_url(
            router_base_url,
            platform
                .get_processor()
                .get_account()
                .publishable_key
                .as_ref(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to construct finish redirection url")?
        .to_string();

    let connector_request_reference_id = payment_data
        .payment_attempt
        .connector_request_reference_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("connector_request_reference_id not found in payment_attempt")?;

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
    let request = types::ExternalVaultProxyPaymentsData {
        payment_method_data: payment_data
            .external_vault_pmd
            .clone()
            .get_required_value("external vault proxy payment_method_data")?,
        setup_future_usage: Some(payment_data.payment_intent.setup_future_usage),
        mandate_id: payment_data.mandate_data.clone(),
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
        customer_id: customer_id.clone(),
        surcharge_details: None,
        request_extended_authorization: None,
        request_incremental_authorization: matches!(
            payment_data
                .payment_intent
                .request_incremental_authorization,
            RequestIncrementalAuthorization::True
        ),
        metadata: payment_data.payment_intent.metadata.clone().expose_option(),
        authentication_data: None,
        customer_acceptance: None,
        split_payments: None,
        merchant_order_reference_id: payment_data.payment_intent.merchant_reference_id.clone(),
        integrity_object: None,
        shipping_cost: payment_data.payment_intent.amount_details.shipping_cost,
        additional_payment_method_data: None,
        merchant_account_id: None,
        merchant_config_currency: None,
        connector_testing_data: None,
        order_id: None,
    };
    let connector_mandate_request_reference_id = payment_data
        .payment_attempt
        .connector_token_details
        .as_ref()
        .and_then(|detail| detail.get_connector_token_request_reference_id());

    // Construct RouterDataV2 for external vault proxy
    let router_data_v2 = construct_external_vault_proxy_router_data_v2(
        state,
        platform.get_processor().get_account(),
        merchant_connector_account,
        &payment_data,
        request,
        connector_request_reference_id.clone(),
        connector_customer_id.clone(),
        customer_id.clone(),
        header_payload.clone(),
    )
    .await?;

    // Convert RouterDataV2 to old RouterData (v1) using the existing RouterDataConversion trait
    let router_data =
        flow_common_types::ExternalVaultProxyFlowData::to_old_router_data(router_data_v2)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Cannot construct router data for making the unified connector service call",
            )?;

    Ok(router_data)
}
#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data_for_capture<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentCaptureData<api::Capture>,
    connector_id: &str,
    platform: &domain::Platform,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
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

    let connector_mandate_request_reference_id = payment_data
        .payment_attempt
        .connector_token_details
        .as_ref()
        .and_then(|detail| detail.get_connector_token_request_reference_id());

    let connector = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector_id,
        api::GetToken::Connector,
        payment_data.payment_attempt.merchant_connector_id.clone(),
    )?;

    let connector_request_reference_id = payment_data
        .payment_attempt
        .connector_request_reference_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("connector_request_reference_id not found in payment_attempt")?;

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
            .connector_transaction_id(&payment_data.payment_attempt)?
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
        split_payments: None,
        webhook_url: None,
    };

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
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
        payment_method_type: Some(payment_data.payment_attempt.payment_method_subtype),
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
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
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
    platform: &domain::Platform,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
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

    let attempt = &payment_data.payment_attempt;

    let connector_request_reference_id = payment_data
        .payment_attempt
        .connector_request_reference_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("connector_request_reference_id not found in payment_attempt")?;

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
        connector_reference_id: attempt.connector_response_reference_id.clone(),
        setup_future_usage: Some(payment_intent.setup_future_usage),
    };

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
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
        payment_method_type: Some(attempt.payment_method_subtype),
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
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
    };

    Ok(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_cancel_router_data_v2<'a>(
    state: &'a SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
    payment_data: &hyperswitch_domain_models::payments::PaymentCancelData<api::Void>,
    request: types::PaymentsCancelData,
    connector_request_reference_id: String,
    customer_id: Option<common_utils::id_type::CustomerId>,
    connector_id: &str,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<
    RouterDataV2<
        api::Void,
        flow_common_types::PaymentFlowData,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    >,
> {
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let payment_cancel_data = flow_common_types::PaymentFlowData {
        merchant_id: merchant_account.get_id().clone(),
        customer_id,
        connector_customer: None,
        connector: connector_id.to_owned(),
        payment_id: payment_data
            .payment_attempt
            .payment_id
            .get_string_repr()
            .to_owned(),
        attempt_id: payment_data
            .payment_attempt
            .get_id()
            .get_string_repr()
            .to_owned(),
        status: payment_data.payment_attempt.status,
        payment_method: payment_data.payment_attempt.payment_method_type,
        description: payment_data
            .payment_intent
            .description
            .as_ref()
            .map(|description| description.get_string_repr())
            .map(ToOwned::to_owned),
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: payment_data.payment_attempt.authentication_type,
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: payment_data.payment_attempt.preprocessing_step_id.clone(),
        payment_method_balance: None,
        connector_api_version: None,
        connector_request_reference_id,
        test_mode: Some(true),
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        connector_response: None,
        payment_method_status: None,
    };

    let router_data_v2 = RouterDataV2 {
        flow: PhantomData,
        tenant_id: state.tenant.tenant_id.clone(),
        resource_common_data: payment_cancel_data,
        connector_auth_type: auth_type,
        request,
        response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
    };

    Ok(router_data_v2)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_router_data_for_cancel<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentCancelData<
        hyperswitch_domain_models::router_flow_types::Void,
    >,
    connector_id: &str,
    platform: &domain::Platform,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
    _merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<types::PaymentsCancelRouterData> {
    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    // TODO: Take Globalid and convert to connector reference id
    let customer_id = customer
        .to_owned()
        .map(|customer| common_utils::id_type::CustomerId::try_from(customer.id.clone()))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Invalid global customer generated, not able to convert to reference id",
        )?;
    let payment_intent = payment_data.get_payment_intent();
    let attempt = payment_data.get_payment_attempt();
    let connector_request_reference_id = payment_data
        .payment_attempt
        .connector_request_reference_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("connector_request_reference_id not found in payment_attempt")?;

    let request = types::PaymentsCancelData {
        amount: Some(attempt.amount_details.get_net_amount().get_amount_as_i64()),
        currency: Some(payment_intent.amount_details.currency),
        connector_transaction_id: attempt
            .get_connector_payment_id()
            .unwrap_or_default()
            .to_string(),
        cancellation_reason: attempt.cancellation_reason.clone(),
        connector_meta: attempt.connector_metadata.clone().expose_option(),
        browser_info: None,
        metadata: None,
        minor_amount: Some(attempt.amount_details.get_net_amount()),
        webhook_url: None,
        capture_method: Some(payment_intent.capture_method),
        split_payments: None,
    };

    // Construct RouterDataV2 for cancel operation
    let router_data_v2 = construct_cancel_router_data_v2(
        state,
        platform.get_processor().get_account(),
        merchant_connector_account,
        &payment_data,
        request,
        connector_request_reference_id.clone(),
        customer_id.clone(),
        connector_id,
        header_payload.clone(),
    )
    .await?;

    // Convert RouterDataV2 to old RouterData (v1) using the existing RouterDataConversion trait
    let router_data = flow_common_types::PaymentFlowData::to_old_router_data(router_data_v2)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the unified connector service call",
        )?;

    Ok(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data_for_sdk_session<'a>(
    state: &'a SessionState,
    payment_data: hyperswitch_domain_models::payments::PaymentIntentData<api::Session>,
    connector_id: &str,
    platform: &domain::Platform,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
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
    let billing_address = payment_data
        .payment_intent
        .billing_address
        .as_ref()
        .map(|billing_address| billing_address.clone().into_inner());
    // fetch email from customer or billing address (fallback)
    let email = customer
        .as_ref()
        .and_then(|customer| customer.email.clone())
        .map(pii::Email::from)
        .or(billing_address
            .as_ref()
            .and_then(|address| address.email.clone()));
    // fetch customer name from customer or billing address (fallback)
    let customer_name = customer
        .as_ref()
        .and_then(|customer| customer.name.clone())
        .map(|name| name.into_inner())
        .or(billing_address.and_then(|address| {
            address
                .address
                .as_ref()
                .and_then(|address_details| address_details.get_optional_full_name())
        }));
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
        .clone()
        .and_then(|feature_metadata| feature_metadata.apple_pay_recurring_details)
        .map(|apple_pay_recurring_details| {
            ForeignInto::foreign_into((apple_pay_recurring_details, apple_pay_amount))
        });

    let order_tax_amount = payment_data
        .payment_intent
        .amount_details
        .tax_details
        .clone()
        .and_then(|tax| tax.get_default_tax_amount());

    let payment_attempt = payment_data.get_payment_attempt();
    let payment_method = Some(payment_attempt.payment_method_type);
    let payment_method_type = Some(payment_attempt.payment_method_subtype);

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
        customer_name,
        metadata: payment_data.payment_intent.metadata,
        order_tax_amount,
        shipping_cost: payment_data.payment_intent.amount_details.shipping_cost,
        payment_method,
        payment_method_type,
    };

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
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
        payment_method_type,
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
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
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
    platform: &domain::Platform,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
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
            .get_connector_customer_id(merchant_connector_account)
            .map(String::from)
    });

    let payment_method = payment_data.payment_attempt.payment_method_type;

    let router_base_url = &state.base_url;
    let attempt = &payment_data.payment_attempt;

    let complete_authorize_url = Some(helpers::create_complete_authorize_url(
        router_base_url,
        attempt,
        connector_id,
        None,
    ));

    let webhook_url = match merchant_connector_account {
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(
            merchant_connector_account,
        ) => Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            merchant_connector_account.get_id().get_string_repr(),
        )),
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            payment_data.webhook_url
        }
    };

    let router_return_url = payment_data
        .payment_intent
        .create_finish_redirection_url(
            router_base_url,
            platform
                .get_processor()
                .get_account()
                .publishable_key
                .as_ref(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to construct finish redirection url")?
        .to_string();

    let connector_request_reference_id = payment_data
        .payment_attempt
        .connector_request_reference_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("connector_request_reference_id not found in payment_attempt")?;

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
        customer_acceptance: None,
        mandate_id: None,
        setup_future_usage: Some(payment_data.payment_intent.setup_future_usage),
        off_session: None,
        tokenization: None,
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
            RequestIncrementalAuthorization::True
        ),
        metadata: payment_data.payment_intent.metadata,
        minor_amount: Some(payment_data.payment_attempt.amount_details.get_net_amount()),
        shipping_cost: payment_data.payment_intent.amount_details.shipping_cost,
        capture_method: Some(payment_data.payment_intent.capture_method),
        complete_authorize_url,
        connector_testing_data: None,
        customer_id: None,
        enable_partial_authorization: None,
        payment_channel: None,
        enrolled_for_3ds: true,
        related_transaction_id: None,
        is_stored_credential: None,
        billing_descriptor: None,
        split_payments: None,
        partner_merchant_identifier_details: None,
    };
    let connector_mandate_request_reference_id = payment_data
        .payment_attempt
        .connector_token_details
        .as_ref()
        .and_then(|detail| detail.get_connector_token_request_reference_id());

    // TODO: evaluate the fields in router data, if they are required or not
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
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
        payment_method_type: Some(payment_data.payment_attempt.payment_method_subtype),
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
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
    };

    Ok(router_data)
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data<'a, F, T>(
    state: &'a SessionState,
    payment_data: PaymentData<F>,
    connector_id: &str,
    platform: &domain::Platform,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    payment_method: Option<common_enums::PaymentMethod>,
    payment_method_type: Option<common_enums::PaymentMethodType>,
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

    let payment_method = payment_data
        .payment_attempt
        .payment_method
        .or(payment_method)
        .get_required_value("payment_method")?;

    let payment_method_type = payment_data
        .payment_attempt
        .payment_method_type
        .or(payment_method_type);

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
    let order_details = payment_data
        .payment_intent
        .order_details
        .as_ref()
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
    let l2_l3_data =
        (state.conf.l2_l3_data_config.enabled && payment_data.is_l2_l3_enabled).then(|| {
            let shipping_address = unified_address.get_shipping();
            let billing_address = unified_address.get_payment_billing();
            let merchant_tax_registration_id = platform
                .get_processor()
                .get_account()
                .get_merchant_tax_registration_id();

            Box::new(types::L2L3Data {
                order_info: Some(types::OrderInfo {
                    order_date: payment_data.payment_intent.order_date,
                    order_details: order_details.clone(),
                    merchant_order_reference_id: payment_data
                        .payment_intent
                        .merchant_order_reference_id
                        .clone(),
                    discount_amount: payment_data.payment_intent.discount_amount,
                    shipping_cost: payment_data.payment_intent.shipping_cost,
                    duty_amount: payment_data.payment_intent.duty_amount,
                }),
                tax_info: Some(types::TaxInfo {
                    tax_status: payment_data.payment_intent.tax_status,
                    customer_tax_registration_id: customer.as_ref().and_then(|customer| {
                        customer
                            .tax_registration_id
                            .as_ref()
                            .map(|tax_registration_id| tax_registration_id.clone().into_inner())
                    }),
                    merchant_tax_registration_id,
                    shipping_amount_tax: payment_data.payment_intent.shipping_amount_tax,
                    order_tax_amount: payment_data
                        .payment_attempt
                        .net_amount
                        .get_order_tax_amount(),
                }),
                customer_info: Some(types::CustomerInfo {
                    customer_id: payment_data.payment_intent.customer_id.clone(),
                    customer_email: payment_data.email,
                    customer_name: customer.as_ref().and_then(|customer_data| {
                        customer_data
                            .name
                            .as_ref()
                            .map(|name| name.clone().into_inner())
                    }),
                    customer_phone_number: customer.as_ref().and_then(|customer_data| {
                        customer_data
                            .phone
                            .as_ref()
                            .map(|phone| phone.clone().into_inner())
                    }),
                    customer_phone_country_code: customer
                        .as_ref()
                        .and_then(|customer_data| customer_data.phone_country_code.clone()),
                }),
                billing_details: billing_address
                    .as_ref()
                    .and_then(|addr| addr.address.as_ref())
                    .and_then(|details| details.city.clone())
                    .map(|city| types::BillingDetails {
                        address_city: Some(city),
                    }),
                shipping_details: shipping_address
                    .and_then(|address| address.address.as_ref())
                    .cloned(),
            })
        });
    crate::logger::debug!("unified address details {:?}", unified_address);

    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
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
        payment_method_type,
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
        payment_method_status: payment_data
            .payment_method_info
            .clone()
            .map(|info| info.status),
        payment_method_token: payment_data
            .pm_token
            .map(|token| types::PaymentMethodToken::Token(Secret::new(token))),
        connector_customer: core_utils::get_connector_customer_id(
            &state.conf,
            connector_id,
            payment_data.connector_customer_id.clone(),
            &payment_data.payment_intent.customer_id,
            &payment_data.payment_method_info,
            &payment_data.payment_attempt,
        )?,
        recurring_mandate_payment_data: payment_data.recurring_mandate_payment_data,
        connector_request_reference_id: core_utils::get_connector_request_reference_id(
            &state.conf,
            platform.get_processor().get_account().get_id(),
            &payment_data.payment_intent,
            &payment_data.payment_attempt,
            connector_id,
        )?,
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
        raw_connector_response: None,
        is_payment_id_from_merchant: payment_data.payment_intent.is_payment_id_from_merchant,
        l2_l3_data,
        minor_amount_capturable: None,
        authorized_amount: None,
    };

    Ok(router_data)
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_router_data_for_update_metadata<'a>(
    state: &'a SessionState,
    payment_data: PaymentData<api::UpdateMetadata>,
    connector_id: &str,
    platform: &domain::Platform,
    customer: &'a Option<domain::Customer>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    merchant_recipient_data: Option<types::MerchantRecipientData>,
    header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
) -> RouterResult<
    types::RouterData<
        api::UpdateMetadata,
        types::PaymentsUpdateMetadataData,
        types::PaymentsResponseData,
    >,
> {
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

    // [#44]: why should response be filled during request
    let response = Err(hyperswitch_domain_models::router_data::ErrorResponse {
        code: "IR_20".to_string(),
        message: "Update metadata is not implemented for this connector".to_string(),
        reason: None,
        status_code: http::StatusCode::BAD_REQUEST.as_u16(),
        attempt_status: None,
        connector_transaction_id: None,
        network_decline_code: None,
        network_advice_code: None,
        network_error_message: None,
        connector_metadata: None,
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
        merchant_id: platform.get_processor().get_account().get_id().clone(),
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
        payment_method_type: payment_data.payment_attempt.payment_method_type,
        connector_auth_type: auth_type,
        description: payment_data.payment_intent.description.clone(),
        address: unified_address,
        auth_type: payment_data
            .payment_attempt
            .authentication_type
            .unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        request: types::PaymentsUpdateMetadataData::try_from(additional_data)?,
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
            platform.get_processor().get_account().get_id(),
            &payment_data.payment_intent,
            &payment_data.payment_attempt,
            connector_id,
        )?,
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
        raw_connector_response: None,
        is_payment_id_from_merchant: payment_data.payment_intent.is_payment_id_from_merchant,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
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
        platform: &domain::Platform,
    ) -> RouterResponse<Self>;
}

#[cfg(all(feature = "v2", feature = "olap"))]
pub fn generate_revenue_recovery_get_intent_response<F, D>(
    payment_data: D,
    recovery_status: common_enums::RecoveryStatus,
    card_attached: u32,
) -> RevenueRecoveryGetIntentResponse
where
    F: Clone,
    D: OperationSessionGetters<F>,
{
    let payment_intent = payment_data.get_payment_intent();
    let client_secret = payment_data.get_client_secret();

    RevenueRecoveryGetIntentResponse {
        id: payment_intent.id.clone(),
        profile_id: payment_intent.profile_id.clone(),
        status: recovery_status, // Note: field is named 'status' not 'recovery_status'
        amount_details: api_models::payments::AmountDetailsResponse::foreign_from(
            payment_intent.amount_details.clone(),
        ),
        client_secret: client_secret.clone(),
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
        customer_present: payment_intent.customer_present,
        description: payment_intent.description.clone(),
        return_url: payment_intent.return_url.clone(),
        setup_future_usage: payment_intent.setup_future_usage,
        apply_mit_exemption: payment_intent.apply_mit_exemption,
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
        payment_link_enabled: payment_intent.enable_payment_link,
        payment_link_config: payment_intent
            .payment_link_config
            .clone()
            .map(ForeignFrom::foreign_from),
        request_incremental_authorization: payment_intent.request_incremental_authorization,
        split_txns_enabled: payment_intent.split_txns_enabled,
        expires_on: payment_intent.session_expiry,
        frm_metadata: payment_intent.frm_metadata.clone(),
        request_external_three_ds_authentication: payment_intent
            .request_external_three_ds_authentication,
        enable_partial_authorization: payment_intent.enable_partial_authorization,
        card_attached,
    }
}

/// Generate a response from the given Data. This should be implemented on a payment data object
pub trait GenerateResponse<Response>
where
    Self: Sized,
{
    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        self,
        state: &SessionState,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
        platform: &domain::Platform,
        profile: &domain::Profile,
        connector_response_data: Option<common_types::domain::ConnectorResponseData>,
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
        platform: &domain::Platform,
        profile: &domain::Profile,
        _connector_response_data: Option<common_types::domain::ConnectorResponseData>,
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

        let headers = connector_http_status_code
            .map(|status_code| {
                vec![(
                    X_CONNECTOR_HTTP_STATUS_CODE.to_string(),
                    Maskable::new_normal(status_code.to_string()),
                )]
            })
            .unwrap_or_default();

        Ok(services::ApplicationResponse::JsonWithHeaders((
            response, headers,
        )))
    }
}

#[cfg(feature = "v2")]
impl<F> GenerateResponse<api_models::payments::PaymentsCancelResponse>
    for hyperswitch_domain_models::payments::PaymentCancelData<F>
where
    F: Clone,
{
    fn generate_response(
        self,
        state: &SessionState,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
        platform: &domain::Platform,
        profile: &domain::Profile,
        _connector_response_data: Option<common_types::domain::ConnectorResponseData>,
    ) -> RouterResponse<api_models::payments::PaymentsCancelResponse> {
        let payment_intent = &self.payment_intent;
        let payment_attempt = &self.payment_attempt;

        let amount = api_models::payments::PaymentAmountDetailsResponse::foreign_from((
            &payment_intent.amount_details,
            &payment_attempt.amount_details,
        ));

        let connector = payment_attempt
            .connector
            .as_ref()
            .and_then(|conn| api_enums::Connector::from_str(conn).ok());
        let error = payment_attempt
            .error
            .as_ref()
            .map(api_models::payments::ErrorDetails::foreign_from);

        let response = api_models::payments::PaymentsCancelResponse {
            id: payment_intent.id.clone(),
            status: payment_intent.status,
            cancellation_reason: payment_attempt.cancellation_reason.clone(),
            amount,
            customer_id: payment_intent.customer_id.clone(),
            connector,
            created: payment_intent.created_at,
            payment_method_type: Some(payment_attempt.payment_method_type),
            payment_method_subtype: Some(payment_attempt.payment_method_subtype),
            attempts: None,
            return_url: payment_intent.return_url.clone(),
            error,
        };

        let headers = connector_http_status_code
            .map(|status_code| {
                vec![(
                    X_CONNECTOR_HTTP_STATUS_CODE.to_string(),
                    Maskable::new_normal(status_code.to_string()),
                )]
            })
            .unwrap_or_default();

        Ok(services::ApplicationResponse::JsonWithHeaders((
            response, headers,
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
        _platform: &domain::Platform,
    ) -> RouterResponse<Self> {
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                session_token: payment_data.get_sessions_token(),
                payment_id: payment_data.get_payment_intent().id.clone(),
                vault_details: payment_data.get_optional_external_vault_session_details(),
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
        _platform: &domain::Platform,
    ) -> RouterResponse<Self> {
        let payment_intent = payment_data.get_payment_intent();
        let client_secret = payment_data.get_client_secret();

        let is_cit_transaction = payment_intent.setup_future_usage.is_off_session();

        let mandate_type = if payment_intent.customer_present
            == common_enums::PresenceOfCustomerDuringPayment::Absent
        {
            Some(api::MandateTransactionType::RecurringMandateTransaction)
        } else if is_cit_transaction {
            Some(api::MandateTransactionType::NewMandateTransaction)
        } else {
            None
        };

        let payment_type = helpers::infer_payment_type(
            payment_intent.amount_details.order_amount.into(),
            mandate_type.as_ref(),
        );

        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                id: payment_intent.id.clone(),
                profile_id: payment_intent.profile_id.clone(),
                status: payment_intent.status,
                amount_details: api_models::payments::AmountDetailsResponse::foreign_from(
                    payment_intent.amount_details.clone(),
                ),
                client_secret: client_secret.clone(),
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
                customer_present: payment_intent.customer_present,
                description: payment_intent.description.clone(),
                return_url: payment_intent.return_url.clone(),
                setup_future_usage: payment_intent.setup_future_usage,
                apply_mit_exemption: payment_intent.apply_mit_exemption,
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
                payment_link_enabled: payment_intent.enable_payment_link,
                payment_link_config: payment_intent
                    .payment_link_config
                    .clone()
                    .map(ForeignFrom::foreign_from),
                request_incremental_authorization: payment_intent.request_incremental_authorization,
                split_txns_enabled: payment_intent.split_txns_enabled,
                expires_on: payment_intent.session_expiry,
                frm_metadata: payment_intent.frm_metadata.clone(),
                request_external_three_ds_authentication: payment_intent
                    .request_external_three_ds_authentication,
                payment_type,
                enable_partial_authorization: payment_intent.enable_partial_authorization,
            },
            vec![],
        )))
    }
}

#[cfg(feature = "v2")]
impl<F, Op, D> ToResponse<F, D, Op> for api::PaymentAttemptListResponse
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
        _platform: &domain::Platform,
    ) -> RouterResponse<Self> {
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                payment_attempt_list: payment_data
                    .list_payments_attempts()
                    .iter()
                    .map(api_models::payments::PaymentAttemptResponse::foreign_from)
                    .collect(),
            },
            vec![],
        )))
    }
}

#[cfg(feature = "v2")]
impl<F> GenerateResponse<api_models::payments::PaymentsResponse>
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
        platform: &domain::Platform,
        profile: &domain::Profile,
        connector_response_data: Option<common_types::domain::ConnectorResponseData>,
    ) -> RouterResponse<api_models::payments::PaymentsResponse> {
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

        let merchant_connector_id = payment_attempt.merchant_connector_id.clone();

        let error = payment_attempt
            .error
            .as_ref()
            .map(api_models::payments::ErrorDetails::foreign_from);

        let payment_address = self.payment_address;

        let raw_connector_response =
            connector_response_data.and_then(|data| data.raw_connector_response);

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
            platform
                .get_processor()
                .get_account()
                .publishable_key
                .clone(),
        )?;

        let next_action = if payment_intent.status.is_in_terminal_state() {
            None
        } else {
            let next_action_containing_wait_screen =
                wait_screen_next_steps_check(payment_attempt.clone())?;

            let upi_next_action = payment_attempt.get_upi_next_action()?;

            payment_attempt
                .redirection_data
                .as_ref()
                .map(|_| api_models::payments::NextActionData::RedirectToUrl { redirect_to_url })
                .or(upi_next_action)
                .or(next_action_containing_wait_screen.map(|wait_screen_data| {
                    api_models::payments::NextActionData::WaitScreenInformation {
                        display_from_timestamp: wait_screen_data.display_from_timestamp,
                        display_to_timestamp: wait_screen_data.display_to_timestamp,
                        poll_config: wait_screen_data.poll_config,
                    }
                }))
        };

        let connector_token_details = payment_attempt
            .connector_token_details
            .and_then(Option::<api_models::payments::ConnectorTokenDetails>::foreign_from);

        let return_url = payment_intent
            .return_url
            .clone()
            .or(profile.return_url.clone());

        let headers = connector_http_status_code
            .map(|status_code| {
                vec![(
                    X_CONNECTOR_HTTP_STATUS_CODE.to_string(),
                    Maskable::new_normal(status_code.to_string()),
                )]
            })
            .unwrap_or_default();

        let response = api_models::payments::PaymentsResponse {
            id: payment_intent.id.clone(),
            status: payment_intent.status,
            amount,
            customer_id: payment_intent.customer_id.clone(),
            connector: Some(connector),
            created: payment_intent.created_at,
            modified_at: payment_intent.modified_at,
            payment_method_data,
            payment_method_type: Some(payment_attempt.payment_method_type),
            payment_method_subtype: Some(payment_attempt.payment_method_subtype),
            next_action,
            connector_transaction_id: payment_attempt.connector_payment_id.clone(),
            connector_reference_id: payment_attempt.connector_response_reference_id.clone(),
            connector_token_details,
            merchant_connector_id,
            browser_info: None,
            error,
            return_url,
            authentication_type: payment_intent.authentication_type,
            authentication_type_applied: Some(payment_attempt.authentication_type),
            payment_method_id: payment_attempt.payment_method_id,
            attempts: None,
            billing: None,  //TODO: add this
            shipping: None, //TODO: add this
            is_iframe_redirection_enabled: None,
            merchant_reference_id: payment_intent.merchant_reference_id.clone(),
            raw_connector_response,
            feature_metadata: payment_intent
                .feature_metadata
                .map(|feature_metadata| feature_metadata.convert_back()),
            metadata: payment_intent.metadata,
        };

        Ok(services::ApplicationResponse::JsonWithHeaders((
            response, headers,
        )))
    }
}

#[cfg(feature = "v2")]
impl GenerateResponse<api_models::payments::PaymentsResponse>
    for crate::core::split_payments::SplitPaymentResponseData
{
    fn generate_response(
        self,
        state: &SessionState,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
        platform: &domain::Platform,
        profile: &domain::Profile,
        connector_response_data: Option<common_types::domain::ConnectorResponseData>,
    ) -> RouterResponse<api_models::payments::PaymentsResponse> {
        let payment_intent = self.primary_payment_response_data.payment_intent.clone();
        let payment_attempt = self.primary_payment_response_data.payment_attempt.clone();

        let intent_amount_details = &payment_intent.amount_details;
        let attempt_amount_details = &payment_attempt.amount_details;

        let net_amount = intent_amount_details.calculate_net_amount();

        let amount = api_models::payments::PaymentAmountDetailsResponse {
            order_amount: intent_amount_details.order_amount,
            currency: intent_amount_details.currency,
            shipping_cost: attempt_amount_details.get_shipping_cost(),
            order_tax_amount: attempt_amount_details.get_order_tax_amount(),
            external_tax_calculation: intent_amount_details.skip_external_tax_calculation,
            surcharge_calculation: intent_amount_details.skip_surcharge_calculation,
            surcharge_amount: attempt_amount_details.get_surcharge_amount(),
            tax_on_surcharge: attempt_amount_details.get_tax_on_surcharge(),
            net_amount,
            amount_to_capture: attempt_amount_details.get_amount_to_capture(),
            amount_capturable: attempt_amount_details.get_amount_capturable(),
            amount_captured: Some(net_amount),
        };

        let connector = payment_attempt
            .connector
            .clone()
            .get_required_value("connector")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector is none when constructing response")?;

        let merchant_connector_id = payment_attempt.merchant_connector_id.clone();

        let error = payment_attempt
            .error
            .as_ref()
            .map(api_models::payments::ErrorDetails::foreign_from);

        let payment_address = self.primary_payment_response_data.payment_address;

        let raw_connector_response =
            connector_response_data.and_then(|data| data.raw_connector_response);

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
            platform
                .get_processor()
                .get_account()
                .publishable_key
                .clone(),
        )?;

        let next_action = if payment_intent.status.is_in_terminal_state() {
            None
        } else {
            let next_action_containing_wait_screen =
                wait_screen_next_steps_check(payment_attempt.clone())?;

            payment_attempt
                .redirection_data
                .as_ref()
                .map(|_| api_models::payments::NextActionData::RedirectToUrl { redirect_to_url })
                .or(next_action_containing_wait_screen.map(|wait_screen_data| {
                    api_models::payments::NextActionData::WaitScreenInformation {
                        display_from_timestamp: wait_screen_data.display_from_timestamp,
                        display_to_timestamp: wait_screen_data.display_to_timestamp,
                        poll_config: wait_screen_data.poll_config,
                    }
                }))
        };

        let connector_token_details = payment_attempt
            .connector_token_details
            .and_then(Option::<api_models::payments::ConnectorTokenDetails>::foreign_from);

        let return_url = payment_intent
            .return_url
            .clone()
            .or(profile.return_url.clone());

        let headers = connector_http_status_code
            .map(|status_code| {
                vec![(
                    X_CONNECTOR_HTTP_STATUS_CODE.to_string(),
                    Maskable::new_normal(status_code.to_string()),
                )]
            })
            .unwrap_or_default();

        let response = api_models::payments::PaymentsResponse {
            id: payment_intent.id.clone(),
            status: payment_intent.status,
            amount,
            customer_id: payment_intent.customer_id.clone(),
            connector: Some(connector),
            created: payment_intent.created_at,
            modified_at: payment_intent.modified_at,
            payment_method_data,
            payment_method_type: Some(payment_attempt.payment_method_type),
            payment_method_subtype: Some(payment_attempt.payment_method_subtype),
            next_action,
            connector_transaction_id: payment_attempt.connector_payment_id.clone(),
            connector_reference_id: payment_attempt.connector_response_reference_id.clone(),
            connector_token_details,
            merchant_connector_id,
            browser_info: None,
            error,
            return_url,
            authentication_type: payment_intent.authentication_type,
            authentication_type_applied: Some(payment_attempt.authentication_type),
            payment_method_id: payment_attempt.payment_method_id,
            attempts: None,
            billing: None,  //TODO: add this
            shipping: None, //TODO: add this
            is_iframe_redirection_enabled: None,
            merchant_reference_id: payment_intent.merchant_reference_id.clone(),
            raw_connector_response,
            feature_metadata: payment_intent
                .feature_metadata
                .map(|feature_metadata| feature_metadata.convert_back()),
            metadata: payment_intent.metadata,
        };

        Ok(services::ApplicationResponse::JsonWithHeaders((
            response, headers,
        )))
    }
}

#[cfg(feature = "v2")]
impl<F> GenerateResponse<api_models::payments::PaymentsResponse>
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
        platform: &domain::Platform,
        profile: &domain::Profile,
        connector_response_data: Option<common_types::domain::ConnectorResponseData>,
    ) -> RouterResponse<api_models::payments::PaymentsResponse> {
        let payment_intent = self.payment_intent;
        let payment_attempt = &self.payment_attempt;

        let amount = api_models::payments::PaymentAmountDetailsResponse::foreign_from((
            &payment_intent.amount_details,
            &payment_attempt.amount_details,
        ));

        let connector = payment_attempt.connector.clone();

        let merchant_connector_id = payment_attempt.merchant_connector_id.clone();

        let error = payment_attempt
            .error
            .as_ref()
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

        let raw_connector_response =
            connector_response_data.and_then(|data| data.raw_connector_response);

        let connector_token_details = self
            .payment_attempt
            .connector_token_details
            .clone()
            .and_then(Option::<api_models::payments::ConnectorTokenDetails>::foreign_from);

        let return_url = payment_intent.return_url.or(profile.return_url.clone());

        let headers = connector_http_status_code
            .map(|status_code| {
                vec![(
                    X_CONNECTOR_HTTP_STATUS_CODE.to_string(),
                    Maskable::new_normal(status_code.to_string()),
                )]
            })
            .unwrap_or_default();

        let response = api_models::payments::PaymentsResponse {
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
            created: payment_intent.created_at,
            modified_at: payment_intent.modified_at,
            payment_method_data,
            payment_method_type: Some(payment_attempt.payment_method_type),
            payment_method_subtype: Some(payment_attempt.payment_method_subtype),
            connector_transaction_id: payment_attempt.connector_payment_id.clone(),
            connector_reference_id: payment_attempt.connector_response_reference_id.clone(),
            merchant_connector_id,
            browser_info: None,
            connector_token_details,
            payment_method_id: payment_attempt.payment_method_id.clone(),
            error,
            authentication_type_applied: payment_attempt.authentication_applied,
            authentication_type: payment_intent.authentication_type,
            next_action: None,
            attempts,
            return_url,
            is_iframe_redirection_enabled: payment_intent.is_iframe_redirection_enabled,
            merchant_reference_id: payment_intent.merchant_reference_id.clone(),
            raw_connector_response,
            feature_metadata: payment_intent
                .feature_metadata
                .map(|feature_metadata| feature_metadata.convert_back()),
            metadata: payment_intent.metadata,
        };

        Ok(services::ApplicationResponse::JsonWithHeaders((
            response, headers,
        )))
    }
}

#[cfg(feature = "v2")]
impl<F> GenerateResponse<api_models::payments::PaymentAttemptResponse>
    for hyperswitch_domain_models::payments::PaymentAttemptRecordData<F>
where
    F: Clone,
{
    fn generate_response(
        self,
        _state: &SessionState,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
        _platform: &domain::Platform,
        _profile: &domain::Profile,
        _connector_response_data: Option<common_types::domain::ConnectorResponseData>,
    ) -> RouterResponse<api_models::payments::PaymentAttemptResponse> {
        let payment_attempt = self.payment_attempt;
        let response = api_models::payments::PaymentAttemptResponse::foreign_from(&payment_attempt);
        Ok(services::ApplicationResponse::JsonWithHeaders((
            response,
            vec![],
        )))
    }
}

#[cfg(feature = "v2")]
impl<F> GenerateResponse<api_models::payments::PaymentAttemptRecordResponse>
    for hyperswitch_domain_models::payments::PaymentAttemptRecordData<F>
where
    F: Clone,
{
    fn generate_response(
        self,
        _state: &SessionState,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
        _platform: &domain::Platform,
        _profile: &domain::Profile,
        _connector_response_data: Option<common_types::domain::ConnectorResponseData>,
    ) -> RouterResponse<api_models::payments::PaymentAttemptRecordResponse> {
        let payment_attempt = self.payment_attempt;
        let payment_intent = self.payment_intent;
        let response = api_models::payments::PaymentAttemptRecordResponse {
            id: payment_attempt.id.clone(),
            status: payment_attempt.status,
            amount: payment_attempt.amount_details.get_net_amount(),
            payment_intent_feature_metadata: payment_intent
                .feature_metadata
                .as_ref()
                .map(api_models::payments::FeatureMetadata::foreign_from),
            payment_attempt_feature_metadata: payment_attempt
                .feature_metadata
                .as_ref()
                .map(api_models::payments::PaymentAttemptFeatureMetadata::foreign_from),
            error_details: payment_attempt
                .error
                .map(api_models::payments::RecordAttemptErrorDetails::from),
            created_at: payment_attempt.created_at,
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

#[cfg(feature = "v1")]
impl<F, Op, D> ToResponse<F, D, Op> for api::PaymentsUpdateMetadataResponse
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
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                payment_id: payment_data.get_payment_intent().payment_id.clone(),
                metadata: payment_data
                    .get_payment_intent()
                    .metadata
                    .clone()
                    .map(Secret::new),
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
    #[cfg(feature = "v2")]
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

    #[cfg(feature = "v1")]
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

#[cfg(feature = "v2")]
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
) -> RouterResponse<api_models::payments::PaymentsResponse>
where
    Op: Debug,
    D: OperationSessionGetters<F>,
{
    todo!()
}

#[cfg(feature = "v1")]
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

    use hyperswitch_interfaces::consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE, NO_ERROR_REASON};

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
                X_CONNECTOR_HTTP_STATUS_CODE.to_string(),
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
    let connector_name = payment_attempt.connector.as_deref().unwrap_or_default();
    let router_return_url = helpers::create_redirect_url(
        &base_url.to_string(),
        &payment_attempt,
        connector_name,
        payment_data.get_creds_identifier(),
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

        // Early exit for terminal payment statuses - don't evaluate next_action at all
        if payment_intent.status.is_in_terminal_state() {
            next_action_response = None;
        } else {
            let bank_transfer_next_steps = bank_transfer_next_steps_check(payment_attempt.clone())?;

            let next_action_voucher = voucher_next_steps_check(payment_attempt.clone())?;

            let next_action_mobile_payment = mobile_payment_next_steps_check(&payment_attempt)?;

            let next_action_containing_qr_code_url =
                qr_code_next_steps_check(payment_attempt.clone())?;

            let papal_sdk_next_action = paypal_sdk_next_steps_check(payment_attempt.clone())?;

            let next_action_containing_fetch_qr_code_url =
                fetch_qr_code_url_next_steps_check(payment_attempt.clone())?;

            let next_action_containing_wait_screen =
                wait_screen_next_steps_check(payment_attempt.clone())?;

            let upi_next_action = payment_attempt.get_upi_next_action()?;

            let next_action_invoke_hidden_frame =
                next_action_invoke_hidden_frame(&payment_attempt)?;

            if payment_intent.status == enums::IntentStatus::RequiresCustomerAction
                || bank_transfer_next_steps.is_some()
                || next_action_voucher.is_some()
                || next_action_containing_qr_code_url.is_some()
                || next_action_containing_wait_screen.is_some()
                || upi_next_action.is_some()
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
                            .or(upi_next_action)
                            .or(next_action_containing_wait_screen.map(|wait_screen_data| {
                                api_models::payments::NextActionData::WaitScreenInformation {
                                    display_from_timestamp: wait_screen_data.display_from_timestamp,
                                    display_to_timestamp: wait_screen_data.display_to_timestamp,
                                    poll_config: wait_screen_data.poll_config,
                                }
                            }))
                            .or(payment_attempt.authentication_data.as_ref().map(|_| {
                                // Check if iframe redirection is enabled in the business profile
                                let redirect_url = helpers::create_startpay_url(
                                    base_url,
                                    &payment_attempt,
                                    &payment_intent,
                                );
                                // Check if redirection inside popup is enabled in the payment intent
                                if payment_intent.is_iframe_redirection_enabled.unwrap_or(false) {
                                    api_models::payments::NextActionData::RedirectInsidePopup {
                                        popup_url: redirect_url,
                                        redirect_response_url:router_return_url
                                    }
                                } else {
                                    api_models::payments::NextActionData::RedirectToUrl {
                                        redirect_to_url: redirect_url,
                                    }
                                }
                            }))
                            .or(match payment_data.get_authentication(){
                                Some(authentication_store) => {
                                    let authentication = &authentication_store.authentication;
                                    if payment_intent.status == common_enums::IntentStatus::RequiresCustomerAction && authentication_store.cavv.is_none() && authentication.is_separate_authn_required(){
                                        // if preAuthn and separate authentication needed.
                                        let poll_config = payment_data.get_poll_config().unwrap_or_default();
                                        let request_poll_id = core_utils::get_external_authentication_request_poll_id(&payment_intent.payment_id);
                                        let payment_connector_name = payment_attempt.connector
                                            .as_ref()
                                            .get_required_value("connector")?;
                                        let is_jwt_flow = authentication.is_jwt_flow()
                                            .change_context(errors::ApiErrorResponse::InternalServerError)
                                            .attach_printable("Failed to determine if the authentication is JWT flow")?;
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
                                                        three_ds_method_key: if is_jwt_flow {
                                                            Some(api_models::payments::ThreeDsMethodKey::JWT)
                                                        } else {
                                                            Some(api_models::payments::ThreeDsMethodKey::ThreeDsMethodData)
                                                        },
                                                        // In JWT flow, we need to wait for post message to get the result
                                                        consume_post_message_for_three_ds_method_completion: is_jwt_flow,
                                                    }
                                                }).unwrap_or(api_models::payments::ThreeDsMethodData::AcsThreeDsMethodData {
                                                        three_ds_method_data_submission: false,
                                                        three_ds_method_data: None,
                                                        three_ds_method_url: None,
                                                        three_ds_method_key: None,
                                                        consume_post_message_for_three_ds_method_completion: false,
                                                }),
                                                poll_config: api_models::payments::PollConfigResponse {poll_id: request_poll_id, delay_in_secs: poll_config.delay_in_secs, frequency: poll_config.frequency},
                                                message_version: authentication.message_version.as_ref()
                                                .map(|version| version.to_string()),
                                                directory_server_id: authentication.directory_server_id.clone(),
                                                card_network: payment_method_data_response.as_ref().and_then(|method_data|method_data.get_card_network()),
                                                three_ds_connector: authentication.authentication_connector.clone(),
                                            },
                                        })
                                    }else{
                                        None
                                    }
                                },
                                None => None
                            })
                            .or(match next_action_invoke_hidden_frame{
                                Some(threeds_invoke_data) => Some(construct_connector_invoke_hidden_frame(
                                    threeds_invoke_data,
                                )?),
                                None => None
                            });
            }
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
            customer_acceptance: d.customer_acceptance.clone(),

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

        let manual_retry_allowed = match payment_data.get_is_manual_retry_enabled() {
            Some(true) => helpers::is_manual_retry_allowed(
                &payment_intent.status,
                &payment_attempt.status,
                connector_request_reference_id_config,
                &merchant_id,
            ),
            Some(false) | None => None,
        };

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
            setup_future_usage: payment_attempt.setup_future_usage_applied,
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
            error_code: payment_attempt
                .error_code
                .filter(|code| code != NO_ERROR_CODE),
            error_message: payment_attempt
                .error_message
                .filter(|message| message != NO_ERROR_MESSAGE),
            error_reason: payment_attempt
                .error_reason
                .filter(|reason| reason != NO_ERROR_REASON),
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
            manual_retry_allowed,
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
            network_transaction_id: payment_attempt.network_transaction_id,
            payment_method_status: payment_data
                .get_payment_method_info()
                .map(|info| info.status),
            updated: Some(payment_intent.modified_at),
            split_payments: payment_attempt.charges,
            frm_metadata: payment_intent.frm_metadata,
            merchant_order_reference_id: payment_intent.merchant_order_reference_id,
            order_tax_amount,
            connector_mandate_id,
            mit_category: payment_intent.mit_category,
            tokenization: payment_intent.tokenization,
            shipping_cost: payment_intent.shipping_cost,
            capture_before: payment_attempt.capture_before,
            extended_authorization_applied: payment_attempt.extended_authorization_applied,
            extended_authorization_last_applied_at: payment_attempt
                .extended_authorization_last_applied_at,
            card_discovery: payment_attempt.card_discovery,
            force_3ds_challenge: payment_intent.force_3ds_challenge,
            force_3ds_challenge_trigger: payment_intent.force_3ds_challenge_trigger,
            issuer_error_code: payment_attempt.issuer_error_code,
            issuer_error_message: payment_attempt.issuer_error_message,
            is_iframe_redirection_enabled: payment_intent.is_iframe_redirection_enabled,
            whole_connector_response: payment_data.get_whole_connector_response(),
            payment_channel: payment_intent.payment_channel,
            enable_partial_authorization: payment_intent.enable_partial_authorization,
            enable_overcapture: payment_intent.enable_overcapture,
            is_overcapture_enabled: payment_attempt.is_overcapture_enabled,
            network_details: payment_attempt
                .network_details
                .map(NetworkDetails::foreign_from),
            is_stored_credential: payment_attempt.is_stored_credential,
            request_extended_authorization: payment_attempt.request_extended_authorization,
            billing_descriptor: payment_intent.billing_descriptor,
            partner_merchant_identifier_details: payment_intent.partner_merchant_identifier_details,
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

pub fn next_action_invoke_hidden_frame(
    payment_attempt: &storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::PaymentsConnectorThreeDsInvokeData>> {
    let connector_three_ds_invoke_data: Option<
        Result<api_models::payments::PaymentsConnectorThreeDsInvokeData, _>,
    > = payment_attempt
        .connector_metadata
        .clone()
        .map(|metadata| metadata.parse_value("PaymentsConnectorThreeDsInvokeData"));

    let three_ds_invoke_data = connector_three_ds_invoke_data.transpose().ok().flatten();
    Ok(three_ds_invoke_data)
}

pub fn construct_connector_invoke_hidden_frame(
    connector_three_ds_invoke_data: api_models::payments::PaymentsConnectorThreeDsInvokeData,
) -> RouterResult<api_models::payments::NextActionData> {
    let iframe_data = api_models::payments::IframeData::ThreedsInvokeAndCompleteAutorize {
        three_ds_method_data_submission: connector_three_ds_invoke_data
            .three_ds_method_data_submission,
        three_ds_method_data: Some(connector_three_ds_invoke_data.three_ds_method_data),
        three_ds_method_url: connector_three_ds_invoke_data.three_ds_method_url,
        directory_server_id: connector_three_ds_invoke_data.directory_server_id,
        message_version: connector_three_ds_invoke_data.message_version,
    };

    Ok(api_models::payments::NextActionData::InvokeHiddenIframe { iframe_data })
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
            error_reason: None,
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
            extended_authorization_last_applied_at: pa.extended_authorization_last_applied_at,
            order_tax_amount: None,
            connector_mandate_id:None,
            shipping_cost: None,
            card_discovery: pa.card_discovery,
            mit_category: pi.mit_category,
            tokenization:pi.tokenization,
            force_3ds_challenge: pi.force_3ds_challenge,
            force_3ds_challenge_trigger: pi.force_3ds_challenge_trigger,
            whole_connector_response: None,
            issuer_error_code: pa.issuer_error_code,
            issuer_error_message: pa.issuer_error_message,
            is_iframe_redirection_enabled:pi.is_iframe_redirection_enabled,
            payment_channel: pi.payment_channel,
            network_transaction_id: None,
            enable_partial_authorization: pi.enable_partial_authorization,
            enable_overcapture: pi.enable_overcapture,
            is_overcapture_enabled: pa.is_overcapture_enabled,
            network_details: pa.network_details.map(NetworkDetails::foreign_from),
            is_stored_credential:pa.is_stored_credential,
            request_extended_authorization: pa.request_extended_authorization,
            billing_descriptor: pi.billing_descriptor,
            partner_merchant_identifier_details: pi.partner_merchant_identifier_details,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<(storage::PaymentIntent, Option<storage::PaymentAttempt>)>
    for api_models::payments::PaymentsListResponseItem
{
    fn foreign_from((pi, pa): (storage::PaymentIntent, Option<storage::PaymentAttempt>)) -> Self {
        let is_split_payment = pi.is_split_payment();
        Self {
            id: pi.id,
            merchant_id: pi.merchant_id,
            profile_id: pi.profile_id,
            customer_id: pi.customer_id,
            payment_method_id: pa.as_ref().and_then(|p| p.payment_method_id.clone()),
            status: pi.status,
            amount: api_models::payments::PaymentAmountDetailsResponse::foreign_from((
                &pi.amount_details,
                pa.as_ref().map(|p| &p.amount_details),
            )),
            created: pi.created_at,
            payment_method_type: pa.as_ref().and_then(|p| p.payment_method_type.into()),
            payment_method_subtype: pa.as_ref().and_then(|p| p.payment_method_subtype.into()),
            connector: pa.as_ref().and_then(|p| p.connector.clone()),
            merchant_connector_id: pa.as_ref().and_then(|p| p.merchant_connector_id.clone()),
            customer: None,
            merchant_reference_id: pi.merchant_reference_id,
            connector_payment_id: pa.as_ref().and_then(|p| p.connector_payment_id.clone()),
            connector_response_reference_id: pa
                .as_ref()
                .and_then(|p| p.connector_response_reference_id.clone()),
            metadata: pi.metadata,
            description: pi.description.map(|val| val.get_string_repr().to_string()),
            authentication_type: pi.authentication_type,
            capture_method: Some(pi.capture_method),
            setup_future_usage: Some(pi.setup_future_usage),
            attempt_count: pi.attempt_count,
            error: pa
                .as_ref()
                .and_then(|p| p.error.as_ref())
                .map(api_models::payments::ErrorDetails::foreign_from),
            cancellation_reason: pa.as_ref().and_then(|p| p.cancellation_reason.clone()),
            order_details: None,
            return_url: pi.return_url,
            statement_descriptor: pi.statement_descriptor,
            allowed_payment_method_types: pi.allowed_payment_method_types,
            authorization_count: pi.authorization_count,
            modified_at: pa.as_ref().map(|p| p.modified_at),
            is_split_payment,
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
                border_color: None,
                display_text: None,
            },
            api_models::payments::QrCodeInformation::QrDataUrl {
                image_data_url,
                display_to_timestamp,
            } => Self::QrCodeInformation {
                image_data_url: Some(image_data_url),
                display_to_timestamp,
                qr_code_url: None,
                border_color: None,
                display_text: None,
            },
            api_models::payments::QrCodeInformation::QrCodeImageUrl {
                qr_code_url,
                display_to_timestamp,
            } => Self::QrCodeInformation {
                qr_code_url: Some(qr_code_url),
                image_data_url: None,
                display_to_timestamp,
                border_color: None,
                display_text: None,
            },
            api_models::payments::QrCodeInformation::QrColorDataUrl {
                color_image_data_url,
                display_to_timestamp,
                border_color,
                display_text,
            } => Self::QrCodeInformation {
                qr_code_url: None,
                image_data_url: Some(color_image_data_url),
                display_to_timestamp,
                border_color,
                display_text,
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

#[cfg(feature = "v2")]
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
            .map(types::BrowserInformation::from);

        let complete_authorize_url = Some(helpers::create_complete_authorize_url(
            router_base_url,
            attempt,
            connector_name,
            payment_data.creds_identifier.as_deref(),
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

        let payment_method_data = payment_data.payment_method_data.or_else(|| {
            if payment_data.mandate_id.is_some() {
                Some(domain::PaymentMethodData::MandatePayment)
            } else {
                None
            }
        });

        let amount = payment_data
            .payment_attempt
            .get_total_amount()
            .get_amount_as_i64();

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
            .and_then(|data| data.get_id().clone().try_into().ok());

        let merchant_order_reference_id = payment_data
            .payment_intent
            .merchant_reference_id
            .map(|s| s.get_string_repr().to_string());

        let shipping_cost = payment_data.payment_intent.amount_details.shipping_cost;

        Ok(Self {
            payment_method_data: payment_method_data
                .unwrap_or(domain::PaymentMethodData::Card(domain::Card::default())),
            amount,
            order_tax_amount: None, // V2 doesn't currently support order tax amount
            email: None,            // V2 doesn't store email directly in payment_intent
            customer_name,
            currency: payment_data.currency,
            confirm: true,
            capture_method: Some(payment_data.payment_intent.capture_method),
            router_return_url,
            webhook_url,
            complete_authorize_url,
            setup_future_usage: Some(payment_data.payment_intent.setup_future_usage),
            mandate_id: payment_data.mandate_id.clone(),
            off_session: get_off_session(payment_data.mandate_id.as_ref(), None),
            customer_acceptance: None,
            setup_mandate_details: None,
            browser_info,
            order_details: None,
            order_category: None,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
            payment_experience: None,
            payment_method_type: Some(payment_data.payment_attempt.payment_method_subtype),
            surcharge_details: None,
            customer_id,
            request_incremental_authorization: false,
            metadata: payment_data
                .payment_intent
                .metadata
                .clone()
                .map(|m| m.expose()),
            authentication_data: None,
            request_extended_authorization: None,
            split_payments: None,
            minor_amount: payment_data.payment_attempt.get_total_amount(),
            merchant_order_reference_id,
            integrity_object: None,
            shipping_cost,
            additional_payment_method_data: None,
            merchant_account_id: None,
            merchant_config_currency: None,
            connector_testing_data: None,
            order_id: None,
            mit_category: None,
            tokenization: None,
            locale: None,
            payment_channel: None,
            enable_partial_authorization: None,
            enable_overcapture: None,
            is_stored_credential: None,
            billing_descriptor: None,
            partner_merchant_identifier_details: None,
        })
    }
}

fn get_off_session(
    mandate_id: Option<&MandateIds>,
    off_session_flag: Option<bool>,
) -> Option<bool> {
    match (mandate_id, off_session_flag) {
        (_, Some(false)) => Some(false),
        (Some(_), _) | (_, Some(true)) => Some(true),
        (None, None) => None,
    }
}

#[cfg(feature = "v1")]
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

        let connector_metadata = additional_data
            .payment_data
            .payment_intent
            .connector_metadata
            .clone()
            .map(|cm| {
                cm.parse_value::<api_models::payments::ConnectorMetadata>("ConnectorMetadata")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed parsing ConnectorMetadata")
            })
            .transpose()?;

        let order_category = connector_metadata.as_ref().and_then(|cm| {
            cm.noon
                .as_ref()
                .and_then(|noon| noon.order_category.clone())
        });

        let braintree_metadata = connector_metadata
            .as_ref()
            .and_then(|cm| cm.braintree.clone());

        let merchant_account_id = braintree_metadata
            .as_ref()
            .and_then(|braintree| braintree.merchant_account_id.clone());
        let merchant_config_currency =
            braintree_metadata.and_then(|braintree| braintree.merchant_config_currency);

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
            payment_data.creds_identifier.as_deref(),
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

        let additional_payment_method_data: Option<api_models::payments::AdditionalPaymentData> =
            payment_data.payment_attempt
                .payment_method_data
                .as_ref().map(|data| data.clone().parse_value("AdditionalPaymentData"))
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse AdditionalPaymentData from payment_data.payment_attempt.payment_method_data")?;

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

        let connector = api_models::enums::Connector::from_str(connector_name)
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector",
            })
            .attach_printable_lazy(|| {
                format!("unable to parse connector name {connector_name:?}")
            })?;

        let connector_testing_data = connector_metadata
            .and_then(|cm| match connector {
                api_models::enums::Connector::Adyen => cm
                    .adyen
                    .map(|adyen_cm| adyen_cm.testing)
                    .map(|testing_data| {
                        serde_json::to_value(testing_data)
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to parse Adyen testing data")
                    }),
                _ => None,
            })
            .transpose()?
            .map(pii::SecretSerdeValue::new);
        let is_off_session = get_off_session(
            payment_data.mandate_id.as_ref(),
            payment_data.payment_intent.off_session,
        );

        let billing_descriptor = payment_data.payment_intent.get_billing_descriptor();

        Ok(Self {
            payment_method_data: (payment_method_data.get_required_value("payment_method_data")?),
            setup_future_usage: payment_data.payment_attempt.setup_future_usage_applied,
            mandate_id: payment_data.mandate_id.clone(),
            off_session: is_off_session,
            setup_mandate_details: payment_data.setup_mandate.clone(),
            confirm: payment_data.payment_attempt.confirm,
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
            ),
            metadata: additional_data.payment_data.payment_intent.metadata,
            authentication_data: payment_data
                .authentication
                .as_ref()
                .map(AuthenticationData::foreign_try_from)
                .transpose()?
                .or(payment_data
                    .external_authentication_data
                    .as_ref()
                    .map(AuthenticationData::foreign_try_from)
                    .transpose()?),
            customer_acceptance: payment_data.customer_acceptance,
            request_extended_authorization: attempt.request_extended_authorization,
            split_payments,
            merchant_order_reference_id,
            integrity_object: None,
            additional_payment_method_data,
            shipping_cost,
            merchant_account_id,
            merchant_config_currency,
            connector_testing_data,
            mit_category: payment_data.payment_intent.mit_category,
            tokenization: payment_data.payment_intent.tokenization,
            order_id: None,
            locale: Some(additional_data.state.locale.clone()),
            payment_channel: payment_data.payment_intent.payment_channel,
            enable_partial_authorization: payment_data.payment_intent.enable_partial_authorization,
            enable_overcapture: payment_data.payment_intent.enable_overcapture,
            is_stored_credential: payment_data.payment_attempt.is_stored_credential,
            billing_descriptor,
            partner_merchant_identifier_details: payment_data
                .payment_intent
                .partner_merchant_identifier_details,
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsExtendAuthorizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsExtendAuthorizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        let amount = payment_data.payment_attempt.get_total_amount();

        Ok(Self {
            minor_amount: amount,
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(&payment_data.payment_attempt)?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            connector_meta: payment_data.payment_attempt.connector_metadata,
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
            connector_reference_id: payment_data
                .payment_attempt
                .connector_response_reference_id
                .clone(),
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
        })
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>>
    for types::PaymentsIncrementalAuthorizationData
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let payment_attempt = &payment_data.payment_attempt;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
            payment_attempt.merchant_connector_id.clone(),
        )?;
        let incremental_details = payment_data
            .incremental_authorization_details
            .as_ref()
            .ok_or(
                report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("missing incremental_authorization_details in payment_data"),
            )?;
        Ok(Self {
            total_amount: incremental_details.total_amount.get_amount_as_i64(),
            additional_amount: incremental_details.additional_amount.get_amount_as_i64(),
            reason: incremental_details.reason.clone(),
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_attempt)?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            connector_meta: payment_attempt.connector_metadata.clone(),
        })
    }
}

#[cfg(feature = "v2")]
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
        let incremental_details = payment_data
            .incremental_authorization_details
            .as_ref()
            .ok_or(
                report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("missing incremental_authorization_details in payment_data"),
            )?;
        Ok(Self {
            total_amount: incremental_details.total_amount.get_amount_as_i64(),
            additional_amount: incremental_details.additional_amount.get_amount_as_i64(),
            reason: incremental_details.reason.clone(),
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(&payment_data.payment_attempt)?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            connector_meta: payment_data
                .payment_attempt
                .connector_metadata
                .map(|secret| secret.expose()),
        })
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
                .connector_transaction_id(&payment_data.payment_attempt)?
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
            split_payments: None,
            webhook_url: None,
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

        let router_base_url = &additional_data.router_base_url;
        let attempt = &payment_data.payment_attempt;

        let merchant_connector_account_id = payment_data
            .payment_attempt
            .merchant_connector_id
            .as_ref()
            .map(|mca_id| mca_id.get_string_repr())
            .ok_or(errors::ApiErrorResponse::MerchantAccountNotFound)?;
        let webhook_url: Option<_> = Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            merchant_connector_account_id,
        ));
        Ok(Self {
            capture_method: payment_data.get_capture_method(),
            amount_to_capture: amount_to_capture.get_amount_as_i64(), // This should be removed once we start moving to connector module
            minor_amount_to_capture: amount_to_capture,
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(&payment_data.payment_attempt)?
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
            split_payments: payment_data.payment_intent.split_payments,
            webhook_url,
        })
    }
}

#[cfg(feature = "v2")]
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
            .map(types::BrowserInformation::from);

        let amount = payment_data.payment_attempt.amount_details.get_net_amount();

        let router_base_url = &additional_data.router_base_url;
        let attempt = &payment_data.payment_attempt;

        let merchant_connector_account_id = payment_data
            .payment_attempt
            .merchant_connector_id
            .as_ref()
            .map(|mca_id| mca_id.get_string_repr())
            .ok_or(errors::ApiErrorResponse::MerchantAccountNotFound)?;
        let webhook_url: Option<_> = Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            merchant_connector_account_id,
        ));
        let capture_method = payment_data.payment_intent.capture_method;
        Ok(Self {
            amount: Some(amount.get_amount_as_i64()), // This should be removed once we start moving to connector module
            minor_amount: Some(amount),
            currency: Some(payment_data.payment_intent.amount_details.currency),
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(&payment_data.payment_attempt)?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            cancellation_reason: payment_data.payment_attempt.cancellation_reason,
            connector_meta: payment_data
                .payment_attempt
                .connector_metadata
                .clone()
                .expose_option(),
            browser_info,
            metadata: payment_data.payment_intent.metadata.expose_option(),
            webhook_url,
            capture_method: Some(capture_method),
            split_payments: None,
        })
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

        let router_base_url = &additional_data.router_base_url;
        let attempt = &payment_data.payment_attempt;

        let merchant_connector_account_id = payment_data
            .payment_attempt
            .merchant_connector_id
            .as_ref()
            .map(|mca_id| mca_id.get_string_repr())
            .ok_or(errors::ApiErrorResponse::MerchantAccountNotFound)?;
        let webhook_url: Option<_> = Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            merchant_connector_account_id,
        ));
        let capture_method = payment_data.payment_attempt.capture_method;
        Ok(Self {
            amount: Some(amount.get_amount_as_i64()), // This should be removed once we start moving to connector module
            minor_amount: Some(amount),
            currency: Some(payment_data.currency),
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(&payment_data.payment_attempt)?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            cancellation_reason: payment_data.payment_attempt.cancellation_reason,
            connector_meta: payment_data.payment_attempt.connector_metadata,
            browser_info,
            metadata: payment_data.payment_intent.metadata,
            webhook_url,
            capture_method,
            split_payments: payment_data.payment_intent.split_payments.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsCancelPostCaptureData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsCancelPostCaptureData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        let amount = payment_data.payment_attempt.get_total_amount();

        Ok(Self {
            minor_amount: Some(amount),
            currency: Some(payment_data.currency),
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(&payment_data.payment_attempt)?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            cancellation_reason: payment_data.payment_attempt.cancellation_reason,
            connector_meta: payment_data.payment_attempt.connector_metadata,
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
            setup_future_usage: payment_data.payment_attempt.setup_future_usage_applied,
            router_return_url,
        })
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsUpdateMetadataData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsUpdateMetadataData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data.clone();
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        Ok(Self {
            metadata: payment_data
                .payment_intent
                .metadata
                .map(Secret::new)
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("payment_intent.metadata not found")?,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(&payment_data.payment_attempt)?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
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

        let order_tax_amount = payment_data
            .payment_intent
            .amount_details
            .tax_details
            .clone()
            .and_then(|tax| tax.get_default_tax_amount());

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
            customer_name: None,
            metadata: payment_data.payment_intent.metadata,
            order_tax_amount,
            shipping_cost: payment_data.payment_intent.amount_details.shipping_cost,
            payment_method: Some(payment_data.payment_attempt.payment_method_type),
            payment_method_type: Some(payment_data.payment_attempt.payment_method_subtype),
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

        let order_tax_amount = payment_data
            .payment_intent
            .tax_details
            .clone()
            .and_then(|tax| tax.get_default_tax_amount());

        let shipping_cost = payment_data.payment_intent.shipping_cost;

        let metadata = payment_data
            .payment_intent
            .metadata
            .clone()
            .map(Secret::new);

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
            customer_name: None,
            order_tax_amount,
            shipping_cost,
            metadata,
            payment_method: payment_data.payment_attempt.payment_method,
            payment_method_type: payment_data.payment_attempt.payment_method_type,
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

impl ForeignFrom<diesel_models::types::RedirectResponse>
    for api_models::payments::RedirectResponse
{
    fn foreign_from(redirect_res: diesel_models::types::RedirectResponse) -> Self {
        Self {
            param: redirect_res.param,
            json_payload: redirect_res.json_payload,
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
        let complete_authorize_url = Some(helpers::create_complete_authorize_url(
            router_base_url,
            attempt,
            connector_name,
            payment_data.creds_identifier.as_deref(),
        ));

        let connector = api_models::enums::Connector::from_str(connector_name)
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector",
            })
            .attach_printable_lazy(|| {
                format!("unable to parse connector name {connector_name:?}")
            })?;

        let connector_testing_data = payment_data
            .payment_intent
            .connector_metadata
            .as_ref()
            .map(|cm| {
                cm.clone()
                    .parse_value::<api_models::payments::ConnectorMetadata>("ConnectorMetadata")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed parsing ConnectorMetadata")
            })
            .transpose()?
            .and_then(|cm| match connector {
                api_models::enums::Connector::Adyen => cm
                    .adyen
                    .map(|adyen_cm| adyen_cm.testing)
                    .map(|testing_data| {
                        serde_json::to_value(testing_data)
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to parse Adyen testing data")
                    }),
                _ => None,
            })
            .transpose()?
            .map(pii::SecretSerdeValue::new);

        let is_off_session = get_off_session(
            payment_data.mandate_id.as_ref(),
            payment_data.payment_intent.off_session,
        );

        let billing_descriptor = payment_data.payment_intent.get_billing_descriptor();

        Ok(Self {
            currency: payment_data.currency,
            confirm: true,
            amount: Some(amount.get_amount_as_i64()), //need to change once we move to connector module
            minor_amount: Some(amount),
            payment_method_data: (payment_data
                .payment_method_data
                .get_required_value("payment_method_data")?),
            setup_future_usage: payment_data.payment_attempt.setup_future_usage_applied,
            off_session: is_off_session,
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
            ),
            metadata: payment_data.payment_intent.metadata.clone().map(Into::into),
            shipping_cost: payment_data.payment_intent.shipping_cost,
            webhook_url,
            complete_authorize_url,
            capture_method: payment_data.payment_attempt.capture_method,
            connector_testing_data,
            customer_id: payment_data.payment_intent.customer_id,
            enable_partial_authorization: payment_data.payment_intent.enable_partial_authorization,
            payment_channel: payment_data.payment_intent.payment_channel,
            related_transaction_id: None,
            enrolled_for_3ds: true,
            is_stored_credential: payment_data.payment_attempt.is_stored_credential,
            billing_descriptor,
            split_payments: payment_data.payment_intent.split_payments.clone(),
            tokenization: payment_data.payment_intent.tokenization,
            partner_merchant_identifier_details: payment_data
                .payment_intent
                .partner_merchant_identifier_details,
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

        let redirect_response = payment_data.redirect_response.clone().map(|redirect| {
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
            payment_data.creds_identifier.as_deref(),
        ));
        let braintree_metadata = payment_data
            .payment_intent
            .connector_metadata
            .clone()
            .map(|cm| {
                cm.parse_value::<api_models::payments::ConnectorMetadata>("ConnectorMetadata")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed parsing ConnectorMetadata")
            })
            .transpose()?
            .and_then(|cm| cm.braintree);

        let merchant_account_id = braintree_metadata
            .as_ref()
            .and_then(|braintree| braintree.merchant_account_id.clone());
        let merchant_config_currency =
            braintree_metadata.and_then(|braintree| braintree.merchant_config_currency);

        let is_off_session = get_off_session(
            payment_data.mandate_id.as_ref(),
            payment_data.payment_intent.off_session,
        );

        let router_return_url = Some(helpers::create_redirect_url(
            &additional_data.router_base_url.to_string(),
            &payment_data.payment_attempt,
            connector_name,
            payment_data.clone().get_creds_identifier(),
        ));
        Ok(Self {
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            mandate_id: payment_data.mandate_id.clone(),
            off_session: is_off_session,
            setup_mandate_details: payment_data.setup_mandate.clone(),
            confirm: payment_data.payment_attempt.confirm,
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            capture_method: payment_data.payment_attempt.capture_method,
            amount: amount.get_amount_as_i64(), // need to change once we move to connector module
            minor_amount: amount,
            currency: payment_data.currency,
            browser_info,
            email: payment_data.email,
            payment_method_data: payment_data.payment_method_data,
            connector_transaction_id: payment_data
                .payment_attempt
                .get_connector_payment_id()
                .map(ToString::to_string),
            redirect_response,
            connector_meta: payment_data.payment_attempt.connector_metadata,
            complete_authorize_url,
            metadata: payment_data.payment_intent.metadata,
            customer_acceptance: payment_data.customer_acceptance,
            merchant_account_id,
            merchant_config_currency,
            threeds_method_comp_ind: payment_data.threeds_method_comp_ind,
            is_stored_credential: payment_data.payment_attempt.is_stored_credential,
            payment_method_type: payment_data.payment_attempt.payment_method_type,
            authentication_data: payment_data
                .authentication
                .as_ref()
                .map(router_request_types::UcsAuthenticationData::foreign_try_from)
                .transpose()?,
            tokenization: payment_data.payment_intent.tokenization,
            router_return_url,
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
            payment_data.creds_identifier.as_deref(),
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
            payment_method_data,
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
            split_payments: payment_data.payment_intent.split_payments,
            metadata: payment_data.payment_intent.metadata.map(Secret::new),
            customer_acceptance: payment_data.customer_acceptance,
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            is_stored_credential: payment_data.payment_attempt.is_stored_credential,
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
            tax_registration_id: customer.tax_registration_id,
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
        let payment_method_data: Option<
            api_models::payments::PaymentMethodDataResponseWithBilling,
        > = attempt
            .payment_method_data
            .clone()
            .and_then(|data| serde_json::from_value(data.expose().clone()).ok());
        Self {
            id: attempt.get_id().to_owned(),
            status: attempt.status,
            amount: api_models::payments::PaymentAttemptAmountDetails::foreign_from(
                &attempt.amount_details,
            ),
            connector: attempt.connector.clone(),
            error: attempt
                .error
                .as_ref()
                .map(api_models::payments::ErrorDetails::foreign_from),
            authentication_type: attempt.authentication_type,
            created_at: attempt.created_at,
            modified_at: attempt.modified_at,
            cancellation_reason: attempt.cancellation_reason.clone(),
            payment_token: attempt
                .connector_token_details
                .as_ref()
                .and_then(|details| details.connector_mandate_id.clone()),
            connector_metadata: attempt.connector_metadata.clone(),
            payment_experience: attempt.payment_experience,
            payment_method_type: attempt.payment_method_type,
            connector_reference_id: attempt.connector_response_reference_id.clone(),
            payment_method_subtype: attempt.get_payment_method_type(),
            connector_payment_id: attempt
                .get_connector_payment_id()
                .map(|str| common_utils::types::ConnectorTransactionId::from(str.to_owned())),
            payment_method_id: attempt.payment_method_id.clone(),
            client_source: attempt.client_source.clone(),
            client_version: attempt.client_version.clone(),
            feature_metadata: attempt
                .feature_metadata
                .as_ref()
                .map(api_models::payments::PaymentAttemptFeatureMetadata::foreign_from),
            payment_method_data,
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
impl ForeignFrom<&diesel_models::types::BillingConnectorPaymentDetails>
    for api_models::payments::BillingConnectorPaymentDetails
{
    fn foreign_from(metadata: &diesel_models::types::BillingConnectorPaymentDetails) -> Self {
        Self {
            payment_processor_token: metadata.payment_processor_token.clone(),
            connector_customer_id: metadata.connector_customer_id.clone(),
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<&diesel_models::types::BillingConnectorPaymentMethodDetails>
    for api_models::payments::BillingConnectorPaymentMethodDetails
{
    fn foreign_from(metadata: &diesel_models::types::BillingConnectorPaymentMethodDetails) -> Self {
        match metadata {
            diesel_models::types::BillingConnectorPaymentMethodDetails::Card(card_details) => {
                Self::Card(api_models::payments::BillingConnectorAdditionalCardInfo {
                    card_issuer: card_details.card_issuer.clone(),
                    card_network: card_details.card_network.clone(),
                })
            }
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<&hyperswitch_domain_models::payments::payment_attempt::ErrorDetails>
    for api_models::payments::ErrorDetails
{
    fn foreign_from(
        error_details: &hyperswitch_domain_models::payments::payment_attempt::ErrorDetails,
    ) -> Self {
        Self {
            code: error_details.code.to_owned(),
            message: error_details.message.to_owned(),
            reason: error_details.reason.clone(),
            unified_code: error_details.unified_code.clone(),
            unified_message: error_details.unified_message.clone(),
            network_advice_code: error_details.network_advice_code.clone(),
            network_decline_code: error_details.network_decline_code.clone(),
            network_error_message: error_details.network_error_message.clone(),
        }
    }
}

#[cfg(feature = "v2")]
impl
    ForeignFrom<
        &hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptFeatureMetadata,
    > for api_models::payments::PaymentAttemptFeatureMetadata
{
    fn foreign_from(
        feature_metadata: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptFeatureMetadata,
    ) -> Self {
        let revenue_recovery = feature_metadata.revenue_recovery.as_ref().map(|recovery| {
            api_models::payments::PaymentAttemptRevenueRecoveryData {
                attempt_triggered_by: recovery.attempt_triggered_by,
                charge_id: recovery.charge_id.clone(),
            }
        });
        Self { revenue_recovery }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<&diesel_models::types::FeatureMetadata> for api_models::payments::FeatureMetadata {
    fn foreign_from(feature_metadata: &diesel_models::types::FeatureMetadata) -> Self {
        let revenue_recovery = feature_metadata
            .payment_revenue_recovery_metadata
            .as_ref()
            .map(|payment_revenue_recovery_metadata| {
                api_models::payments::PaymentRevenueRecoveryMetadata {
                    total_retry_count: payment_revenue_recovery_metadata.total_retry_count,
                    payment_connector_transmission: Some(
                        payment_revenue_recovery_metadata.payment_connector_transmission,
                    ),
                    connector: payment_revenue_recovery_metadata.connector,
                    billing_connector_id: payment_revenue_recovery_metadata
                        .billing_connector_id
                        .clone(),
                    active_attempt_payment_connector_id: payment_revenue_recovery_metadata
                        .active_attempt_payment_connector_id
                        .clone(),
                    payment_method_type: payment_revenue_recovery_metadata.payment_method_type,
                    payment_method_subtype: payment_revenue_recovery_metadata
                        .payment_method_subtype,
                    billing_connector_payment_details:
                        api_models::payments::BillingConnectorPaymentDetails::foreign_from(
                            &payment_revenue_recovery_metadata.billing_connector_payment_details,
                        ),
                    invoice_next_billing_time: payment_revenue_recovery_metadata
                        .invoice_next_billing_time,
                        billing_connector_payment_method_details:payment_revenue_recovery_metadata
                        .billing_connector_payment_method_details.as_ref().map(api_models::payments::BillingConnectorPaymentMethodDetails::foreign_from),
                    first_payment_attempt_network_advice_code: payment_revenue_recovery_metadata
                        .first_payment_attempt_network_advice_code
                        .clone(),
                    first_payment_attempt_network_decline_code: payment_revenue_recovery_metadata
                        .first_payment_attempt_network_decline_code
                        .clone(),
                    first_payment_attempt_pg_error_code: payment_revenue_recovery_metadata
                        .first_payment_attempt_pg_error_code
                        .clone(),
                    invoice_billing_started_at_time: payment_revenue_recovery_metadata
                        .invoice_billing_started_at_time,
                }
            });
        let apple_pay_details = feature_metadata
            .apple_pay_recurring_details
            .clone()
            .map(api_models::payments::ApplePayRecurringDetails::foreign_from);
        let redirect_res = feature_metadata
            .redirect_response
            .clone()
            .map(api_models::payments::RedirectResponse::foreign_from);
        Self {
            revenue_recovery,
            apple_pay_recurring_details: apple_pay_details,
            redirect_response: redirect_res,
            search_tags: feature_metadata.search_tags.clone(),
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
            custom_message_for_payment_method_types: config.custom_message_for_payment_method_types,
            payment_button_colour: config.payment_button_colour,
            skip_status_screen: config.skip_status_screen,
            background_colour: config.background_colour,
            payment_button_text_colour: config.payment_button_text_colour,
            sdk_ui_rules: config.sdk_ui_rules,
            payment_link_ui_rules: config.payment_link_ui_rules,
            enable_button_only_on_form_ready: config.enable_button_only_on_form_ready,
            payment_form_header_text: config.payment_form_header_text,
            payment_form_label_type: config.payment_form_label_type,
            show_card_terms: config.show_card_terms,
            is_setup_mandate_flow: config.is_setup_mandate_flow,
            color_icon_card_cvc_error: config.color_icon_card_cvc_error,
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
            custom_message_for_payment_method_types: config.custom_message_for_payment_method_types,
            payment_button_colour: config.payment_button_colour,
            skip_status_screen: config.skip_status_screen,
            background_colour: config.background_colour,
            payment_button_text_colour: config.payment_button_text_colour,
            sdk_ui_rules: config.sdk_ui_rules,
            payment_link_ui_rules: config.payment_link_ui_rules,
            enable_button_only_on_form_ready: config.enable_button_only_on_form_ready,
            payment_form_header_text: config.payment_form_header_text,
            payment_form_label_type: config.payment_form_label_type,
            show_card_terms: config.show_card_terms,
            is_setup_mandate_flow: config.is_setup_mandate_flow,
            color_icon_card_cvc_error: config.color_icon_card_cvc_error,
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
            None,
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

impl ForeignFrom<DieselNetworkDetails> for NetworkDetails {
    fn foreign_from(value: DieselNetworkDetails) -> Self {
        Self {
            network_advice_code: value.network_advice_code,
        }
    }
}

impl ForeignFrom<NetworkDetails> for DieselNetworkDetails {
    fn foreign_from(value: NetworkDetails) -> Self {
        Self {
            network_advice_code: value.network_advice_code,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<diesel_models::ConnectorTokenDetails>
    for Option<api_models::payments::ConnectorTokenDetails>
{
    fn foreign_from(value: diesel_models::ConnectorTokenDetails) -> Self {
        let connector_token_request_reference_id =
            value.connector_token_request_reference_id.clone();
        value.connector_mandate_id.clone().map(|mandate_id| {
            api_models::payments::ConnectorTokenDetails {
                token: mandate_id,
                connector_token_request_reference_id,
            }
        })
    }
}

impl
    ForeignFrom<(
        Self,
        Option<&api_models::payments::AdditionalPaymentData>,
        Option<enums::PaymentMethod>,
    )> for Option<enums::PaymentMethodType>
{
    fn foreign_from(
        req: (
            Self,
            Option<&api_models::payments::AdditionalPaymentData>,
            Option<enums::PaymentMethod>,
        ),
    ) -> Self {
        let (payment_method_type, additional_pm_data, payment_method) = req;

        match (additional_pm_data, payment_method, payment_method_type) {
            (
                Some(api_models::payments::AdditionalPaymentData::Card(card_info)),
                Some(enums::PaymentMethod::Card),
                original_type,
            ) => {
                let bin_card_type = card_info.card_type.as_ref().and_then(|card_type_str| {
                    let normalized_type = card_type_str.trim().to_lowercase();
                    if normalized_type.is_empty() {
                        return None;
                    }
                    api_models::enums::PaymentMethodType::from_str(&normalized_type)
                        .map_err(|_| {
                            crate::logger::warn!("Invalid BIN card_type: '{}'", card_type_str);
                        })
                        .ok()
                });

                match (original_type, bin_card_type) {
                    // Override when there's a mismatch
                    (
                        Some(
                            original @ (enums::PaymentMethodType::Debit
                            | enums::PaymentMethodType::Credit),
                        ),
                        Some(bin_type),
                    ) if original != bin_type => {
                        crate::logger::info!("BIN lookup override: {} -> {}", original, bin_type);
                        bin_card_type
                    }
                    // Use BIN lookup if no original type exists
                    (None, Some(bin_type)) => {
                        crate::logger::info!(
                            "BIN lookup override: No original payment method type, using BIN result={}",
                            bin_type
                        );
                        Some(bin_type)
                    }
                    // Default
                    _ => original_type,
                }
            }
            // Skip BIN lookup for non-card payments
            _ => payment_method_type,
        }
    }
}

#[cfg(feature = "v1")]
impl From<pm_types::TokenResponse> for domain::NetworkTokenData {
    fn from(token_response: pm_types::TokenResponse) -> Self {
        Self {
            token_number: token_response.authentication_details.token,
            token_exp_month: token_response.token_details.exp_month,
            token_exp_year: token_response.token_details.exp_year,
            token_cryptogram: Some(token_response.authentication_details.cryptogram),
            card_issuer: None,
            card_network: Some(token_response.network),
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: None,
            eci: None,
        }
    }
}
impl ForeignFrom<&hyperswitch_domain_models::router_data::ErrorResponse> for DieselNetworkDetails {
    fn foreign_from(err: &hyperswitch_domain_models::router_data::ErrorResponse) -> Self {
        Self {
            network_advice_code: err.network_advice_code.clone(),
        }
    }
}

impl ForeignFrom<common_types::three_ds_decision_rule_engine::ThreeDSDecision>
    for common_enums::AuthenticationType
{
    fn foreign_from(
        three_ds_decision: common_types::three_ds_decision_rule_engine::ThreeDSDecision,
    ) -> Self {
        match three_ds_decision {
            common_types::three_ds_decision_rule_engine::ThreeDSDecision::NoThreeDs => Self::NoThreeDs,
            common_types::three_ds_decision_rule_engine::ThreeDSDecision::ChallengeRequested
            | common_types::three_ds_decision_rule_engine::ThreeDSDecision::ChallengePreferred
            | common_types::three_ds_decision_rule_engine::ThreeDSDecision::ThreeDsExemptionRequestedTra
            | common_types::three_ds_decision_rule_engine::ThreeDSDecision::ThreeDsExemptionRequestedLowValue
            | common_types::three_ds_decision_rule_engine::ThreeDSDecision::IssuerThreeDsExemptionRequested => Self::ThreeDs,
        }
    }
}

impl ForeignFrom<common_types::three_ds_decision_rule_engine::ThreeDSDecision>
    for Option<common_enums::ScaExemptionType>
{
    fn foreign_from(
        three_ds_decision: common_types::three_ds_decision_rule_engine::ThreeDSDecision,
    ) -> Self {
        match three_ds_decision {
            common_types::three_ds_decision_rule_engine::ThreeDSDecision::ThreeDsExemptionRequestedTra => {
                Some(common_enums::ScaExemptionType::TransactionRiskAnalysis)
            }
            common_types::three_ds_decision_rule_engine::ThreeDSDecision::ThreeDsExemptionRequestedLowValue => {
                Some(common_enums::ScaExemptionType::LowValue)
            }
            common_types::three_ds_decision_rule_engine::ThreeDSDecision::NoThreeDs
            | common_types::three_ds_decision_rule_engine::ThreeDSDecision::ChallengeRequested
            | common_types::three_ds_decision_rule_engine::ThreeDSDecision::ChallengePreferred
            | common_types::three_ds_decision_rule_engine::ThreeDSDecision::IssuerThreeDsExemptionRequested => {
                None
            }
        }
    }
}
