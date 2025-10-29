use api_models::{payments::AdditionalPaymentData, webhooks::IncomingWebhookEvent};
use common_enums::{Currency, FraudCheckStatus};
use common_utils::{
    ext_traits::ValueExt,
    id_type,
    pii::Email,
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_flow_types::Fulfillment,
    router_request_types::{
        fraud_check::{FraudCheckFulfillmentData, FulfillmentStatus},
        BrowserInformation, ResponseId,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_interfaces::errors::ConnectorError;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    types::{
        FrmCheckoutRouterData, FrmFulfillmentRouterData, FrmTransactionRouterData,
        ResponseRouterData,
    },
    utils::{
        convert_amount, AddressDetailsData as _, FraudCheckCheckoutRequest,
        FraudCheckTransactionRequest, RouterData as _,
    },
};

type Error = error_stack::Report<ConnectorError>;

pub struct RiskifiedRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl<T> From<(StringMajorUnit, T)> for RiskifiedRouterData<T> {
    fn from((amount, router_data): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct RiskifiedPaymentsCheckoutRequest {
    order: CheckoutRequest,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct CheckoutRequest {
    id: String,
    note: Option<String>,
    email: Option<Email>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    created_at: PrimitiveDateTime,
    currency: Option<Currency>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    updated_at: PrimitiveDateTime,
    gateway: Option<String>,
    browser_ip: Option<std::net::IpAddr>,
    total_price: StringMajorUnit,
    total_discounts: i64,
    cart_token: String,
    referring_site: String,
    line_items: Vec<LineItem>,
    discount_codes: Vec<DiscountCodes>,
    shipping_lines: Vec<ShippingLines>,
    payment_details: Option<PaymentDetails>,
    customer: RiskifiedCustomer,
    billing_address: Option<OrderAddress>,
    shipping_address: Option<OrderAddress>,
    source: Source,
    client_details: ClientDetails,
    vendor_name: String,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct PaymentDetails {
    credit_card_bin: Option<Secret<String>>,
    credit_card_number: Option<Secret<String>>,
    credit_card_company: Option<api_models::enums::CardNetwork>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct ShippingLines {
    price: StringMajorUnit,
    title: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct DiscountCodes {
    amount: StringMajorUnit,
    code: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct ClientDetails {
    user_agent: Option<String>,
    accept_language: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct RiskifiedCustomer {
    email: Option<Email>,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    created_at: PrimitiveDateTime,
    verified_email: bool,
    id: id_type::CustomerId,
    account_type: CustomerAccountType,
    orders_count: i32,
    phone: Option<Secret<String>>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CustomerAccountType {
    Guest,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct OrderAddress {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address1: Option<Secret<String>>,
    country_code: Option<common_enums::CountryAlpha2>,
    city: Option<String>,
    province: Option<Secret<String>>,
    phone: Option<Secret<String>>,
    zip: Option<Secret<String>>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct LineItem {
    price: StringMajorUnit,
    quantity: i32,
    title: String,
    product_type: Option<common_enums::ProductType>,
    requires_shipping: Option<bool>,
    product_id: Option<String>,
    category: Option<String>,
    brand: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    DesktopWeb,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct RiskifiedMetadata {
    vendor_name: String,
    shipping_lines: Vec<ShippingLines>,
}

impl TryFrom<&RiskifiedRouterData<&FrmCheckoutRouterData>> for RiskifiedPaymentsCheckoutRequest {
    type Error = Error;
    fn try_from(
        payment: &RiskifiedRouterData<&FrmCheckoutRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_data = payment.router_data.clone();
        let metadata: RiskifiedMetadata = payment_data
            .frm_metadata
            .clone()
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "frm_metadata",
            })?
            .parse_value("Riskified Metadata")
            .change_context(ConnectorError::InvalidDataFormat {
                field_name: "frm_metadata",
            })?;
        let billing_address = payment_data.get_billing()?;
        let shipping_address = payment_data.get_shipping_address_with_phone_number()?;
        let address = payment_data.get_billing_address()?;
        let line_items = payment_data
            .request
            .get_order_details()?
            .iter()
            .map(|order_detail| {
                let price = convert_amount(
                    payment.amount_converter,
                    order_detail.amount,
                    payment_data.request.currency.ok_or_else(|| {
                        ConnectorError::MissingRequiredField {
                            field_name: "currency",
                        }
                    })?,
                )?;

                Ok(LineItem {
                    price,
                    quantity: i32::from(order_detail.quantity),
                    title: order_detail.product_name.clone(),
                    product_type: order_detail.product_type.clone(),
                    requires_shipping: order_detail.requires_shipping,
                    product_id: order_detail.product_id.clone(),
                    category: order_detail.category.clone(),
                    brand: order_detail.brand.clone(),
                })
            })
            .collect::<Result<Vec<_>, Self::Error>>()?;

        Ok(Self {
            order: CheckoutRequest {
                id: payment_data.attempt_id.clone(),
                email: payment_data.request.email.clone(),
                created_at: common_utils::date_time::now(),
                updated_at: common_utils::date_time::now(),
                gateway: payment_data.request.gateway.clone(),
                total_price: payment.amount.clone(),
                cart_token: payment_data.attempt_id.clone(),
                line_items,
                source: Source::DesktopWeb,
                billing_address: OrderAddress::try_from(billing_address).ok(),
                shipping_address: OrderAddress::try_from(shipping_address).ok(),
                total_discounts: 0,
                currency: payment_data.request.currency,
                referring_site: "hyperswitch.io".to_owned(),
                discount_codes: Vec::new(),
                shipping_lines: metadata.shipping_lines,
                customer: RiskifiedCustomer {
                    email: payment_data.request.email.clone(),

                    first_name: address.get_first_name().ok().cloned(),
                    last_name: address.get_last_name().ok().cloned(),
                    created_at: common_utils::date_time::now(),
                    verified_email: false,
                    id: payment_data.get_customer_id()?,
                    account_type: CustomerAccountType::Guest,
                    orders_count: 0,
                    phone: billing_address
                        .clone()
                        .phone
                        .and_then(|phone_data| phone_data.number),
                },
                browser_ip: payment_data
                    .request
                    .browser_info
                    .as_ref()
                    .and_then(|browser_info| browser_info.ip_address),
                client_details: ClientDetails {
                    user_agent: payment_data
                        .request
                        .browser_info
                        .as_ref()
                        .and_then(|browser_info| browser_info.user_agent.clone()),
                    accept_language: payment_data.request.browser_info.as_ref().and_then(
                        |browser_info: &BrowserInformation| browser_info.language.clone(),
                    ),
                },
                note: payment_data.description.clone(),
                vendor_name: metadata.vendor_name,
                payment_details: match payment_data.request.payment_method_data.as_ref() {
                    Some(AdditionalPaymentData::Card(card_info)) => Some(PaymentDetails {
                        credit_card_bin: card_info.card_isin.clone().map(Secret::new),
                        credit_card_number: card_info
                            .last4
                            .clone()
                            .map(|last_four| format!("XXXX-XXXX-XXXX-{last_four}"))
                            .map(Secret::new),
                        credit_card_company: card_info.card_network.clone(),
                    }),
                    Some(_) | None => None,
                },
            },
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct RiskifiedPaymentsResponse {
    order: OrderResponse,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct OrderResponse {
    id: String,
    status: PaymentStatus,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct RiskifiedFulfilmentResponse {
    order: OrderFulfilmentResponse,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct OrderFulfilmentResponse {
    id: String,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FulfilmentStatus {
    Fulfilled,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    Captured,
    Created,
    Submitted,
    Approved,
    Declined,
    Processing,
}

impl<F, T> TryFrom<ResponseRouterData<F, RiskifiedPaymentsResponse, T, FraudCheckResponseData>>
    for RouterData<F, T, FraudCheckResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, RiskifiedPaymentsResponse, T, FraudCheckResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order.id),
                status: FraudCheckStatus::from(item.response.order.status),
                connector_metadata: None,
                score: None,
                reason: item.response.order.description.map(serde_json::Value::from),
            }),
            ..item.data
        })
    }
}

impl From<PaymentStatus> for FraudCheckStatus {
    fn from(item: PaymentStatus) -> Self {
        match item {
            PaymentStatus::Approved => Self::Legit,
            PaymentStatus::Declined => Self::Fraud,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct TransactionFailedRequest {
    checkout: FailedTransactionData,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct FailedTransactionData {
    id: String,
    payment_details: Vec<DeclinedPaymentDetails>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct DeclinedPaymentDetails {
    authorization_error: AuthorizationError,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct AuthorizationError {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    created_at: PrimitiveDateTime,
    error_code: Option<String>,
    message: Option<String>,
}

impl TryFrom<&FrmTransactionRouterData> for TransactionFailedRequest {
    type Error = Error;
    fn try_from(item: &FrmTransactionRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            checkout: FailedTransactionData {
                id: item.attempt_id.clone(),
                payment_details: [DeclinedPaymentDetails {
                    authorization_error: AuthorizationError {
                        created_at: common_utils::date_time::now(),
                        error_code: item.request.error_code.clone(),
                        message: item.request.error_message.clone(),
                    },
                }]
                .to_vec(),
            },
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct RiskifiedFailedTransactionResponse {
    checkout: OrderResponse,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub enum RiskifiedTransactionResponse {
    FailedResponse(RiskifiedFailedTransactionResponse),
    SuccessResponse(RiskifiedPaymentsResponse),
}

impl<F, T>
    TryFrom<ResponseRouterData<F, RiskifiedFailedTransactionResponse, T, FraudCheckResponseData>>
    for RouterData<F, T, FraudCheckResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, RiskifiedFailedTransactionResponse, T, FraudCheckResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.checkout.id),
                status: FraudCheckStatus::from(item.response.checkout.status),
                connector_metadata: None,
                score: None,
                reason: item
                    .response
                    .checkout
                    .description
                    .map(serde_json::Value::from),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct TransactionSuccessRequest {
    order: SuccessfulTransactionData,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct SuccessfulTransactionData {
    id: String,
    decision: TransactionDecisionData,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct TransactionDecisionData {
    external_status: TransactionStatus,
    reason: Option<String>,
    amount: StringMajorUnit,
    currency: Currency,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    decided_at: PrimitiveDateTime,
    payment_details: Vec<TransactionPaymentDetails>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct TransactionPaymentDetails {
    authorization_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Approved,
}

impl TryFrom<&RiskifiedRouterData<&FrmTransactionRouterData>> for TransactionSuccessRequest {
    type Error = Error;
    fn try_from(
        item_data: &RiskifiedRouterData<&FrmTransactionRouterData>,
    ) -> Result<Self, Self::Error> {
        let item = item_data.router_data.clone();
        Ok(Self {
            order: SuccessfulTransactionData {
                id: item.attempt_id.clone(),
                decision: TransactionDecisionData {
                    external_status: TransactionStatus::Approved,
                    reason: None,
                    amount: item_data.amount.clone(),
                    currency: item.request.get_currency()?,
                    decided_at: common_utils::date_time::now(),
                    payment_details: [TransactionPaymentDetails {
                        authorization_id: item.request.connector_transaction_id.clone(),
                    }]
                    .to_vec(),
                },
            },
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct RiskifiedFulfillmentRequest {
    order: OrderFulfillment,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FulfillmentRequestStatus {
    Success,
    Cancelled,
    Error,
    Failure,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct OrderFulfillment {
    id: String,
    fulfillments: FulfilmentData,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct FulfilmentData {
    fulfillment_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    created_at: PrimitiveDateTime,
    status: Option<FulfillmentRequestStatus>,
    tracking_company: String,
    tracking_number: String,
    tracking_url: Option<String>,
}

impl TryFrom<&FrmFulfillmentRouterData> for RiskifiedFulfillmentRequest {
    type Error = Error;
    fn try_from(item: &FrmFulfillmentRouterData) -> Result<Self, Self::Error> {
        let tracking_number = item
            .request
            .fulfillment_req
            .tracking_numbers
            .as_ref()
            .and_then(|numbers| numbers.first().cloned())
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "tracking_number",
            })?;
        let tracking_url = item
            .request
            .fulfillment_req
            .tracking_urls
            .as_ref()
            .and_then(|urls| urls.first().cloned().map(|url| url.to_string()));
        Ok(Self {
            order: OrderFulfillment {
                id: item.attempt_id.clone(),
                fulfillments: FulfilmentData {
                    fulfillment_id: item.payment_id.clone(),
                    created_at: common_utils::date_time::now(),
                    status: item
                        .request
                        .fulfillment_req
                        .fulfillment_status
                        .clone()
                        .and_then(get_fulfillment_status),
                    tracking_company: item
                        .request
                        .fulfillment_req
                        .tracking_company
                        .clone()
                        .ok_or(ConnectorError::MissingRequiredField {
                            field_name: "tracking_company",
                        })?,
                    tracking_number,
                    tracking_url,
                },
            },
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            Fulfillment,
            RiskifiedFulfilmentResponse,
            FraudCheckFulfillmentData,
            FraudCheckResponseData,
        >,
    > for RouterData<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<
            Fulfillment,
            RiskifiedFulfilmentResponse,
            FraudCheckFulfillmentData,
            FraudCheckResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(FraudCheckResponseData::FulfillmentResponse {
                order_id: item.response.order.id,
                shipment_ids: Vec::new(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct ErrorResponse {
    pub error: ErrorData,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct ErrorData {
    pub message: String,
}

impl TryFrom<&hyperswitch_domain_models::address::Address> for OrderAddress {
    type Error = Error;
    fn try_from(
        address_info: &hyperswitch_domain_models::address::Address,
    ) -> Result<Self, Self::Error> {
        let address = address_info
            .clone()
            .address
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "address",
            })?;
        Ok(Self {
            first_name: address.first_name.clone(),
            last_name: address.last_name.clone(),
            address1: address.line1.clone(),
            country_code: address.country,
            city: address.city.clone(),
            province: address.state.clone(),
            zip: address.zip.clone(),
            phone: address_info
                .phone
                .clone()
                .and_then(|phone_data| phone_data.number),
        })
    }
}

fn get_fulfillment_status(status: FulfillmentStatus) -> Option<FulfillmentRequestStatus> {
    match status {
        FulfillmentStatus::COMPLETE => Some(FulfillmentRequestStatus::Success),
        FulfillmentStatus::CANCELED => Some(FulfillmentRequestStatus::Cancelled),
        FulfillmentStatus::PARTIAL | FulfillmentStatus::REPLACEMENT => None,
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskifiedWebhookBody {
    pub id: String,
    pub status: RiskifiedWebhookStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum RiskifiedWebhookStatus {
    Approved,
    Declined,
}

impl From<RiskifiedWebhookStatus> for IncomingWebhookEvent {
    fn from(value: RiskifiedWebhookStatus) -> Self {
        match value {
            RiskifiedWebhookStatus::Declined => Self::FrmRejected,
            RiskifiedWebhookStatus::Approved => Self::FrmApproved,
        }
    }
}
