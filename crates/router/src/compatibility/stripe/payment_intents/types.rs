use api_models::{payments, refunds};
use common_utils::{ext_traits::StringExt, pii as secret};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    pii::{self, PeekInterface},
    types::{
        api::enums as api_enums,
        transformers::{ForeignFrom, ForeignInto},
    },
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
            phone: Some(payments::PhoneDetails {
                number: details.phone,
                country_code: details.address.as_ref().and_then(|a| a.country.clone()),
            }),

            address: details.address,
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

impl From<StripePaymentMethodType> for api_enums::PaymentMethod {
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
    pub metadata: Option<secret::SecretSerdeValue>,
}

#[derive(PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodDetails {
    Card(StripeCard),
}

impl From<StripeCard> for payments::Card {
    fn from(card: StripeCard) -> Self {
        Self {
            card_number: card.number,
            card_exp_month: card.exp_month,
            card_exp_year: card.exp_year,
            card_holder_name: masking::Secret::new("stripe_cust".to_owned()),
            card_cvc: card.cvc,
            card_issuer: None,
            card_network: None,
        }
    }
}

impl From<StripePaymentMethodDetails> for payments::PaymentMethodData {
    fn from(item: StripePaymentMethodDetails) -> Self {
        match item {
            StripePaymentMethodDetails::Card(card) => Self::Card(payments::Card::from(card)),
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
            phone: Some(payments::PhoneDetails {
                number: details.phone,
                country_code: details.address.as_ref().and_then(|a| a.country.clone()),
            }),
            address: details.address,
        }
    }
}
#[derive(PartialEq, Eq, Deserialize, Clone)]
pub struct StripePaymentIntentRequest {
    pub amount: Option<i64>, //amount in cents, hence passed as integer
    pub connector: Option<Vec<api_enums::Connector>>,
    pub currency: Option<String>,
    #[serde(rename = "amount_to_capture")]
    pub amount_capturable: Option<i64>,
    pub confirm: Option<bool>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    pub customer: Option<String>,
    pub description: Option<String>,
    pub payment_method_data: Option<StripePaymentMethodData>,
    pub receipt_email: Option<pii::Secret<String, pii::Email>>,
    pub return_url: Option<url::Url>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub shipping: Option<Shipping>,
    pub billing_details: Option<StripeBillingDetails>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: Option<api_models::payments::Metadata>,
    pub client_secret: Option<pii::Secret<String>>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
}

impl TryFrom<StripePaymentIntentRequest> for payments::PaymentsRequest {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(item: StripePaymentIntentRequest) -> errors::RouterResult<Self> {
        Ok(Self {
            amount: item.amount.map(|amount| amount.into()),
            connector: item.connector,
            currency: item
                .currency
                .as_ref()
                .map(|c| c.to_uppercase().parse_enum("currency"))
                .transpose()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "currency",
                })?,
            capture_method: item.capture_method,
            amount_to_capture: item.amount_capturable,
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
                    .map(|spmd| payments::PaymentMethodData::from(spmd.to_owned()))
            }),
            payment_method: item
                .payment_method_data
                .as_ref()
                .map(|pmd| api_enums::PaymentMethod::from(pmd.stype.to_owned())),
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
            authentication_type: item.payment_method_options.map(|pmo| {
                let StripePaymentMethodOptions::Card {
                    request_three_d_secure,
                } = pmo;

                request_three_d_secure.foreign_into()
            }),
            ..Self::default()
        })
    }
}

#[derive(Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentStatus {
    Succeeded,
    Canceled,
    #[default]
    Processing,
    RequiresAction,
    RequiresPaymentMethod,
    RequiresConfirmation,
    RequiresCapture,
}

