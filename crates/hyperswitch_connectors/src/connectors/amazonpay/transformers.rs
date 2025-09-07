use std::collections::HashMap;

use common_enums::{enums, CaptureMethod};
use common_utils::{errors::CustomResult, pii, types::StringMajorUnit};
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{consts, errors};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{is_refund_failure, RouterData as _},
};

pub struct AmazonpayRouterData<T> {
    pub amount: StringMajorUnit,
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

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayFinalizeRequest {
    charge_amount: ChargeAmount,
    shipping_address: AddressDetails,
    payment_intent: PaymentIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChargeAmount {
    amount: StringMajorUnit,
    currency_code: common_enums::Currency,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddressDetails {
    name: Secret<String>,
    address_line_1: Secret<String>,
    address_line_2: Option<Secret<String>>,
    address_line_3: Option<Secret<String>>,
    city: String,
    state_or_region: Secret<String>,
    postal_code: Secret<String>,
    country_code: Option<common_enums::CountryAlpha2>,
    phone_number: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub enum PaymentIntent {
    AuthorizeWithCapture,
}

fn get_amazonpay_capture_type(
    item: Option<CaptureMethod>,
) -> CustomResult<PaymentIntent, errors::ConnectorError> {
    match item {
        Some(CaptureMethod::Automatic) | None => Ok(PaymentIntent::AuthorizeWithCapture),
        Some(_) => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
    }
}

impl TryFrom<&AmazonpayRouterData<&PaymentsAuthorizeRouterData>> for AmazonpayFinalizeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AmazonpayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let charge_amount = ChargeAmount {
            amount: item.amount.clone(),
            currency_code: item.router_data.request.currency,
        };
        let shipping_address = AddressDetails {
            name: item.router_data.get_required_shipping_full_name()?,
            address_line_1: item.router_data.get_required_shipping_line1()?,
            address_line_2: item.router_data.get_optional_shipping_line2(),
            address_line_3: item.router_data.get_optional_shipping_line3(),
            city: item.router_data.get_required_shipping_city()?,
            state_or_region: item.router_data.get_required_shipping_state()?,
            postal_code: item.router_data.get_required_shipping_zip()?,
            country_code: item.router_data.get_optional_shipping_country(),
            phone_number: item.router_data.get_required_shipping_phone_number()?,
        };
        let payment_intent = get_amazonpay_capture_type(item.router_data.request.capture_method)?;
        Ok(Self {
            charge_amount,
            shipping_address,
            payment_intent,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDetails {
    payment_intent: String,
    can_handle_pending_authorization: bool,
    charge_amount: ChargeAmount,
    total_order_amount: ChargeAmount,
    presentment_currency: String,
    soft_descriptor: String,
    allow_overcharge: bool,
    extend_expiration: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CartDetails {
    line_items: Vec<String>,
    delivery_options: Vec<DeliveryOptions>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryOptions {
    id: String,
    price: ChargeAmount,
    shipping_method: ShippingMethod,
    is_default: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShippingMethod {
    shipping_method_name: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecurringMetadata {
    frequency: Frequency,
    amount: ChargeAmount,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Frequency {
    unit: String,
    value: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BuyerDetails {
    buyer_id: Secret<String>,
    name: Secret<String>,
    email: pii::Email,
    phone_number: Secret<String>,
    prime_membership_types: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeStatusDetails {
    state: FinalizeState,
    reason_code: Option<String>,
    reason_description: Option<String>,
    last_updated_timestamp: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum FinalizeState {
    Open,
    Completed,
    Canceled,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySpecifications {
    special_restrictions: Vec<String>,
    address_restrictions: AddressRestrictions,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddressRestrictions {
    r#type: String,
    restrictions: HashMap<String, Restriction>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Restriction {
    pub states_or_regions: Vec<Secret<String>>,
    pub zip_codes: Vec<Secret<String>>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
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
        match item.response.status_details.state {
            FinalizeState::Canceled => {
                Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(ErrorResponse {
                        code: consts::NO_ERROR_CODE.to_owned(),
                        message: "Checkout was not successfully completed".to_owned(),
                        reason: Some("Checkout was not successfully completed due to buyer abandoment, payment decline, or because checkout wasn't confirmed with Finalize Checkout Session.".to_owned()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(item.response.checkout_session_id),
                        network_decline_code: None,
                        network_advice_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
            FinalizeState::Open
            | FinalizeState::Completed => {
                Ok(Self {
                    status: common_enums::AttemptStatus::from(item.response.status_details.state),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(item.response.charge_id),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(item.response.checkout_session_id),
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..item.data
                })
            }
        }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum AmazonpayPaymentStatus {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMetadata {
    provider_reference_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsStatusDetails {
    state: AmazonpayPaymentStatus,
    reason_code: Option<String>,
    reason_description: Option<String>,
    last_updated_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReleaseEnvironment {
    Sandbox,
    Live,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
        match item.response.status_details.state {
            AmazonpayPaymentStatus::Canceled => {
                Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(ErrorResponse {
                        code: consts::NO_ERROR_CODE.to_owned(),
                        message: "Charge was canceled by Amazon or by the merchant".to_owned(),
                        reason: Some("Charge was canceled due to expiration, Amazon, buyer, merchant action, or charge permission cancellation.".to_owned()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(item.response.charge_id),
                        network_decline_code: None,
                        network_advice_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
            AmazonpayPaymentStatus::Declined => {
                Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(ErrorResponse {
                        code: consts::NO_ERROR_CODE.to_owned(),
                        message: "The authorization or capture was declined".to_owned(),
                        reason: Some("Charge was declined due to soft/hard decline, Amazon rejection, or internal processing failure.".to_owned()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(item.response.charge_id),
                        network_decline_code: None,
                        network_advice_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
            _ => {
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
    }
}

#[derive(Debug, Serialize)]
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
            currency_code: item.router_data.request.currency,
        };
        let charge_id = item.router_data.request.connector_transaction_id.clone();
        Ok(Self {
            refund_amount,
            charge_id,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RefundStatus {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    refund_id: String,
    charge_id: String,
    creation_timestamp: String,
    refund_amount: ChargeAmount,
    status_details: RefundStatusDetails,
    soft_descriptor: String,
    release_environment: ReleaseEnvironment,
    disbursement_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        match item.response.status_details.state {
            RefundStatus::Declined => {
                Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(ErrorResponse {
                        code: consts::NO_ERROR_CODE.to_owned(),
                        message: "Amazon has declined the refund.".to_owned(),
                        reason: Some("Amazon has declined the refund because maximum amount has been refunded or there was some other issue.".to_owned()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(item.response.charge_id),
                        network_decline_code: None,
                        network_advice_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
            _ => {
                Ok(Self {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: item.response.refund_id,
                        refund_status: enums::RefundStatus::from(item.response.status_details.state),
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status_details.state);
        let response = if is_refund_failure(refund_status) {
            Err(ErrorResponse {
                code: consts::NO_ERROR_CODE.to_owned(),
                message: "Amazon has declined the refund.".to_owned(),
                reason: Some("Amazon has declined the refund because maximum amount has been refunded or there was some other issue.".to_owned()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.refund_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_id.to_string(),
                refund_status,
            })
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmazonpayErrorResponse {
    pub reason_code: String,
    pub message: String,
}
