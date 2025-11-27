pub mod authentication;
pub mod fraud_check;
pub mod revenue_recovery;
pub mod subscriptions;
pub mod unified_authentication_service;
use api_models::payments::{AdditionalPaymentData, AddressDetails, RequestSurchargeDetails};
use common_types::payments as common_payments_types;
use common_utils::{
    consts, errors,
    ext_traits::OptionExt,
    id_type, pii,
    types::{MinorUnit, SemanticVersion},
};
use diesel_models::{enums as storage_enums, types::OrderDetailsWithAmount};
use error_stack::ResultExt;
use masking::Secret;
use serde::Serialize;
use serde_with::serde_as;

use super::payment_method_data::PaymentMethodData;
use crate::{
    address,
    errors::api_error_response::{ApiErrorResponse, NotImplementedMessage},
    mandates,
    payment_method_data::ExternalVaultPaymentMethodData,
    payments,
    router_data::{self, AccessTokenAuthenticationResponse, RouterData},
    router_flow_types as flows, router_response_types as response_types,
    vault::PaymentMethodCustomVaultingData,
};
#[derive(Debug, Clone, Serialize)]
pub struct PaymentsAuthorizeData {
    pub payment_method_data: PaymentMethodData,
    /// total amount (original_amount + surcharge_amount + tax_on_surcharge_amount)
    /// If connector supports separate field for surcharge amount, consider using below functions defined on `PaymentsAuthorizeData` to fetch original amount and surcharge amount separately
    /// ```text
    /// get_original_amount()
    /// get_surcharge_amount()
    /// get_tax_on_surcharge_amount()
    /// get_total_surcharge_amount() // returns surcharge_amount + tax_on_surcharge_amount
    /// ```
    pub amount: i64,
    pub order_tax_amount: Option<MinorUnit>,
    pub email: Option<pii::Email>,
    pub customer_name: Option<Secret<String>>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub router_return_url: Option<String>,
    pub webhook_url: Option<String>,
    pub complete_authorize_url: Option<String>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub browser_info: Option<BrowserInformation>,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub order_category: Option<String>,
    pub session_token: Option<String>,
    pub enrolled_for_3ds: bool,
    pub related_transaction_id: Option<String>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub surcharge_details: Option<SurchargeDetails>,
    pub customer_id: Option<id_type::CustomerId>,
    pub request_incremental_authorization: bool,
    pub metadata: Option<serde_json::Value>,
    pub authentication_data: Option<AuthenticationData>,
    pub request_extended_authorization:
        Option<common_types::primitive_wrappers::RequestExtendedAuthorizationBool>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,

