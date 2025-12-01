use common_enums::{enums, CountryAlpha2, Currency};
use common_utils::{pii, request::Method, types::MinorUnit};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{PayLaterData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
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
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        RefundsResponseRouterData, ResponseRouterData,
    },
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
}

#[derive(Debug, Serialize)]
pub struct AffirmCompleteAuthorizeRequest {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_affirm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub itinerary: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_channel_type: Option<String>,
    #[serde(rename = "BOPIS", skip_serializing_if = "Option::is_none")]
    pub bopis: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct Discount {
    pub discount_amount: MinorUnit,
    pub discount_display_name: String,
    pub discount_code: Option<String>,
}

impl TryFrom<&AffirmRouterData<&PaymentsAuthorizeRouterData>> for AffirmPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &AffirmRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = &item.router_data;
        let request = &router_data.request;

        let billing = Some(Billing {
            name: Name {
                first: item.router_data.get_optional_billing_first_name(),
                last: item.router_data.get_optional_billing_last_name(),
                full: item.router_data.get_optional_billing_full_name(),
            },
            address: Address {
                line1: item.router_data.get_optional_billing_line1(),
                line2: item.router_data.get_optional_billing_line2(),
                city: item.router_data.get_optional_billing_city(),
                state: item.router_data.get_optional_billing_state(),
                zipcode: item.router_data.get_optional_billing_zip(),
                country: item.router_data.get_optional_billing_country(),
            },
            phone_number: item.router_data.get_optional_billing_phone_number(),
            email: item.router_data.get_optional_billing_email(),
        });

        let shipping = Some(Shipping {
            name: Name {
                first: item.router_data.get_optional_shipping_first_name(),
                last: item.router_data.get_optional_shipping_last_name(),
                full: item.router_data.get_optional_shipping_full_name(),
            },
            address: Address {
                line1: item.router_data.get_optional_shipping_line1(),
                line2: item.router_data.get_optional_shipping_line2(),
                city: item.router_data.get_optional_shipping_city(),
                state: item.router_data.get_optional_shipping_state(),
                zipcode: item.router_data.get_optional_shipping_zip(),
                country: item.router_data.get_optional_shipping_country(),
            },
            phone_number: item.router_data.get_optional_shipping_phone_number(),
            email: item.router_data.get_optional_shipping_email(),
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
                            })
                        })
                        .collect::<Result<Vec<_>, _>>(),
                    None => Err(report!(errors::ConnectorError::MissingRequiredField {
                        field_name: "order_details",
                    })),
                }?;

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

impl From<AffirmTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: AffirmTransactionStatus) -> Self {
        match item {
            AffirmTransactionStatus::Authorized => Self::Authorized,
            AffirmTransactionStatus::AuthExpired => Self::Failure,
            AffirmTransactionStatus::Canceled => Self::Voided,
            AffirmTransactionStatus::Captured => Self::Charged,
            AffirmTransactionStatus::ConfirmationExpired => Self::Failure,
            AffirmTransactionStatus::Confirmed => Self::Authorized,
            AffirmTransactionStatus::Created => Self::Pending,
            AffirmTransactionStatus::Declined => Self::Failure,
            AffirmTransactionStatus::Disputed => Self::Unresolved,
            AffirmTransactionStatus::DisputeRefunded => Self::Unresolved,
            AffirmTransactionStatus::ExpiredAuthorization => Self::Failure,
            AffirmTransactionStatus::ExpiredConfirmation => Self::Failure,
            AffirmTransactionStatus::PartiallyCaptured => Self::Charged,
            AffirmTransactionStatus::Voided => Self::Voided,
            AffirmTransactionStatus::PartiallyVoided => Self::Voided,
        }
    }
}

impl From<AffirmRefundStatus> for common_enums::RefundStatus {
    fn from(item: AffirmRefundStatus) -> Self {
        match item {
            AffirmRefundStatus::PartiallyRefunded => Self::Success,
            AffirmRefundStatus::Refunded => Self::Success,
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
    pub amount: MinorUnit,
    pub amount_refunded: MinorUnit,
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
    pub amount: MinorUnit,
    pub created: String,
    pub currency: Currency,
    pub fee: Option<i64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LoanInformation {
    pub fees: Option<LoanFees>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LoanFees {
    pub amount: Option<MinorUnit>,
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
    Voided,
    PartiallyVoided,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffirmRefundStatus {
    PartiallyRefunded,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmPSyncResponse {
    pub amount: MinorUnit,
    pub amount_refunded: MinorUnit,
    pub authorization_expiration: Option<String>,
    pub checkout_id: String,
    pub created: String,
    pub currency: Currency,
    pub events: Vec<TransactionEvent>,
    pub id: String,
    pub order_id: String,
    pub provider_id: Option<i64>,
    pub remove_tax: Option<bool>,
    pub status: AffirmTransactionStatus,
    pub checkout: Option<Value>,
    pub refund_expires: Option<String>,
    pub remaining_capturable_amount: Option<MinorUnit>,
    pub loan_information: Option<LoanInformation>,
    pub shipping_carrier: Option<String>,
    pub shipping_confirmation: Option<String>,
    pub shipping: Option<Shipping>,
    pub merchant_transaction_id: Option<String>,
    pub settlement_transaction_id: Option<String>,
    pub transaction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmRsyncResponse {
    pub amount: MinorUnit,
    pub amount_refunded: MinorUnit,
    pub authorization_expiration: String,
    pub checkout_id: String,
    pub created: String,
    pub currency: Currency,
    pub events: Vec<TransactionEvent>,
    pub id: String,
    pub order_id: String,
    pub status: AffirmRefundStatus,
    pub refund_expires: Option<String>,
    pub remaining_capturable_amount: Option<MinorUnit>,
    pub shipping_carrier: Option<String>,
    pub shipping_confirmation: Option<String>,
    pub shipping: Option<Shipping>,
    pub merchant_transaction_id: Option<String>,
    pub settlement_transaction_id: Option<String>,
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
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
                let status = enums::AttemptStatus::from(resp.status);
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

impl TryFrom<RefundsResponseRouterData<RSync, AffirmRsyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, AffirmRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
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
            AffirmEventType::Auth => Self::Authorized,
            AffirmEventType::Capture | AffirmEventType::SplitCapture | AffirmEventType::Confirm => {
                Self::Charged
            }
            AffirmEventType::AuthExpired
            | AffirmEventType::ChargeOff
            | AffirmEventType::ConfirmationExpired
            | AffirmEventType::ExpireAuthorization
            | AffirmEventType::ExpireConfirmation => Self::Failure,
            AffirmEventType::Refund | AffirmEventType::RefundVoided => Self::AutoRefunded,
            AffirmEventType::Update => Self::Pending,
            AffirmEventType::Void | AffirmEventType::PartialVoid => Self::Voided,
        }
    }
}

impl TryFrom<PaymentsCaptureResponseRouterData<AffirmCaptureResponse>>
    for PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<AffirmCaptureResponse>,
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

impl TryFrom<PaymentsCancelResponseRouterData<AffirmCancelResponse>> for PaymentsCancelRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<AffirmCancelResponse>,
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
