use api_models::{payments, refunds};
use router_env::logger;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    core::errors,
    pii::{self, PeekInterface},
    types::api::{self as api_types, enums as api_enums},
};

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub struct StripeBillingDetails {
    pub address: Option<payments::AddressDetails>,
    pub email: Option<pii::Secret<String, pii::Email>>,
    pub name: Option<String>,
    pub phone: Option<pii::Secret<String>>,
}

impl From<StripeBillingDetails> for payments::Address {
    fn from(details: StripeBillingDetails) -> Self {
        Self {
            address: details.address,
            phone: Some(payments::PhoneDetails {
                number: details.phone,
                country_code: None,
            }),
        }
    }
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub struct StripeCard {
    pub number: pii::Secret<String, pii::CardNumber>,
    pub exp_month: pii::Secret<String>,
    pub exp_year: pii::Secret<String>,
    pub cvc: pii::Secret<String>,
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodType {
    #[default]
    Card,
}

impl From<StripePaymentMethodType> for api_enums::PaymentMethodType {
    fn from(item: StripePaymentMethodType) -> Self {
        match item {
            StripePaymentMethodType::Card => Self::Card,
        }
    }
}
#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
pub struct StripePaymentMethodData {
    #[serde(rename = "type")]
    pub stype: StripePaymentMethodType,
    pub billing_details: Option<StripeBillingDetails>,
    #[serde(flatten)]
    pub payment_method_details: Option<StripePaymentMethodDetails>, // enum
    pub metadata: Option<Value>,
}

#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodDetails {
    Card(StripeCard),
    #[default]
    BankTransfer,
}

impl From<StripeCard> for payments::CCard {
    fn from(card: StripeCard) -> Self {
        Self {
            card_number: card.number,
            card_exp_month: card.exp_month,
            card_exp_year: card.exp_year,
            card_holder_name: masking::Secret::new("stripe_cust".to_owned()),
            card_cvc: card.cvc,
        }
    }
}
impl From<StripePaymentMethodDetails> for payments::PaymentMethod {
    fn from(item: StripePaymentMethodDetails) -> Self {
        match item {
            StripePaymentMethodDetails::Card(card) => Self::Card(payments::CCard::from(card)),
            StripePaymentMethodDetails::BankTransfer => Self::BankTransfer,
        }
    }
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub struct Shipping {
    pub address: Option<payments::AddressDetails>,
    pub name: Option<String>,
    pub carrier: Option<String>,
    pub phone: Option<pii::Secret<String>>,
    pub tracking_number: Option<pii::Secret<String>>,
}

impl From<Shipping> for payments::Address {
    fn from(details: Shipping) -> Self {
        Self {
            address: details.address,
            phone: Some(payments::PhoneDetails {
                number: details.phone,
                country_code: None,
            }),
        }
    }
}
#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
pub struct StripeSetupIntentRequest {
    pub confirm: Option<bool>,
    pub customer: Option<String>,
    pub description: Option<String>,
    pub payment_method_data: Option<StripePaymentMethodData>,
    pub receipt_email: Option<pii::Secret<String, pii::Email>>,
    pub return_url: Option<String>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub shipping: Option<Shipping>,
    pub billing_details: Option<StripeBillingDetails>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: Option<Value>,
    pub client_secret: Option<pii::Secret<String>>,
}

impl From<StripeSetupIntentRequest> for payments::PaymentsRequest {
    fn from(item: StripeSetupIntentRequest) -> Self {
        Self {
            amount: Some(api_types::Amount::Zero),
            currency: Some(api_enums::Currency::default()),
            capture_method: None,
            amount_to_capture: None,
            confirm: item.confirm,
            customer_id: item.customer,
            email: item.receipt_email,
            name: item
                .billing_details
                .as_ref()
                .and_then(|b| b.name.as_ref().map(|x| masking::Secret::new(x.to_owned()))),
            phone: item.shipping.as_ref().and_then(|s| s.phone.clone()),
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
            client_secret: item.client_secret.map(|s| s.peek().clone()),
            ..Default::default()
        }
    }
}

#[derive(Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StripeSetupStatus {
    Succeeded,
    Canceled,
    #[default]
    Processing,
    RequiresAction,
    RequiresPaymentMethod,
    RequiresConfirmation,
}

impl From<api_enums::IntentStatus> for StripeSetupStatus {
    fn from(item: api_enums::IntentStatus) -> Self {
        match item {
            api_enums::IntentStatus::Succeeded => Self::Succeeded,
            api_enums::IntentStatus::Failed => Self::Canceled,
            api_enums::IntentStatus::Processing => Self::Processing,
            api_enums::IntentStatus::RequiresCustomerAction => Self::RequiresAction,
            api_enums::IntentStatus::RequiresPaymentMethod => Self::RequiresPaymentMethod,
            api_enums::IntentStatus::RequiresConfirmation => Self::RequiresConfirmation,
            api_enums::IntentStatus::RequiresCapture => {
                logger::error!("Invalid status change");
                Self::Canceled
            }
            api_enums::IntentStatus::Cancelled => Self::Canceled,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CancellationReason {
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
pub struct StripePaymentCancelRequest {
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
#[derive(Default, Eq, PartialEq, Serialize)]
pub struct StripeSetupIntentResponse {
    pub id: Option<String>,
    pub object: String,
    pub status: StripeSetupStatus,
    pub client_secret: Option<masking::Secret<String>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,
    pub customer: Option<String>,
    pub refunds: Option<Vec<refunds::RefundResponse>>,
    pub mandate_id: Option<String>,
}

impl From<payments::PaymentsResponse> for StripeSetupIntentResponse {
    fn from(resp: payments::PaymentsResponse) -> Self {
        Self {
            object: "setup_intent".to_owned(),
            status: StripeSetupStatus::from(resp.status),
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
        let time = time::OffsetDateTime::from_unix_timestamp(time).map_err(|err| {
            logger::error!("Error: from_unix_timestamp: {}", err);
            errors::ApiErrorResponse::InvalidRequestData {
                message: "Error while converting timestamp".to_string(),
            }
        })?;

        Ok(Some(time::PrimitiveDateTime::new(time.date(), time.time())))
    } else {
        Ok(None)
    }
}
