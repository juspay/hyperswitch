use api_models::payments;
use common_utils::{crypto::Encryptable, date_time, ext_traits::StringExt, pii as secret};
use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    compatibility::stripe::refunds::types as stripe_refunds,
    consts,
    core::errors,
    pii::{self, Email, PeekInterface},
    types::{
        api::{admin, enums as api_enums},
        transformers::{ForeignFrom, ForeignTryFrom},
    },
};

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub struct StripeBillingDetails {
    pub address: Option<payments::AddressDetails>,
    pub email: Option<Email>,
    pub name: Option<String>,
    pub phone: Option<pii::Secret<String>>,
}

impl From<StripeBillingDetails> for payments::Address {
    fn from(details: StripeBillingDetails) -> Self {
        Self {
            phone: Some(payments::PhoneDetails {
                number: details.phone,
                country_code: details.address.as_ref().and_then(|address| {
                    address.country.as_ref().map(|country| country.to_string())
                }),
            }),

            address: details.address,
        }
    }
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone, Debug)]
pub struct StripeCard {
    pub number: cards::CardNumber,
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
                country_code: details.address.as_ref().and_then(|address| {
                    address.country.as_ref().map(|country| country.to_string())
                }),
            }),
            address: details.address,
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct StripePaymentIntentRequest {
    pub id: Option<String>,
    pub amount: Option<i64>, //amount in cents, hence passed as integer
    pub connector: Option<Vec<api_enums::RoutableConnectors>>,
    pub currency: Option<String>,
    #[serde(rename = "amount_to_capture")]
    pub amount_capturable: Option<i64>,
    pub confirm: Option<bool>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    pub customer: Option<String>,
    pub description: Option<String>,
    pub payment_method_data: Option<StripePaymentMethodData>,
    pub receipt_email: Option<Email>,
    pub return_url: Option<url::Url>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub shipping: Option<Shipping>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: Option<api_models::payments::Metadata>,
    pub client_secret: Option<pii::Secret<String>>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
    pub mandate_id: Option<String>,
    pub off_session: Option<bool>,
}

impl TryFrom<StripePaymentIntentRequest> for payments::PaymentsRequest {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(item: StripePaymentIntentRequest) -> errors::RouterResult<Self> {
        let (mandate_options, authentication_type) = match item.payment_method_options {
            Some(pmo) => {
                let StripePaymentMethodOptions::Card {
                    request_three_d_secure,
                    mandate_options,
                }: StripePaymentMethodOptions = pmo;
                (
                    Option::<payments::MandateData>::foreign_try_from((
                        mandate_options,
                        item.currency.to_owned(),
                    ))?,
                    Some(api_enums::AuthenticationType::foreign_from(
                        request_three_d_secure,
                    )),
                )
            }
            None => (None, None),
        };

        let routable_connector: Option<api_enums::RoutableConnectors> =
            item.connector.and_then(|v| v.into_iter().next());

        let routing = routable_connector
            .map(crate::types::api::RoutingAlgorithm::Single)
            .map(|r| {
                serde_json::to_value(r)
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("converting to routing failed")
            })
            .transpose()?;
        let request = Ok(Self {
            payment_id: item.id.map(payments::PaymentIdType::PaymentIntentId),
            amount: item.amount.map(|amount| amount.into()),
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
                .payment_method_data
                .and_then(|pmd| pmd.billing_details.map(payments::Address::from)),
            statement_descriptor_name: item.statement_descriptor,
            statement_descriptor_suffix: item.statement_descriptor_suffix,
            metadata: item.metadata,
            client_secret: item.client_secret.map(|s| s.peek().clone()),
            authentication_type,
            mandate_data: mandate_options,
            merchant_connector_details: item.merchant_connector_details,
            setup_future_usage: item.setup_future_usage,
            mandate_id: item.mandate_id,
            off_session: item.off_session,
            routing,
            ..Self::default()
        });
        request
    }
}

#[derive(Clone, Default, Eq, PartialEq, Serialize, Deserialize, Debug)]
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
            api_enums::IntentStatus::RequiresMerchantAction => Self::RequiresAction,
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

#[derive(Default, Eq, PartialEq, Serialize, Debug)]
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
    pub refunds: Option<Vec<stripe_refunds::StripeRefundResponse>>,
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
    pub email: Option<Email>,
    pub phone: Option<masking::Secret<String>>,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub capture_method: Option<api_models::enums::CaptureMethod>,
    pub name: Option<masking::Secret<String>>,
    pub last_payment_error: Option<LastPaymentError>,
}