    // New amount for amount frame work
    pub minor_amount: MinorUnit,

    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    pub merchant_order_reference_id: Option<String>,
    pub integrity_object: Option<AuthoriseIntegrityObject>,
    pub shipping_cost: Option<MinorUnit>,
    pub additional_payment_method_data: Option<AdditionalPaymentData>,
    pub merchant_account_id: Option<Secret<String>>,
    pub merchant_config_currency: Option<storage_enums::Currency>,
    pub connector_testing_data: Option<pii::SecretSerdeValue>,
    pub order_id: Option<String>,
    pub locale: Option<String>,
    pub payment_channel: Option<common_enums::PaymentChannel>,
    pub enable_partial_authorization:
        Option<common_types::primitive_wrappers::EnablePartialAuthorizationBool>,
    pub enable_overcapture: Option<common_types::primitive_wrappers::EnableOvercaptureBool>,
    pub is_stored_credential: Option<bool>,
    pub mit_category: Option<common_enums::MitCategory>,
    pub billing_descriptor: Option<common_types::payments::BillingDescriptor>,
    pub tokenization: Option<common_enums::Tokenization>,
    pub partner_merchant_identifier_details:
        Option<common_types::payments::PartnerMerchantIdentifierDetails>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalVaultProxyPaymentsData {
    pub payment_method_data: ExternalVaultPaymentMethodData,
    /// total amount (original_amount + surcharge_amount + tax_on_surcharge_amount)
    /// If connector supports separate field for surcharge amount, consider using below functions defined on `PaymentsAuthorizeData` to fetch original amount and surcharge amount separately
    /// ```text
    /// get_original_amount()
    /// get_surcharge_amount()
    /// get_tax_on_surcharge_amount()
    /// get_total_surcharge_amount() // returns surcharge_amount + tax_on_surcharge_amount
    /// ```
    pub amount: i64,
    pub order_tax_amount: Option<MinorUnit>,
    pub email: Option<pii::Email>,
    pub customer_name: Option<Secret<String>>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub router_return_url: Option<String>,
    pub webhook_url: Option<String>,
    pub complete_authorize_url: Option<String>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub browser_info: Option<BrowserInformation>,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub order_category: Option<String>,
    pub session_token: Option<String>,
    pub enrolled_for_3ds: bool,
    pub related_transaction_id: Option<String>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub surcharge_details: Option<SurchargeDetails>,
    pub customer_id: Option<id_type::CustomerId>,
    pub request_incremental_authorization: bool,
    pub metadata: Option<serde_json::Value>,
    pub authentication_data: Option<AuthenticationData>,
    pub request_extended_authorization:
        Option<common_types::primitive_wrappers::RequestExtendedAuthorizationBool>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,

    // New amount for amount frame work
    pub minor_amount: MinorUnit,
    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    pub merchant_order_reference_id: Option<id_type::PaymentReferenceId>,
    pub integrity_object: Option<AuthoriseIntegrityObject>,
    pub shipping_cost: Option<MinorUnit>,
    pub additional_payment_method_data: Option<AdditionalPaymentData>,
    pub merchant_account_id: Option<Secret<String>>,
    pub merchant_config_currency: Option<storage_enums::Currency>,
    pub connector_testing_data: Option<pii::SecretSerdeValue>,
    pub order_id: Option<String>,
}

// Note: Integrity traits for ExternalVaultProxyPaymentsData are not implemented
// as they are not mandatory for this flow. The integrity_check field in RouterData
// will use Ok(()) as default, similar to other flows.

// Implement ConnectorCustomerData conversion for ExternalVaultProxy RouterData
impl
    TryFrom<
        &RouterData<
            flows::ExternalVaultProxy,
            ExternalVaultProxyPaymentsData,
            response_types::PaymentsResponseData,
        >,
    > for ConnectorCustomerData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(
        data: &RouterData<
            flows::ExternalVaultProxy,
            ExternalVaultProxyPaymentsData,
            response_types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.request.email.clone(),
            payment_method_data: None, // External vault proxy doesn't use regular payment method data
            description: None,
            phone: None,
            name: data.request.customer_name.clone(),
            preprocessing_id: data.preprocessing_id.clone(),
            split_payments: data.request.split_payments.clone(),
            setup_future_usage: data.request.setup_future_usage,
            customer_acceptance: data.request.customer_acceptance.clone(),
            customer_id: None,
            billing_address: None,
        })
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct PaymentsPostSessionTokensData {
    // amount here would include amount, surcharge_amount and shipping_cost
    pub amount: MinorUnit,
    /// original amount sent by the merchant
    pub order_amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    pub merchant_order_reference_id: Option<String>,
    pub shipping_cost: Option<MinorUnit>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub router_return_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentsUpdateMetadataData {
    pub metadata: pii::SecretSerdeValue,
    pub connector_transaction_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuthoriseIntegrityObject {
    /// Authorise amount
    pub amount: MinorUnit,
    /// Authorise currency
    pub currency: storage_enums::Currency,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyncIntegrityObject {
    /// Sync amount
    pub amount: Option<MinorUnit>,
    /// Sync currency
    pub currency: Option<storage_enums::Currency>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PaymentsCaptureData {
    pub amount_to_capture: i64,
    pub currency: storage_enums::Currency,
    pub connector_transaction_id: String,
    pub payment_amount: i64,
    pub multiple_capture_data: Option<MultipleCaptureRequestData>,
    pub connector_meta: Option<serde_json::Value>,
    pub browser_info: Option<BrowserInformation>,
    pub metadata: Option<serde_json::Value>,
    // This metadata is used to store the metadata shared during the payment intent request.
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,
    // New amount for amount frame work
    pub minor_payment_amount: MinorUnit,
    pub minor_amount_to_capture: MinorUnit,
    pub integrity_object: Option<CaptureIntegrityObject>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaptureIntegrityObject {
    /// capture amount
    pub capture_amount: Option<MinorUnit>,
    /// capture currency
    pub currency: storage_enums::Currency,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PaymentsIncrementalAuthorizationData {
    pub total_amount: i64,
    pub additional_amount: i64,
    pub currency: storage_enums::Currency,
    pub reason: Option<String>,
    pub connector_transaction_id: String,
    pub connector_meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MultipleCaptureRequestData {
    pub capture_sequence: i16,
    pub capture_reference: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthorizeSessionTokenData {
    pub amount_to_capture: Option<i64>,
    pub currency: storage_enums::Currency,
    pub connector_transaction_id: String,
    pub amount: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectorCustomerData {
    pub description: Option<String>,
    pub email: Option<pii::Email>,
    pub phone: Option<Secret<String>>,
    pub name: Option<Secret<String>>,
    pub preprocessing_id: Option<String>,
    pub payment_method_data: Option<PaymentMethodData>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    pub customer_id: Option<id_type::CustomerId>,
    pub billing_address: Option<AddressDetails>,
}

impl TryFrom<SetupMandateRequestData> for ConnectorCustomerData {
    type Error = error_stack::Report<ApiErrorResponse>;
    fn try_from(data: SetupMandateRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.email,
            payment_method_data: Some(data.payment_method_data),
            description: None,
            phone: None,
            name: None,
            preprocessing_id: None,
            split_payments: data.split_payments,
            setup_future_usage: data.setup_future_usage,
            customer_acceptance: data.customer_acceptance,
            customer_id: None,
            billing_address: None,
        })
    }
}

impl TryFrom<SetupMandateRequestData> for PaymentsPreProcessingData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: SetupMandateRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: Some(data.payment_method_data),
            amount: data.amount,
            minor_amount: data.minor_amount,
            email: data.email,
            currency: Some(data.currency),
            payment_method_type: data.payment_method_type,
            setup_mandate_details: data.setup_mandate_details,
            capture_method: data.capture_method,
            order_details: None,
            router_return_url: data.router_return_url,
            webhook_url: data.webhook_url,
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            surcharge_details: None,
            connector_transaction_id: None,
            mandate_id: data.mandate_id,
            related_transaction_id: None,
            redirect_response: None,
            enrolled_for_3ds: false,
            split_payments: None,
            metadata: data.metadata,
            customer_acceptance: data.customer_acceptance,
            setup_future_usage: data.setup_future_usage,
            is_stored_credential: data.is_stored_credential,
        })
    }
}
impl
    TryFrom<
        &RouterData<flows::Authorize, PaymentsAuthorizeData, response_types::PaymentsResponseData>,
    > for ConnectorCustomerData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(
        data: &RouterData<
            flows::Authorize,
            PaymentsAuthorizeData,
            response_types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.request.email.clone(),
            payment_method_data: Some(data.request.payment_method_data.clone()),
            description: None,
            phone: None,
            name: data.request.customer_name.clone(),
            preprocessing_id: data.preprocessing_id.clone(),
            split_payments: data.request.split_payments.clone(),
            setup_future_usage: data.request.setup_future_usage,
            customer_acceptance: data.request.customer_acceptance.clone(),
            customer_id: None,
            billing_address: None,
        })
    }
}

