use std::net::IpAddr;

use api_models::{payments::AdditionalPaymentData, webhooks::IncomingWebhookEvent};
use common_enums::{AttemptStatus, Currency, FraudCheckStatus, PaymentMethod, PaymentMethodType};
use common_utils::{
    ext_traits::ValueExt,
    id_type,
    pii::Email,
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector, MinorUnit},
};
use error_stack::{self, ResultExt};
pub use hyperswitch_domain_models::router_request_types::fraud_check::RefundMethod;
use hyperswitch_domain_models::{
    address::Address as DomainAddress,
    router_data::RouterData,
    router_flow_types::Fulfillment,
    router_request_types::{
        fraud_check::{self, FraudCheckFulfillmentData, FrmFulfillmentRequest},
        ResponseId,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_interfaces::errors::ConnectorError;
use hyperswitch_masking::{PeekInterface, Secret};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::{
    types::{
        FrmCheckoutRouterData, FrmFulfillmentRouterData, FrmRecordReturnRouterData,
        FrmSaleRouterData, FrmTransactionRouterData, ResponseRouterData,
    },
    utils::{
        convert_amount, AddressDetailsData as _, FraudCheckCheckoutRequest,
        FraudCheckRecordReturnRequest as _, FraudCheckSaleRequest as _,
        FraudCheckTransactionRequest as _, RouterData as _,
    },
};
pub struct SignifydRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
    pub(crate) amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl<T> SignifydRouterData<T> {
    pub fn new(amount: FloatMajorUnit, router_data: T) -> Self {
        Self {
            amount,
            router_data,
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecisionDelivery {
    Sync,
    AsyncOnly,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Purchase {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    created_at: PrimitiveDateTime,
    order_channel: OrderChannel,
    total_price: FloatMajorUnit,
    products: Vec<Products>,
    shipments: Shipments,
    currency: Option<Currency>,
    total_shipping_cost: Option<FloatMajorUnit>,
    confirmation_email: Option<Email>,
    confirmation_phone: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all(serialize = "SCREAMING_SNAKE_CASE", deserialize = "snake_case"))]
pub enum OrderChannel {
    Web,
    Phone,
    MobileApp,
    Social,
    Marketplace,
    InStoreKiosk,
    ScanAndGo,
    SmartTv,
    Mit,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all(serialize = "SCREAMING_SNAKE_CASE", deserialize = "snake_case"))]
pub enum FulfillmentMethod {
    Delivery,
    CounterPickup,
    CubsidePickup,
    LockerPickup,
    StandardShipping,
    ExpeditedShipping,
    GasPickup,
    ScheduledDelivery,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Products {
    item_name: String,
    item_price: FloatMajorUnit,
    item_quantity: i32,
    item_id: Option<String>,
    item_category: Option<String>,
    item_sub_category: Option<String>,
    item_is_digital: Option<bool>,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Shipments {
    destination: Destination,
    fulfillment_method: Option<FulfillmentMethod>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    full_name: Secret<String>,
    organization: Option<String>,
    email: Option<Email>,
    address: Address,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    street_address: Secret<String>,
    unit: Option<Secret<String>>,
    postal_code: Secret<String>,
    city: String,
    province_code: Secret<String>,
    country_code: common_enums::CountryAlpha2,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all(serialize = "SCREAMING_SNAKE_CASE", deserialize = "snake_case"))]
pub enum CoverageRequests {
    Fraud, // use when you need a financial guarantee for Payment Fraud.
    Inr,   // use when you need a financial guarantee for Item Not Received.
    Snad, // use when you need a financial guarantee for fraud alleging items are Significantly Not As Described.
    All,  // use when you need a financial guarantee on all chargebacks.
    None, // use when you do not need a financial guarantee. Suggested actions in decision.checkpointAction are recommendations.
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    client_ip_address: Option<IpAddr>,
    session_id: Option<Secret<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserAccount {
    // `username` (Required - Technical) and `account_number` (Required - Risk)
    // are both populated from Hyperswitch's `customer_id` — HS only carries one
    // customer identifier, and Signifyd's cert kit explicitly requires `username`
    // to be supplied even when it duplicates another field on the same payload.
    username: Option<id_type::CustomerId>,
    email: Option<Email>,
    phone: Option<Secret<String>>,
    account_number: Option<id_type::CustomerId>,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethodKind {
    CreditCard,
    DebitCard,
    GiftCard,
    PrepaidCard,
    SnapCard,
    Other,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    street_address: Option<Secret<String>>,
    unit: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    city: Option<String>,
    province_code: Option<Secret<String>>,
    country_code: Option<common_enums::CountryAlpha2>,
}

impl From<&DomainAddress> for BillingAddress {
    fn from(address: &DomainAddress) -> Self {
        let details = address.address.as_ref();
        Self {
            street_address: details.and_then(|d| d.line1.clone()),
            unit: details.and_then(|d| d.line2.clone()),
            postal_code: details.and_then(|d| d.zip.clone()),
            city: details.and_then(|d| d.city.clone()),
            province_code: details.and_then(|d| d.state.clone()),
            country_code: details.and_then(|d| d.country),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutPaymentDetails {
    account_holder_name: Option<Secret<String>>,
    card_bin: Option<String>,
    // Signifyd's schema (cert kit "signifyd-required-data.csv": "integer")
    // requires `cardExpiryMonth` / `cardExpiryYear` as integers, not strings.
    // HS' `CardData` trait helpers (`get_expiry_month_as_i8`,
    // `get_expiry_year_as_4_digit_i32`) target `Card`, not `AdditionalCardInfo`,
    // so we parse once via `parse_card_digits` at struct construction.
    card_expiry_month: Option<Secret<i32>>,
    card_expiry_year: Option<Secret<i32>>,
    card_last4: Option<String>,
    billing_address: Option<BillingAddress>,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    payment_method: PaymentMethodKind,
    amount: FloatMajorUnit,
    currency: Currency,
    gateway: Option<String>,
    checkout_payment_details: CheckoutPaymentDetails,
}

fn map_payment_method_kind(value: Option<PaymentMethodType>) -> PaymentMethodKind {
    match value {
        Some(PaymentMethodType::Credit) => PaymentMethodKind::CreditCard,
        Some(PaymentMethodType::Debit) => PaymentMethodKind::DebitCard,
        _ => PaymentMethodKind::Other,
    }
}

/// Parse an `Option<Secret<String>>` card-expiry field to `i32`. Used for
/// Signifyd's integer-typed `cardExpiryMonth` / `cardExpiryYear`. Mirrors HS'
/// `CardData::get_expiry_year_as_4_digit_i32` but works on `AdditionalCardInfo`'s
/// `Option<Secret<String>>` (the `CardData` trait targets `Card`, not
/// `AdditionalCardInfo`). Returns `None` if missing or unparseable.
fn parse_card_digits(value: Option<&Secret<String>>) -> Option<Secret<i32>> {
    value.and_then(|s| s.peek().trim().parse::<i32>().ok().map(Secret::new))
}

/// Parse a card-expiry year to a 4-digit `i32`. Mirrors the canonical
/// `CardData::get_expiry_year_4_digit` heuristic (string-length check) before
/// parsing.
fn parse_card_year_4_digit(value: Option<&Secret<String>>) -> Option<Secret<i32>> {
    value.and_then(|s| {
        let raw = s.peek().trim();
        let four_digit = if raw.len() == 2 {
            format!("20{raw}")
        } else {
            raw.to_string()
        };
        four_digit.parse::<i32>().ok().map(Secret::new)
    })
}

fn build_transaction(
    payment_method_data: Option<&AdditionalPaymentData>,
    payment_method_type: Option<PaymentMethodType>,
    amount: FloatMajorUnit,
    currency: Currency,
    gateway: Option<&String>,
    billing_address: Option<&DomainAddress>,
) -> Option<Transaction> {
    let card_info = payment_method_data?.get_additional_card_info()?;

    let checkout_payment_details = CheckoutPaymentDetails {
        account_holder_name: card_info.card_holder_name.clone(),
        card_bin: card_info.card_isin.clone(),
        card_expiry_month: parse_card_digits(card_info.card_exp_month.as_ref()),
        card_expiry_year: parse_card_year_4_digit(card_info.card_exp_year.as_ref()),
        card_last4: card_info.last4.clone(),
        billing_address: billing_address.map(BillingAddress::from),
    };

    Some(Transaction {
        payment_method: map_payment_method_kind(payment_method_type),
        amount,
        currency,
        gateway: gateway.cloned(),
        checkout_payment_details,
    })
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsSaleRequest {
    order_id: String,
    purchase: Purchase,
    decision_delivery: DecisionDelivery,
    coverage_requests: Option<CoverageRequests>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<Device>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_account: Option<UserAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transactions: Option<Vec<Transaction>>,
}

#[derive(Debug, Serialize, Eq, PartialEq, Deserialize, Clone)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct SignifydFrmMetadata {
    pub total_shipping_cost: Option<i64>,
    pub fulfillment_method: Option<FulfillmentMethod>,
    pub coverage_request: Option<CoverageRequests>,
    pub order_channel: OrderChannel,
    #[serde(default)]
    pub session_id: Option<Secret<String>>,
}

fn build_user_account(
    customer_id: Option<&id_type::CustomerId>,
    email: Option<&Email>,
    phone: Option<&Secret<String>>,
) -> Option<UserAccount> {
    let username = customer_id.cloned();
    let account_number = customer_id.cloned();
    let email = email.cloned();
    let phone = phone.cloned();

    if [username.is_none(), email.is_none(), phone.is_none()]
        .iter()
        .all(|is_none| *is_none)
    {
        return None;
    }

    Some(UserAccount {
        username,
        email,
        phone,
        account_number,
    })
}

impl TryFrom<&SignifydRouterData<&FrmSaleRouterData>> for SignifydPaymentsSaleRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(data: &SignifydRouterData<&FrmSaleRouterData>) -> Result<Self, Self::Error> {
        let item = data.router_data;
        let currency = item
            .request
            .currency
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "currency",
            })?;
        let products = item
            .request
            .get_order_details()?
            .iter()
            .map(|order_detail| {
                Ok::<_, error_stack::Report<ConnectorError>>(Products {
                    item_name: order_detail.product_name.clone(),
                    item_price: convert_amount(
                        data.amount_converter,
                        order_detail.amount,
                        currency,
                    )?,
                    item_quantity: i32::from(order_detail.quantity),
                    item_id: order_detail.product_id.clone(),
                    item_category: order_detail.category.clone(),
                    item_sub_category: order_detail.sub_category.clone(),
                    item_is_digital: order_detail
                        .product_type
                        .as_ref()
                        .map(|product| product == &common_enums::ProductType::Digital),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let metadata: SignifydFrmMetadata = item
            .frm_metadata
            .clone()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "frm_metadata",
            })?
            .parse_value("Signifyd Frm Metadata")
            .change_context(ConnectorError::InvalidDataFormat {
                field_name: "frm_metadata",
            })?;
        let ship_address = item.get_shipping_address()?;
        let billing_address = item.get_billing()?;
        let address: Address = Address {
            street_address: ship_address.get_line1()?.clone(),
            unit: None,
            postal_code: ship_address.get_zip()?.clone(),
            city: ship_address.get_city()?.clone(),
            province_code: ship_address.get_state()?.clone(),
            country_code: ship_address.get_country()?.to_owned(),
        };
        let destination: Destination = Destination {
            full_name: ship_address.get_full_name()?,
            organization: None,
            email: None,
            address,
        };

        let total_price = data.amount;
        let total_shipping_cost = metadata
            .total_shipping_cost
            .map(|cost| convert_amount(data.amount_converter, MinorUnit::new(cost), currency))
            .transpose()?;

        let created_at = common_utils::date_time::now();
        let order_channel = metadata.order_channel;
        let shipments = Shipments {
            destination,
            fulfillment_method: metadata.fulfillment_method,
        };
        let confirmation_email = item.request.email.clone();
        let purchase = Purchase {
            created_at,
            order_channel,
            total_price,
            products,
            shipments,
            currency: item.request.currency,
            total_shipping_cost,
            confirmation_email,
            confirmation_phone: billing_address
                .clone()
                .phone
                .and_then(|phone_data| phone_data.number),
        };
        let client_ip_address = item.request.client_ip;
        let session_id = metadata.session_id.clone();
        let device = match (client_ip_address, session_id.clone()) {
            (None, None) => None,
            _ => Some(Device {
                client_ip_address,
                session_id,
            }),
        };
        let user_account = build_user_account(
            item.request.customer_id.as_ref(),
            item.request.email.as_ref(),
            item.request.phone.as_ref(),
        );
        let transactions = build_transaction(
            item.request.payment_method_data.as_ref(),
            item.payment_method_type,
            data.amount,
            currency,
            item.request.gateway.as_ref(),
            item.get_optional_billing(),
        )
        .map(|t| vec![t]);
        Ok(Self {
            order_id: item.attempt_id.clone(),
            purchase,
            decision_delivery: DecisionDelivery::Sync, // Specify SYNC if you require the Response to contain a decision field. If you have registered for a webhook associated with this checkpoint, then the webhook will also be sent when SYNC is specified. If ASYNC_ONLY is specified, then the decision field in the response will be null, and you will require a Webhook integration to receive Signifyd's final decision
            coverage_requests: metadata.coverage_request,
            device,
            user_account,
            transactions,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    created_at: PrimitiveDateTime,
    checkpoint_action: SignifydPaymentStatus,
    checkpoint_action_reason: Option<String>,
    checkpoint_action_policy: Option<String>,
    score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignifydPaymentStatus {
    Accept,
    Challenge,
    Credit,
    Hold,
    Reject,
}

impl From<SignifydPaymentStatus> for FraudCheckStatus {
    fn from(item: SignifydPaymentStatus) -> Self {
        match item {
            SignifydPaymentStatus::Accept => Self::Legit,
            SignifydPaymentStatus::Reject => Self::Fraud,
            SignifydPaymentStatus::Hold => Self::ManualReview,
            SignifydPaymentStatus::Challenge | SignifydPaymentStatus::Credit => Self::Pending,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsResponse {
    signifyd_id: i64,
    order_id: String,
    decision: Decision,
}

impl<F, T> TryFrom<ResponseRouterData<F, SignifydPaymentsResponse, T, FraudCheckResponseData>>
    for RouterData<F, T, FraudCheckResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SignifydPaymentsResponse, T, FraudCheckResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order_id),
                status: FraudCheckStatus::from(item.response.decision.checkpoint_action),
                connector_metadata: None,
                score: item.response.decision.score.and_then(|data| data.to_i32()),
                reason: item
                    .response
                    .decision
                    .checkpoint_action_reason
                    .map(serde_json::Value::from),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SignifydErrorResponse {
    pub messages: Vec<String>,
    pub errors: serde_json::Value,
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transactions {
    transaction_id: String,
    gateway_status_code: String,
    payment_method: PaymentMethod,
    amount: i64,
    currency: Currency,
    gateway: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsTransactionRequest {
    order_id: String,
    checkout_id: String,
    transactions: Transactions,
}

impl From<AttemptStatus> for GatewayStatusCode {
    fn from(item: AttemptStatus) -> Self {
        match item {
            AttemptStatus::Pending => Self::Pending,
            AttemptStatus::Failure => Self::Failure,
            AttemptStatus::Charged => Self::Success,
            _ => Self::Pending,
        }
    }
}

impl TryFrom<&FrmTransactionRouterData> for SignifydPaymentsTransactionRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &FrmTransactionRouterData) -> Result<Self, Self::Error> {
        let currency = item.request.get_currency()?;
        let transactions = Transactions {
            amount: item.request.amount,
            transaction_id: item.clone().payment_id,
            gateway_status_code: GatewayStatusCode::from(item.status).to_string(),
            payment_method: item.payment_method,
            currency,
            gateway: item.request.connector.clone(),
        };
        Ok(Self {
            order_id: item.attempt_id.clone(),
            checkout_id: item.payment_id.clone(),
            transactions,
        })
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewayStatusCode {
    Success,
    Failure,
    #[default]
    Pending,
    Error,
    Cancelled,
    Expired,
    SoftDecline,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsCheckoutRequest {
    checkout_id: String,
    order_id: String,
    purchase: Purchase,
    coverage_requests: Option<CoverageRequests>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<Device>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_account: Option<UserAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transactions: Option<Vec<Transaction>>,
}

impl TryFrom<&SignifydRouterData<&FrmCheckoutRouterData>> for SignifydPaymentsCheckoutRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(data: &SignifydRouterData<&FrmCheckoutRouterData>) -> Result<Self, Self::Error> {
        let item = data.router_data;
        let currency = item
            .request
            .currency
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "currency",
            })?;
        let products = item
            .request
            .get_order_details()?
            .iter()
            .map(|order_detail| {
                Ok::<_, error_stack::Report<ConnectorError>>(Products {
                    item_name: order_detail.product_name.clone(),
                    item_price: convert_amount(
                        data.amount_converter,
                        order_detail.amount,
                        currency,
                    )?,
                    item_quantity: i32::from(order_detail.quantity),
                    item_id: order_detail.product_id.clone(),
                    item_category: order_detail.category.clone(),
                    item_sub_category: order_detail.sub_category.clone(),
                    item_is_digital: order_detail
                        .product_type
                        .as_ref()
                        .map(|product| product == &common_enums::ProductType::Digital),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let metadata: SignifydFrmMetadata = item
            .frm_metadata
            .clone()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "frm_metadata",
            })?
            .parse_value("Signifyd Frm Metadata")
            .change_context(ConnectorError::InvalidDataFormat {
                field_name: "frm_metadata",
            })?;
        let ship_address = item.get_shipping_address()?;
        let billing_address = item.get_billing()?;
        let address: Address = Address {
            street_address: ship_address.get_line1()?.clone(),
            unit: None,
            postal_code: ship_address.get_zip()?.clone(),
            city: ship_address.get_city()?.clone(),
            province_code: ship_address.get_state()?.clone(),
            country_code: ship_address.get_country()?.to_owned(),
        };
        let destination: Destination = Destination {
            full_name: ship_address.get_full_name()?,
            organization: None,
            email: None,
            address,
        };

        let total_price = data.amount;
        let total_shipping_cost = metadata
            .total_shipping_cost
            .map(|cost| convert_amount(data.amount_converter, MinorUnit::new(cost), currency))
            .transpose()?;

        let created_at = common_utils::date_time::now();
        let order_channel = metadata.order_channel;
        let shipments: Shipments = Shipments {
            destination,
            fulfillment_method: metadata.fulfillment_method,
        };
        let confirmation_email = item.request.email.clone();
        let purchase = Purchase {
            created_at,
            order_channel,
            total_price,
            products,
            shipments,
            currency: item.request.currency,
            total_shipping_cost,
            confirmation_email,
            confirmation_phone: billing_address
                .clone()
                .phone
                .and_then(|phone_data| phone_data.number),
        };
        let client_ip_address = item.request.client_ip;
        let session_id = metadata.session_id.clone();
        let device = match (client_ip_address, session_id.clone()) {
            (None, None) => None,
            _ => Some(Device {
                client_ip_address,
                session_id,
            }),
        };
        let user_account = build_user_account(
            item.request.customer_id.as_ref(),
            item.request.email.as_ref(),
            item.request.phone.as_ref(),
        );
        let transactions = build_transaction(
            item.request.payment_method_data.as_ref(),
            item.payment_method_type,
            data.amount,
            currency,
            item.request.gateway.as_ref(),
            item.get_optional_billing(),
        )
        .map(|t| vec![t]);
        Ok(Self {
            checkout_id: item.payment_id.clone(),
            order_id: item.attempt_id.clone(),
            purchase,
            coverage_requests: metadata.coverage_request,
            device,
            user_account,
            transactions,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct FrmFulfillmentSignifydRequest {
    pub order_id: String,
    pub fulfillment_status: Option<FulfillmentStatus>,
    pub fulfillments: Vec<Fulfillments>,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(untagged)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub enum FulfillmentStatus {
    PARTIAL,
    COMPLETE,
    REPLACEMENT,
    CANCELED,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct Fulfillments {
    pub shipment_id: String,
    pub products: Option<Vec<Product>>,
    pub destination: Destination,
    pub fulfillment_method: Option<String>,
    pub carrier: Option<String>,
    pub shipment_status: Option<String>,
    pub tracking_urls: Option<Vec<String>>,
    pub tracking_numbers: Option<Vec<String>>,
    pub shipped_at: Option<String>,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub item_name: String,
    pub item_quantity: i64,
    pub item_id: String,
}

impl TryFrom<&FrmFulfillmentRouterData> for FrmFulfillmentSignifydRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &FrmFulfillmentRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            order_id: item.request.fulfillment_req.order_id.clone(),
            fulfillment_status: item
                .request
                .fulfillment_req
                .fulfillment_status
                .as_ref()
                .map(|fulfillment_status| FulfillmentStatus::from(&fulfillment_status.clone())),
            fulfillments: get_signifyd_fulfillments_from_frm_fulfillment_request(
                &item.request.fulfillment_req,
            ),
        })
    }
}

impl From<&fraud_check::FulfillmentStatus> for FulfillmentStatus {
    fn from(status: &fraud_check::FulfillmentStatus) -> Self {
        match status {
            fraud_check::FulfillmentStatus::PARTIAL => Self::PARTIAL,
            fraud_check::FulfillmentStatus::COMPLETE => Self::COMPLETE,
            fraud_check::FulfillmentStatus::REPLACEMENT => Self::REPLACEMENT,
            fraud_check::FulfillmentStatus::CANCELED => Self::CANCELED,
        }
    }
}
pub(crate) fn get_signifyd_fulfillments_from_frm_fulfillment_request(
    fulfillment_req: &FrmFulfillmentRequest,
) -> Vec<Fulfillments> {
    fulfillment_req
        .fulfillments
        .iter()
        .map(|fulfillment| Fulfillments {
            shipment_id: fulfillment.shipment_id.clone(),
            products: fulfillment
                .products
                .as_ref()
                .map(|products| products.iter().map(|p| Product::from(p.clone())).collect()),
            destination: Destination::from(fulfillment.destination.clone()),
            tracking_urls: fulfillment_req.tracking_urls.clone(),
            tracking_numbers: fulfillment_req.tracking_numbers.clone(),
            fulfillment_method: fulfillment_req.fulfillment_method.clone(),
            carrier: fulfillment_req.carrier.clone(),
            shipment_status: fulfillment_req.shipment_status.clone(),
            shipped_at: fulfillment_req.shipped_at.clone(),
        })
        .collect()
}

impl From<fraud_check::Product> for Product {
    fn from(product: fraud_check::Product) -> Self {
        Self {
            item_name: product.item_name,
            item_quantity: product.item_quantity,
            item_id: product.item_id,
        }
    }
}

impl From<fraud_check::Destination> for Destination {
    fn from(destination: fraud_check::Destination) -> Self {
        Self {
            full_name: destination.full_name,
            organization: destination.organization,
            email: destination.email,
            address: Address::from(destination.address),
        }
    }
}

impl From<fraud_check::Address> for Address {
    fn from(address: fraud_check::Address) -> Self {
        Self {
            street_address: address.street_address,
            unit: address.unit,
            postal_code: address.postal_code,
            city: address.city,
            province_code: address.province_code,
            country_code: address.country_code,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct FrmFulfillmentSignifydApiResponse {
    pub order_id: String,
    pub shipment_ids: Vec<String>,
}

impl
    TryFrom<
        ResponseRouterData<
            Fulfillment,
            FrmFulfillmentSignifydApiResponse,
            FraudCheckFulfillmentData,
            FraudCheckResponseData,
        >,
    > for RouterData<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Fulfillment,
            FrmFulfillmentSignifydApiResponse,
            FraudCheckFulfillmentData,
            FraudCheckResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::FulfillmentResponse {
                order_id: item.response.order_id,
                shipment_ids: item.response.shipment_ids,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct SignifydRefund {
    method: RefundMethod,
    amount: String,
    currency: Currency,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsRecordReturnRequest {
    order_id: String,
    return_id: String,
    refund_transaction_id: Option<String>,
    refund: SignifydRefund,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct SignifydPaymentsRecordReturnResponse {
    return_id: String,
    order_id: String,
}

impl<F, T>
    TryFrom<ResponseRouterData<F, SignifydPaymentsRecordReturnResponse, T, FraudCheckResponseData>>
    for RouterData<F, T, FraudCheckResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            SignifydPaymentsRecordReturnResponse,
            T,
            FraudCheckResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::RecordReturnResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order_id),
                return_id: Some(item.response.return_id.to_string()),
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<&FrmRecordReturnRouterData> for SignifydPaymentsRecordReturnRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &FrmRecordReturnRouterData) -> Result<Self, Self::Error> {
        let currency = item.request.get_currency()?;
        let refund = SignifydRefund {
            method: item.request.refund_method.clone(),
            amount: item.request.amount.to_string(),
            currency,
        };
        Ok(Self {
            return_id: uuid::Uuid::new_v4().to_string(),
            refund_transaction_id: item.request.refund_transaction_id.clone(),
            refund,
            order_id: item.attempt_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignifydWebhookBody {
    pub order_id: String,
    // Signifyd may emit dispositions like UNSET / IN_REVIEW / UNCONFIRMED for
    // in-flight or manually-reviewed cases. Make the field tolerant so the
    // webhook deserializes even when the disposition is not yet a final
    // GOOD/FRAUDULENT decision.
    pub review_disposition: Option<ReviewDisposition>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewDisposition {
    Fraudulent,
    Good,
    Unset,
    Unconfirmed,
    InReview,
    Approved,
    Declined,
    #[serde(other)]
    Unknown,
}

impl From<ReviewDisposition> for IncomingWebhookEvent {
    fn from(value: ReviewDisposition) -> Self {
        match value {
            ReviewDisposition::Fraudulent | ReviewDisposition::Declined => Self::FrmRejected,
            ReviewDisposition::Good | ReviewDisposition::Approved => Self::FrmApproved,
            ReviewDisposition::Unset
            | ReviewDisposition::Unconfirmed
            | ReviewDisposition::InReview
            | ReviewDisposition::Unknown => Self::EventNotSupported,
        }
    }
}