impl From<api_enums::IntentStatus> for StripePaymentStatus {
    fn from(item: api_enums::IntentStatus) -> Self {
        match item {
            api_enums::IntentStatus::Succeeded => Self::Succeeded,
            api_enums::IntentStatus::Failed => Self::Canceled,
            api_enums::IntentStatus::Processing => Self::Processing,
            api_enums::IntentStatus::RequiresCustomerAction => Self::RequiresAction,
            api_enums::IntentStatus::RequiresPaymentMethod => Self::RequiresPaymentMethod,
            api_enums::IntentStatus::RequiresConfirmation => Self::RequiresConfirmation,
            api_enums::IntentStatus::RequiresCapture => Self::RequiresCapture,
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

#[derive(Default, PartialEq, Eq, Deserialize, Clone)]
pub struct StripeCaptureRequest {
    pub amount_to_capture: Option<i64>,
}

#[derive(Default, Eq, PartialEq, Serialize)]
pub struct StripePaymentIntentResponse {
    pub id: Option<String>,
    pub object: &'static str,
    pub amount: i64,
    pub amount_received: Option<i64>,
    pub amount_capturable: Option<i64>,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub client_secret: Option<masking::Secret<String>>,
    pub created: Option<i64>,
    pub customer: Option<String>,
    pub refunds: Option<Vec<refunds::RefundResponse>>,
    pub mandate_id: Option<String>,
    pub metadata: Option<secret::SecretSerdeValue>,
    pub charges: Charges,
    pub connector: Option<String>,
    pub description: Option<String>,
    pub mandate_data: Option<payments::MandateData>,
    pub setup_future_usage: Option<api_models::enums::FutureUsage>,
    pub off_session: Option<bool>,

    pub authentication_type: Option<api_models::enums::AuthenticationType>,
    pub next_action: Option<StripeNextAction>,
    pub cancellation_reason: Option<String>,
    pub payment_method: Option<api_models::enums::PaymentMethod>,
    pub payment_method_data: Option<payments::PaymentMethodDataResponse>,
    pub shipping: Option<payments::Address>,
    pub billing: Option<payments::Address>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub capture_on: Option<time::PrimitiveDateTime>,
    pub payment_token: Option<String>,
    pub email: Option<masking::Secret<String, common_utils::pii::Email>>,
    pub phone: Option<masking::Secret<String>>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub capture_method: Option<api_models::enums::CaptureMethod>,
    pub name: Option<masking::Secret<String>>,
}

impl From<payments::PaymentsResponse> for StripePaymentIntentResponse {
    fn from(resp: payments::PaymentsResponse) -> Self {
        Self {
            object: "payment_intent",
            id: resp.payment_id,
            status: StripePaymentStatus::from(resp.status),
            amount: resp.amount,
            amount_capturable: resp.amount_capturable,
            amount_received: resp.amount_received,
            connector: resp.connector,
            client_secret: resp.client_secret,
            created: resp.created.map(|t| t.assume_utc().unix_timestamp()),
            currency: resp.currency.to_lowercase(),
            customer: resp.customer_id,
            description: resp.description,
            refunds: resp.refunds,
            mandate_id: resp.mandate_id,
            mandate_data: resp.mandate_data,
            setup_future_usage: resp.setup_future_usage,
            off_session: resp.off_session,
            capture_on: resp.capture_on,
            capture_method: resp.capture_method,
            payment_method: resp.payment_method,
            payment_method_data: resp.payment_method_data,
            payment_token: resp.payment_token,
            shipping: resp.shipping,
            billing: resp.billing,
            email: resp.email,
            name: resp.name,
            phone: resp.phone,
            authentication_type: resp.authentication_type,
            statement_descriptor_name: resp.statement_descriptor_name,
            statement_descriptor_suffix: resp.statement_descriptor_suffix,
            next_action: into_stripe_next_action(resp.next_action, resp.return_url),
            cancellation_reason: resp.cancellation_reason,
            error_code: resp.error_code,
            error_message: resp.error_message,
            metadata: resp.metadata,
            charges: Charges::new(),
        }
    }
}

#[derive(Default, Eq, PartialEq, Serialize)]
pub struct Charges {
    object: &'static str,
    data: Vec<String>,
    has_more: bool,
    total_count: i32,
    url: String,
}

impl Charges {
    fn new() -> Self {
        Self {
            object: "list",
            data: vec![],
            has_more: false,
            total_count: 0,
            url: "http://placeholder".to_string(),
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
pub struct StripePaymentIntentListResponse {
    pub object: String,
    pub url: String,
    pub has_more: bool,
    pub data: Vec<StripePaymentIntentResponse>,
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

#[derive(PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodOptions {
    Card {
        request_three_d_secure: Option<Request3DS>,
    },
}

#[derive(Default, Eq, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Request3DS {
    #[default]
    Automatic,
    Any,
}

impl ForeignFrom<Option<Request3DS>> for api_models::enums::AuthenticationType {
    fn foreign_from(item: Option<Request3DS>) -> Self {
        match item.unwrap_or_default() {
            Request3DS::Automatic => Self::NoThreeDs,
            Request3DS::Any => Self::ThreeDs,
        }
    }
}

#[derive(Default, Eq, PartialEq, Serialize)]
pub struct RedirectUrl {
    pub return_url: Option<String>,
    pub url: Option<String>,
}

#[derive(Eq, PartialEq, Serialize)]
pub struct StripeNextAction {
    #[serde(rename = "type")]
    stype: payments::NextActionType,
    redirect_to_url: RedirectUrl,
}

fn into_stripe_next_action(
    next_action: Option<payments::NextAction>,
    return_url: Option<String>,
) -> Option<StripeNextAction> {
    next_action.map(|n| StripeNextAction {
        stype: n.next_action_type,
        redirect_to_url: RedirectUrl {
            return_url,
            url: n.redirect_to_url,
        },
    })
}