impl TryFrom<&RouterData<flows::Session, PaymentsSessionData, response_types::PaymentsResponseData>>
    for ConnectorCustomerData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(
        data: &RouterData<
            flows::Session,
            PaymentsSessionData,
            response_types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.request.email.clone(),
            payment_method_data: None,
            description: None,
            phone: None,
            name: data.request.customer_name.clone(),
            preprocessing_id: data.preprocessing_id.clone(),
            split_payments: None,
            setup_future_usage: None,
            customer_acceptance: None,
            customer_id: None,
            billing_address: None,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentMethodTokenizationData {
    pub payment_method_data: PaymentMethodData,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub browser_info: Option<BrowserInformation>,
    pub currency: storage_enums::Currency,
    pub amount: Option<i64>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
}

impl TryFrom<SetupMandateRequestData> for PaymentMethodTokenizationData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: SetupMandateRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            browser_info: None,
            currency: data.currency,
            amount: data.amount,
            split_payments: None,
            customer_acceptance: data.customer_acceptance,
            setup_future_usage: data.setup_future_usage,
            setup_mandate_details: data.setup_mandate_details,
            mandate_id: data.mandate_id,
            payment_method_type: data.payment_method_type,
        })
    }
}
impl<F> From<&RouterData<F, PaymentsAuthorizeData, response_types::PaymentsResponseData>>
    for PaymentMethodTokenizationData
{
    fn from(
        data: &RouterData<F, PaymentsAuthorizeData, response_types::PaymentsResponseData>,
    ) -> Self {
        Self {
            payment_method_data: data.request.payment_method_data.clone(),
            browser_info: None,
            currency: data.request.currency,
            amount: Some(data.request.amount),
            split_payments: data.request.split_payments.clone(),
            customer_acceptance: data.request.customer_acceptance.clone(),
            setup_future_usage: data.request.setup_future_usage,
            setup_mandate_details: data.request.setup_mandate_details.clone(),
            mandate_id: data.request.mandate_id.clone(),
            payment_method_type: data.payment_method_type,
        }
    }
}

impl TryFrom<PaymentsAuthorizeData> for PaymentMethodTokenizationData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            browser_info: data.browser_info,
            currency: data.currency,
            amount: Some(data.amount),
            split_payments: data.split_payments.clone(),
            customer_acceptance: data.customer_acceptance,
            setup_future_usage: data.setup_future_usage,
            setup_mandate_details: data.setup_mandate_details,
            mandate_id: data.mandate_id,
            payment_method_type: data.payment_method_type,
        })
    }
}

impl TryFrom<CompleteAuthorizeData> for PaymentMethodTokenizationData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: CompleteAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data
                .payment_method_data
                .get_required_value("payment_method_data")
                .change_context(ApiErrorResponse::MissingRequiredField {
                    field_name: "payment_method_data",
                })?,
            browser_info: data.browser_info,
            currency: data.currency,
            amount: Some(data.amount),
            split_payments: None,
            customer_acceptance: data.customer_acceptance,
            setup_future_usage: data.setup_future_usage,
            setup_mandate_details: data.setup_mandate_details,
            mandate_id: data.mandate_id,
            payment_method_type: data.payment_method_type,
        })
    }
}

impl TryFrom<ExternalVaultProxyPaymentsData> for PaymentMethodTokenizationData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(_data: ExternalVaultProxyPaymentsData) -> Result<Self, Self::Error> {
        // TODO: External vault proxy payments should not use regular payment method tokenization
        // This needs to be implemented separately for external vault flows
        Err(ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason(
                "External vault proxy tokenization not implemented".to_string(),
            ),
        }
        .into())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateOrderRequestData {
    pub minor_amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub payment_method_data: Option<PaymentMethodData>,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
}

impl TryFrom<PaymentsAuthorizeData> for CreateOrderRequestData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            minor_amount: data.minor_amount,
            currency: data.currency,
            payment_method_data: Some(data.payment_method_data),
            order_details: data.order_details,
        })
    }
}

