use api_models::{payments, refunds};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{core::errors, types::api::enums as api_enums};

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct StripeBillingDetails {
    pub(crate) address: Option<payments::AddressDetails>,
    pub(crate) email: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) phone: Option<String>,
}

impl From<StripeBillingDetails> for payments::Address {
    fn from(details: StripeBillingDetails) -> Self {
        Self {
            address: details.address,
            phone: Some(payments::PhoneDetails {
                number: details.phone.map(masking::Secret::new),
                country_code: None,
            }),
        }
    }
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct StripeCard {
    pub(crate) number: String,
    pub(crate) exp_month: String,
    pub(crate) exp_year: String,
    pub(crate) cvc: String,
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StripePaymentMethodType {
    #[default]
    Card,
}

impl From<StripePaymentMethodType> for api_enums::PaymentMethodType {
    fn from(item: StripePaymentMethodType) -> Self {
        match item {
            StripePaymentMethodType::Card => api_enums::PaymentMethodType::Card,
        }
    }
}
#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct StripePaymentMethodData {
    #[serde(rename = "type")]
    pub(crate) stype: StripePaymentMethodType,
    pub(crate) billing_details: Option<StripeBillingDetails>,
    #[serde(flatten)]
    pub(crate) payment_method_details: Option<StripePaymentMethodDetails>, // enum
    pub(crate) metadata: Option<Value>,
}

#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StripePaymentMethodDetails {
    Card(StripeCard),
    #[default]
    BankTransfer,
}

impl From<StripeCard> for payments::CCard {
    fn from(card: StripeCard) -> Self {
        Self {
            card_number: masking::Secret::new(card.number),
            card_exp_month: masking::Secret::new(card.exp_month),
            card_exp_year: masking::Secret::new(card.exp_year),
            card_holder_name: masking::Secret::new("stripe_cust".to_owned()),
            card_cvc: masking::Secret::new(card.cvc),
        }
    }
}
impl From<StripePaymentMethodDetails> for payments::PaymentMethod {
    fn from(item: StripePaymentMethodDetails) -> Self {
        match item {
            StripePaymentMethodDetails::Card(card) => {
                payments::PaymentMethod::Card(payments::CCard::from(card))
            }
            StripePaymentMethodDetails::BankTransfer => payments::PaymentMethod::BankTransfer,
        }
    }
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct Shipping {
    pub(crate) address: Option<payments::AddressDetails>,
    pub(crate) name: Option<String>,
    pub(crate) carrier: Option<String>,
    pub(crate) phone: Option<String>,
    pub(crate) tracking_number: Option<String>,
}

impl From<Shipping> for payments::Address {
    fn from(details: Shipping) -> Self {
        Self {
            address: details.address,
            phone: Some(payments::PhoneDetails {
                number: details.phone.map(masking::Secret::new),
                country_code: None,
            }),
        }
    }
}
#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct StripePaymentIntentRequest {
    pub(crate) amount: Option<i64>, //amount in cents, hence passed as integer
    pub(crate) connector: Option<api_enums::Connector>,
    pub(crate) currency: Option<String>,
    #[serde(rename = "amount_to_capture")]
    pub(crate) amount_capturable: Option<i64>,
    pub(crate) confirm: Option<bool>,
    pub(crate) capture_method: Option<api_enums::CaptureMethod>,
    pub(crate) customer: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) payment_method_data: Option<StripePaymentMethodData>,
    pub(crate) receipt_email: Option<String>,
    pub(crate) return_url: Option<String>,
    pub(crate) setup_future_usage: Option<api_enums::FutureUsage>,
    pub(crate) shipping: Option<Shipping>,
    pub(crate) billing_details: Option<StripeBillingDetails>,
    pub(crate) statement_descriptor: Option<String>,
    pub(crate) statement_descriptor_suffix: Option<String>,
    pub(crate) metadata: Option<Value>,
    pub(crate) client_secret: Option<String>,
}

