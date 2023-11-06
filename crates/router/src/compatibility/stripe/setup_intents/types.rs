use std::str::FromStr;

use api_models::payments;
use common_utils::{date_time, ext_traits::StringExt, pii as secret};
use error_stack::{IntoReport, ResultExt};
use router_env::logger;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    compatibility::stripe::{
        payment_intents::types as payment_intent, refunds::types as stripe_refunds,
    },
    consts,
    core::errors,
    pii::{self, PeekInterface},
    types::{
        api::{self as api_types, admin, enums as api_enums},
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::OptionExt,
};

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub struct StripeBillingDetails {
    pub address: Option<payments::AddressDetails>,
    pub email: Option<pii::Email>,
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
    pub number: cards::CardNumber,
    pub exp_month: pii::Secret<String>,
    pub exp_year: pii::Secret<String>,
    pub cvc: pii::Secret<String>,
}

// ApplePay wallet param is not available in stripe Docs
#[derive(Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripeWallet {
    ApplePay(payments::ApplePayWalletData),
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodType {
    #[default]
    Card,
    Wallet,
}

impl From<StripePaymentMethodType> for api_enums::PaymentMethod {
    fn from(item: StripePaymentMethodType) -> Self {
        match item {
            StripePaymentMethodType::Card => Self::Card,
            StripePaymentMethodType::Wallet => Self::Wallet,
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
    Wallet(StripeWallet),
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
            bank_code: None,
            card_issuing_country: None,
            card_type: None,
            nick_name: None,
        }
    }
}

impl From<StripeWallet> for payments::WalletData {
    fn from(wallet: StripeWallet) -> Self {
        match wallet {
            StripeWallet::ApplePay(data) => Self::ApplePay(data),
        }
    }
}