impl TryFrom<ExternalVaultProxyPaymentsData> for CreateOrderRequestData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: ExternalVaultProxyPaymentsData) -> Result<Self, Self::Error> {
        Ok(Self {
            minor_amount: data.minor_amount,
            currency: data.currency,
            payment_method_data: None,
            order_details: data.order_details,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentsPreProcessingData {
    pub payment_method_data: Option<PaymentMethodData>,
    pub amount: Option<i64>,
    pub email: Option<pii::Email>,
    pub currency: Option<storage_enums::Currency>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub router_return_url: Option<String>,
    pub webhook_url: Option<String>,
    pub complete_authorize_url: Option<String>,
    pub surcharge_details: Option<SurchargeDetails>,
    pub browser_info: Option<BrowserInformation>,
    pub connector_transaction_id: Option<String>,
    pub enrolled_for_3ds: bool,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub related_transaction_id: Option<String>,
    pub redirect_response: Option<CompleteAuthorizeRedirectResponse>,
    pub metadata: Option<Secret<serde_json::Value>>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    // New amount for amount frame work
    pub minor_amount: Option<MinorUnit>,
    pub is_stored_credential: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GiftCardBalanceCheckRequestData {
    pub payment_method_data: PaymentMethodData,
    pub currency: Option<storage_enums::Currency>,
    pub minor_amount: Option<MinorUnit>,
}

impl TryFrom<PaymentsAuthorizeData> for PaymentsPreProcessingData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: Some(data.payment_method_data),
            amount: Some(data.amount),
            minor_amount: Some(data.minor_amount),
            email: data.email,
            currency: Some(data.currency),
            payment_method_type: data.payment_method_type,
            setup_mandate_details: data.setup_mandate_details,
            capture_method: data.capture_method,
            order_details: data.order_details,
            router_return_url: data.router_return_url,
            webhook_url: data.webhook_url,
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            surcharge_details: data.surcharge_details,
            connector_transaction_id: None,
            mandate_id: data.mandate_id,
            related_transaction_id: data.related_transaction_id,
            redirect_response: None,
            enrolled_for_3ds: data.enrolled_for_3ds,
            split_payments: data.split_payments,
            metadata: data.metadata.map(Secret::new),
            customer_acceptance: data.customer_acceptance,
            setup_future_usage: data.setup_future_usage,
            is_stored_credential: data.is_stored_credential,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentsPreAuthenticateData {
    pub payment_method_data: PaymentMethodData,
    pub amount: i64,
    pub email: Option<pii::Email>,
    pub currency: Option<storage_enums::Currency>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub router_return_url: Option<String>,
    pub complete_authorize_url: Option<String>,
    pub browser_info: Option<BrowserInformation>,
    pub enrolled_for_3ds: bool,
    pub customer_name: Option<Secret<String>>,
    pub metadata: Option<pii::SecretSerdeValue>,
    // New amount for amount frame work
    pub minor_amount: MinorUnit,
}

impl TryFrom<PaymentsAuthorizeData> for PaymentsPreAuthenticateData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            customer_name: data.customer_name,
            metadata: data.metadata.map(Secret::new),
            amount: data.amount,
            minor_amount: data.minor_amount,
            email: data.email,
            currency: Some(data.currency),
            payment_method_type: data.payment_method_type,
            router_return_url: data.router_return_url,
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            enrolled_for_3ds: data.enrolled_for_3ds,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentsAuthenticateData {
    pub payment_method_data: Option<PaymentMethodData>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub amount: Option<i64>,
    pub email: Option<pii::Email>,
    pub currency: Option<storage_enums::Currency>,
    pub complete_authorize_url: Option<String>,
    pub browser_info: Option<BrowserInformation>,
    pub redirect_response: Option<CompleteAuthorizeRedirectResponse>,
    pub minor_amount: Option<MinorUnit>,
}

impl TryFrom<CompleteAuthorizeData> for PaymentsAuthenticateData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: CompleteAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            payment_method_type: data.payment_method_type,
            amount: Some(data.amount),
            minor_amount: Some(data.minor_amount),
            email: data.email,
            currency: Some(data.currency),
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            redirect_response: data.redirect_response,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentsPostAuthenticateData {
    pub payment_method_data: Option<PaymentMethodData>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub amount: Option<i64>,
    pub email: Option<pii::Email>,
    pub currency: Option<storage_enums::Currency>,
    pub browser_info: Option<BrowserInformation>,
    pub connector_transaction_id: Option<String>,
    pub redirect_response: Option<CompleteAuthorizeRedirectResponse>,
    // New amount for amount frame work
    pub minor_amount: Option<MinorUnit>,
}

impl TryFrom<CompleteAuthorizeData> for PaymentsPostAuthenticateData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: CompleteAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_type: data.payment_method_type,
            payment_method_data: data.payment_method_data,
            amount: Some(data.amount),
            minor_amount: Some(data.minor_amount),
            email: data.email,
            currency: Some(data.currency),
            browser_info: data.browser_info,
            connector_transaction_id: None,
            redirect_response: data.redirect_response,
        })
    }
}

impl TryFrom<CompleteAuthorizeData> for PaymentsPreProcessingData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: CompleteAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            amount: Some(data.amount),
            minor_amount: Some(data.minor_amount),
            email: data.email,
            currency: Some(data.currency),
            payment_method_type: None,
            setup_mandate_details: data.setup_mandate_details,
            capture_method: data.capture_method,
            order_details: None,
            router_return_url: None,
            webhook_url: None,
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            surcharge_details: None,
            connector_transaction_id: data.connector_transaction_id,
            mandate_id: data.mandate_id,
            related_transaction_id: None,
            redirect_response: data.redirect_response,
            split_payments: None,
            enrolled_for_3ds: true,
            metadata: data.connector_meta.map(Secret::new),
            customer_acceptance: data.customer_acceptance,
            setup_future_usage: data.setup_future_usage,
            is_stored_credential: data.is_stored_credential,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentsPostProcessingData {
    pub payment_method_data: PaymentMethodData,
    pub customer_id: Option<id_type::CustomerId>,
    pub connector_transaction_id: Option<String>,
    pub country: Option<common_enums::CountryAlpha2>,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub header_payload: Option<payments::HeaderPayload>,
}

