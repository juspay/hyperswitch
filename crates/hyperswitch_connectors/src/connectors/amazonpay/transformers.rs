use std::collections::HashMap;

use common_enums::{enums, CaptureMethod};
use common_utils::{errors::CustomResult, types::StringMajorUnit};
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

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
#[serde(rename_all = "camelCase")]
pub struct AmazonpayFinalizeRequest {
    charge_amount: ChargeAmount,
    total_order_amount: Option<TotalOrderAmount>,
    shipping_address: AddressDetails,
    payment_intent: PaymentIntent,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChargeAmount {
    amount: StringMajorUnit,
    currency_code: common_enums::Currency,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TotalOrderAmount {
    amount: Option<StringMajorUnit>,
    currency_code: Option<common_enums::Currency>,
    can_handle_pending_authorization: Option<bool>,
    supplementary_data: Option<String>,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddressDetails {
    name: Option<String>,
    address_line_1: Option<String>,
    address_line_2: Option<String>,
    address_line_3: Option<String>,
    city: Option<String>,
    // country: Option<String>,
    // district: Option<String>,
    state_or_region: Option<String>,
    postal_code: Option<String>,
    country_code: Option<common_enums::CountryAlpha2>,
    phone_number: Option<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub enum PaymentIntent {
    Authorize,
    #[default]
    AuthorizeWithCapture,
}

fn get_amazonpay_capture_type(
    item: Option<CaptureMethod>,
) -> CustomResult<Option<PaymentIntent>, errors::ConnectorError> {
    match item {
        Some(CaptureMethod::Manual) => Ok(Some(PaymentIntent::Authorize)),
        Some(CaptureMethod::Automatic) | None => Ok(Some(PaymentIntent::AuthorizeWithCapture)),
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
        let charge_amount = ChargeAmount {
            amount: item.amount.clone(),
            currency_code: common_enums::Currency::USD,
        };
        let shipping_address_details = item.router_data.address.get_shipping();
        let shipping_address = if let Some(shipping) = shipping_address_details {
            if let Some(address_details) = shipping.address.as_ref() {
                AddressDetails {
                    name: address_details
                        .get_optional_full_name()
                        .map(|secret_name| secret_name.peek().to_string()),
                    address_line_1: address_details
                        .line1
                        .clone()
                        .map(|l1| l1.peek().to_string()),
                    address_line_2: address_details
                        .line2
                        .clone()
                        .map(|l2| l2.peek().to_string()),
                    address_line_3: address_details
                        .line3
                        .clone()
                        .map(|l3| l3.peek().to_string()),
                    city: address_details.city.clone(),
                    // country: address_details.country.map(|country| country.to_string()),
                    // district: None, // If no specific field is available, set to None
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
                        .map(|phone_number| phone_number.peek().to_string()),
                }
            } else {
                AddressDetails::default()
            }
        } else {
            AddressDetails::default()
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

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayFinalizeResponse {
    checkout_session_id: String,
    web_checkout_details: WebCheckoutDetails,
    product_type: Option<String>,
    payment_details: Option<PaymentDetails>,
    cart_details: CartDetails,
    charge_permission_type: String,
    order_type: Option<String>,
    recurring_metadata: Option<RecurringMetadata>,
    payment_method_on_file_metadata: Option<String>,
    processor_specifications: Option<String>,
    merchant_details: Option<String>,
    merchant_metadata: Option<MerchantMetadata>,
    supplementary_data: Option<String>,
    buyer: Option<BuyerDetails>,
    billing_address: Option<AddressDetails>,
    payment_preferences: Option<String>,
    status_details: FinalizeStatusDetails,
    shipping_address: Option<AddressDetails>,
    platform_id: Option<String>,
    charge_permission_id: String,
    charge_id: String,
    constraints: Option<String>,
    creation_timestamp: String,
    expiration_timestamp: Option<String>,
    store_id: Option<String>,
    provider_metadata: Option<ProviderMetadata>,
    release_environment: Option<ReleaseEnvironment>,
    checkout_button_text: Option<String>,
    delivery_specifications: Option<DeliverySpecifications>,
    tokens: Option<String>,
    disbursement_details: Option<String>,
    channel_type: Option<String>,
    payment_processing_meta_data: PaymentProcessingMetaData,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebCheckoutDetails {
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

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDetails {
    payment_intent: String,
    can_handle_pending_authorization: bool,
    charge_amount: ChargeAmount,
    total_order_amount: ChargeAmount, // have to see
    presentment_currency: String,
    soft_descriptor: String,
    allow_overcharge: bool,
    extend_expiration: bool,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CartDetails {
    line_items: Vec<String>,
    delivery_options: Vec<DeliveryOptions>,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryOptions {
    id: String,
    price: ChargeAmount,
    shipping_method: ShippingMethod,
    is_default: bool,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShippingMethod {
    shipping_method_name: String,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecurringMetadata {
    frequency: Frequency,
    amount: ChargeAmount,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Frequency {
    unit: String,
    value: String,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BuyerDetails {
    buyer_id: String,
    name: String,
    email: String,
    phone_number: String,
    prime_membership_types: Vec<String>,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeStatusDetails {
    state: FinalizeState,
    reason_code: Option<String>,
    reason_description: Option<String>,
    last_updated_timestamp: String,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
pub enum FinalizeState {
    #[default]
    Open,
    Completed,
    Canceled,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySpecifications {
    special_restrictions: Vec<String>,
    address_restrictions: AddressRestrictions,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddressRestrictions {
    r#type: String,
    restrictions: HashMap<String, Restriction>,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Restriction {
    pub states_or_regions: Vec<String>,
    pub zip_codes: Vec<String>,
}

#[derive(Default, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentProcessingMetaData {
    payment_processing_model: String,
}

impl From<FinalizeState> for common_enums::AttemptStatus {
    fn from(item: FinalizeState) -> Self {
        match item {
            FinalizeState::Open => Self::Pending,
            FinalizeState::Completed => Self::Charged,
            FinalizeState::Canceled => Self::Failure,
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

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayPaymentsRequest {
    charge_amount: ChargeAmount,
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
            AmazonpayPaymentStatus::Declined => Self::CaptureFailed,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayPaymentsResponse {
    charge_id: String,
    charge_amount: ChargeAmount,
    charge_permission_id: String,
    capture_amount: Option<ChargeAmount>,
    refunded_amount: Option<ChargeAmount>,
    soft_descriptor: Option<String>,
    provider_metadata: Option<ProviderMetadata>,
    converted_amount: Option<ChargeAmount>,
    conversion_rate: Option<f64>,
    channel: Option<String>,
    charge_initiator: Option<String>,
    status_details: PaymentsStatusDetails,
    creation_timestamp: String,
    expiration_timestamp: String,
    release_environment: Option<ReleaseEnvironment>,
    merchant_metadata: Option<MerchantMetadata>,
    platform_id: Option<String>,
    web_checkout_details: Option<WebCheckoutDetails>,
    disbursement_details: Option<String>,
    payment_method: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMetadata {
    provider_reference_id: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsStatusDetails {
    state: AmazonpayPaymentStatus,
    reason_code: Option<String>,
    reason_description: Option<String>,
    last_updated_timestamp: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReleaseEnvironment {
    #[default]
    Sandbox,
    Live,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantMetadata {
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
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayRefundRequest {
    pub refund_amount: ChargeAmount,
    pub charge_id: String,
}

impl<F> TryFrom<&AmazonpayRouterData<&RefundsRouterData<F>>> for AmazonpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AmazonpayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let refund_amount = ChargeAmount {
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
    refund_amount: ChargeAmount,
    status_details: RefundStatusDetails,
    soft_descriptor: String,
    release_environment: String,
    disbursement_details: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundStatusDetails {
    state: RefundStatus,
    reason_code: Option<String>,
    reason_description: Option<String>,
    last_updated_timestamp: String,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_id,
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
                connector_refund_id: item.response.refund_id,
                refund_status: enums::RefundStatus::from(item.response.status_details.state),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayErrorResponse {
    pub reason_code: String,
    pub message: String,
}
