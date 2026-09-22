use std::str::FromStr;

use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    id_type,
    types::{MinorUnit, StringMinorUnit},
};
use error_stack::ResultExt;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use hyperswitch_domain_models::revenue_recovery;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{subscriptions as subscription_request_types, ResponseId},
    router_response_types::{
        subscriptions as subscription_response_types, ConnectorCustomerResponseData,
        PaymentsResponseData, RefundsResponseData,
    },
    types::{
        ConnectorCustomerRouterData, PaymentsAuthorizeRouterData, RefundsRouterData,
        SubscriptionCancelRouterData, SubscriptionCreateRouterData, SubscriptionPauseRouterData,
        SubscriptionResumeRouterData,
    },
};
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::{
    router_flow_types::revenue_recovery as recovery_router_flows,
    router_request_types::revenue_recovery as recovery_request_types,
    router_response_types::revenue_recovery as recovery_response_types,
    types as recovery_router_data_types,
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{convert_uppercase, PaymentsAuthorizeRequestData},
};
pub mod auth_headers {
    pub const STRIPE_API_VERSION: &str = "stripe-version";
    pub const STRIPE_VERSION: &str = "2022-11-15";
}

fn primitive_date_time_from_timestamp(
    value: i64,
) -> CustomResult<PrimitiveDateTime, errors::ConnectorError> {
    time::OffsetDateTime::from_unix_timestamp(value)
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
        .map(|date_time| PrimitiveDateTime::new(date_time.date(), date_time.time()))
}