impl From<StripePaymentIntentRequest> for payments::PaymentsRequest {
    fn from(item: StripePaymentIntentRequest) -> Self {
        payments::PaymentsRequest {
            amount: item.amount.map(|amount| amount.into()),
            connector: item.connector,
            currency: item.currency.as_ref().map(|c| c.to_uppercase()),
            capture_method: item.capture_method,
            amount_to_capture: item.amount_capturable,
            confirm: item.confirm,
            customer_id: item.customer,
            email: item.receipt_email.map(masking::Secret::new),
            name: item
                .billing_details
                .as_ref()
                .and_then(|b| b.name.as_ref().map(|x| masking::Secret::new(x.to_owned()))),
            phone: item
                .shipping
                .as_ref()
                .and_then(|s| s.phone.as_ref().map(|x| masking::Secret::new(x.to_owned()))),
            description: item.description,
            return_url: item.return_url,
            payment_method_data: item.payment_method_data.as_ref().and_then(|pmd| {
                pmd.payment_method_details
                    .as_ref()
                    .map(|spmd| payments::PaymentMethod::from(spmd.to_owned()))
            }),
            payment_method: item
                .payment_method_data
                .as_ref()
                .map(|pmd| api_enums::PaymentMethodType::from(pmd.stype.to_owned())),
            shipping: item
                .shipping
                .as_ref()
                .map(|s| payments::Address::from(s.to_owned())),
            billing: item
                .billing_details
                .as_ref()
                .map(|b| payments::Address::from(b.to_owned())),
            statement_descriptor_name: item.statement_descriptor,
            statement_descriptor_suffix: item.statement_descriptor_suffix,
            metadata: item.metadata,
            client_secret: item.client_secret,
            ..Default::default()
        }
    }
}

#[derive(Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StripePaymentStatus {
    Succeeded,
    Canceled,
    #[default]
    Processing,
    RequiresAction,
    RequiresPaymentMethod,
    RequiresConfirmation,
    RequiresCapture,
}

