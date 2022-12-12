use router_env::logger;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    core::errors,
    pii::Secret,
    types::api::{
        self as api_types, enums as api_enums, Address, AddressDetails, CCard,
        PaymentListConstraints, PaymentMethod, PaymentsCancelRequest, PaymentsRequest,
        PaymentsResponse, PhoneDetails, RefundResponse,
    },
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
pub(crate) struct StripeSetupIntentRequest {
    pub(crate) confirm: Option<bool>,
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

impl From<StripeSetupIntentRequest> for PaymentsRequest {
    fn from(item: StripeSetupIntentRequest) -> Self {
        PaymentsRequest {
            amount: Some(api_types::Amount::Zero),
            currency: Some(api_enums::Currency::default().to_string()),
            capture_method: None,
            amount_to_capture: None,
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
                .map(|pmd| api_enums::PaymentMethodType::from(pmd.stype.to_owned())),
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
pub(crate) enum StripeSetupStatus {
    Succeeded,
    Canceled,
    #[default]
    Processing,
    RequiresAction,
    RequiresPaymentMethod,
    RequiresConfirmation,
}

// TODO: Verify if the status are correct
impl From<api_enums::IntentStatus> for StripeSetupStatus {
    fn from(item: api_enums::IntentStatus) -> Self {
        match item {
            api_enums::IntentStatus::Succeeded => StripeSetupStatus::Succeeded,
            api_enums::IntentStatus::Failed => StripeSetupStatus::Canceled, // TODO: should we show canceled or  processing
            api_enums::IntentStatus::Processing => StripeSetupStatus::Processing,
            api_enums::IntentStatus::RequiresCustomerAction => StripeSetupStatus::RequiresAction,
            api_enums::IntentStatus::RequiresPaymentMethod => {
                StripeSetupStatus::RequiresPaymentMethod
            }
            api_enums::IntentStatus::RequiresConfirmation => {
                StripeSetupStatus::RequiresConfirmation
            }
            api_enums::IntentStatus::RequiresCapture => {
                logger::error!("Invalid status change");
                StripeSetupStatus::Canceled
            }
            api_enums::IntentStatus::Cancelled => StripeSetupStatus::Canceled,
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
#[derive(Default, Eq, PartialEq, Serialize)]
pub(crate) struct StripeSetupIntentResponse {
    pub(crate) id: Option<String>,
    pub(crate) object: String,
    pub(crate) status: StripeSetupStatus,
    pub(crate) client_secret: Option<Secret<String>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub(crate) created: Option<time::PrimitiveDateTime>,
    pub(crate) customer: Option<String>,
    pub(crate) refunds: Option<Vec<RefundResponse>>,
    pub(crate) mandate_id: Option<String>,
}

impl From<PaymentsResponse> for StripeSetupIntentResponse {
    fn from(resp: PaymentsResponse) -> Self {
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