impl From<StripePaymentMethodDetails> for payments::PaymentMethodData {
    fn from(item: StripePaymentMethodDetails) -> Self {
        match item {
            StripePaymentMethodDetails::Card(card) => Self::Card(payments::Card::from(card)),
            StripePaymentMethodDetails::Wallet(wallet) => {
                Self::Wallet(payments::WalletData::from(wallet))
            }
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

#[derive(Default, Deserialize, Clone)]
pub struct StripeSetupIntentRequest {
    pub confirm: Option<bool>,
    pub customer: Option<String>,
    pub connector: Option<Vec<api_enums::RoutableConnectors>>,
    pub description: Option<String>,
    pub currency: Option<String>,
    pub payment_method_data: Option<StripePaymentMethodData>,
    pub receipt_email: Option<pii::Email>,
    pub return_url: Option<url::Url>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub shipping: Option<Shipping>,
    pub billing_details: Option<StripeBillingDetails>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: Option<secret::SecretSerdeValue>,
    pub client_secret: Option<pii::Secret<String>>,
    pub payment_method_options: Option<payment_intent::StripePaymentMethodOptions>,
    pub payment_method: Option<String>,
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
    pub receipt_ipaddress: Option<String>,
    pub user_agent: Option<String>,
    pub mandate_data: Option<payment_intent::MandateData>,
    pub connector_metadata: Option<payments::ConnectorMetadata>,
}

impl TryFrom<StripeSetupIntentRequest> for payments::PaymentsRequest {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(item: StripeSetupIntentRequest) -> errors::RouterResult<Self> {
        let routable_connector: Option<api_enums::RoutableConnectors> =
            item.connector.and_then(|v| {
                v.into_iter()
                    .next()
                    .map(api_enums::RoutableConnectors::from)
            });

        let routing = routable_connector
            .map(|connector| {
                api_models::routing::RoutingAlgorithm::Single(Box::new(
                    api_models::routing::RoutableConnectorChoice {
                        #[cfg(feature = "backwards_compatibility")]
                        choice_kind: api_models::routing::RoutableChoiceKind::FullStruct,
                        connector,
                        #[cfg(feature = "connector_choice_mca_id")]
                        merchant_connector_id: None,
                        #[cfg(not(feature = "connector_choice_mca_id"))]
                        sub_label: None,
                    },
                ))
            })
            .map(|r| {
                serde_json::to_value(r)
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("converting to routing failed")
            })
            .transpose()?;
        let ip_address = item
            .receipt_ipaddress
            .map(|ip| std::net::IpAddr::from_str(ip.as_str()))
            .transpose()
            .into_report()
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "receipt_ipaddress".to_string(),
                expected_format: "127.0.0.1".to_string(),
            })?;
        let metadata_object = item
            .metadata
            .clone()
            .parse_value("metadata")
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "metadata mapping failed",
            })?;
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
            metadata: metadata_object,
            client_secret: item.client_secret.map(|s| s.peek().clone()),
            setup_future_usage: item.setup_future_usage,
            merchant_connector_details: item.merchant_connector_details,
            routing,
            authentication_type: match item.payment_method_options {
                Some(pmo) => {
                    let payment_intent::StripePaymentMethodOptions::Card {
                        request_three_d_secure,
                    }: payment_intent::StripePaymentMethodOptions = pmo;
                    Some(api_enums::AuthenticationType::foreign_from(
                        request_three_d_secure,
                    ))
                }
                None => None,
            },
            mandate_data: ForeignTryFrom::foreign_try_from((
                item.mandate_data,
                item.currency.to_owned(),
            ))?,
            browser_info: Some(
                serde_json::to_value(crate::types::BrowserInformation {
                    ip_address,
                    user_agent: item.user_agent,
                    ..Default::default()
                })
                .into_report()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("convert to browser info failed")?,
            ),
            connector_metadata: item.connector_metadata,
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
            api_enums::IntentStatus::RequiresCapture
            | api_enums::IntentStatus::PartiallyCaptured => {
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

#[derive(Eq, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StripeNextAction {
    RedirectToUrl {
        redirect_to_url: RedirectUrl,
    },
    DisplayBankTransferInformation {
        bank_transfer_steps_and_charges_details: payments::BankTransferNextStepsData,
    },
    ThirdPartySdkSessionToken {
        session_token: Option<payments::SessionToken>,
    },
    QrCodeInformation {
        image_data_url: url::Url,
        display_to_timestamp: Option<i64>,
    },
    DisplayVoucherInformation {
        voucher_details: payments::VoucherNextStepData,
    },
    WaitScreenInformation {
        display_from_timestamp: i128,
        display_to_timestamp: Option<i128>,
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
        payments::NextActionData::ThirdPartySdkSessionToken { session_token } => {
            StripeNextAction::ThirdPartySdkSessionToken { session_token }
        }
        payments::NextActionData::QrCodeInformation {
            image_data_url,
            display_to_timestamp,
        } => StripeNextAction::QrCodeInformation {
            image_data_url,
            display_to_timestamp,
        },
        payments::NextActionData::DisplayVoucherInformation { voucher_details } => {
            StripeNextAction::DisplayVoucherInformation { voucher_details }
        }
        payments::NextActionData::WaitScreenInformation {
            display_from_timestamp,
            display_to_timestamp,
        } => StripeNextAction::WaitScreenInformation {
            display_from_timestamp,
            display_to_timestamp,
        },
    })
}

#[derive(Default, Eq, PartialEq, Serialize)]
pub struct StripeSetupIntentResponse {
    pub id: Option<String>,
    pub object: String,
    pub status: StripeSetupStatus,
    pub client_secret: Option<masking::Secret<String>>,
    pub metadata: Option<secret::SecretSerdeValue>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,
    pub customer: Option<String>,
    pub refunds: Option<Vec<stripe_refunds::StripeRefundResponse>>,
    pub mandate_id: Option<String>,
    pub next_action: Option<StripeNextAction>,
    pub last_payment_error: Option<LastPaymentError>,
    pub charges: payment_intent::Charges,
    pub connector_transaction_id: Option<String>,
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
            charges: payment_intent::Charges::new(),
            created: resp.created,
            customer: resp.customer_id,
            metadata: resp.metadata,
            id: resp.payment_id,
            refunds: resp
                .refunds
                .map(|a| a.into_iter().map(Into::into).collect()),
            mandate_id: resp.mandate_id,
            next_action: into_stripe_next_action(resp.next_action, resp.return_url),
            last_payment_error: resp.error_code.map(|code| -> LastPaymentError {
                LastPaymentError {
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
                }
            }),
            connector_transaction_id: resp.connector_transaction_id,
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
    pub limit: u32,
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

fn default_limit() -> u32 {
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