impl<F> TryFrom<RouterData<F, PaymentsAuthorizeData, response_types::PaymentsResponseData>>
    for PaymentsPostProcessingData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(
        data: RouterData<F, PaymentsAuthorizeData, response_types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.request.payment_method_data,
            connector_transaction_id: match data.response {
                Ok(response_types::PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(id),
                    ..
                }) => Some(id.clone()),
                _ => None,
            },
            customer_id: data.request.customer_id,
            country: data
                .address
                .get_payment_billing()
                .and_then(|bl| bl.address.as_ref())
                .and_then(|address| address.country),
            connector_meta_data: data.connector_meta_data.clone(),
            header_payload: data.header_payload,
        })
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct CompleteAuthorizeData {
    pub payment_method_data: Option<PaymentMethodData>,
    pub amount: i64,
    pub email: Option<pii::Email>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub redirect_response: Option<CompleteAuthorizeRedirectResponse>,
    pub browser_info: Option<BrowserInformation>,
    pub connector_transaction_id: Option<String>,
    pub connector_meta: Option<serde_json::Value>,
    pub complete_authorize_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    pub authentication_data: Option<UcsAuthenticationData>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    // New amount for amount frame work
    pub minor_amount: MinorUnit,
    pub merchant_account_id: Option<Secret<String>>,
    pub merchant_config_currency: Option<storage_enums::Currency>,
    pub threeds_method_comp_ind: Option<api_models::payments::ThreeDsCompletionIndicator>,
    pub is_stored_credential: Option<bool>,
    pub tokenization: Option<common_enums::Tokenization>,
    pub router_return_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompleteAuthorizeRedirectResponse {
    pub params: Option<Secret<String>>,
    pub payload: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PaymentsSyncData {
    //TODO : add fields based on the connector requirements
    pub connector_transaction_id: ResponseId,
    pub encoded_data: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub connector_meta: Option<serde_json::Value>,
    pub sync_type: SyncRequestType,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub currency: storage_enums::Currency,
    pub payment_experience: Option<common_enums::PaymentExperience>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,
    pub amount: MinorUnit,
    pub integrity_object: Option<SyncIntegrityObject>,
    pub connector_reference_id: Option<String>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub enum SyncRequestType {
    MultipleCaptureSync(Vec<String>),
    #[default]
    SinglePaymentSync,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PaymentsCancelData {
    pub amount: Option<i64>,
    pub currency: Option<storage_enums::Currency>,
    pub connector_transaction_id: String,
    pub cancellation_reason: Option<String>,
    pub connector_meta: Option<serde_json::Value>,
    pub browser_info: Option<BrowserInformation>,
    pub metadata: Option<serde_json::Value>,
    // This metadata is used to store the metadata shared during the payment intent request.

    // minor amount data for amount framework
    pub minor_amount: Option<MinorUnit>,
    pub webhook_url: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PaymentsCancelPostCaptureData {
    pub currency: Option<storage_enums::Currency>,
    pub connector_transaction_id: String,
    pub cancellation_reason: Option<String>,
    pub connector_meta: Option<serde_json::Value>,
    // minor amount data for amount framework
    pub minor_amount: Option<MinorUnit>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PaymentsExtendAuthorizationData {
    pub minor_amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub connector_transaction_id: String,
    pub connector_meta: Option<serde_json::Value>,
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsRejectData {
    pub amount: Option<i64>,
    pub currency: Option<storage_enums::Currency>,
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsApproveData {
    pub amount: Option<i64>,
    pub currency: Option<storage_enums::Currency>,
}

#[derive(Clone, Debug, Default, Serialize, serde::Deserialize)]
pub struct BrowserInformation {
    pub color_depth: Option<u8>,
    pub java_enabled: Option<bool>,
    pub java_script_enabled: Option<bool>,
    pub language: Option<String>,
    pub screen_height: Option<u32>,
    pub screen_width: Option<u32>,
    pub time_zone: Option<i32>,
    pub ip_address: Option<std::net::IpAddr>,
    pub accept_header: Option<String>,
    pub user_agent: Option<String>,
    pub os_type: Option<String>,
    pub os_version: Option<String>,
    pub device_model: Option<String>,
    pub accept_language: Option<String>,
    pub referer: Option<String>,
}

#[cfg(feature = "v2")]
impl From<common_utils::types::BrowserInformation> for BrowserInformation {
    fn from(value: common_utils::types::BrowserInformation) -> Self {
        Self {
            color_depth: value.color_depth,
            java_enabled: value.java_enabled,
            java_script_enabled: value.java_script_enabled,
            language: value.language,
            screen_height: value.screen_height,
            screen_width: value.screen_width,
            time_zone: value.time_zone,
            ip_address: value.ip_address,
            accept_header: value.accept_header,
            user_agent: value.user_agent,
            os_type: value.os_type,
            os_version: value.os_version,
            device_model: value.device_model,
            accept_language: value.accept_language,
            referer: value.referer,
        }
    }
}

#[cfg(feature = "v1")]
impl From<api_models::payments::BrowserInformation> for BrowserInformation {
    fn from(value: api_models::payments::BrowserInformation) -> Self {
        Self {
            color_depth: value.color_depth,
            java_enabled: value.java_enabled,
            java_script_enabled: value.java_script_enabled,
            language: value.language,
            screen_height: value.screen_height,
            screen_width: value.screen_width,
            time_zone: value.time_zone,
            ip_address: value.ip_address,
            accept_header: value.accept_header,
            user_agent: value.user_agent,
            os_type: value.os_type,
            os_version: value.os_version,
            device_model: value.device_model,
            accept_language: value.accept_language,
            referer: value.referer,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub enum ResponseId {
    ConnectorTransactionId(String),
    EncodedData(String),
    #[default]
    NoResponseId,
}
impl ResponseId {
    pub fn get_connector_transaction_id(
        &self,
    ) -> errors::CustomResult<String, errors::ValidationError> {
        match self {
            Self::ConnectorTransactionId(txn_id) => Ok(txn_id.to_string()),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "connector_transaction_id",
            })
            .attach_printable("Expected connector transaction ID not found"),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, Serialize)]
pub struct SurchargeDetails {
    /// original_amount
    pub original_amount: MinorUnit,
    /// surcharge value
    pub surcharge: common_utils::types::Surcharge,
    /// tax on surcharge value
    pub tax_on_surcharge:
        Option<common_utils::types::Percentage<{ consts::SURCHARGE_PERCENTAGE_PRECISION_LENGTH }>>,
    /// surcharge amount for this payment
    pub surcharge_amount: MinorUnit,
    /// tax on surcharge amount for this payment
    pub tax_on_surcharge_amount: MinorUnit,
}

impl SurchargeDetails {
    pub fn get_total_surcharge_amount(&self) -> MinorUnit {
        self.surcharge_amount + self.tax_on_surcharge_amount
    }
}

#[cfg(feature = "v1")]
impl
    From<(
        &RequestSurchargeDetails,
        &payments::payment_attempt::PaymentAttempt,
    )> for SurchargeDetails
{
    fn from(
        (request_surcharge_details, payment_attempt): (
            &RequestSurchargeDetails,
            &payments::payment_attempt::PaymentAttempt,
        ),
    ) -> Self {
        let surcharge_amount = request_surcharge_details.surcharge_amount;
        let tax_on_surcharge_amount = request_surcharge_details.tax_amount.unwrap_or_default();
        Self {
            original_amount: payment_attempt.net_amount.get_order_amount(),
            surcharge: common_utils::types::Surcharge::Fixed(
                request_surcharge_details.surcharge_amount,
            ),
            tax_on_surcharge: None,
            surcharge_amount,
            tax_on_surcharge_amount,
        }
    }
}

#[cfg(feature = "v2")]
impl
    From<(
        &RequestSurchargeDetails,
        &payments::payment_attempt::PaymentAttempt,
    )> for SurchargeDetails
{
    fn from(
        (_request_surcharge_details, _payment_attempt): (
            &RequestSurchargeDetails,
            &payments::payment_attempt::PaymentAttempt,
        ),
    ) -> Self {
        todo!()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UcsAuthenticationData {
    pub eci: Option<String>,
    pub cavv: Option<Secret<String>>,
    pub threeds_server_transaction_id: Option<String>,
    pub message_version: Option<SemanticVersion>,
    pub ds_trans_id: Option<String>,
    pub acs_trans_id: Option<String>,
    pub trans_status: Option<common_enums::TransactionStatus>,
    pub transaction_id: Option<String>,
    pub ucaf_collection_indicator: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthenticationData {
    pub eci: Option<String>,
    pub cavv: Secret<String>,
    pub threeds_server_transaction_id: Option<String>,
    pub message_version: Option<SemanticVersion>,
    pub ds_trans_id: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub challenge_code: Option<String>,
    pub challenge_cancel: Option<String>,
    pub challenge_code_reason: Option<String>,
    pub message_extension: Option<pii::SecretSerdeValue>,
    pub acs_trans_id: Option<String>,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub transaction_status: Option<common_enums::TransactionStatus>,
    pub cb_network_params: Option<api_models::payments::NetworkParams>,
    pub exemption_indicator: Option<common_enums::ExemptionIndicator>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RefundsData {
    pub refund_id: String,
    pub connector_transaction_id: String,

    pub connector_refund_id: Option<String>,
    pub currency: storage_enums::Currency,
    /// Amount for the payment against which this refund is issued
    pub payment_amount: i64,

    pub reason: Option<String>,
    pub webhook_url: Option<String>,
    /// Amount to be refunded
    pub refund_amount: i64,
    /// Arbitrary metadata required for refund
    pub connector_metadata: Option<serde_json::Value>,
    /// refund method
    pub refund_connector_metadata: Option<pii::SecretSerdeValue>,
    pub browser_info: Option<BrowserInformation>,
    /// Charges associated with the payment
    pub split_refunds: Option<SplitRefundsRequest>,

    // New amount for amount frame work
    pub minor_payment_amount: MinorUnit,
    pub minor_refund_amount: MinorUnit,
    pub integrity_object: Option<RefundIntegrityObject>,
    pub refund_status: storage_enums::RefundStatus,
    pub merchant_account_id: Option<Secret<String>>,
    pub merchant_config_currency: Option<storage_enums::Currency>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub additional_payment_method_data: Option<AdditionalPaymentData>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RefundIntegrityObject {
    /// refund currency
    pub currency: storage_enums::Currency,
    /// refund amount
    pub refund_amount: MinorUnit,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub enum SplitRefundsRequest {
    StripeSplitRefund(StripeSplitRefund),
    AdyenSplitRefund(common_types::domain::AdyenSplitData),
    XenditSplitRefund(common_types::domain::XenditSplitSubMerchantData),
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct StripeSplitRefund {
    pub charge_id: String,
    pub transfer_account_id: String,
    pub charge_type: api_models::enums::PaymentChargeType,
    pub options: ChargeRefundsOptions,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct ChargeRefunds {
    pub charge_id: String,
    pub transfer_account_id: String,
    pub charge_type: api_models::enums::PaymentChargeType,
    pub options: ChargeRefundsOptions,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, Serialize)]
pub enum ChargeRefundsOptions {
    Destination(DestinationChargeRefund),
    Direct(DirectChargeRefund),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, Serialize)]
pub struct DirectChargeRefund {
    pub revert_platform_fee: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, Serialize)]
pub struct DestinationChargeRefund {
    pub revert_platform_fee: bool,
    pub revert_transfer: bool,
}

#[derive(Debug, Clone)]
pub struct AccessTokenAuthenticationRequestData {
    pub auth_creds: router_data::ConnectorAuthType,
}

impl TryFrom<router_data::ConnectorAuthType> for AccessTokenAuthenticationRequestData {
    type Error = ApiErrorResponse;
    fn try_from(connector_auth: router_data::ConnectorAuthType) -> Result<Self, Self::Error> {
        Ok(Self {
            auth_creds: connector_auth,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AccessTokenRequestData {
    pub app_id: Secret<String>,
    pub id: Option<Secret<String>>,
    pub authentication_token: Option<AccessTokenAuthenticationResponse>,
    // Add more keys if required
}

// This is for backward compatibility
impl TryFrom<router_data::ConnectorAuthType> for AccessTokenRequestData {
    type Error = ApiErrorResponse;
    fn try_from(connector_auth: router_data::ConnectorAuthType) -> Result<Self, Self::Error> {
        match connector_auth {
            router_data::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                app_id: api_key,
                id: None,
                authentication_token: None,
            }),
            router_data::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
                authentication_token: None,
            }),
            router_data::ConnectorAuthType::SignatureKey { api_key, key1, .. } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
                authentication_token: None,
            }),
            router_data::ConnectorAuthType::MultiAuthKey { api_key, key1, .. } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
                authentication_token: None,
            }),

            _ => Err(ApiErrorResponse::InvalidDataValue {
                field_name: "connector_account_details",
            }),
        }
    }
}

impl
    TryFrom<(
        router_data::ConnectorAuthType,
        Option<AccessTokenAuthenticationResponse>,
    )> for AccessTokenRequestData
{
    type Error = ApiErrorResponse;
    fn try_from(
        (connector_auth, authentication_token): (
            router_data::ConnectorAuthType,
            Option<AccessTokenAuthenticationResponse>,
        ),
    ) -> Result<Self, Self::Error> {
        let mut access_token_request_data = Self::try_from(connector_auth)?;
        access_token_request_data.authentication_token = authentication_token;
        Ok(access_token_request_data)
    }
}

#[derive(Default, Debug, Clone)]
pub struct AcceptDisputeRequestData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
    pub dispute_status: storage_enums::DisputeStatus,
}

#[derive(Default, Debug, Clone)]
pub struct DefendDisputeRequestData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
}

#[derive(Default, Debug, Clone)]
pub struct SubmitEvidenceRequestData {
    pub dispute_id: String,
    pub dispute_status: storage_enums::DisputeStatus,
    pub connector_dispute_id: String,
    pub access_activity_log: Option<String>,
    pub billing_address: Option<String>,
    //cancellation policy
    pub cancellation_policy: Option<Vec<u8>>,
    pub cancellation_policy_file_type: Option<String>,
    pub cancellation_policy_provider_file_id: Option<String>,
    pub cancellation_policy_disclosure: Option<String>,
    pub cancellation_rebuttal: Option<String>,
    //customer communication
    pub customer_communication: Option<Vec<u8>>,
    pub customer_communication_file_type: Option<String>,
    pub customer_communication_provider_file_id: Option<String>,
    pub customer_email_address: Option<String>,
    pub customer_name: Option<String>,
    pub customer_purchase_ip: Option<String>,
    //customer signature
    pub customer_signature: Option<Vec<u8>>,
    pub customer_signature_file_type: Option<String>,
    pub customer_signature_provider_file_id: Option<String>,
    //product description
    pub product_description: Option<String>,
    //receipts
    pub receipt: Option<Vec<u8>>,
    pub receipt_file_type: Option<String>,
    pub receipt_provider_file_id: Option<String>,
    //refund policy
    pub refund_policy: Option<Vec<u8>>,
    pub refund_policy_file_type: Option<String>,
    pub refund_policy_provider_file_id: Option<String>,
    pub refund_policy_disclosure: Option<String>,
    pub refund_refusal_explanation: Option<String>,
    //service docs
    pub service_date: Option<String>,
    pub service_documentation: Option<Vec<u8>>,
    pub service_documentation_file_type: Option<String>,
    pub service_documentation_provider_file_id: Option<String>,
    //shipping details docs
    pub shipping_address: Option<String>,
    pub shipping_carrier: Option<String>,
    pub shipping_date: Option<String>,
    pub shipping_documentation: Option<Vec<u8>>,
    pub shipping_documentation_file_type: Option<String>,
    pub shipping_documentation_provider_file_id: Option<String>,
    pub shipping_tracking_number: Option<String>,
    //invoice details
    pub invoice_showing_distinct_transactions: Option<Vec<u8>>,
    pub invoice_showing_distinct_transactions_file_type: Option<String>,
    pub invoice_showing_distinct_transactions_provider_file_id: Option<String>,
    //subscription details
    pub recurring_transaction_agreement: Option<Vec<u8>>,
    pub recurring_transaction_agreement_file_type: Option<String>,
    pub recurring_transaction_agreement_provider_file_id: Option<String>,
    //uncategorized details
    pub uncategorized_file: Option<Vec<u8>>,
    pub uncategorized_file_type: Option<String>,
    pub uncategorized_file_provider_file_id: Option<String>,
    pub uncategorized_text: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct FetchDisputesRequestData {
    pub created_from: time::PrimitiveDateTime,
    pub created_till: time::PrimitiveDateTime,
}

#[derive(Clone, Debug)]
pub struct RetrieveFileRequestData {
    pub provider_file_id: String,
    pub connector_dispute_id: Option<String>,
}

#[serde_as]
#[derive(Clone, Debug, Serialize)]
pub struct UploadFileRequestData {
    pub file_key: String,
    #[serde(skip)]
    pub file: Vec<u8>,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub file_type: mime::Mime,
    pub file_size: i32,
    pub dispute_id: String,
    pub connector_dispute_id: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PayoutsData {
    pub payout_id: id_type::PayoutId,
    pub amount: i64,
    pub connector_payout_id: Option<String>,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub payout_type: Option<storage_enums::PayoutType>,
    pub entity_type: storage_enums::PayoutEntityType,
    pub customer_details: Option<CustomerDetails>,
    pub vendor_details: Option<api_models::payouts::PayoutVendorAccountDetails>,

    // New minor amount for amount framework
    pub minor_amount: MinorUnit,
    pub priority: Option<storage_enums::PayoutSendPriority>,
    pub connector_transfer_method_id: Option<String>,
    pub webhook_url: Option<String>,
    pub browser_info: Option<BrowserInformation>,
    pub payout_connector_metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, Default, Clone)]
pub struct CustomerDetails {
    pub customer_id: Option<id_type::CustomerId>,
    pub name: Option<Secret<String, masking::WithType>>,
    pub email: Option<pii::Email>,
    pub phone: Option<Secret<String, masking::WithType>>,
    pub phone_country_code: Option<String>,
    pub tax_registration_id: Option<Secret<String, masking::WithType>>,
}

#[derive(Debug, Clone)]
pub struct VerifyWebhookSourceRequestData {
    pub webhook_headers: actix_web::http::header::HeaderMap,
    pub webhook_body: Vec<u8>,
    pub merchant_secret: api_models::webhooks::ConnectorWebhookSecrets,
}

#[derive(Debug, Clone)]
pub struct MandateRevokeRequestData {
    pub mandate_id: String,
    pub connector_mandate_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentsSessionData {
    pub amount: i64,
    pub currency: common_enums::Currency,
    pub country: Option<common_enums::CountryAlpha2>,
    pub surcharge_details: Option<SurchargeDetails>,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub email: Option<pii::Email>,
    // Minor Unit amount for amount frame work
    pub minor_amount: MinorUnit,
    pub apple_pay_recurring_details: Option<api_models::payments::ApplePayRecurringPaymentRequest>,
    pub customer_name: Option<Secret<String>>,
    pub order_tax_amount: Option<MinorUnit>,
    pub shipping_cost: Option<MinorUnit>,
    pub metadata: Option<Secret<serde_json::Value>>,
    /// The specific payment method type for which the session token is being generated
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub payment_method: Option<common_enums::PaymentMethod>,
}

#[derive(Debug, Clone, Default)]
pub struct PaymentsTaxCalculationData {
    pub amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub shipping_cost: Option<MinorUnit>,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub shipping_address: address::Address,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SdkPaymentsSessionUpdateData {
    pub order_tax_amount: MinorUnit,
    // amount here would include amount, surcharge_amount, order_tax_amount and shipping_cost
    pub amount: MinorUnit,
    /// original amount sent by the merchant
    pub order_amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub session_id: Option<String>,
    pub shipping_cost: Option<MinorUnit>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupMandateRequestData {
    pub currency: storage_enums::Currency,
    pub payment_method_data: PaymentMethodData,
    pub amount: Option<i64>,
    pub confirm: bool,
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub router_return_url: Option<String>,
    pub webhook_url: Option<String>,
    pub browser_info: Option<BrowserInformation>,
    pub email: Option<pii::Email>,
    pub customer_name: Option<Secret<String>>,
    pub return_url: Option<String>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub request_incremental_authorization: bool,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub complete_authorize_url: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub enrolled_for_3ds: bool,
    pub related_transaction_id: Option<String>,

    // MinorUnit for amount framework
    pub minor_amount: Option<MinorUnit>,
    pub shipping_cost: Option<MinorUnit>,
    pub connector_testing_data: Option<pii::SecretSerdeValue>,
    pub customer_id: Option<id_type::CustomerId>,
    pub enable_partial_authorization:
        Option<common_types::primitive_wrappers::EnablePartialAuthorizationBool>,
    pub payment_channel: Option<storage_enums::PaymentChannel>,
    pub is_stored_credential: Option<bool>,
    pub billing_descriptor: Option<common_types::payments::BillingDescriptor>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,
    pub tokenization: Option<common_enums::Tokenization>,
    pub partner_merchant_identifier_details:
        Option<common_types::payments::PartnerMerchantIdentifierDetails>,
}

#[derive(Debug, Clone)]
pub struct VaultRequestData {
    pub payment_method_vaulting_data: Option<PaymentMethodCustomVaultingData>,
    pub connector_vault_id: Option<String>,
    pub connector_customer_id: Option<String>,
    pub should_generate_multiple_tokens: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DisputeSyncData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
}
