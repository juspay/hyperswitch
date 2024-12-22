use std::collections::HashMap;

use common_enums::{enums, CaptureMethod};
use common_utils::{errors::CustomResult, types::StringMajorUnit};
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsCompleteAuthorizeRequestData,
};

pub struct AmazonpayRouterData<T> {
    pub amount: StringMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for AmazonpayRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AmazonpayFinalizeRequest {
    charge_amount: AmazonpayChargeAmount,
    total_order_amount: Option<AmazonpayTotalOrderAmount>,
    shipping_address: AmazonpayAddressDetails,
    payment_intent: AmazonpayPaymentIntent,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmazonpayChargeAmount {
    amount: StringMajorUnit,
    currency_code: common_enums::Currency,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AmazonpayTotalOrderAmount {
    amount: Option<StringMajorUnit>,
    currency_code: Option<common_enums::Currency>,
    can_handle_pending_authorization: Option<bool>,
    supplementary_data: Option<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AmazonpayAddressDetails {
    name: Option<String>,
    address_line_1: Option<String>,
    address_line_2: Option<String>,
    address_line_3: Option<String>,
    city: Option<String>,
    country: Option<String>,
    district: Option<String>,
    state_or_region: Option<String>,
    postal_code: Option<String>,
    country_code: Option<common_enums::CountryAlpha2>,
    phone_number: Option<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub enum AmazonpayPaymentIntent {
    Authorize,
    AuthorizeWithCapture,
    #[default]
    Confirm,
}

fn get_amazonpay_capture_type(
    item: Option<CaptureMethod>,
) -> CustomResult<Option<AmazonpayPaymentIntent>, errors::ConnectorError> {
    match item {
        Some(CaptureMethod::Manual) => Ok(Some(AmazonpayPaymentIntent::Authorize)),
        Some(CaptureMethod::Automatic) => Ok(Some(AmazonpayPaymentIntent::AuthorizeWithCapture)),
        Some(CaptureMethod::SequentialAutomatic) | None => {
            Ok(Some(AmazonpayPaymentIntent::Confirm))
        }
        Some(item) => Err(errors::ConnectorError::FlowNotSupported {
            flow: item.to_string(),
            connector: "Amazonpay".to_string(),
        }
        .into()),
    }
}

impl TryFrom<&AmazonpayRouterData<&PaymentsAuthorizeRouterData>> for AmazonpayFinalizeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AmazonpayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let charge_amount = AmazonpayChargeAmount {
            amount: item.amount.clone(),
            currency_code: common_enums::Currency::USD,
        };
        let shipping_address_details = item.router_data.address.get_shipping();
        let shipping_address = if let Some(shipping) = shipping_address_details {
            if let Some(address_details) = shipping.address.as_ref() {
                AmazonpayAddressDetails {
                    name: address_details
                        .get_optional_full_name()
                        .map(|secret_name| secret_name.peek().to_string()), // Map name
                    address_line_1: address_details
                        .line1
                        .clone()
                        .map(|l1| l1.peek().to_string()), // Unwrap Secret and convert to string
                    address_line_2: address_details
                        .line2
                        .clone()
                        .map(|l2| l2.peek().to_string()),
                    address_line_3: address_details
                        .line3
                        .clone()
                        .map(|l3| l3.peek().to_string()),
                    city: address_details.city.clone(),
                    country: address_details.country.map(|country| country.to_string()), // Assuming CountryAlpha2 has a to_string implementation
                    district: None, // If no specific field is available, set to None
                    state_or_region: address_details
                        .state
                        .clone()
                        .map(|state| state.peek().to_string()),
                    postal_code: address_details
                        .zip
                        .clone()
                        .map(|zip| zip.peek().to_string()),
                    country_code: address_details.country,
                    phone_number: shipping
                        .phone
                        .as_ref()
                        .and_then(|phone| phone.number.as_ref())
                        .map(|phone_number| phone_number.peek().to_string()), // Map phone number
                }
            } else {
                AmazonpayAddressDetails::default()
            }
        } else {
            AmazonpayAddressDetails::default()
        };
        let payment_intent = get_amazonpay_capture_type(item.router_data.request.capture_method)?
            .unwrap_or_default();
        Ok(Self {
            charge_amount,
            total_order_amount: None,
            shipping_address,
            payment_intent,
        })
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayFinalizeResponse {
    checkout_session_id: String,
    web_checkout_details: AmazonpayWebCheckoutDetails,
    product_type: Option<String>,
    payment_details: AmazonpayPaymentDetails,
    cart_details: AmazonpayCartDetails,
    charge_permission_type: String,
    order_type: Option<String>,
    recurring_metadata: AmazonpayRecurringMetadata,
    payment_method_on_file_metadata: Option<String>, // not sure
    processor_specifications: Option<String>,        // not sure
    merchant_details: Option<String>,
    merchant_metadata: AmazonpayMerchantMetadata,
    supplementary_data: String,
    buyer: AmazonpayBuyerDetails,
    billing_address: AmazonpayAddressDetails,
    payment_preferences: Option<String>, // not sure
    status_details: AmazonpayFinalizeStatusDetails,
    shipping_address: AmazonpayAddressDetails,
    platform_id: Option<String>,
    charge_permission_id: String,
    charge_id: Option<String>,
    constraints: Option<String>, // not sure
    creation_timestamp: String,
    expiration_timestamp: String,
    store_id: Option<String>,
    provider_metadata: Option<AmazonpayProviderMetadata>,
    release_environment: Option<AmazonpayReleaseEnvironment>,
    checkout_button_text: Option<String>,
    delivery_specifications: AmazonpayDeliverySpecifications,
    tokens: Option<String>,               // not sure
    disbursement_details: Option<String>, // not sure
    payment_processing_meta_data: AmazonpayPaymentProcessingMetaData,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayWebCheckoutDetails {
    checkout_review_return_url: Option<String>,
    checkout_result_return_url: Option<String>,
    amazon_pay_redirect_url: Option<String>,
    authorize_result_return_url: Option<String>,
    sign_in_return_url: Option<String>,
    sign_in_cancel_url: Option<String>,
    checkout_error_url: Option<String>,
    sign_in_error_url: Option<String>,
    amazon_pay_decline_url: Option<String>,
    checkout_cancel_url: Option<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayPaymentDetails {
    payment_intent: String,
    can_handle_pending_authorization: bool,
    charge_amount: AmazonpayChargeAmount,
    total_order_amount: AmazonpayChargeAmount, // have to see
    presentment_currency: String,
    soft_descriptor: String,
    allow_overcharge: bool,
    extend_expiration: bool,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayCartDetails {
    line_items: Vec<String>,
    delivery_options: Vec<AmazonpayDeliveryOptions>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayDeliveryOptions {
    id: String,
    price: AmazonpayChargeAmount,
    shipping_method: AmazonpayShippingMethod,
    is_default: bool,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayShippingMethod {
    shipping_method_name: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayRecurringMetadata {
    frequency: Frequency,
    amount: AmazonpayChargeAmount,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Frequency {
    unit: String,
    value: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayBuyerDetails {
    buyer_id: String,
    name: String,
    email: String,
    phone_number: String,
    prime_membership_types: Vec<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayFinalizeStatusDetails {
    state: AmazonpayFinalizeState,
    reason_code: String,
    reason_description: String,
    last_updated_timestamp: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub enum AmazonpayFinalizeState {
    #[default]
    Open,
    Completed,
    Canceled,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayDeliverySpecifications {
    special_restrictions: Vec<String>,
    address_restrictions: AmazonpayAddressRestrictions,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayAddressRestrictions {
    r#type: String,
    restrictions: HashMap<String, Restriction>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Restriction {
    pub states_or_regions: Vec<String>,
    pub zip_codes: Vec<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayPaymentProcessingMetaData {
    payment_processing_model: String,
}

impl From<AmazonpayFinalizeState> for common_enums::AttemptStatus {
    fn from(item: AmazonpayFinalizeState) -> Self {
        match item {
            AmazonpayFinalizeState::Open => Self::AuthenticationPending, // or Started?
            AmazonpayFinalizeState::Completed => Self::AuthenticationSuccessful,
            AmazonpayFinalizeState::Canceled => Self::AuthenticationFailed, // or Failure?
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, AmazonpayFinalizeResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AmazonpayFinalizeResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status_details.state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.charge_permission_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AmazonpayPaymentsRequest {
    charge_amount: AmazonpayChargeAmount,
    charge_permission_id: String,
    capture_now: Option<bool>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AmazonpayCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&AmazonpayRouterData<&PaymentsCompleteAuthorizeRouterData>>
    for AmazonpayPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AmazonpayRouterData<&PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let charge_amount = AmazonpayChargeAmount {
            amount: item.amount.clone(),
            currency_code: common_enums::Currency::USD,
        };
        let charge_permission_id = item.router_data.connector_request_reference_id.clone();
        let capture_now: Option<bool> = Some(item.router_data.request.is_auto_capture()?);
        Ok(Self {
            charge_amount,
            charge_permission_id,
            capture_now,
        })
    }
}

pub struct AmazonpayAuthType {
    pub(super) public_key: Secret<String>,
    pub(super) private_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AmazonpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                public_key: api_key.to_owned(),
                private_key: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum AmazonpayPaymentStatus {
    #[default]
    AuthorizationInitiated,
    Authorized,
    Canceled,
    Captured,
    CaptureInitiated,
    Declined,
}

impl From<AmazonpayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: AmazonpayPaymentStatus) -> Self {
        match item {
            AmazonpayPaymentStatus::AuthorizationInitiated => Self::Pending,
            AmazonpayPaymentStatus::Authorized => Self::Authorized,
            AmazonpayPaymentStatus::Canceled => Self::Voided,
            AmazonpayPaymentStatus::Captured => Self::Charged,
            AmazonpayPaymentStatus::CaptureInitiated => Self::CaptureInitiated,
            AmazonpayPaymentStatus::Declined => Self::AuthorizationFailed, // handle CaptureFailed
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayPaymentsResponse {
    charge_id: String,
    charge_amount: AmazonpayChargeAmount,
    charge_permission_id: String,
    capture_amount: Option<AmazonpayChargeAmount>,
    refunded_amount: AmazonpayChargeAmount,
    soft_descriptor: Option<String>,
    provider_metadata: AmazonpayProviderMetadata,
    converted_amount: Option<AmazonpayChargeAmount>,
    conversion_rate: Option<f64>,
    channel: Option<String>, // not sure
    charge_initiator: Option<String>,
    status_details: AmazonpayPaymentsStatusDetails,
    creation_timestamp: String,
    expiration_timestamp: String,
    release_environment: AmazonpayReleaseEnvironment,
    merchant_metadata: AmazonpayMerchantMetadata,
    platform_id: Option<String>,
    web_checkout_details: AmazonpayWebCheckoutDetails,
    disburement_details: Option<String>, // not sure
    payment_method: Option<String>,      // not sure
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayProviderMetadata {
    provider_reference_id: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayPaymentsStatusDetails {
    state: AmazonpayPaymentStatus,
    reason_code: Option<String>,
    reason_description: Option<String>,
    last_updated_timestamp: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AmazonpayReleaseEnvironment {
    #[default]
    Sandbox,
    Live,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayMerchantMetadata {
    merchant_reference_id: Option<String>,
    merchant_store_name: Option<String>,
    note_to_buyer: Option<String>,
    custom_information: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, AmazonpayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AmazonpayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status_details.state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.charge_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmazonpayCaptureRequest {
    pub capture_amount: AmazonpayChargeAmount,
}

impl TryFrom<&AmazonpayRouterData<&PaymentsCaptureRouterData>> for AmazonpayCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AmazonpayRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let capture_amount = AmazonpayChargeAmount {
            amount: item.amount.clone(),
            currency_code: common_enums::Currency::USD,
        };
        Ok(Self { capture_amount })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmazonpayCancelRequest {
    pub cancellation_reason: Option<String>, // ig only String should be the datatype
}

impl TryFrom<&AmazonpayRouterData<&PaymentsCancelRouterData>> for AmazonpayCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AmazonpayRouterData<&PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        let cancellation_reason = item.router_data.request.cancellation_reason.clone();
        Ok(Self {
            cancellation_reason,
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct AmazonpayRefundRequest {
    pub refund_amount: AmazonpayChargeAmount,
    pub charge_id: String,
}

impl<F> TryFrom<&AmazonpayRouterData<&RefundsRouterData<F>>> for AmazonpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AmazonpayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let refund_amount = AmazonpayChargeAmount {
            amount: item.amount.clone(),
            currency_code: common_enums::Currency::USD,
        };
        let charge_id = item.router_data.request.connector_transaction_id.clone();
        Ok(Self {
            refund_amount,
            charge_id,
        })
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    #[default]
    RefundInitiated,
    Refunded,
    Declined,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::RefundInitiated => Self::Pending,
            RefundStatus::Refunded => Self::Success,
            RefundStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    refund_id: String,
    charge_id: String,
    creation_timestamp: String,
    refund_amount: AmazonpayChargeAmount,
    status_details: AmazonpayRefundStatusDetails,
    soft_descriptor: String,
    release_environment: String,
    disbursement_details: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayRefundStatusDetails {
    state: RefundStatus,
    reason_code: String,
    reason_description: String,
    last_updated_timestamp: String,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_id.clone(),
                refund_status: enums::RefundStatus::from(item.response.status_details.state),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status_details.state),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayErrorResponse {
    pub reason_code: String,
    pub message: String,
}