// TODO: Verigy if the status are correct
impl From<api_enums::IntentStatus> for StripePaymentStatus {
    fn from(item: api_enums::IntentStatus) -> Self {
        match item {
            api_enums::IntentStatus::Succeeded => StripePaymentStatus::Succeeded,
            api_enums::IntentStatus::Failed => StripePaymentStatus::Canceled, // TODO: should we show canceled or  processing
            api_enums::IntentStatus::Processing => StripePaymentStatus::Processing,
            api_enums::IntentStatus::RequiresCustomerAction => StripePaymentStatus::RequiresAction,
            api_enums::IntentStatus::RequiresPaymentMethod => {
                StripePaymentStatus::RequiresPaymentMethod
            }
            api_enums::IntentStatus::RequiresConfirmation => {
                StripePaymentStatus::RequiresConfirmation
            }
            api_enums::IntentStatus::RequiresCapture => StripePaymentStatus::RequiresCapture,
            api_enums::IntentStatus::Cancelled => StripePaymentStatus::Canceled,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CancellationReason {
    Duplicate,
    Fraudulent,
    RequestedByCustomer,
    Abandoned,
}

impl ToString for CancellationReason {
    fn to_string(&self) -> String {
        String::from(match self {
            Self::Duplicate => "duplicate",
            Self::Fraudulent => "fradulent",
            Self::RequestedByCustomer => "requested_by_customer",
            Self::Abandoned => "abandoned",
        })
    }
}
#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub(crate) struct StripePaymentCancelRequest {
    cancellation_reason: Option<CancellationReason>,
}

impl From<StripePaymentCancelRequest> for payments::PaymentsCancelRequest {
    fn from(item: StripePaymentCancelRequest) -> Self {
        Self {
            cancellation_reason: item.cancellation_reason.map(|c| c.to_string()),
            ..Self::default()
        }
    }
}

#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct StripeCaptureRequest {
    pub(crate) amount_to_capture: Option<i64>,
}

#[derive(Default, Eq, PartialEq, Serialize)]
pub(crate) struct StripePaymentIntentResponse {
    pub(crate) id: Option<String>,
    pub(crate) object: String,
    pub(crate) amount: i64,
    pub(crate) amount_received: Option<i64>,
    pub(crate) amount_capturable: Option<i64>,
    pub(crate) currency: String,
    pub(crate) status: StripePaymentStatus,
    pub(crate) client_secret: Option<masking::Secret<String>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub(crate) created: Option<time::PrimitiveDateTime>,
    pub(crate) customer: Option<String>,
    pub(crate) refunds: Option<Vec<refunds::RefundResponse>>,
    pub(crate) mandate_id: Option<String>,
}

impl From<payments::PaymentsResponse> for StripePaymentIntentResponse {
    fn from(resp: payments::PaymentsResponse) -> Self {
        Self {
            object: "payment_intent".to_owned(),
            amount: resp.amount,
            amount_received: resp.amount_received,
            amount_capturable: resp.amount_capturable,
            currency: resp.currency.to_lowercase(),
            status: StripePaymentStatus::from(resp.status),
            client_secret: resp.client_secret,
            created: resp.created,
            customer: resp.customer_id,
            id: resp.payment_id,
            refunds: resp.refunds,
            mandate_id: resp.mandate_id,
        }
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StripePaymentListConstraints {
    pub customer: Option<String>,
    pub starting_after: Option<String>,
    pub ending_before: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub created: Option<i64>,
    #[serde(rename = "created[lt]")]
    pub created_lt: Option<i64>,
    #[serde(rename = "created[gt]")]
    pub created_gt: Option<i64>,
    #[serde(rename = "created[lte]")]
    pub created_lte: Option<i64>,
    #[serde(rename = "created[gte]")]
    pub created_gte: Option<i64>,
}

fn default_limit() -> i64 {
    10
}

impl TryFrom<StripePaymentListConstraints> for payments::PaymentListConstraints {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(item: StripePaymentListConstraints) -> Result<Self, Self::Error> {
        Ok(Self {
            customer_id: item.customer,
            starting_after: item.starting_after,
            ending_before: item.ending_before,
            limit: item.limit,
            created: from_timestamp_to_datetime(item.created)?,
            created_lt: from_timestamp_to_datetime(item.created_lt)?,
            created_gt: from_timestamp_to_datetime(item.created_gt)?,
            created_lte: from_timestamp_to_datetime(item.created_lte)?,
            created_gte: from_timestamp_to_datetime(item.created_gte)?,
        })
    }
}

#[inline]
fn from_timestamp_to_datetime(
    time: Option<i64>,
) -> Result<Option<time::PrimitiveDateTime>, errors::ApiErrorResponse> {
    if let Some(time) = time {
        let time = time::OffsetDateTime::from_unix_timestamp(time).map_err(|_| {
            errors::ApiErrorResponse::InvalidRequestData {
                message: "Error while converting timestamp".to_string(),
            }
        })?;

        Ok(Some(time::PrimitiveDateTime::new(time.date(), time.time())))
    } else {
        Ok(None)
    }
}

#[derive(Default, Eq, PartialEq, Serialize)]
pub(crate) struct StripePaymentIntentListResponse {
    pub(crate) object: String,
    pub(crate) url: String,
    pub(crate) has_more: bool,
    pub(crate) data: Vec<StripePaymentIntentResponse>,
}

impl From<payments::PaymentListResponse> for StripePaymentIntentListResponse {
    fn from(it: payments::PaymentListResponse) -> Self {
        Self {
            object: "list".to_string(),
            url: "/v1/payment_intents".to_string(),
            has_more: false,
            data: it.data.into_iter().map(Into::into).collect(),
        }
    }
}
