use api_models::{payments};
use router_env::logger;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use common_utils::{date_time,  ext_traits::StringExt};
use error_stack::{ResultExt};
use crate::{
    compatibility::stripe::{payment_intents::types::Charges, refunds::types as stripe_refunds},
    consts,
    core::errors,
    pii::{self, PeekInterface},
    types::{
        api::{self as api_types, enums as api_enums, admin},
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
    pub metadata: Option<Value>,
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
            address: details.address,
            phone: Some(payments::PhoneDetails {
                number: details.phone,
                country_code: None,
            }),
        }
    }
}

#[derive(PartialEq, Eq, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodOptions {
    Card {
        request_three_d_secure: Option<Request3DS>,
        mandate_options: Option<MandateOption>,
    },
}


#[derive(Default, Eq, PartialEq, Serialize, Deserialize, Clone, Debug)]
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

#[derive(PartialEq, Eq, Deserialize, Clone, Default, Debug)]
pub struct MandateOption {
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub accepted_at: Option<time::PrimitiveDateTime>,
    pub user_agent: Option<String>,
    pub ip_address: Option<pii::Secret<String, common_utils::pii::IpAddress>>,
    pub amount: Option<i64>,
}

impl From<MandateOption> for payments::MandateData {
    fn from(mandate_options: MandateOption) -> payments::MandateData {
        Self {
            mandate_type:payments::MandateType::MultiUse(None),
            customer_acceptance: payments::CustomerAcceptance {
                acceptance_type: payments::AcceptanceType::Online,
                accepted_at: mandate_options.accepted_at,
                online: Some(payments::OnlineMandate {
                    ip_address: mandate_options.ip_address.unwrap_or_default(),
                    user_agent: mandate_options.user_agent.unwrap_or_default(),
                }),
            },
        }
    }
}

#[derive(Default, PartialEq, Eq, Deserialize, Clone)]

pub struct StripeSetupIntentRequest {
    pub confirm: Option<bool>,
    pub customer: Option<String>,
    pub description: Option<String>,
    pub currency: Option<String>,
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
    pub payment_method_options : Option<StripePaymentMethodOptions>,
    pub payment_method: Option<String>,
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

impl TryFrom<StripeSetupIntentRequest> for payments::PaymentsRequest {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(item: StripeSetupIntentRequest) -> errors::RouterResult<Self> {
        let (mandate_options, authentication_type) = match item.payment_method_options {
            Some(pmo) => {
                let StripePaymentMethodOptions::Card {
                    request_three_d_secure,
                    mandate_options,
                }: StripePaymentMethodOptions = pmo;
                (mandate_options.map(|mandate|payments::MandateData::from(mandate)), Some(request_three_d_secure.foreign_into()))
            }
            None => (None, None),
        };
        let request = Ok(Self {
            amount: Some(api_types::Amount::Zero),
            capture_method: None,
            amount_to_capture: None,
            confirm: item.confirm,
            customer_id: item.customer,
            currency: item
                .currency
                .as_ref()
                .map(|c| c.to_uppercase().parse_enum("currency"))
                .transpose()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "currency",
                })?,
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
            setup_future_usage: item.setup_future_usage,
            merchant_connector_details: item.merchant_connector_details,
            // mandate_data: item.payment_method_options.map(|pmo| {
            //     let StripePaymentMethodOptions::Card {
            //         mandate_options,
            //     } = pmo;
            //     mandate_options.map(|mandate| (payments::MandateData::from(mandate)))
            // }).unwrap_or_default(),
            authentication_type,
            mandate_data:mandate_options,
            ..Default::default()
        });
        request
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
            api_enums::IntentStatus::RequiresMerchantAction => Self::RequiresAction,
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

pub(crate) fn into_stripe_next_action(
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

#[derive(Default, Eq, PartialEq, Serialize)]
pub struct StripeSetupIntentResponse {
    pub id: Option<String>,
    pub object: String,
    pub status: StripeSetupStatus,
    pub client_secret: Option<masking::Secret<String>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,
    pub customer: Option<String>,
    pub refunds:  Option<Vec<stripe_refunds::StripeRefundResponse>>,
    pub mandate_id: Option<String>,
    pub next_action: Option<StripeNextAction>,
    pub last_payment_error: Option<LastPaymentError>,
    pub charges: Charges
}

#[derive(Default, Eq, PartialEq, Serialize)]
pub struct LastPaymentError {
    charge: Option<String>,
    code: Option<String>,
    decline_code: Option<String>,
    message: String,
    param: Option<String>,
    payment_method: StripePaymentMethod,
    #[serde(rename = "type")]
    error_type: String,
}

#[derive(Default, Eq, PartialEq, Serialize)]
pub struct StripePaymentMethod {
    #[serde(rename = "id")]
    payment_method_id: String,
    object: &'static str,
    card: Option<StripeCard>,
    created: u64,
    #[serde(rename = "type")]
    method_type: String,
    livemode: bool,
}


impl From<payments::PaymentsResponse> for StripeSetupIntentResponse {
    fn from(resp: payments::PaymentsResponse) -> Self {
        Self {
            object: "setup_intent".to_owned(),
            status: StripeSetupStatus::from(resp.status),
            client_secret: resp.client_secret,
            charges: Charges::new(),
            created: resp.created,
            customer: resp.customer_id,
            id: resp.payment_id,
            refunds: resp
                     .refunds
                     .map(|a| a.into_iter().map(Into::into).collect()),
            mandate_id: resp.mandate_id,
            next_action: into_stripe_next_action(resp.next_action, resp.return_url),
            last_payment_error: resp.error_code.map(|code| -> LastPaymentError {LastPaymentError {
                charge: None,
                code: Some(code.to_owned()),
                decline_code: None,
                message: resp
                    .error_message
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                param: None,
                payment_method: StripePaymentMethod {
                    payment_method_id: "place_holder_id".to_string(),
                    object: "payment_method",
                    card: None,
                    created: u64::try_from(date_time::now().assume_utc().unix_timestamp())
                        .unwrap_or_default(),
                    method_type: "card".to_string(),
                    livemode: false,
                },
                error_type: code,
            }})
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