#[derive(Default, Eq, PartialEq, Serialize, Debug)]
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
            refunds: resp
                .refunds
                .map(|a| a.into_iter().map(Into::into).collect()),
            mandate_id: resp.mandate_id,
            mandate_data: resp.mandate_data,
            setup_future_usage: resp.setup_future_usage,
            off_session: resp.off_session,
            capture_on: resp.capture_on,
            capture_method: resp.capture_method,
            payment_method: resp.payment_method,
            payment_method_data: resp.payment_method_data.clone(),
            payment_token: resp.payment_token,
            shipping: resp.shipping,
            billing: resp.billing,
            email: resp.email.map(|inner| inner.into()),
            name: resp.name.map(Encryptable::into_inner),
            phone: resp.phone.map(Encryptable::into_inner),
            authentication_type: resp.authentication_type,
            statement_descriptor_name: resp.statement_descriptor_name,
            statement_descriptor_suffix: resp.statement_descriptor_suffix,
            next_action: into_stripe_next_action(resp.next_action, resp.return_url),
            cancellation_reason: resp.cancellation_reason,
            metadata: resp.metadata,
            charges: Charges::new(),
            last_payment_error: resp.error_code.map(|code| LastPaymentError {
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
            }),
        }
    }
}

#[derive(Default, Eq, PartialEq, Serialize, Debug)]
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

#[derive(Default, Eq, PartialEq, Serialize, Debug)]
pub struct Charges {
    object: &'static str,
    data: Vec<String>,
    has_more: bool,
    total_count: i32,
    url: String,
}

impl Charges {
    pub fn new() -> Self {
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

#[derive(PartialEq, Eq, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodOptions {
    Card {
        request_three_d_secure: Option<Request3DS>,
        mandate_options: Option<MandateOption>,
    },
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripeMandateType {
    SingleUse,
    MultiUse,
}

#[derive(PartialEq, Eq, Clone, Default, Deserialize, Serialize, Debug)]
pub struct MandateOption {
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub accepted_at: Option<time::PrimitiveDateTime>,
    pub user_agent: Option<String>,
    pub ip_address: Option<pii::Secret<String, common_utils::pii::IpAddress>>,
    pub mandate_type: Option<StripeMandateType>,
    pub amount: Option<i64>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub start_date: Option<time::PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub end_date: Option<time::PrimitiveDateTime>,
}

impl ForeignTryFrom<(Option<MandateOption>, Option<String>)> for Option<payments::MandateData> {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(
        (mandate_options, currency): (Option<MandateOption>, Option<String>),
    ) -> errors::RouterResult<Self> {
        let currency = currency
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "currency",
            })
            .into_report()
            .and_then(|c| {
                c.to_uppercase().parse_enum("currency").change_context(
                    errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "currency",
                    },
                )
            })?;
        let mandate_data = mandate_options.map(|mandate| payments::MandateData {
            mandate_type: match mandate.mandate_type {
                Some(item) => match item {
                    StripeMandateType::SingleUse => Some(payments::MandateType::SingleUse(
                        payments::MandateAmountData {
                            amount: mandate.amount.unwrap_or_default(),
                            currency,
                            start_date: mandate.start_date,
                            end_date: mandate.end_date,
                            metadata: None,
                        },
                    )),
                    StripeMandateType::MultiUse => Some(payments::MandateType::MultiUse(None)),
                },
                None => Some(api_models::payments::MandateType::MultiUse(None)),
            },
            customer_acceptance: Some(payments::CustomerAcceptance {
                acceptance_type: payments::AcceptanceType::Online,
                accepted_at: mandate.accepted_at,
                online: Some(payments::OnlineMandate {
                    ip_address: mandate.ip_address.unwrap_or_default(),
                    user_agent: mandate.user_agent.unwrap_or_default(),
                }),
            }),
        });
        Ok(mandate_data)
    }
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

#[derive(Default, Eq, PartialEq, Serialize, Debug)]
pub struct RedirectUrl {
    pub return_url: Option<String>,
    pub url: Option<String>,
}

#[derive(Eq, PartialEq, serde::Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StripeNextAction {
    RedirectToUrl {
        redirect_to_url: RedirectUrl,
    },
    DisplayBankTransferInformation {
        bank_transfer_steps_and_charges_details: payments::BankTransferNextStepsData,
    },
}

pub(crate) fn into_stripe_next_action(
    next_action: Option<payments::NextActionData>,
    return_url: Option<String>,
) -> Option<StripeNextAction> {
    next_action.map(|next_action_data| match next_action_data {
        payments::NextActionData::RedirectToUrl { redirect_to_url } => {
            StripeNextAction::RedirectToUrl {
                redirect_to_url: RedirectUrl {
                    return_url,
                    url: Some(redirect_to_url),
                },
            }
        }
        payments::NextActionData::DisplayBankTransferInformation {
            bank_transfer_steps_and_charges_details,
        } => StripeNextAction::DisplayBankTransferInformation {
            bank_transfer_steps_and_charges_details,
        },
    })
}