fn parse_subscription_id(
    value: String,
) -> Result<id_type::SubscriptionId, error_stack::Report<errors::ConnectorError>> {
    id_type::SubscriptionId::try_from(std::borrow::Cow::Owned(value))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

fn parse_invoice_id(
    value: String,
) -> Result<id_type::InvoiceId, error_stack::Report<errors::ConnectorError>> {
    id_type::InvoiceId::try_from(std::borrow::Cow::Owned(value))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

fn parse_customer_id(
    value: String,
) -> Result<id_type::CustomerId, error_stack::Report<errors::ConnectorError>> {
    id_type::CustomerId::try_from(std::borrow::Cow::Owned(value))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

#[derive(Debug, Serialize, PartialEq)]
pub struct StripebillingCustomerCreateRequest {
    pub email: Option<common_utils::pii::Email>,
    pub name: Option<Secret<String>>,
    #[serde(rename = "metadata[hyperswitch_customer_id]")]
    pub hyperswitch_customer_id: Option<String>,
}

impl TryFrom<&ConnectorCustomerRouterData> for StripebillingCustomerCreateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: item.request.email.clone(),
            name: item.request.name.clone(),
            hyperswitch_customer_id: item
                .request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripebillingCustomerResponse {
    pub id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, StripebillingCustomerResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, StripebillingCustomerResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(item.response.id),
            )),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripebillingListResponse<T> {
    pub data: Vec<T>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripebillingProduct {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionItems,
            StripebillingListResponse<StripebillingProduct>,
            subscription_request_types::GetSubscriptionItemsRequest,
            subscription_response_types::GetSubscriptionItemsResponse,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionItems,
        subscription_request_types::GetSubscriptionItemsRequest,
        subscription_response_types::GetSubscriptionItemsResponse,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionItems,
            StripebillingListResponse<StripebillingProduct>,
            subscription_request_types::GetSubscriptionItemsRequest,
            subscription_response_types::GetSubscriptionItemsResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let offset = item.data.request.offset.unwrap_or(0) as usize;
        let list = item
            .response
            .data
            .into_iter()
            .skip(offset)
            .map(|product| subscription_response_types::SubscriptionItems {
                subscription_provider_item_id: product.id,
                name: product.name,
                description: product.description,
            })
            .collect();
        Ok(Self {
            response: Ok(subscription_response_types::GetSubscriptionItemsResponse { list }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripebillingRecurringPrice {
    pub interval: StripebillingPeriodUnit,
    pub interval_count: i64,
    pub trial_period_days: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StripebillingPeriodUnit {
    Day,
    Week,
    Month,
    Year,
}

impl From<StripebillingPeriodUnit> for subscription_response_types::PeriodUnit {
    fn from(value: StripebillingPeriodUnit) -> Self {
        match value {
            StripebillingPeriodUnit::Day => Self::Day,
            StripebillingPeriodUnit::Week => Self::Week,
            StripebillingPeriodUnit::Month => Self::Month,
            StripebillingPeriodUnit::Year => Self::Year,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripebillingPrice {
    pub id: String,
    pub product: Option<String>,
    pub unit_amount: Option<MinorUnit>,
    pub currency: String,
    pub recurring: Option<StripebillingRecurringPrice>,
}

fn parse_currency(value: &str) -> Result<common_enums::Currency, errors::ConnectorError> {
    common_enums::Currency::from_str(&value.to_uppercase())
        .map_err(|_| errors::ConnectorError::ResponseDeserializationFailed)
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionItemPrices,
            StripebillingListResponse<StripebillingPrice>,
            subscription_request_types::GetSubscriptionItemPricesRequest,
            subscription_response_types::GetSubscriptionItemPricesResponse,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionItemPrices,
        subscription_request_types::GetSubscriptionItemPricesRequest,
        subscription_response_types::GetSubscriptionItemPricesResponse,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionItemPrices,
            StripebillingListResponse<StripebillingPrice>,
            subscription_request_types::GetSubscriptionItemPricesRequest,
            subscription_response_types::GetSubscriptionItemPricesResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let list = item
            .response
            .data
            .into_iter()
            .filter_map(|mut price| price.recurring.take().map(|recurring| (price, recurring)))
            .map(|(price, recurring)| {
                Ok(subscription_response_types::SubscriptionItemPrices {
                    price_id: price.id,
                    item_id: price.product,
                    amount: price.unit_amount.unwrap_or(MinorUnit::new(0)),
                    currency: parse_currency(&price.currency)?,
                    interval: recurring.interval.into(),
                    interval_count: recurring.interval_count,
                    trial_period: recurring.trial_period_days,
                    trial_period_unit: recurring
                        .trial_period_days
                        .map(|_| subscription_response_types::PeriodUnit::Day),
                })
            })
            .collect::<Result<Vec<_>, errors::ConnectorError>>()?;
        Ok(Self {
            response: Ok(subscription_response_types::GetSubscriptionItemPricesResponse { list }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionEstimate,
            StripebillingPrice,
            subscription_request_types::GetSubscriptionEstimateRequest,
            subscription_response_types::GetSubscriptionEstimateResponse,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionEstimate,
        subscription_request_types::GetSubscriptionEstimateRequest,
        subscription_response_types::GetSubscriptionEstimateResponse,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionEstimate,
            StripebillingPrice,
            subscription_request_types::GetSubscriptionEstimateRequest,
            subscription_response_types::GetSubscriptionEstimateResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let amount = item.response.unit_amount.unwrap_or(MinorUnit::new(0));
        let currency = parse_currency(&item.response.currency)?;
        let line_item = subscription_response_types::SubscriptionLineItem {
            item_id: item.response.id,
            item_type: "plan".to_string(),
            description: "Stripe recurring price".to_string(),
            amount,
            currency,
            unit_amount: Some(amount),
            quantity: 1,
            pricing_model: Some("per_unit".to_string()),
        };
        Ok(Self {
            response: Ok(
                subscription_response_types::GetSubscriptionEstimateResponse {
                    sub_total: amount,
                    total: amount,
                    credits_applied: None,
                    amount_paid: None,
                    amount_due: Some(amount),
                    currency,
                    next_billing_at: None,
                    line_items: vec![line_item],
                    customer_id: None,
                },
            ),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct StripebillingSubscriptionCreateRequest {
    pub customer: String,
    #[serde(rename = "items[0][price]")]
    pub price: String,
    #[serde(rename = "items[0][quantity]")]
    pub quantity: u32,
    pub collection_method: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_until_due: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_payment_method: Option<Secret<String>>,
    #[serde(rename = "metadata[hyperswitch_subscription_id]")]
    pub hyperswitch_subscription_id: String,
    #[serde(rename = "expand[0]")]
    pub expand_latest_invoice: &'static str,
}

impl TryFrom<&SubscriptionCreateRouterData> for StripebillingSubscriptionCreateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &SubscriptionCreateRouterData) -> Result<Self, Self::Error> {
        let subscription_item = item.request.subscription_items.first().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "subscription_items".into(),
            },
        )?;
        let customer = item.request.connector_customer_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_customer_id".into(),
            },
        )?;
        let (collection_method, days_until_due, default_payment_method) =
            match item.request.auto_collection {
                subscription_request_types::SubscriptionAutoCollection::On => (
                    "charge_automatically",
                    None,
                    Some(item.request.default_payment_method.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "default_payment_method".into(),
                        },
                    )?),
                ),
                subscription_request_types::SubscriptionAutoCollection::Off => {
                    ("send_invoice", Some(1), None)
                }
            };
        Ok(Self {
            customer,
            price: subscription_item.item_price_id.clone(),
            quantity: subscription_item.quantity.unwrap_or(1),
            collection_method,
            days_until_due,
            default_payment_method,
            hyperswitch_subscription_id: item.request.subscription_id.get_string_repr().to_string(),
            expand_latest_invoice: "latest_invoice",
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingSubscriptionStatus {
    Incomplete,
    IncompleteExpired,
    Trialing,
    Active,
    PastDue,
    Canceled,
    Unpaid,
    Paused,
}

impl From<StripebillingSubscriptionStatus> for subscription_response_types::SubscriptionStatus {
    fn from(value: StripebillingSubscriptionStatus) -> Self {
        match value {
            StripebillingSubscriptionStatus::Incomplete => Self::Pending,
            StripebillingSubscriptionStatus::IncompleteExpired => Self::Failed,
            StripebillingSubscriptionStatus::Trialing => Self::Trial,
            StripebillingSubscriptionStatus::Active => Self::Active,
            StripebillingSubscriptionStatus::PastDue | StripebillingSubscriptionStatus::Unpaid => {
                Self::Unpaid
            }
            StripebillingSubscriptionStatus::Canceled => Self::Cancelled,
            StripebillingSubscriptionStatus::Paused => Self::Paused,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingInvoiceStatus {
    Draft,
    Open,
    Paid,
    Uncollectible,
    Void,
}

impl From<StripebillingInvoiceStatus> for common_enums::connector_enums::InvoiceStatus {
    fn from(value: StripebillingInvoiceStatus) -> Self {
        match value {
            StripebillingInvoiceStatus::Draft | StripebillingInvoiceStatus::Open => {
                Self::InvoiceCreated
            }
            StripebillingInvoiceStatus::Paid => Self::InvoicePaid,
            StripebillingInvoiceStatus::Uncollectible | StripebillingInvoiceStatus::Void => {
                Self::PaymentFailed
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripebillingInvoice {
    pub id: String,
    pub amount_due: MinorUnit,
    pub currency: String,
    pub status: Option<StripebillingInvoiceStatus>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StripebillingExpandableInvoice {
    Id(String),
    Object(StripebillingInvoice),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripebillingSubscriptionResponse {
    pub id: String,
    pub status: StripebillingSubscriptionStatus,
    pub customer: String,
    pub created: i64,
    pub current_period_end: Option<i64>,
    pub latest_invoice: Option<StripebillingExpandableInvoice>,
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCreate,
            StripebillingSubscriptionResponse,
            subscription_request_types::SubscriptionCreateRequest,
            subscription_response_types::SubscriptionCreateResponse,
        >,
    > for SubscriptionCreateRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCreate,
            StripebillingSubscriptionResponse,
            subscription_request_types::SubscriptionCreateRequest,
            subscription_response_types::SubscriptionCreateResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let connector_subscription_id = parse_subscription_id(item.response.id.clone())?;
        let customer_id = item.data.request.customer_id.clone();
        let invoice_details = match item.response.latest_invoice {
            Some(StripebillingExpandableInvoice::Object(invoice)) => {
                Some(subscription_response_types::SubscriptionInvoiceData {
                    id: parse_invoice_id(invoice.id)?,
                    total: invoice.amount_due,
                    currency_code: parse_currency(&invoice.currency)?,
                    status: invoice.status.map(Into::into),
                    billing_address: None,
                })
            }
            _ => None,
        };
        let currency_code = invoice_details
            .as_ref()
            .map(|invoice| invoice.currency_code)
            .unwrap_or(common_enums::Currency::USD);
        let total_amount = invoice_details
            .as_ref()
            .map(|invoice| invoice.total)
            .unwrap_or(MinorUnit::new(0));
        let response = subscription_response_types::SubscriptionCreateResponse {
            subscription_id: connector_subscription_id,
            status: item.response.status.into(),
            customer_id,
            currency_code,
            total_amount,
            next_billing_at: item
                .response
                .current_period_end
                .map(primitive_date_time_from_timestamp)
                .transpose()?,
            created_at: Some(primitive_date_time_from_timestamp(item.response.created)?),
            invoice_details,
        };
        Ok(Self {
            response: Ok(response),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionPause,
            StripebillingSubscriptionResponse,
            subscription_request_types::SubscriptionPauseRequest,
            subscription_response_types::SubscriptionPauseResponse,
        >,
    > for SubscriptionPauseRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionPause,
            StripebillingSubscriptionResponse,
            subscription_request_types::SubscriptionPauseRequest,
            subscription_response_types::SubscriptionPauseResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let subscription_id = parse_subscription_id(item.response.id)?;
        Ok(Self {
            response: Ok(subscription_response_types::SubscriptionPauseResponse {
                subscription_id,
                status: subscription_response_types::SubscriptionStatus::Paused,
                paused_at: Some(common_utils::date_time::now()),
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionResume,
            StripebillingSubscriptionResponse,
            subscription_request_types::SubscriptionResumeRequest,
            subscription_response_types::SubscriptionResumeResponse,
        >,
    > for SubscriptionResumeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionResume,
            StripebillingSubscriptionResponse,
            subscription_request_types::SubscriptionResumeRequest,
            subscription_response_types::SubscriptionResumeResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let subscription_id = parse_subscription_id(item.response.id)?;
        Ok(Self {
            response: Ok(subscription_response_types::SubscriptionResumeResponse {
                subscription_id,
                status: item.response.status.into(),
                next_billing_at: item
                    .response
                    .current_period_end
                    .map(primitive_date_time_from_timestamp)
                    .transpose()?,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCancel,
            StripebillingSubscriptionResponse,
            subscription_request_types::SubscriptionCancelRequest,
            subscription_response_types::SubscriptionCancelResponse,
        >,
    > for SubscriptionCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCancel,
            StripebillingSubscriptionResponse,
            subscription_request_types::SubscriptionCancelRequest,
            subscription_response_types::SubscriptionCancelResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let subscription_id = parse_subscription_id(item.response.id)?;
        let cancelled_at = matches!(
            item.data.request.cancel_option,
            None | Some(api_models::subscription::CancelOption::Immediately)
        )
        .then(common_utils::date_time::now);
        Ok(Self {
            response: Ok(subscription_response_types::SubscriptionCancelResponse {
                subscription_id,
                status: item.response.status.into(),
                cancelled_at,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct StripebillingPauseRequest {
    #[serde(rename = "pause_collection[behavior]")]
    pub behavior: &'static str,
}

#[derive(Debug, Serialize)]
pub struct StripebillingResumeRequest {
    pub pause_collection: &'static str,
}

#[derive(Debug, Serialize)]
pub struct StripebillingCancelAtPeriodEndRequest {
    pub cancel_at_period_end: bool,
}

#[derive(Debug, Serialize)]
pub struct StripebillingCancelAtRequest {
    pub cancel_at: i64,
}

//TODO: Fill the struct with respective fields
pub struct StripebillingRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for StripebillingRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct StripebillingPaymentsRequest {
    amount: StringMinorUnit,
    card: StripebillingCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct StripebillingCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&StripebillingRouterData<&PaymentsAuthorizeRouterData>>
    for StripebillingPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StripebillingRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = StripebillingCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct StripebillingAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for StripebillingAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum StripebillingPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<StripebillingPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: StripebillingPaymentStatus) -> Self {
        match item {
            StripebillingPaymentStatus::Succeeded => Self::Charged,
            StripebillingPaymentStatus::Failed => Self::Failure,
            StripebillingPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StripebillingPaymentsResponse {
    status: StripebillingPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, StripebillingPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, StripebillingPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
                payment_account_reference: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct StripebillingRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&StripebillingRouterData<&RefundsRouterData<F>>> for StripebillingRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StripebillingRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone, Copy)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
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

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
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
pub struct StripebillingErrorResponse {
    pub error: StripebillingErrorDetails,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct StripebillingErrorDetails {
    pub code: Option<String>,
    pub message: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub param: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripebillingWebhookBody {
    #[serde(rename = "type")]
    pub event_type: StripebillingEventType,
    pub data: StripebillingWebhookData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripebillingInvoiceBody {
    #[serde(rename = "type")]
    pub event_type: StripebillingEventType,
    pub data: StripebillingInvoiceData,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingEventType {
    #[serde(rename = "invoice.created")]
    InvoiceGenerated,
    #[serde(rename = "invoice.paid", alias = "invoice.payment_succeeded")]
    PaymentSucceeded,
    #[serde(rename = "invoice.payment_failed")]
    PaymentFailed,
    #[serde(rename = "invoice.voided")]
    InvoiceDeleted,
    #[serde(rename = "customer.subscription.updated")]
    SubscriptionUpdated,
    #[serde(rename = "customer.subscription.deleted")]
    SubscriptionDeleted,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookEventBody {
    #[serde(rename = "type")]
    pub event_type: StripebillingEventType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingSubscriptionWebhookBody {
    #[serde(rename = "type")]
    pub event_type: StripebillingEventType,
    pub data: StripebillingSubscriptionWebhookData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingSubscriptionWebhookData {
    pub object: StripebillingSubscriptionWebhookObject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingSubscriptionWebhookObject {
    pub id: String,
    pub status: StripebillingSubscriptionStatus,
    pub pause_collection: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl StripebillingSubscriptionWebhookBody {
    pub fn subscription_webhook_data(
        self,
    ) -> CustomResult<
        hyperswitch_domain_models::router_flow_types::SubscriptionWebhookData,
        errors::ConnectorError,
    > {
        let status = match self.event_type {
            StripebillingEventType::SubscriptionDeleted => {
                common_enums::SubscriptionStatus::Cancelled
            }
            StripebillingEventType::SubscriptionUpdated
                if self.data.object.pause_collection.is_some() =>
            {
                common_enums::SubscriptionStatus::Paused
            }
            StripebillingEventType::SubscriptionUpdated => common_enums::SubscriptionStatus::from(
                subscription_response_types::SubscriptionStatus::from(self.data.object.status),
            ),
            _ => return Err(errors::ConnectorError::WebhookBodyDecodingFailed.into()),
        };

        Ok(
            hyperswitch_domain_models::router_flow_types::SubscriptionWebhookData {
                connector_subscription_id: parse_subscription_id(self.data.object.id.clone())
                    .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?,
                hyperswitch_subscription_id: self
                    .data
                    .object
                    .metadata
                    .get("hyperswitch_subscription_id")
                    .cloned()
                    .map(parse_subscription_id)
                    .transpose()
                    .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?,
                hyperswitch_subscription_binding: self
                    .data
                    .object
                    .metadata
                    .get("hyperswitch_subscription_binding")
                    .cloned(),
                status,
            },
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookData {
    pub object: StripebillingWebhookObject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingInvoiceData {
    pub object: StripebillingWebhookObject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookObject {
    #[serde(rename = "id")]
    pub invoice_id: String,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    pub customer: String,
    #[serde(rename = "amount_remaining")]
    pub amount: MinorUnit,
    pub amount_due: Option<MinorUnit>,
    pub amount_paid: Option<MinorUnit>,
    pub charge: Option<String>,
    pub payment_intent: Option<String>,
    pub subscription: Option<String>,
    pub subscription_details: Option<StripebillingInvoiceSubscriptionDetails>,
    pub parent: Option<StripebillingInvoiceParent>,
    pub status: Option<StripebillingInvoiceStatus>,
    pub billing_reason: Option<String>,
    pub customer_address: Option<StripebillingInvoiceBillingAddress>,
    pub attempt_count: u16,
    pub lines: StripebillingWebhookLinesObject,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripebillingInvoiceParent {
    pub subscription_details: Option<StripebillingInvoiceSubscriptionDetails>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripebillingInvoiceSubscriptionDetails {
    pub subscription: Option<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl StripebillingWebhookObject {
    pub fn subscription_mit_payment_data(
        &self,
    ) -> CustomResult<
        hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData,
        errors::ConnectorError,
    > {
        let invoice_id = parse_invoice_id(self.invoice_id.clone())
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let connector_subscription_id = self
            .subscription
            .clone()
            .or_else(|| {
                self.parent
                    .as_ref()
                    .and_then(|parent| parent.subscription_details.as_ref())
                    .and_then(|details| details.subscription.clone())
            })
            .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let subscription_id = parse_subscription_id(connector_subscription_id)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let customer_id = parse_customer_id(self.customer.clone())
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let amount_due = match self.status.as_ref() {
            Some(StripebillingInvoiceStatus::Paid) => {
                self.amount_paid.or(self.amount_due).unwrap_or(self.amount)
            }
            _ => self.amount,
        };

        Ok(
            hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData {
                invoice_id,
                amount_due,
                currency_code: self.currency,
                status: self.status.clone().map(Into::into),
                customer_id,
                subscription_id,
                hyperswitch_subscription_id: self
                    .subscription_details
                    .as_ref()
                    .or_else(|| {
                        self.parent
                            .as_ref()
                            .and_then(|parent| parent.subscription_details.as_ref())
                    })
                    .and_then(|details| details.metadata.get("hyperswitch_subscription_id"))
                    .cloned()
                    .map(parse_subscription_id)
                    .transpose()
                    .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?,
                hyperswitch_subscription_binding: self
                    .subscription_details
                    .as_ref()
                    .or_else(|| {
                        self.parent
                            .as_ref()
                            .and_then(|parent| parent.subscription_details.as_ref())
                    })
                    .and_then(|details| details.metadata.get("hyperswitch_subscription_binding"))
                    .cloned(),
                first_invoice: self.billing_reason.as_deref() == Some("subscription_create"),
                billing_period_end: self.lines.data.iter().map(|line| line.period.end).max(),
            },
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookLinesObject {
    pub data: Vec<StripebillingWebhookLinesData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookLinesData {
    pub period: StripebillingWebhookLineDataPeriod,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookLineDataPeriod {
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub end: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub start: PrimitiveDateTime,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingInvoiceBillingAddress {
    pub country: Option<enums::CountryAlpha2>,
    pub city: Option<String>,
    pub address_line1: Option<Secret<String>>,
    pub address_line2: Option<Secret<String>>,
    pub zip_code: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripebillingInvoiceObject {
    #[serde(rename = "id")]
    pub invoice_id: String,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    #[serde(rename = "amount_remaining")]
    pub amount: MinorUnit,
    pub attempt_count: Option<u16>,
}

impl StripebillingWebhookBody {
    pub fn get_webhook_object_from_body(body: &[u8]) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body: Self = body
            .parse_struct::<Self>("StripebillingWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(webhook_body)
    }
}

impl StripebillingInvoiceBody {
    pub fn get_invoice_webhook_data_from_body(
        body: &[u8],
    ) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("StripebillingInvoiceBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }

    pub fn subscription_mit_payment_data(
        &self,
    ) -> CustomResult<
        hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData,
        errors::ConnectorError,
    > {
        let mut payment_data = self.data.object.subscription_mit_payment_data()?;

        // Stripe 续费失败后通常仍将账单保持为 open，因此必须结合事件类型判断最终结果。
        payment_data.status = match self.event_type {
            StripebillingEventType::PaymentSucceeded => {
                Some(common_enums::connector_enums::InvoiceStatus::InvoicePaid)
            }
            StripebillingEventType::PaymentFailed => {
                Some(common_enums::connector_enums::InvoiceStatus::PaymentFailed)
            }
            StripebillingEventType::InvoiceGenerated
            | StripebillingEventType::InvoiceDeleted
            | StripebillingEventType::SubscriptionUpdated
            | StripebillingEventType::SubscriptionDeleted => payment_data.status,
        };

        Ok(payment_data)
    }
}

impl From<StripebillingInvoiceBillingAddress> for api_models::payments::Address {
    fn from(item: StripebillingInvoiceBillingAddress) -> Self {
        Self {
            address: Some(api_models::payments::AddressDetails::from(item)),
            phone: None,
            email: None,
        }
    }
}

impl From<StripebillingInvoiceBillingAddress> for api_models::payments::AddressDetails {
    fn from(item: StripebillingInvoiceBillingAddress) -> Self {
        Self {
            city: item.city,
            state: item.state,
            country: item.country,
            zip: item.zip_code,
            line1: item.address_line1,
            line2: item.address_line2,
            line3: None,
            first_name: None,
            last_name: None,
            origin_zip: None,
        }
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<StripebillingInvoiceBody> for revenue_recovery::RevenueRecoveryInvoiceData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: StripebillingInvoiceBody) -> Result<Self, Self::Error> {
        let merchant_reference_id =
            id_type::PaymentReferenceId::from_str(&item.data.object.invoice_id)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let next_billing_at = item
            .data
            .object
            .lines
            .data
            .first()
            .map(|linedata| linedata.period.end);
        let billing_started_at = item
            .data
            .object
            .lines
            .data
            .first()
            .map(|linedata| linedata.period.start);
        Ok(Self {
            amount: item.data.object.amount,
            currency: item.data.object.currency,
            merchant_reference_id,
            billing_address: item
                .data
                .object
                .customer_address
                .map(api_models::payments::Address::from),
            retry_count: Some(item.data.object.attempt_count),
            next_billing_at,
            billing_started_at,
            metadata: None,
            // TODO! This field should be handled for billing connnector integrations
            enable_partial_authorization: None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripebillingRecoveryDetailsData {
    #[serde(rename = "id")]
    pub charge_id: String,
    pub status: StripebillingChargeStatus,
    pub amount: MinorUnit,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    pub customer: String,
    pub payment_method: String,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub created: PrimitiveDateTime,
    pub payment_method_details: StripePaymentMethodDetails,
    #[serde(rename = "invoice")]
    pub invoice_id: String,
    pub payment_intent: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripePaymentMethodDetails {
    #[serde(rename = "type")]
    pub type_of_payment_method: StripebillingPaymentMethod,
    #[serde(rename = "card")]
    pub card_details: StripeBillingCardDetails,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingPaymentMethod {
    Card,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripeBillingCardDetails {
    pub network: StripebillingCardNetwork,
    pub funding: StripebillingFundingTypes,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingCardNetwork {
    Visa,
    Mastercard,
    AmericanExpress,
    JCB,
    DinersClub,
    Discover,
    CartesBancaires,
    UnionPay,
    Interac,
    RuPay,
    Maestro,
    Star,
    Pulse,
    Accel,
    Nyce,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename = "snake_case")]
pub enum StripebillingFundingTypes {
    #[serde(rename = "credit")]
    Credit,
    #[serde(rename = "debit")]
    Debit,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingChargeStatus {
    Succeeded,
    Failed,
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
// This is the default hard coded mca Id to find the stripe account associated with the stripe biliing
// Context : Since we dont have the concept of connector_reference_id in stripebilling because payments always go through stripe.
// While creating stripebilling we will hard code the stripe account id to string "stripebilling" in mca feature metadata. So we have to pass the same as account_reference_id here in response.
const MCA_ID_IDENTIFIER_FOR_STRIPE_IN_STRIPEBILLING_MCA_FEAATURE_METADATA: &str = "stripebilling";

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    TryFrom<
        ResponseRouterData<
            recovery_router_flows::BillingConnectorPaymentsSync,
            StripebillingRecoveryDetailsData,
            recovery_request_types::BillingConnectorPaymentsSyncRequest,
            recovery_response_types::BillingConnectorPaymentsSyncResponse,
        >,
    > for recovery_router_data_types::BillingConnectorPaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            recovery_router_flows::BillingConnectorPaymentsSync,
            StripebillingRecoveryDetailsData,
            recovery_request_types::BillingConnectorPaymentsSyncRequest,
            recovery_response_types::BillingConnectorPaymentsSyncResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let charge_details = item.response;
        let merchant_reference_id =
            id_type::PaymentReferenceId::from_str(charge_details.invoice_id.as_str())
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "invoice_id".into(),
                })?;
        let connector_transaction_id = Some(common_utils::types::ConnectorTransactionId::from(
            charge_details.payment_intent,
        ));

        Ok(Self {
            response: Ok(
                recovery_response_types::BillingConnectorPaymentsSyncResponse {
                    status: charge_details.status.into(),
                    amount: charge_details.amount,
                    currency: charge_details.currency,
                    merchant_reference_id,
                    connector_account_reference_id:
                        MCA_ID_IDENTIFIER_FOR_STRIPE_IN_STRIPEBILLING_MCA_FEAATURE_METADATA
                            .to_string(),
                    connector_transaction_id,
                    error_code: charge_details.failure_code,
                    error_message: charge_details.failure_message,
                    processor_payment_method_token: charge_details.payment_method,
                    connector_customer_id: charge_details.customer,
                    transaction_created_at: Some(charge_details.created),
                    payment_method_sub_type: common_enums::PaymentMethodType::from(
                        charge_details.payment_method_details.card_details.funding,
                    ),
                    payment_method_type: common_enums::PaymentMethod::from(
                        charge_details.payment_method_details.type_of_payment_method,
                    ),
                    // Todo: Fetch Card issuer details. Generally in the other billing connector we are getting card_issuer using the card bin info. But stripe dosent provide any such details. We should find a way for stripe billing case
                    charge_id: Some(charge_details.charge_id.clone()),
                    // Need to populate these card info field
                    card_info: api_models::payments::AdditionalCardInfo {
                        card_network: Some(common_enums::CardNetwork::from(
                            charge_details.payment_method_details.card_details.network,
                        )),
                        card_isin: None,
                        card_issuer: None,
                        card_type: None,
                        card_subtype: None,
                        card_segment_type: None,
                        funding_source: None,
                        card_issuing_country: None,
                        card_issuing_country_code: None,
                        bank_code: None,
                        last4: None,
                        card_extended_bin: None,
                        card_exp_month: None,
                        card_exp_year: None,
                        card_holder_name: None,
                        payment_checks: None,
                        authentication_data: None,
                        is_regulated: None,
                        signature_network: None,
                        auth_code: None,
                    },
                },
            ),
            ..item.data
        })
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<StripebillingChargeStatus> for enums::AttemptStatus {
    fn from(status: StripebillingChargeStatus) -> Self {
        match status {
            StripebillingChargeStatus::Succeeded => Self::Charged,
            StripebillingChargeStatus::Failed => Self::Failure,
        }
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<StripebillingFundingTypes> for common_enums::PaymentMethodType {
    fn from(funding: StripebillingFundingTypes) -> Self {
        match funding {
            StripebillingFundingTypes::Credit => Self::Credit,
            StripebillingFundingTypes::Debit => Self::Debit,
        }
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<StripebillingPaymentMethod> for common_enums::PaymentMethod {
    fn from(method: StripebillingPaymentMethod) -> Self {
        match method {
            StripebillingPaymentMethod::Card => Self::Card,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StripebillingRecordBackResponse {
    pub id: String,
}

#[cfg(feature = "v1")]
impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::revenue_recovery::InvoiceRecordBack,
            StripebillingRecordBackResponse,
            hyperswitch_domain_models::router_request_types::revenue_recovery::InvoiceRecordBackRequest,
            hyperswitch_domain_models::router_response_types::revenue_recovery::InvoiceRecordBackResponse,
        >,
    > for hyperswitch_domain_models::types::InvoiceRecordBackRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::revenue_recovery::InvoiceRecordBack,
            StripebillingRecordBackResponse,
            hyperswitch_domain_models::router_request_types::revenue_recovery::InvoiceRecordBackRequest,
            hyperswitch_domain_models::router_response_types::revenue_recovery::InvoiceRecordBackResponse,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(
                hyperswitch_domain_models::router_response_types::revenue_recovery::InvoiceRecordBackResponse {
                    merchant_reference_id: id_type::PaymentReferenceId::from_str(&item.response.id)
                        .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
                    connector_transaction_id: None,
                },
            ),
            ..item.data
        })
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    TryFrom<
        ResponseRouterData<
            recovery_router_flows::InvoiceRecordBack,
            StripebillingRecordBackResponse,
            recovery_request_types::InvoiceRecordBackRequest,
            recovery_response_types::InvoiceRecordBackResponse,
        >,
    > for recovery_router_data_types::InvoiceRecordBackRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            recovery_router_flows::InvoiceRecordBack,
            StripebillingRecordBackResponse,
            recovery_request_types::InvoiceRecordBackRequest,
            recovery_response_types::InvoiceRecordBackResponse,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(recovery_response_types::InvoiceRecordBackResponse {
                merchant_reference_id: id_type::PaymentReferenceId::from_str(
                    item.response.id.as_str(),
                )
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "invoice_id in the response".into(),
                })?,
                // Stripe Billing's record-back does not return a usable transaction id.
                connector_transaction_id: None,
            }),
            ..item.data
        })
    }
}

impl From<StripebillingCardNetwork> for enums::CardNetwork {
    fn from(item: StripebillingCardNetwork) -> Self {
        match item {
            StripebillingCardNetwork::Visa => Self::Visa,
            StripebillingCardNetwork::Mastercard => Self::Mastercard,
            StripebillingCardNetwork::AmericanExpress => Self::AmericanExpress,
            StripebillingCardNetwork::JCB => Self::JCB,
            StripebillingCardNetwork::DinersClub => Self::DinersClub,
            StripebillingCardNetwork::Discover => Self::Discover,
            StripebillingCardNetwork::CartesBancaires => Self::CartesBancaires,
            StripebillingCardNetwork::UnionPay => Self::UnionPay,
            StripebillingCardNetwork::Interac => Self::Interac,
            StripebillingCardNetwork::RuPay => Self::RuPay,
            StripebillingCardNetwork::Maestro => Self::Maestro,
            StripebillingCardNetwork::Star => Self::Star,
            StripebillingCardNetwork::Pulse => Self::Pulse,
            StripebillingCardNetwork::Accel => Self::Accel,
            StripebillingCardNetwork::Nyce => Self::Nyce,
        }
    }
}

#[cfg(test)]
mod subscription_tests {
    use std::marker::PhantomData;

    use common_utils::{id_type, types::MinorUnit};
    use hyperswitch_domain_models::{
        connector_endpoints::ConnectorParams,
        router_data::{ConnectorAuthType, RouterData},
        router_flow_types::subscriptions::SubscriptionCreate,
        router_request_types::subscriptions::{
            SubscriptionAutoCollection, SubscriptionCreateRequest, SubscriptionItem,
        },
        router_response_types::subscriptions::SubscriptionCreateResponse,
    };

    use crate::types::ResponseRouterData;

    use super::*;

    fn subscription_create_router_data(
        items: Vec<SubscriptionItem>,
        connector_customer_id: Option<&str>,
        default_payment_method: Option<&str>,
    ) -> RouterData<SubscriptionCreate, SubscriptionCreateRequest, SubscriptionCreateResponse> {
        RouterData {
            flow: PhantomData,
            merchant_id: id_type::MerchantId::get_irrelevant_merchant_id(),
            customer_id: None,
            connector_customer: None,
            connector: "stripebilling".to_string(),
            payment_id: "irrelevant payment in subscription create flow".to_string(),
            attempt_id: "irrelevant attempt in subscription create flow".to_string(),
            tenant_id: id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            status: Default::default(),
            payment_method: Default::default(),
            payment_method_type: None,
            connector_auth_type: ConnectorAuthType::default(),
            description: None,
            address: Default::default(),
            auth_type: Default::default(),
            connector_meta_data: None,
            connector_wallets_details: None,
            amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            payment_method_balance: None,
            connector_api_version: None,
            request: SubscriptionCreateRequest {
                customer_id: id_type::CustomerId::try_from(std::borrow::Cow::Borrowed(
                    "customer_hyperswitch",
                ))
                .unwrap(),
                connector_customer_id: connector_customer_id.map(str::to_string),
                subscription_id: id_type::SubscriptionId::try_from(std::borrow::Cow::Borrowed(
                    "subscription_hyperswitch",
                ))
                .unwrap(),
                subscription_items: items,
                default_payment_method: default_payment_method
                    .map(|value| Secret::new(value.to_string())),
                billing_address: Default::default(),
                auto_collection: SubscriptionAutoCollection::On,
                connector_params: ConnectorParams::default(),
            },
            response: Err(Default::default()),
            connector_request_reference_id: "subscription_hyperswitch".to_string(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: Some(true),
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            dispute_id: None,
            refund_id: None,
            payout_id: None,
            connector_response: None,
            payment_method_status: None,
            minor_amount_captured: None,
            minor_amount_capturable: None,
            authorized_amount: None,
            integrity_check: Ok(()),
            additional_merchant_data: None,
            header_payload: None,
            connector_mandate_request_reference_id: None,
            l2_l3_data: None,
            authentication_id: None,
            psd2_sca_exemption_type: None,
            raw_connector_response: None,
            is_payment_id_from_merchant: None,
            customer_document_details: None,
            customer_date_of_birth: None,
            feature_data: None,
            sender_payment_instrument_id: None,
            connector_returned_payment_method_details: None,
        }
    }

    #[test]
    fn subscription_creation_maps_request_and_success_response() {
        let data = subscription_create_router_data(
            vec![SubscriptionItem {
                item_price_id: "price_monthly".to_string(),
                quantity: Some(2),
            }],
            Some("cus_stripe"),
            Some("pm_saved"),
        );
        let request = StripebillingSubscriptionCreateRequest::try_from(&data).unwrap();

        assert_eq!(request.customer, "cus_stripe");
        assert_eq!(request.price, "price_monthly");
        assert_eq!(request.quantity, 2);
        assert_eq!(request.collection_method, "charge_automatically");
        assert_eq!(
            request.default_payment_method,
            Some(Secret::new("pm_saved".to_string()))
        );

        let transformed = SubscriptionCreateRouterData::try_from(ResponseRouterData {
            response: StripebillingSubscriptionResponse {
                id: "sub_stripe".to_string(),
                status: StripebillingSubscriptionStatus::Active,
                customer: "cus_stripe".to_string(),
                created: 1_767_225_600,
                current_period_end: Some(1_769_904_000),
                latest_invoice: Some(StripebillingExpandableInvoice::Object(
                    StripebillingInvoice {
                        id: "in_first".to_string(),
                        amount_due: MinorUnit::new(1_000),
                        currency: "usd".to_string(),
                        status: Some(StripebillingInvoiceStatus::Paid),
                    },
                )),
            },
            data,
            http_code: 200,
        })
        .unwrap();
        let response = transformed.response.unwrap();

        assert_eq!(response.subscription_id.get_string_repr(), "sub_stripe");
        assert_eq!(
            response.status,
            subscription_response_types::SubscriptionStatus::Active
        );
        assert_eq!(response.total_amount, MinorUnit::new(1_000));
        assert_eq!(response.currency_code, common_enums::Currency::USD);
        assert_eq!(
            response
                .invoice_details
                .as_ref()
                .map(|invoice| invoice.id.get_string_repr()),
            Some("in_first")
        );
    }

    #[test]
    fn subscription_creation_rejects_missing_connector_customer_or_payment_method() {
        let item = SubscriptionItem {
            item_price_id: "price_monthly".to_string(),
            quantity: Some(1),
        };

        assert!(StripebillingSubscriptionCreateRequest::try_from(
            &subscription_create_router_data(vec![item.clone()], None, Some("pm_saved"))
        )
        .is_err());
        assert!(StripebillingSubscriptionCreateRequest::try_from(
            &subscription_create_router_data(vec![item], Some("cus_stripe"), None)
        )
        .is_err());
    }

    #[test]
    fn subscription_creation_rejects_connector_response_with_invalid_identifiers() {
        let data = subscription_create_router_data(
            vec![SubscriptionItem {
                item_price_id: "price_monthly".to_string(),
                quantity: Some(1),
            }],
            Some("cus_stripe"),
            Some("pm_saved"),
        );

        let result = SubscriptionCreateRouterData::try_from(ResponseRouterData {
            response: StripebillingSubscriptionResponse {
                id: "invalid subscription id".to_string(),
                status: StripebillingSubscriptionStatus::Active,
                customer: "cus_stripe".to_string(),
                created: 1_767_225_600,
                current_period_end: None,
                latest_invoice: None,
            },
            data,
            http_code: 200,
        });

        assert!(result.is_err());
    }

    #[test]
    fn stripe_subscription_uses_automatic_collection() {
        let request = StripebillingSubscriptionCreateRequest {
            customer: "cus_test".to_string(),
            price: "price_monthly".to_string(),
            quantity: 1,
            collection_method: "charge_automatically",
            days_until_due: None,
            default_payment_method: Some(Secret::new("pm_saved".to_string())),
            hyperswitch_subscription_id: "sub_hyperswitch".to_string(),
            expand_latest_invoice: "latest_invoice",
        };

        assert_eq!(request.collection_method, "charge_automatically");
        let form = serde_urlencoded::to_string(request)
            .expect("Stripe automatic-collection subscription request should serialize");
        assert!(form.contains("default_payment_method=pm_saved"));
        assert!(!form.contains("days_until_due"));
    }

    #[test]
    fn parses_subscription_deleted_webhook_event() {
        let event =
            serde_json::from_str::<StripebillingEventType>(r#""customer.subscription.deleted""#)
                .expect("Stripe subscription.deleted event should parse");

        assert!(matches!(event, StripebillingEventType::SubscriptionDeleted));
    }

    #[test]
    fn subscription_updated_with_pause_collection_maps_to_paused() {
        let webhook: StripebillingSubscriptionWebhookBody = serde_json::from_str(
            r#"{
                "type":"customer.subscription.updated",
                "data":{"object":{
                    "id":"sub_stripe",
                    "status":"active",
                    "pause_collection":{"behavior":"void"},
                    "metadata":{"hyperswitch_subscription_id":"sub_hyperswitch"}
                }}
            }"#,
        )
        .expect("Stripe subscription.updated event should parse");

        let data = webhook
            .subscription_webhook_data()
            .expect("paused status should be extracted");

        assert_eq!(
            data.connector_subscription_id.get_string_repr(),
            "sub_stripe"
        );
        assert!(matches!(
            data.status,
            common_enums::SubscriptionStatus::Paused
        ));
        assert_eq!(
            data.hyperswitch_subscription_id
                .as_ref()
                .map(|id| id.get_string_repr()),
            Some("sub_hyperswitch")
        );
    }

    #[test]
    fn subscription_deleted_maps_to_cancelled() {
        let webhook: StripebillingSubscriptionWebhookBody = serde_json::from_str(
            r#"{
                "type":"customer.subscription.deleted",
                "data":{"object":{
                    "id":"sub_stripe",
                    "status":"canceled",
                    "pause_collection":null
                }}
            }"#,
        )
        .expect("Stripe subscription.deleted event should parse");

        let data = webhook
            .subscription_webhook_data()
            .expect("cancelled status should be extracted");

        assert!(matches!(
            data.status,
            common_enums::SubscriptionStatus::Cancelled
        ));
    }

    #[test]
    fn parses_recurring_price_from_stripe() {
        let price: StripebillingPrice = serde_json::from_str(
            r#"{
                "id":"price_monthly",
                "product":"prod_hyperswitch",
                "unit_amount":500,
                "currency":"usd",
                "recurring":{"interval":"month","interval_count":1,"trial_period_days":null}
            }"#,
        )
        .expect("Stripe recurring price should deserialize");

        assert_eq!(price.id, "price_monthly");
        assert_eq!(price.unit_amount, Some(MinorUnit::new(500)));
        assert!(matches!(
            price.recurring.map(|recurring| recurring.interval),
            Some(StripebillingPeriodUnit::Month)
        ));
    }

    #[test]
    fn invoice_created_webhook_produces_renewal_data() {
        let webhook: StripebillingInvoiceBody = serde_json::from_str(
            r#"{
                "type":"invoice.created",
                "data":{"object":{
                    "id":"in_renewal",
                    "currency":"usd",
                    "customer":"cus_stripe",
                    "amount_remaining":500,
                    "charge":null,
                    "payment_intent":null,
                    "subscription":"sub_stripe",
                    "subscription_details":{
                        "subscription":"sub_stripe",
                        "metadata":{"hyperswitch_subscription_id":"sub_hyperswitch"}
                    },
                    "status":"open",
                    "billing_reason":"subscription_cycle",
                    "customer_address":null,
                    "attempt_count":0,
                    "lines":{"data":[{"period":{"start":1767225600,"end":1769904000}}]}
                }}
            }"#,
        )
        .expect("Stripe invoice.created webhook should deserialize");

        let mit_data = webhook
            .data
            .object
            .subscription_mit_payment_data()
            .expect("renewal data should be extracted");

        assert_eq!(mit_data.amount_due, MinorUnit::new(500));
        assert_eq!(mit_data.currency_code, common_enums::Currency::USD);
        assert_eq!(mit_data.subscription_id.get_string_repr(), "sub_stripe");
        assert!(mit_data.billing_period_end.is_some());
        assert!(!mit_data.first_invoice);
    }

    #[test]
    fn invoice_payment_succeeded_uses_paid_amount() {
        let webhook: StripebillingInvoiceBody = serde_json::from_str(
            r#"{
                "type":"invoice.payment_succeeded",
                "data":{"object":{
                    "id":"in_paid_renewal",
                    "currency":"usd",
                    "customer":"cus_stripe",
                    "amount_due":500,
                    "amount_paid":500,
                    "amount_remaining":0,
                    "charge":null,
                    "payment_intent":null,
                    "subscription":"sub_stripe",
                    "status":"paid",
                    "billing_reason":"subscription_cycle",
                    "customer_address":null,
                    "attempt_count":1,
                    "lines":{"data":[{"period":{"start":1767225600,"end":1769904000}}]}
                }}
            }"#,
        )
        .expect("Stripe invoice.payment_succeeded webhook should deserialize");

        let mit_data = webhook
            .subscription_mit_payment_data()
            .expect("paid renewal data should be extracted");

        assert_eq!(mit_data.amount_due, MinorUnit::new(500));
        assert!(matches!(
            mit_data.status,
            Some(common_enums::connector_enums::InvoiceStatus::InvoicePaid)
        ));
    }

    #[test]
    fn invoice_payment_failed_overrides_open_invoice_status() {
        let webhook: StripebillingInvoiceBody = serde_json::from_str(
            r#"{
                "type":"invoice.payment_failed",
                "data":{"object":{
                    "id":"in_failed_renewal",
                    "currency":"usd",
                    "customer":"cus_stripe",
                    "amount_due":500,
                    "amount_paid":0,
                    "amount_remaining":500,
                    "charge":null,
                    "payment_intent":null,
                    "subscription":"sub_stripe",
                    "status":"open",
                    "billing_reason":"subscription_cycle",
                    "customer_address":null,
                    "attempt_count":1,
                    "lines":{"data":[{"period":{"start":1767225600,"end":1769904000}}]}
                }}
            }"#,
        )
        .expect("Stripe invoice.payment_failed webhook should deserialize");

        let mit_data = webhook
            .subscription_mit_payment_data()
            .expect("failed renewal data should be extracted");

        assert_eq!(mit_data.amount_due, MinorUnit::new(500));
        assert!(matches!(
            mit_data.status,
            Some(common_enums::connector_enums::InvoiceStatus::PaymentFailed)
        ));
    }

    #[test]
    fn first_invoice_is_marked_as_cit() {
        let webhook: StripebillingInvoiceBody = serde_json::from_str(
            r#"{
                "type":"invoice.created",
                "data":{"object":{
                    "id":"in_first_invoice",
                    "currency":"usd",
                    "customer":"cus_stripe",
                    "amount_remaining":500,
                    "charge":null,
                    "payment_intent":null,
                    "subscription":"sub_stripe",
                    "subscription_details":{
                        "subscription":"sub_stripe",
                        "metadata":{"hyperswitch_subscription_id":"sub_hyperswitch"}
                    },
                    "status":"open",
                    "billing_reason":"subscription_create",
                    "customer_address":null,
                    "attempt_count":0,
                    "lines":{"data":[{"period":{"start":1767225600,"end":1769904000}}]}
                }}
            }"#,
        )
        .expect("first-invoice webhook should deserialize");

        let mit_data = webhook
            .data
            .object
            .subscription_mit_payment_data()
            .expect("first-invoice data should be extracted");

        assert!(mit_data.first_invoice);
        assert_eq!(
            mit_data
                .hyperswitch_subscription_id
                .as_ref()
                .map(|id| id.get_string_repr()),
            Some("sub_hyperswitch")
        );
    }

    #[test]
    fn parses_subscription_from_new_stripe_invoice_parent() {
        let webhook: StripebillingInvoiceBody = serde_json::from_str(
            r#"{
                "type":"invoice.created",
                "data":{"object":{
                    "id":"in_parent_shape",
                    "currency":"usd",
                    "customer":"cus_stripe",
                    "amount_remaining":500,
                    "charge":null,
                    "payment_intent":null,
                    "subscription":null,
                    "parent":{
                        "subscription_details":{"subscription":"sub_parent_shape"}
                    },
                    "status":"open",
                    "billing_reason":"subscription_cycle",
                    "customer_address":null,
                    "attempt_count":0,
                    "lines":{"data":[{"period":{"start":1767225600,"end":1769904000}}]}
                }}
            }"#,
        )
        .expect("new Stripe Invoice parent shape should deserialize");

        let mit_data = webhook
            .data
            .object
            .subscription_mit_payment_data()
            .expect("subscription identifier should be extracted from the Invoice parent");

        assert_eq!(
            mit_data.subscription_id.get_string_repr(),
            "sub_parent_shape"
        );
    }

    #[test]
    fn parses_stripe_error_envelope() {
        let response: StripebillingErrorResponse = serde_json::from_str(
            r#"{
                "error": {
                    "type": "invalid_request_error",
                    "code": "resource_missing",
                    "message": "No such price",
                    "param": "items[0][price]"
                }
            }"#,
        )
        .expect("Stripe error response should parse using the documented envelope");

        assert_eq!(response.error.code.as_deref(), Some("resource_missing"));
        assert_eq!(response.error.message.as_deref(), Some("No such price"));
    }
}
