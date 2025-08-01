use common_enums::{enums, CountryAlpha2, Currency};
use common_utils::{pii, request::Method, types::MinorUnit};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{PayLaterData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsCancelData, PaymentsCaptureData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, RouterData as OtherRouterData},
};
pub struct AffirmRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for AffirmRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AffirmPaymentsRequest {
    pub merchant: Merchant,
    pub items: Vec<Item>,
    pub shipping: Option<Shipping>,
    pub billing: Option<Billing>,
    pub total: MinorUnit,
    pub currency: Currency,
    pub order_id: Option<String>,
    pub metadata: Option<Metadata>,
    pub discounts: Option<Vec<Discount>>,
    pub tax_amount: Option<MinorUnit>,
    pub shipping_amount: Option<MinorUnit>,
    pub checkout_expiration: Option<String>,
    pub expiration_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AffirmCompleteAuthorizeRequest {
    pub expand: Option<String>,
    pub order_id: Option<String>,
    pub reference_id: Option<String>,
    pub transaction_id: String,
}

impl TryFrom<&PaymentsCompleteAuthorizeRouterData> for AffirmCompleteAuthorizeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        let transaction_id = item.request.connector_transaction_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_transaction_id",
            },
        )?;

        let reference_id = item.reference_id.clone();
        let order_id = item.connector_request_reference_id.clone();
        Ok(Self {
            transaction_id,
            order_id: Some(order_id),
            reference_id,
            expand: None,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Merchant {
    pub public_api_key: Secret<String>,
    pub user_confirmation_url: String,
    pub user_cancel_url: String,
    pub user_confirmation_url_action: Option<String>,
    pub use_vcn: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Item {
    pub display_name: String,
    pub sku: String,
    pub unit_price: MinorUnit,
    pub qty: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_type: Option<common_enums::ProductType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_tax_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tax_amount: Option<MinorUnit>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shipping {
    pub name: Name,
    pub address: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<pii::Email>,
}
#[derive(Debug, Serialize)]
pub struct Billing {
    pub name: Name,
    pub address: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<pii::Email>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Name {
    pub first: Option<Secret<String>>,
    pub last: Option<Secret<String>>,
    pub full: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Address {
    pub line1: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line2: Option<Secret<String>>,
    pub city: Option<String>,
    pub state: Option<Secret<String>>,
    pub zipcode: Option<Secret<String>>,
    pub country: Option<CountryAlpha2>,
}

#[derive(Debug, Serialize)]
pub struct Metadata {
    pub shipping_type: Option<String>,
    pub entity_name: Option<String>,
    pub platform_type: Option<String>,
    pub platform_version: Option<String>,
    pub platform_affirm: Option<String>,
    pub webhook_session_id: Option<String>,
    pub mode: Option<String>,
    pub customer: Option<Value>,
    pub itinerary: Option<Vec<Value>>,
    pub checkout_channel_type: Option<String>,
    #[serde(rename = "BOPIS")]
    pub bopis: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct Discount {
    pub discount_amount: MinorUnit,
    pub discount_display_name: String,
    pub discount_code: Option<String>,
}

fn get_str(key: &str, raw: &Value) -> Option<String> {
    raw.get(key).and_then(|v| v.as_str().map(|s| s.to_owned()))
}

fn get_bool(key: &str, riskdata: &Value) -> Option<bool> {
    riskdata.get(key).and_then(|v| v.as_bool())
}

fn extract_metadata(raw: &Value) -> Option<Metadata> {
    Some(Metadata {
        shipping_type: get_str("shipping_type", raw),
        entity_name: get_str("entity_name", raw),
        platform_type: get_str("platform_type", raw),
        platform_version: get_str("platform_version", raw),
        platform_affirm: get_str("platform_affirm", raw),
        webhook_session_id: get_str("webhook_session_id", raw),
        mode: get_str("mode", raw),
        customer: raw.get("customer").cloned(),
        itinerary: raw.get("itinerary").and_then(|v| v.as_array().cloned()),
        checkout_channel_type: get_str("checkout_channel_type", raw),
        bopis: get_bool("BOPIS", raw),
    })
}

impl TryFrom<&AffirmRouterData<&PaymentsAuthorizeRouterData>> for AffirmPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &AffirmRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = &item.router_data;
        let request = &router_data.request;

        let billing = item
            .router_data
            .get_optional_billing()
            .and_then(|billing_address| {
                billing_address.address.as_ref().map(|address| Billing {
                    name: Name {
                        first: address.first_name.clone(),
                        last: address.last_name.clone(),
                        full: address.get_optional_full_name(),
                    },
                    address: Address {
                        line1: address.line1.clone(),
                        line2: address.line2.clone(),
                        city: address.city.clone(),
                        state: address.state.clone(),
                        zipcode: address.zip.clone(),
                        country: address.country,
                    },
                    phone_number: billing_address
                        .phone
                        .as_ref()
                        .and_then(|phone| phone.number.as_ref().cloned()),
                    email: billing_address.email.clone(),
                })
            });

        let shipping = item
            .router_data
            .get_optional_shipping()
            .and_then(|shipping_address| {
                shipping_address.address.as_ref().map(|address| Shipping {
                    name: Name {
                        first: address.first_name.clone(),
                        last: address.last_name.clone(),
                        full: address.get_optional_full_name(),
                    },
                    address: Address {
                        line1: address.line1.clone(),
                        line2: address.line2.clone(),
                        city: address.city.clone(),
                        state: address.state.clone(),
                        zipcode: address.zip.clone(),
                        country: address.country,
                    },
                    phone_number: shipping_address
                        .phone
                        .as_ref()
                        .and_then(|phone| phone.number.as_ref().cloned()),
                    email: shipping_address.email.clone(),
                })
            });

        match request.payment_method_data.clone() {
            PaymentMethodData::PayLater(PayLaterData::AffirmRedirect {}) => {
                let items = match request.order_details.clone() {
                    Some(order_details) => order_details
                        .iter()
                        .map(|data| {
                            Ok(Item {
                                display_name: data.product_name.clone(),
                                sku: data.product_id.clone().unwrap_or_default(),
                                unit_price: data.amount,
                                qty: data.quantity.into(),
                                item_image_url: data.product_img_link.clone(),
                                item_url: None,
                                category: data.category.clone(),
                                sub_category: data.sub_category.clone(),
                                brand: data.brand.clone(),
                                product_type: data.product_type.clone(),
                                product_tax_code: data.product_tax_code.clone(),
                                tax_rate: data.tax_rate,
                                total_tax_amount: data.total_tax_amount,
                            })
                        })
                        .collect::<Result<Vec<_>, _>>(),
                    None => Err(report!(errors::ConnectorError::MissingRequiredField {
                        field_name: "order_details",
                    })),
                }?;

                let metadata: Option<Metadata> = item
                    .router_data
                    .request
                    .metadata
                    .as_ref()
                    .and_then(extract_metadata);

                let auth_type = AffirmAuthType::try_from(&item.router_data.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                let public_api_key = auth_type.public_key;
                let merchant = Merchant {
                    public_api_key,
                    user_confirmation_url: request.get_complete_authorize_url()?,
                    user_cancel_url: request.get_router_return_url()?,
                    user_confirmation_url_action: None,
                    use_vcn: None,
                    name: None,
                };

                Ok(Self {
                    merchant,
                    items,
                    shipping,
                    billing,
                    total: item.amount,
                    currency: request.currency,
                    order_id: Some(item.router_data.connector_request_reference_id.clone()),
                    metadata,
                    discounts: None,
                    tax_amount: request.order_tax_amount,
                    shipping_amount: request.shipping_cost,
                    checkout_expiration: None,
                    expiration_time: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}
pub struct AffirmAuthType {
    pub public_key: Secret<String>,
    pub private_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AffirmAuthType {
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

#[derive(Debug, Copy, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AffirmPaymentStatus {
    Authorized,
    Completed,
    Succeeded,
    Failed,
    Pending,
    Processing,
    Abandoned,
    Settled,
    Refunded,
    Declined,
}

impl From<AffirmPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: AffirmPaymentStatus) -> Self {
        match item {
            AffirmPaymentStatus::Authorized => Self::Authorizing,
            AffirmPaymentStatus::Completed
            | AffirmPaymentStatus::Succeeded
            | AffirmPaymentStatus::Settled => Self::Charged,
            AffirmPaymentStatus::Failed
            | AffirmPaymentStatus::Declined
            | AffirmPaymentStatus::Abandoned => Self::Failure,
            AffirmPaymentStatus::Pending | AffirmPaymentStatus::Processing => Self::Authorizing,
            AffirmPaymentStatus::Refunded => Self::Charged,
        }
    }
}

impl From<AffirmTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: AffirmTransactionStatus) -> Self {
        match item {
            AffirmTransactionStatus::Authorized => Self::Authorized,
            AffirmTransactionStatus::AuthExpired => Self::AuthorizationFailed,
            AffirmTransactionStatus::Canceled => Self::Voided,
            AffirmTransactionStatus::Captured => Self::Charged,
            AffirmTransactionStatus::ConfirmationExpired => Self::AuthorizationFailed,
            AffirmTransactionStatus::Confirmed => Self::Authorized,
            AffirmTransactionStatus::Created => Self::Pending,
            AffirmTransactionStatus::Declined => Self::Failure,
            AffirmTransactionStatus::Disputed => Self::Unresolved,
            AffirmTransactionStatus::DisputeRefunded => Self::AutoRefunded,
            AffirmTransactionStatus::ExpiredAuthorization => Self::AuthorizationFailed,
            AffirmTransactionStatus::ExpiredConfirmation => Self::AuthorizationFailed,
            AffirmTransactionStatus::PartiallyCaptured => Self::PartialCharged,
            AffirmTransactionStatus::PartiallyRefunded => Self::PartialChargedAndChargeable,
            AffirmTransactionStatus::Refunded => Self::AutoRefunded,
            AffirmTransactionStatus::Voided => Self::Voided,
            AffirmTransactionStatus::PartiallyVoided => Self::VoidInitiated,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AffirmPaymentsResponse {
    checkout_id: String,
    redirect_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmCompleteAuthorizeResponse {
    pub id: String,
    pub status: AffirmTransactionStatus,
    pub amount: i64,
    pub amount_refunded: i64,
    pub authorization_expiration: String,
    pub checkout_id: String,
    pub created: String,
    pub currency: Currency,
    pub events: Vec<TransactionEvent>,
    pub settlement_transaction_id: Option<String>,
    pub transaction_id: String,
    pub order_id: String,
    pub shipping_carrier: Option<String>,
    pub shipping_confirmation: Option<String>,
    pub shipping: Option<Shipping>,
    pub agent_alias: Option<String>,
    pub merchant_transaction_id: Option<String>,
    pub provider_id: Option<i64>,
    pub remove_tax: Option<bool>,
    pub checkout: Option<Value>,
    pub refund_expires: Option<String>,
    pub remaining_capturable_amount: Option<i64>,
    pub loan_information: Option<LoanInformation>,
    pub user_id: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransactionEvent {
    pub id: String,
    pub amount: i64,
    pub created: String,
    pub currency: Currency,
    pub fee: Option<i64>,
    pub fee_refunded: Option<i64>,
    pub reference_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: AffirmEventType,
    pub settlement_transaction_id: Option<String>,
    pub transaction_id: String,
    pub order_id: String,
    pub shipping_carrier: Option<String>,
    pub shipping_confirmation: Option<String>,
    pub shipping: Option<Shipping>,
    pub agent_alias: Option<String>,
    pub merchant_transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LoanInformation {
    pub fees: Option<LoanFees>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LoanFees {
    pub amount: Option<i64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffirmTransactionStatus {
    Authorized,
    AuthExpired,
    Canceled,
    Captured,
    ConfirmationExpired,
    Confirmed,
    Created,
    Declined,
    Disputed,
    DisputeRefunded,
    ExpiredAuthorization,
    ExpiredConfirmation,
    PartiallyCaptured,
    PartiallyRefunded,
    Refunded,
    Voided,
    PartiallyVoided,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmPSyncResponse {
    pub amount: i64,
    pub amount_refunded: i64,
    pub authorization_expiration: String,
    pub checkout_id: String,
    pub created: String,
    pub currency: Currency,
    pub events: Vec<TransactionEvent>,
    pub id: String,
    pub order_id: String,
    pub provider_id: Option<i64>,
    pub remove_tax: Option<bool>,
    pub status: AffirmEventType,
    pub checkout: Option<Value>,
    pub refund_expires: Option<String>,
    pub remaining_capturable_amount: Option<i64>,
    pub loan_information: Option<LoanInformation>,
    pub shipping_carrier: Option<String>,
    pub shipping_confirmation: Option<String>,
    pub shipping: Option<Shipping>,
    pub agent_alias: Option<String>,
    pub merchant_transaction_id: Option<String>,
    pub settlement_transaction_id: Option<String>,
    pub transaction_id: String,
    pub user_id: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "response_type", content = "data")]
pub enum AffirmResponseWrapper {
    Authorize(AffirmPaymentsResponse),
    Psync(Box<AffirmPSyncResponse>),
}

impl<F, T> TryFrom<ResponseRouterData<F, AffirmResponseWrapper, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, AffirmResponseWrapper, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match &item.response {
            AffirmResponseWrapper::Authorize(resp) => {
                let redirection_data = url::Url::parse(&resp.redirect_url)
                    .ok()
                    .map(|url| RedirectForm::from((url, Method::Get)));

                Ok(Self {
                    status: enums::AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(resp.checkout_id.clone()),
                        redirection_data: Box::new(redirection_data),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        charges: None,
                        incremental_authorization_allowed: None,
                    }),
                    ..item.data
                })
            }
            AffirmResponseWrapper::Psync(resp) => {
                let status = enums::AttemptStatus::from(resp.status.clone());
                Ok(Self {
                    status,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(resp.id.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        charges: None,
                        incremental_authorization_allowed: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, AffirmCompleteAuthorizeResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AffirmCompleteAuthorizeResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
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
pub struct AffirmRefundRequest {
    pub amount: MinorUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
}

impl<F> TryFrom<&AffirmRouterData<&RefundsRouterData<F>>> for AffirmRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &AffirmRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let reference_id = item.router_data.request.connector_transaction_id.clone();

        Ok(Self {
            amount: item.amount.to_owned(),
            reference_id: Some(reference_id),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmRefundResponse {
    pub id: String,
    pub amount: MinorUnit,
    pub created: String,
    pub currency: Currency,
    pub fee: Option<MinorUnit>,
    pub fee_refunded: Option<MinorUnit>,
    pub reference_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: AffirmEventType,
    pub settlement_transaction_id: Option<String>,
    pub transaction_id: String,
    pub order_id: String,
    pub shipping_carrier: Option<String>,
    pub shipping_confirmation: Option<String>,
    pub shipping: Option<Shipping>,
    pub agent_alias: Option<String>,
    pub merchant_transaction_id: Option<String>,
}

impl From<AffirmEventType> for enums::RefundStatus {
    fn from(event_type: AffirmEventType) -> Self {
        match event_type {
            AffirmEventType::Refund => Self::Success,
            _ => Self::Pending,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, AffirmRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, AffirmRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.event_type),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, AffirmRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, AffirmRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.event_type),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AffirmErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

#[derive(Debug, Serialize)]
pub struct AffirmCaptureRequest {
    pub order_id: Option<String>,
    pub reference_id: Option<String>,
    pub amount: MinorUnit,
    pub shipping_carrier: Option<String>,
    pub shipping_confirmation: Option<String>,
}

impl TryFrom<&AffirmRouterData<&PaymentsCaptureRouterData>> for AffirmCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &AffirmRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let reference_id = match item.router_data.connector_request_reference_id.clone() {
            ref_id if ref_id.is_empty() => None,
            ref_id => Some(ref_id),
        };

        let amount = item.amount;

        Ok(Self {
            reference_id,
            amount,
            order_id: None,
            shipping_carrier: None,
            shipping_confirmation: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmCaptureResponse {
    pub id: String,
    pub amount: MinorUnit,
    pub created: String,
    pub currency: Currency,
    pub fee: Option<MinorUnit>,
    pub fee_refunded: Option<MinorUnit>,
    pub reference_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: AffirmEventType,
    pub settlement_transaction_id: Option<String>,
    pub transaction_id: String,
    pub order_id: String,
    pub shipping_carrier: Option<String>,
    pub shipping_confirmation: Option<String>,
    pub shipping: Option<Shipping>,
    pub agent_alias: Option<String>,
    pub merchant_transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AffirmEventType {
    Auth,
    AuthExpired,
    Capture,
    ChargeOff,
    Confirm,
    ConfirmationExpired,
    ExpireAuthorization,
    ExpireConfirmation,
    Refund,
    SplitCapture,
    Update,
    Void,
    PartialVoid,
    RefundVoided,
}

impl From<AffirmEventType> for enums::AttemptStatus {
    fn from(event_type: AffirmEventType) -> Self {
        match event_type {
            AffirmEventType::Auth => Self::Authorizing,
            AffirmEventType::AuthExpired => Self::AuthorizationFailed,
            AffirmEventType::Capture | AffirmEventType::SplitCapture | AffirmEventType::Confirm => {
                Self::Charged
            }
            AffirmEventType::ChargeOff => Self::Failure,
            AffirmEventType::ConfirmationExpired => Self::Failure,
            AffirmEventType::ExpireAuthorization | AffirmEventType::ExpireConfirmation => {
                Self::AuthorizationFailed
            }
            AffirmEventType::Refund | AffirmEventType::RefundVoided => Self::AutoRefunded,
            AffirmEventType::Update => Self::Pending,
            AffirmEventType::Void | AffirmEventType::PartialVoid => Self::Voided,
        }
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, AffirmCaptureResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            AffirmCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.event_type.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
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

impl TryFrom<&PaymentsCancelRouterData> for AffirmCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let request = &item.request;

        let reference_id = request.connector_transaction_id.clone();
        let amount = item
            .request
            .amount
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "amount",
            })?;
        Ok(Self {
            reference_id: Some(reference_id),
            amount,
            merchant_transaction_id: None,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AffirmCancelRequest {
    pub reference_id: Option<String>,
    pub amount: i64,
    pub merchant_transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmCancelResponse {
    pub id: String,
    pub amount: MinorUnit,
    pub created: String,
    pub currency: Currency,
    pub fee: Option<MinorUnit>,
    pub fee_refunded: Option<MinorUnit>,
    pub reference_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: AffirmEventType,
    pub settlement_transaction_id: Option<String>,
    pub transaction_id: String,
    pub order_id: String,
    pub shipping_carrier: Option<String>,
    pub shipping_confirmation: Option<String>,
    pub shipping: Option<Shipping>,
    pub agent_alias: Option<String>,
    pub merchant_transaction_id: Option<String>,
}

impl<F>
    TryFrom<ResponseRouterData<F, AffirmCancelResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AffirmCancelResponse, PaymentsCancelData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.event_type.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
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
