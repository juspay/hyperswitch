use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    core::errors,
    pii::Secret,
    types::{
        api::{
            Address, AddressDetails, CCard, PaymentListConstraints, PaymentListResponse,
            PaymentMethod, PaymentsCancelRequest, PaymentsRequest, PaymentsResponse, PhoneDetails,
            RefundResponse,
        },
        storage::enums::{self, FutureUsage, IntentStatus, PaymentMethodType},
    },
    utils::custom_serde,
};

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct StripeBillingDetails {
    pub(crate) address: Option<AddressDetails>,
    pub(crate) email: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) phone: Option<String>,
}

impl From<StripeBillingDetails> for Address {
    fn from(details: StripeBillingDetails) -> Self {
        Self {
            address: details.address,
            phone: Some(PhoneDetails {
                number: details.phone.map(Secret::new),
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

impl From<StripePaymentMethodType> for PaymentMethodType {
    fn from(item: StripePaymentMethodType) -> Self {
        match item {
            StripePaymentMethodType::Card => PaymentMethodType::Card,
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

impl From<StripeCard> for CCard {
    fn from(card: StripeCard) -> Self {
        Self {
            card_number: Secret::new(card.number),
            card_exp_month: Secret::new(card.exp_month),
            card_exp_year: Secret::new(card.exp_year),
            card_holder_name: Secret::new("stripe_cust".to_owned()),
            card_cvc: Secret::new(card.cvc),
        }
    }
}
impl From<StripePaymentMethodDetails> for PaymentMethod {
    fn from(item: StripePaymentMethodDetails) -> Self {
        match item {
            StripePaymentMethodDetails::Card(card) => PaymentMethod::Card(CCard::from(card)),
            StripePaymentMethodDetails::BankTransfer => PaymentMethod::BankTransfer,
        }
    }
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct Shipping {
    pub(crate) address: Option<AddressDetails>,
    pub(crate) name: Option<String>,
    pub(crate) carrier: Option<String>,
    pub(crate) phone: Option<String>,
    pub(crate) tracking_number: Option<String>,
}

impl From<Shipping> for Address {
    fn from(details: Shipping) -> Self {
        Self {
            address: details.address,
            phone: Some(PhoneDetails {
                number: details.phone.map(Secret::new),
                country_code: None,
            }),
        }
    }
}
#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct StripePaymentIntentRequest {
    pub(crate) amount: Option<i32>, //amount in cents, hence passed as integer
    pub(crate) currency: Option<String>,
    #[serde(rename = "amount_to_capture")]
    pub(crate) amount_capturable: Option<i32>,
    pub(crate) confirm: Option<bool>,
    pub(crate) capture_method: Option<enums::CaptureMethod>,
    pub(crate) customer: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) payment_method_data: Option<StripePaymentMethodData>,
    pub(crate) receipt_email: Option<String>,
    pub(crate) return_url: Option<String>,
    pub(crate) setup_future_usage: Option<FutureUsage>,
    pub(crate) shipping: Option<Shipping>,
    pub(crate) billing_details: Option<StripeBillingDetails>,
    pub(crate) statement_descriptor: Option<String>,
    pub(crate) statement_descriptor_suffix: Option<String>,
    pub(crate) metadata: Option<Value>,
    pub(crate) client_secret: Option<String>,
}

impl From<StripePaymentIntentRequest> for PaymentsRequest {
    fn from(item: StripePaymentIntentRequest) -> Self {
        PaymentsRequest {
            amount: item.amount,
            currency: item.currency.as_ref().map(|c| c.to_uppercase()),
            capture_method: item.capture_method,
            amount_to_capture: item.amount_capturable,
            confirm: item.confirm,
            customer_id: item.customer,
            email: item.receipt_email.map(Secret::new),
            name: item
                .billing_details
                .as_ref()
                .and_then(|b| b.name.as_ref().map(|x| Secret::new(x.to_owned()))),
            phone: item
                .shipping
                .as_ref()
                .and_then(|s| s.phone.as_ref().map(|x| Secret::new(x.to_owned()))),
            description: item.description,
            return_url: item.return_url,
            payment_method_data: item.payment_method_data.as_ref().and_then(|pmd| {
                pmd.payment_method_details
                    .as_ref()
                    .map(|spmd| PaymentMethod::from(spmd.to_owned()))
            }),
            payment_method: item
                .payment_method_data
                .as_ref()
                .map(|pmd| PaymentMethodType::from(pmd.stype.to_owned())),
            shipping: item.shipping.as_ref().map(|s| Address::from(s.to_owned())),
            billing: item
                .billing_details
                .as_ref()
                .map(|b| Address::from(b.to_owned())),
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
impl From<IntentStatus> for StripePaymentStatus {
    fn from(item: IntentStatus) -> Self {
        match item {
            IntentStatus::Succeeded => StripePaymentStatus::Succeeded,
            IntentStatus::Failed => StripePaymentStatus::Canceled, // TODO: should we show canceled or  processing
            IntentStatus::Processing => StripePaymentStatus::Processing,
            IntentStatus::RequiresCustomerAction => StripePaymentStatus::RequiresAction,
            IntentStatus::RequiresPaymentMethod => StripePaymentStatus::RequiresPaymentMethod,
            IntentStatus::RequiresConfirmation => StripePaymentStatus::RequiresConfirmation,
            IntentStatus::RequiresCapture => StripePaymentStatus::RequiresCapture,
            IntentStatus::Cancelled => StripePaymentStatus::Canceled,
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

impl From<StripePaymentCancelRequest> for PaymentsCancelRequest {
    fn from(item: StripePaymentCancelRequest) -> Self {
        Self {
            cancellation_reason: item.cancellation_reason.map(|c| c.to_string()),
            ..Self::default()
        }
    }
}

#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
pub(crate) struct StripeCaptureRequest {
    pub(crate) amount_to_capture: Option<i32>,
}

#[derive(Default, Eq, PartialEq, Serialize)]
pub(crate) struct StripePaymentIntentResponse {
    pub(crate) id: Option<String>,
    pub(crate) object: String,
    pub(crate) amount: i32,
    pub(crate) amount_received: Option<i32>,
    pub(crate) amount_capturable: Option<i32>,
    pub(crate) currency: String,
    pub(crate) status: StripePaymentStatus,
    pub(crate) client_secret: Option<Secret<String>>,
    #[serde(with = "custom_serde::iso8601::option")]
    pub(crate) created: Option<time::PrimitiveDateTime>,
    pub(crate) customer: Option<String>,
    pub(crate) refunds: Option<Vec<RefundResponse>>,
    pub(crate) mandate_id: Option<String>,
}

impl From<PaymentsResponse> for StripePaymentIntentResponse {
    fn from(resp: PaymentsResponse) -> Self {
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

impl TryFrom<StripePaymentListConstraints> for PaymentListConstraints {
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

impl From<PaymentListResponse> for StripePaymentIntentListResponse {
    fn from(it: PaymentListResponse) -> Self {
        Self {
            object: "list".to_string(),
            url: "/v1/payment_intents".to_string(),
            has_more: false,
            data: it.data.into_iter().map(Into::into).collect(),
        }
    }
}
