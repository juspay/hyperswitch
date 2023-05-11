use api_models::{self, enums as api_enums, payments};
use base64::Engine;
use common_utils::{errors::CustomResult, ext_traits::ByteSliceExt, pii, pii::Email};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, ExposeOptionInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;
use uuid::Uuid;

use crate::{
    collect_missing_value_keys, consts,
    core::errors,
    services,
    types::{self, api, storage::enums, transformers::ForeignFrom},
    utils::OptionExt,
};

pub struct StripeAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for StripeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = item {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StripeCaptureMethod {
    Manual,
    #[default]
    Automatic,
}

impl From<Option<enums::CaptureMethod>> for StripeCaptureMethod {
    fn from(item: Option<enums::CaptureMethod>) -> Self {
        match item {
            Some(p) => match p {
                enums::CaptureMethod::ManualMultiple => Self::Manual,
                enums::CaptureMethod::Manual => Self::Manual,
                enums::CaptureMethod::Automatic => Self::Automatic,
                enums::CaptureMethod::Scheduled => Self::Manual,
            },
            None => Self::Automatic,
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Auth3ds {
    #[default]
    Automatic,
    Any,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", untagged)]
pub enum StripeMandateType {
    Online {
        #[serde(rename = "mandate_data[customer_acceptance][type]")]
        mandate_type_enum: StripeMandateTypeEnum,
        #[serde(rename = "mandate_data[customer_acceptance][online][ip_address]")]
        ip_address: Secret<String, pii::IpAddress>,
        #[serde(rename = "mandate_data[customer_acceptance][online][user_agent]")]
        user_agent: String,
    },
    Offline {
        #[serde(rename = "mandate_data[customer_acceptance][type]")]
        mandate_type_enum: StripeMandateTypeEnum,
    },
}
#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripeMandateTypeEnum {
    Online,
    Offline,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeMandateRequest {
    #[serde(flatten)]
    mandate_type: StripeMandateType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PaymentIntentRequest {
    pub amount: i64, //amount in cents, hence passed as integer
    pub currency: String,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor: Option<String>,
    #[serde(rename = "metadata[order_id]")]
    pub metadata_order_id: String,
    #[serde(rename = "metadata[txn_id]")]
    pub metadata_txn_id: String,
    #[serde(rename = "metadata[txn_uuid]")]
    pub metadata_txn_uuid: String,
    pub return_url: String,
    pub confirm: bool,
    pub mandate: Option<String>,
    pub payment_method: Option<String>,
    pub customer: Option<String>,
    #[serde(flatten)]
    pub setup_mandate_details: Option<StripeMandateRequest>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub shipping: StripeShippingAddress,
    #[serde(flatten)]
    pub billing: StripeBillingAddress,
    #[serde(flatten)]
    pub payment_data: Option<StripePaymentMethodData>,
    pub capture_method: StripeCaptureMethod,
    pub payment_method_options: Option<StripePaymentMethodOptions>, // For mandate txns using network_txns_id, needs to be validated
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SetupIntentRequest {
    #[serde(rename = "metadata[order_id]")]
    pub metadata_order_id: String,
    #[serde(rename = "metadata[txn_id]")]
    pub metadata_txn_id: String,
    #[serde(rename = "metadata[txn_uuid]")]
    pub metadata_txn_uuid: String,
    pub confirm: bool,
    pub usage: Option<enums::FutureUsage>,
    pub customer: Option<String>,
    pub off_session: Option<bool>,
    pub return_url: Option<String>,
    #[serde(flatten)]
    pub payment_data: StripePaymentMethodData,
    pub payment_method_options: Option<StripePaymentMethodOptions>, // For mandate txns using network_txns_id, needs to be validated
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeCardData {
    #[serde(rename = "payment_method_types[]")]
    pub payment_method_types: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[card][number]")]
    pub payment_method_data_card_number: cards::CardNumber,
    #[serde(rename = "payment_method_data[card][exp_month]")]
    pub payment_method_data_card_exp_month: Secret<String>,
    #[serde(rename = "payment_method_data[card][exp_year]")]
    pub payment_method_data_card_exp_year: Secret<String>,
    #[serde(rename = "payment_method_data[card][cvc]")]
    pub payment_method_data_card_cvc: Secret<String>,
    #[serde(rename = "payment_method_options[card][request_three_d_secure]")]
    pub payment_method_auth_type: Auth3ds,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripePayLaterData {
    #[serde(rename = "payment_method_types[]")]
    pub payment_method_types: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TokenRequest {
    #[serde(flatten)]
    pub token_data: StripePaymentMethodData,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct StripeTokenResponse {
    pub id: String,
    pub object: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CustomerRequest {
    pub description: Option<String>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
    pub name: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct StripeCustomerResponse {
    pub id: String,
    pub description: Option<String>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
    pub name: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripeBankName {
    Eps {
        #[serde(rename = "payment_method_data[eps][bank]")]
        bank_name: StripeBankNames,
    },
    Ideal {
        #[serde(rename = "payment_method_data[ideal][bank]")]
        ideal_bank_name: StripeBankNames,
    },
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BankSpecificData {
    Sofort {
        #[serde(rename = "payment_method_options[sofort][preferred_language]")]
        preferred_language: String,
        #[serde(rename = "payment_method_data[sofort][country]")]
        country: api_enums::CountryAlpha2,
    },
}

fn get_bank_name(
    stripe_pm_type: &StripePaymentMethodType,
    bank_redirect_data: &api_models::payments::BankRedirectData,
) -> Result<Option<StripeBankName>, errors::ConnectorError> {
    match (stripe_pm_type, bank_redirect_data) {
        (
            StripePaymentMethodType::Eps,
            api_models::payments::BankRedirectData::Eps { ref bank_name, .. },
        ) => Ok(Some(StripeBankName::Eps {
            bank_name: StripeBankNames::try_from(bank_name)?,
        })),
        (
            StripePaymentMethodType::Ideal,
            api_models::payments::BankRedirectData::Ideal { bank_name, .. },
        ) => Ok(Some(StripeBankName::Ideal {
            ideal_bank_name: StripeBankNames::try_from(bank_name)?,
        })),
        (StripePaymentMethodType::Sofort | StripePaymentMethodType::Giropay, _) => Ok(None),
        _ => Err(errors::ConnectorError::MismatchedPaymentData),
    }
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeBankRedirectData {
    #[serde(rename = "payment_method_types[]")]
    pub payment_method_types: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    // Required only for eps and ideal
    #[serde(flatten)]
    pub bank_name: Option<StripeBankName>,
    #[serde(flatten)]
    pub bank_specific_data: Option<BankSpecificData>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "payment_method_data[type]")]
pub enum BankDebitData {
    #[serde(rename = "us_bank_account")]
    Ach {
        #[serde(rename = "payment_method_data[us_bank_account][account_holder_type]")]
        account_holder_type: String,
        #[serde(rename = "payment_method_data[us_bank_account][account_number]")]
        account_number: Secret<String>,
        #[serde(rename = "payment_method_data[us_bank_account][routing_number]")]
        routing_number: Secret<String>,
    },
    #[serde(rename = "sepa_debit")]
    Sepa {
        #[serde(rename = "payment_method_data[sepa_debit][iban]")]
        iban: Secret<String>,
    },
    #[serde(rename = "au_becs_debit")]
    Becs {
        #[serde(rename = "payment_method_data[au_becs_debit][account_number]")]
        account_number: Secret<String>,
        #[serde(rename = "payment_method_data[au_becs_debit][bsb_number]")]
        bsb_number: Secret<String>,
    },
    #[serde(rename = "bacs_debit")]
    Bacs {
        #[serde(rename = "payment_method_data[bacs_debit][account_number]")]
        account_number: Secret<String>,
        #[serde(rename = "payment_method_data[bacs_debit][sort_code]")]
        sort_code: Secret<String>,
    },
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeBankDebitData {
    #[serde(rename = "payment_method_types[0]")]
    pub payment_method_types: StripePaymentMethodType,
    #[serde(flatten)]
    pub bank_specific_data: BankDebitData,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeBankDebitType {
    #[serde(rename = "payment_method_types[]")]
    pub payment_method_types: StripePaymentMethodType,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripeBankDebitEnum {
    Type(StripeBankDebitType),
    Data(StripeBankDebitData),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripePaymentMethodData {
    Card(StripeCardData),
    PayLater(StripePayLaterData),
    Wallet(StripeWallet),
    BankRedirect(StripeBankRedirectData),
    BankDebit(StripeBankDebitEnum),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripeWallet {
    ApplepayToken(StripeApplePay),
    GooglepayToken(GooglePayToken),
    ApplepayPayment(ApplepayPayment),
    WechatpayPayment(WechatpayPayment),
    AlipayPayment(AlipayPayment),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeApplePay {
    pub pk_token: String,
    pub pk_token_instrument_name: String,
    pub pk_token_payment_network: String,
    pub pk_token_transaction_id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GooglePayToken {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[card][token]")]
    pub token: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ApplepayPayment {
    #[serde(rename = "payment_method_data[card][token]")]
    pub token: String,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_types: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AlipayPayment {
    #[serde(rename = "payment_method_types[]")]
    pub payment_method_types: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct WechatpayPayment {
    #[serde(rename = "payment_method_types[]")]
    pub payment_method_types: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_options[wechat_pay][client]")]
    pub client: WechatClient,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum WechatClient {
    Web,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GooglepayPayment {
    #[serde(rename = "payment_method_data[card][token]")]
    pub token: String,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_types: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodType {
    Card,
    Klarna,
    Affirm,
    AfterpayClearpay,
    Eps,
    Giropay,
    Ideal,
    Sofort,
    ApplePay,
    #[serde(rename = "us_bank_account")]
    Ach,
    #[serde(rename = "sepa_debit")]
    Sepa,
    #[serde(rename = "au_becs_debit")]
    Becs,
    #[serde(rename = "bacs_debit")]
    Bacs,
    #[serde(rename = "wechat_pay")]
    Wechatpay,
    Alipay,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripeBankNames {
    AbnAmro,
    ArzteUndApothekerBank,
    AsnBank,
    AustrianAnadiBankAg,
    BankAustria,
    BankhausCarlSpangler,
    BankhausSchelhammerUndSchatteraAg,
    BawagPskAg,
    BksBankAg,
    BrullKallmusBankAg,
    BtvVierLanderBank,
    Bunq,
    CapitalBankGraweGruppeAg,
    Dolomitenbank,
    EasybankAg,
    ErsteBankUndSparkassen,
    Handelsbanken,
    HypoAlpeadriabankInternationalAg,
    HypoNoeLbFurNiederosterreichUWien,
    HypoOberosterreichSalzburgSteiermark,
    HypoTirolBankAg,
    HypoVorarlbergBankAg,
    HypoBankBurgenlandAktiengesellschaft,
    Ing,
    Knab,
    MarchfelderBank,
    OberbankAg,
    RaiffeisenBankengruppeOsterreich,
    SchoellerbankAg,
    SpardaBankWien,
    VolksbankGruppe,
    VolkskreditbankAg,
    VrBankBraunau,
    Moneyou,
    Rabobank,
    Regiobank,
    Revolut,
    SnsBank,
    TriodosBank,
    VanLanschot,
}

impl TryFrom<WebhookEventStatus> for api_models::webhooks::IncomingWebhookEvent {
    type Error = errors::ConnectorError;
    fn try_from(value: WebhookEventStatus) -> Result<Self, Self::Error> {
        Ok(match value {
            WebhookEventStatus::WarningNeedsResponse => Self::DisputeOpened,
            WebhookEventStatus::WarningClosed => Self::DisputeCancelled,
            WebhookEventStatus::WarningUnderReview => Self::DisputeChallenged,
            WebhookEventStatus::Won => Self::DisputeWon,
            WebhookEventStatus::Lost => Self::DisputeLost,
            _ => Err(errors::ConnectorError::WebhookEventTypeNotFound)?,
        })
    }
}

impl TryFrom<&api_models::enums::BankNames> for StripeBankNames {
    type Error = errors::ConnectorError;
    fn try_from(bank: &api_models::enums::BankNames) -> Result<Self, Self::Error> {
        Ok(match bank {
            api_models::enums::BankNames::AbnAmro => Self::AbnAmro,
            api_models::enums::BankNames::ArzteUndApothekerBank => Self::ArzteUndApothekerBank,
            api_models::enums::BankNames::AsnBank => Self::AsnBank,
            api_models::enums::BankNames::AustrianAnadiBankAg => Self::AustrianAnadiBankAg,
            api_models::enums::BankNames::BankAustria => Self::BankAustria,
            api_models::enums::BankNames::BankhausCarlSpangler => Self::BankhausCarlSpangler,
            api_models::enums::BankNames::BankhausSchelhammerUndSchatteraAg => {
                Self::BankhausSchelhammerUndSchatteraAg
            }
            api_models::enums::BankNames::BawagPskAg => Self::BawagPskAg,
            api_models::enums::BankNames::BksBankAg => Self::BksBankAg,
            api_models::enums::BankNames::BrullKallmusBankAg => Self::BrullKallmusBankAg,
            api_models::enums::BankNames::BtvVierLanderBank => Self::BtvVierLanderBank,
            api_models::enums::BankNames::Bunq => Self::Bunq,
            api_models::enums::BankNames::CapitalBankGraweGruppeAg => {
                Self::CapitalBankGraweGruppeAg
            }
            api_models::enums::BankNames::Dolomitenbank => Self::Dolomitenbank,
            api_models::enums::BankNames::EasybankAg => Self::EasybankAg,
            api_models::enums::BankNames::ErsteBankUndSparkassen => Self::ErsteBankUndSparkassen,
            api_models::enums::BankNames::Handelsbanken => Self::Handelsbanken,
            api_models::enums::BankNames::HypoAlpeadriabankInternationalAg => {
                Self::HypoAlpeadriabankInternationalAg
            }
            api_models::enums::BankNames::HypoNoeLbFurNiederosterreichUWien => {
                Self::HypoNoeLbFurNiederosterreichUWien
            }
            api_models::enums::BankNames::HypoOberosterreichSalzburgSteiermark => {
                Self::HypoOberosterreichSalzburgSteiermark
            }
            api_models::enums::BankNames::HypoTirolBankAg => Self::HypoTirolBankAg,
            api_models::enums::BankNames::HypoVorarlbergBankAg => Self::HypoVorarlbergBankAg,
            api_models::enums::BankNames::HypoBankBurgenlandAktiengesellschaft => {
                Self::HypoBankBurgenlandAktiengesellschaft
            }
            api_models::enums::BankNames::Ing => Self::Ing,
            api_models::enums::BankNames::Knab => Self::Knab,
            api_models::enums::BankNames::MarchfelderBank => Self::MarchfelderBank,
            api_models::enums::BankNames::OberbankAg => Self::OberbankAg,
            api_models::enums::BankNames::RaiffeisenBankengruppeOsterreich => {
                Self::RaiffeisenBankengruppeOsterreich
            }
            api_models::enums::BankNames::Rabobank => Self::Rabobank,
            api_models::enums::BankNames::Regiobank => Self::Regiobank,
            api_models::enums::BankNames::Revolut => Self::Revolut,
            api_models::enums::BankNames::SnsBank => Self::SnsBank,
            api_models::enums::BankNames::TriodosBank => Self::TriodosBank,
            api_models::enums::BankNames::VanLanschot => Self::VanLanschot,
            api_models::enums::BankNames::Moneyou => Self::Moneyou,
            api_models::enums::BankNames::SchoellerbankAg => Self::SchoellerbankAg,
            api_models::enums::BankNames::SpardaBankWien => Self::SpardaBankWien,
            api_models::enums::BankNames::VolksbankGruppe => Self::VolksbankGruppe,
            api_models::enums::BankNames::VolkskreditbankAg => Self::VolkskreditbankAg,
            api_models::enums::BankNames::VrBankBraunau => Self::VrBankBraunau,
            _ => Err(errors::ConnectorError::NotSupported {
                message: api_enums::PaymentMethod::BankRedirect.to_string(),
                connector: "Stripe",
                payment_experience: api_enums::PaymentExperience::RedirectToUrl.to_string(),
            })?,
        })
    }
}

fn validate_shipping_address_against_payment_method(
    shipping_address: &StripeShippingAddress,
    payment_method: &StripePaymentMethodType,
) -> Result<(), error_stack::Report<errors::ConnectorError>> {
    if let StripePaymentMethodType::AfterpayClearpay = payment_method {
        let missing_fields = collect_missing_value_keys!(
            ("shipping.address.first_name", shipping_address.name),
            ("shipping.address.line1", shipping_address.line1),
            ("shipping.address.country", shipping_address.country),
            ("shipping.address.zip", shipping_address.zip)
        );

        if !missing_fields.is_empty() {
            return Err(errors::ConnectorError::MissingRequiredFields {
                field_names: missing_fields,
            })
            .into_report();
        }
    }

    Ok(())
}

fn infer_stripe_pay_later_type(
    pm_type: &enums::PaymentMethodType,
    experience: &enums::PaymentExperience,
) -> Result<StripePaymentMethodType, errors::ConnectorError> {
    if &enums::PaymentExperience::RedirectToUrl == experience {
        match pm_type {
            enums::PaymentMethodType::Klarna => Ok(StripePaymentMethodType::Klarna),
            enums::PaymentMethodType::Affirm => Ok(StripePaymentMethodType::Affirm),
            enums::PaymentMethodType::AfterpayClearpay => {
                Ok(StripePaymentMethodType::AfterpayClearpay)
            }
            _ => Err(errors::ConnectorError::NotSupported {
                message: pm_type.to_string(),
                connector: "stripe",
                payment_experience: experience.to_string(),
            }),
        }
    } else {
        Err(errors::ConnectorError::NotSupported {
            message: pm_type.to_string(),
            connector: "stripe",
            payment_experience: experience.to_string(),
        })
    }
}

fn infer_stripe_bank_redirect_issuer(
    payment_method_type: Option<&enums::PaymentMethodType>,
) -> Result<StripePaymentMethodType, errors::ConnectorError> {
    match payment_method_type {
        Some(storage_models::enums::PaymentMethodType::Giropay) => {
            Ok(StripePaymentMethodType::Giropay)
        }
        Some(storage_models::enums::PaymentMethodType::Ideal) => Ok(StripePaymentMethodType::Ideal),
        Some(storage_models::enums::PaymentMethodType::Sofort) => {
            Ok(StripePaymentMethodType::Sofort)
        }
        Some(storage_models::enums::PaymentMethodType::Eps) => Ok(StripePaymentMethodType::Eps),
        None => Err(errors::ConnectorError::MissingRequiredField {
            field_name: "payment_method_type",
        }),
        _ => Err(errors::ConnectorError::MismatchedPaymentData),
    }
}

impl TryFrom<(&api_models::payments::PayLaterData, StripePaymentMethodType)>
    for StripeBillingAddress
{
    type Error = errors::ConnectorError;

    fn try_from(
        (pay_later_data, pm_type): (&api_models::payments::PayLaterData, StripePaymentMethodType),
    ) -> Result<Self, Self::Error> {
        match (pay_later_data, pm_type) {
            (
                payments::PayLaterData::KlarnaRedirect {
                    billing_email,
                    billing_country,
                },
                StripePaymentMethodType::Klarna,
            ) => Ok(Self {
                email: Some(billing_email.to_owned()),
                country: Some(billing_country.to_owned()),
                ..Self::default()
            }),
            (payments::PayLaterData::AffirmRedirect {}, StripePaymentMethodType::Affirm) => {
                Ok(Self::default())
            }
            (
                payments::PayLaterData::AfterpayClearpayRedirect {
                    billing_email,
                    billing_name,
                },
                StripePaymentMethodType::AfterpayClearpay,
            ) => Ok(Self {
                email: Some(billing_email.to_owned()),
                name: Some(billing_name.to_owned()),
                ..Self::default()
            }),
            _ => Err(errors::ConnectorError::MismatchedPaymentData),
        }
    }
}

impl From<&payments::BankDebitBilling> for StripeBillingAddress {
    fn from(item: &payments::BankDebitBilling) -> Self {
        Self {
            email: Some(item.email.to_owned()),
            country: item
                .address
                .as_ref()
                .and_then(|address| address.country.to_owned()),
            name: Some(item.name.to_owned()),
            city: item
                .address
                .as_ref()
                .and_then(|address| address.city.to_owned()),
            address_line1: item
                .address
                .as_ref()
                .and_then(|address| address.line1.to_owned()),
            address_line2: item
                .address
                .as_ref()
                .and_then(|address| address.line2.to_owned()),
            zip_code: item
                .address
                .as_ref()
                .and_then(|address| address.zip.to_owned()),
        }
    }
}

impl TryFrom<&payments::BankRedirectData> for StripeBillingAddress {
    type Error = errors::ConnectorError;

    fn try_from(bank_redirection_data: &payments::BankRedirectData) -> Result<Self, Self::Error> {
        match bank_redirection_data {
            payments::BankRedirectData::Eps {
                billing_details, ..
            } => Ok(Self {
                name: Some(billing_details.billing_name.clone()),
                ..Self::default()
            }),
            payments::BankRedirectData::Giropay {
                billing_details, ..
            } => Ok(Self {
                name: Some(billing_details.billing_name.clone()),
                ..Self::default()
            }),
            payments::BankRedirectData::Ideal {
                billing_details, ..
            } => Ok(Self {
                name: Some(billing_details.billing_name.clone()),
                ..Self::default()
            }),
            _ => Ok(Self::default()),
        }
    }
}

fn get_bank_specific_data(
    bank_redirect_data: &payments::BankRedirectData,
) -> Option<BankSpecificData> {
    match bank_redirect_data {
        payments::BankRedirectData::Sofort {
            country,
            preferred_language,
            ..
        } => Some(BankSpecificData::Sofort {
            country: country.to_owned(),
            preferred_language: preferred_language.to_owned(),
        }),
        _ => None,
    }
}

fn get_bank_debit_data(
    bank_debit_data: &payments::BankDebitData,
) -> (StripePaymentMethodType, BankDebitData, StripeBillingAddress) {
    match bank_debit_data {
        payments::BankDebitData::AchBankDebit {
            billing_details,
            account_number,
            routing_number,
            ..
        } => {
            let ach_data = BankDebitData::Ach {
                account_holder_type: "individual".to_string(),
                account_number: account_number.to_owned(),
                routing_number: routing_number.to_owned(),
            };

            let billing_data = StripeBillingAddress::from(billing_details);
            (StripePaymentMethodType::Ach, ach_data, billing_data)
        }
        payments::BankDebitData::SepaBankDebit {
            billing_details,
            iban,
            ..
        } => {
            let sepa_data = BankDebitData::Sepa {
                iban: iban.to_owned(),
            };

            let billing_data = StripeBillingAddress::from(billing_details);
            (StripePaymentMethodType::Sepa, sepa_data, billing_data)
        }
        payments::BankDebitData::BecsBankDebit {
            billing_details,
            account_number,
            bsb_number,
        } => {
            let becs_data = BankDebitData::Becs {
                account_number: account_number.to_owned(),
                bsb_number: bsb_number.to_owned(),
            };

            let billing_data = StripeBillingAddress::from(billing_details);
            (StripePaymentMethodType::Becs, becs_data, billing_data)
        }
        payments::BankDebitData::BacsBankDebit {
            billing_details,
            account_number,
            sort_code,
        } => {
            let bacs_data = BankDebitData::Bacs {
                account_number: account_number.to_owned(),
                sort_code: sort_code.to_owned(),
            };

            let billing_data = StripeBillingAddress::from(billing_details);
            (StripePaymentMethodType::Bacs, bacs_data, billing_data)
        }
    }
}

fn create_stripe_payment_method(
    pm_type: Option<&enums::PaymentMethodType>,
    experience: Option<&enums::PaymentExperience>,
    payment_method_data: &api_models::payments::PaymentMethodData,
    auth_type: enums::AuthenticationType,
) -> Result<
    (
        StripePaymentMethodData,
        StripePaymentMethodType,
        StripeBillingAddress,
    ),
    error_stack::Report<errors::ConnectorError>,
> {
    match payment_method_data {
        payments::PaymentMethodData::Card(card_details) => {
            let payment_method_auth_type = match auth_type {
                enums::AuthenticationType::ThreeDs => Auth3ds::Any,
                enums::AuthenticationType::NoThreeDs => Auth3ds::Automatic,
            };
            Ok((
                StripePaymentMethodData::Card(StripeCardData {
                    payment_method_types: StripePaymentMethodType::Card,
                    payment_method_data_type: StripePaymentMethodType::Card,
                    payment_method_data_card_number: card_details.card_number.clone(),
                    payment_method_data_card_exp_month: card_details.card_exp_month.clone(),
                    payment_method_data_card_exp_year: card_details.card_exp_year.clone(),
                    payment_method_data_card_cvc: card_details.card_cvc.clone(),
                    payment_method_auth_type,
                }),
                StripePaymentMethodType::Card,
                StripeBillingAddress::default(),
            ))
        }
        payments::PaymentMethodData::PayLater(pay_later_data) => {
            let pm_type = pm_type.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_type",
            })?;

            let pm_experience = experience.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_experience",
            })?;

            let stripe_pm_type = infer_stripe_pay_later_type(pm_type, pm_experience)?;

            let billing_address = StripeBillingAddress::try_from((pay_later_data, stripe_pm_type))?;

            Ok((
                StripePaymentMethodData::PayLater(StripePayLaterData {
                    payment_method_types: stripe_pm_type,
                    payment_method_data_type: stripe_pm_type,
                }),
                stripe_pm_type,
                billing_address,
            ))
        }
        payments::PaymentMethodData::BankRedirect(bank_redirect_data) => {
            let billing_address = StripeBillingAddress::try_from(bank_redirect_data)?;
            let pm_type = infer_stripe_bank_redirect_issuer(pm_type)?;
            let bank_specific_data = get_bank_specific_data(bank_redirect_data);
            let bank_name = get_bank_name(&pm_type, bank_redirect_data)?;
            Ok((
                StripePaymentMethodData::BankRedirect(StripeBankRedirectData {
                    payment_method_types: pm_type,
                    payment_method_data_type: pm_type,
                    bank_name,
                    bank_specific_data,
                }),
                pm_type,
                billing_address,
            ))
        }
        payments::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
            payments::WalletData::ApplePay(applepay_data) => Ok((
                StripePaymentMethodData::Wallet(StripeWallet::ApplepayToken(StripeApplePay {
                    pk_token: String::from_utf8(
                        consts::BASE64_ENGINE
                            .decode(&applepay_data.payment_data)
                            .into_report()
                            .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                    )
                    .into_report()
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                    pk_token_instrument_name: applepay_data.payment_method.pm_type.to_owned(),
                    pk_token_payment_network: applepay_data.payment_method.network.to_owned(),
                    pk_token_transaction_id: applepay_data.transaction_identifier.to_owned(),
                })),
                StripePaymentMethodType::ApplePay,
                StripeBillingAddress::default(),
            )),

            payments::WalletData::WeChatPayRedirect(_) => Ok((
                StripePaymentMethodData::Wallet(StripeWallet::WechatpayPayment(WechatpayPayment {
                    client: WechatClient::Web,
                    payment_method_types: StripePaymentMethodType::Wechatpay,
                    payment_method_data_type: StripePaymentMethodType::Wechatpay,
                })),
                StripePaymentMethodType::Wechatpay,
                StripeBillingAddress::default(),
            )),
            payments::WalletData::AliPay(_) => Ok((
                StripePaymentMethodData::Wallet(StripeWallet::AlipayPayment(AlipayPayment {
                    payment_method_types: StripePaymentMethodType::Alipay,
                    payment_method_data_type: StripePaymentMethodType::Alipay,
                })),
                StripePaymentMethodType::Alipay,
                StripeBillingAddress::default(),
            )),
            payments::WalletData::GooglePay(gpay_data) => Ok((
                StripePaymentMethodData::try_from(gpay_data)?,
                StripePaymentMethodType::Card,
                StripeBillingAddress::default(),
            )),
            _ => Err(errors::ConnectorError::NotImplemented(
                "This wallet is not implemented for stripe".to_string(),
            )
            .into()),
        },
        payments::PaymentMethodData::BankDebit(bank_debit_data) => {
            let (pm_type, bank_debit_data, billing_address) = get_bank_debit_data(bank_debit_data);

            let pm_data = StripePaymentMethodData::BankDebit(StripeBankDebitEnum::Data(
                StripeBankDebitData {
                    payment_method_types: pm_type,
                    bank_specific_data: bank_debit_data,
                },
            ));

            Ok((pm_data, pm_type, billing_address))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "this payment method for stripe".to_string(),
        )
        .into()),
    }
}

impl TryFrom<&payments::GooglePayWalletData> for StripePaymentMethodData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(gpay_data: &payments::GooglePayWalletData) -> Result<Self, Self::Error> {
        Ok(Self::Wallet(StripeWallet::GooglepayToken(GooglePayToken {
            token: gpay_data
                .tokenization_data
                .token
                .as_bytes()
                .parse_struct::<StripeGpayToken>("StripeGpayToken")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
                .id,
            payment_type: StripePaymentMethodType::Card,
        })))
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PaymentIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let metadata_order_id = item.payment_id.to_string();
        let metadata_txn_id = format!("{}_{}_{}", item.merchant_id, item.payment_id, "1");
        let metadata_txn_uuid = Uuid::new_v4().to_string(); //Fetch autogenerated txn_uuid from Database.

        let shipping_address = match item.address.shipping.clone() {
            Some(mut shipping) => StripeShippingAddress {
                city: shipping.address.as_mut().and_then(|a| a.city.take()),
                country: shipping.address.as_mut().and_then(|a| a.country.take()),
                line1: shipping.address.as_mut().and_then(|a| a.line1.take()),
                line2: shipping.address.as_mut().and_then(|a| a.line2.take()),
                zip: shipping.address.as_mut().and_then(|a| a.zip.take()),
                state: shipping.address.as_mut().and_then(|a| a.state.take()),
                name: shipping.address.as_mut().and_then(|a| {
                    a.first_name.as_ref().map(|first_name| {
                        format!(
                            "{} {}",
                            first_name.clone().expose(),
                            a.last_name.clone().expose_option().unwrap_or_default()
                        )
                        .into()
                    })
                }),
                phone: shipping.phone.map(|p| {
                    format!(
                        "{}{}",
                        p.country_code.unwrap_or_default(),
                        p.number.expose_option().unwrap_or_default()
                    )
                    .into()
                }),
            },
            None => StripeShippingAddress::default(),
        };
        let mut payment_method_options = None;

        let (mut payment_data, payment_method, mandate, billing_address) = {
            match item
                .request
                .mandate_id
                .clone()
                .and_then(|mandate_ids| mandate_ids.mandate_reference_id)
            {
                Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                    connector_mandate_ids,
                )) => (
                    None,
                    connector_mandate_ids.payment_method_id,
                    connector_mandate_ids.connector_mandate_id,
                    StripeBillingAddress::default(),
                ),
                Some(api_models::payments::MandateReferenceId::NetworkMandateId(
                    network_transaction_id,
                )) => {
                    payment_method_options = Some(StripePaymentMethodOptions::Card {
                        mandate_options: None,
                        network_transaction_id: None,
                        mit_exemption: Some(MitExemption {
                            network_transaction_id,
                        }),
                    });
                    (None, None, None, StripeBillingAddress::default())
                }
                _ => {
                    let (payment_method_data, payment_method_type, billing_address) =
                        create_stripe_payment_method(
                            item.request.payment_method_type.as_ref(),
                            item.request.payment_experience.as_ref(),
                            &item.request.payment_method_data,
                            item.auth_type,
                        )?;

                    validate_shipping_address_against_payment_method(
                        &shipping_address,
                        &payment_method_type,
                    )?;

                    (Some(payment_method_data), None, None, billing_address)
                }
            }
        };

        if payment_method.is_some() {
            payment_data = get_payment_method_data_from_mandate_metadata(item);
        }
        payment_data = match item.request.payment_method_data {
            payments::PaymentMethodData::Wallet(payments::WalletData::ApplePay(_)) => Some(
                StripePaymentMethodData::Wallet(StripeWallet::ApplepayPayment(ApplepayPayment {
                    token: item
                        .payment_method_token
                        .to_owned()
                        .get_required_value("payment_token")
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                    payment_method_types: StripePaymentMethodType::Card,
                })),
            ),
            _ => payment_data,
        };

        let mut setup_mandate_details =
            item.request
                .setup_mandate_details
                .as_ref()
                .and_then(|mandate_details| {
                    match mandate_details.customer_acceptance.acceptance_type {
                        payments::AcceptanceType::Online => {
                            mandate_details.customer_acceptance.online.as_ref().map(
                                |online_details| StripeMandateRequest {
                                    mandate_type: StripeMandateType::Online {
                                        mandate_type_enum: StripeMandateTypeEnum::Online,
                                        ip_address: online_details.ip_address.clone(),
                                        user_agent: online_details.user_agent.clone(),
                                    },
                                },
                            )
                        }
                        payments::AcceptanceType::Offline => Some(StripeMandateRequest {
                            mandate_type: StripeMandateType::Offline {
                                mandate_type_enum: StripeMandateTypeEnum::Offline,
                            },
                        }),
                    }
                });
        setup_mandate_details = if setup_mandate_details.is_none() {
            //stripe requires us to send mandate_data when payment method is bank debit attached to the customer
            match &payment_data {
                Some(StripePaymentMethodData::BankDebit(_)) => Some(StripeMandateRequest {
                    mandate_type: StripeMandateType::Offline {
                        mandate_type_enum: StripeMandateTypeEnum::Offline,
                    },
                }),
                Some(_) => None,
                None => None,
            }
        } else {
            setup_mandate_details
        };
        Ok(Self {
            amount: item.request.amount, //hopefully we don't loose some cents here
            currency: item.request.currency.to_string(), //we need to copy the value and not transfer ownership
            statement_descriptor_suffix: item.request.statement_descriptor_suffix.clone(),
            statement_descriptor: item.request.statement_descriptor.clone(),
            metadata_order_id,
            metadata_txn_id,
            metadata_txn_uuid,
            return_url: item
                .request
                .router_return_url
                .clone()
                .unwrap_or_else(|| "https://juspay.in/".to_string()),
            confirm: true, // Stripe requires confirm to be true if return URL is present
            description: item.description.clone(),
            shipping: shipping_address,
            billing: billing_address,
            capture_method: StripeCaptureMethod::from(item.request.capture_method),
            payment_data,
            mandate,
            payment_method_options,
            payment_method,
            customer: item.connector_customer.to_owned(),
            setup_mandate_details,
            off_session: item.request.off_session,
            setup_future_usage: item.request.setup_future_usage,
        })
    }
}

fn get_payment_method_data_from_mandate_metadata(
    item: &types::PaymentsAuthorizeRouterData,
) -> Option<StripePaymentMethodData> {
    match item.mandate_metadata.clone() {
        Some(value) => {
            let payment_method_data =
                serde_json::from_value::<api_models::payments::PaymentMethodData>(value.expose())
                    .ok();
            match payment_method_data {
                Some(payments::PaymentMethodData::BankDebit(data)) => {
                    let stripe_payment_method_data = create_stripe_payment_method(
                        item.request.payment_method_type.as_ref(),
                        item.request.payment_experience.as_ref(),
                        &payments::PaymentMethodData::BankDebit(data),
                        item.auth_type,
                    )
                    .ok();
                    let stripe_payment_method_data = stripe_payment_method_data.map(|t| t.0);
                    match stripe_payment_method_data {
                        Some(StripePaymentMethodData::BankDebit(stripe_bank_debit_enum)) => {
                            match stripe_bank_debit_enum {
                                StripeBankDebitEnum::Type(stripe_bank_debit_type) => {
                                    Some(StripePaymentMethodData::BankDebit(
                                        StripeBankDebitEnum::Type(stripe_bank_debit_type),
                                    ))
                                }
                                StripeBankDebitEnum::Data(stripe_bank_debit_data) => Some(
                                    StripePaymentMethodData::BankDebit(StripeBankDebitEnum::Type(
                                        StripeBankDebitType::from(stripe_bank_debit_data),
                                    )),
                                ),
                            }
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        None => None,
    }
}

impl From<StripeBankDebitData> for StripeBankDebitType {
    fn from(value: StripeBankDebitData) -> Self {
        Self {
            payment_method_types: value.payment_method_types,
        }
    }
}

impl TryFrom<&types::VerifyRouterData> for SetupIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::VerifyRouterData) -> Result<Self, Self::Error> {
        let metadata_order_id = item.payment_id.to_string();
        let metadata_txn_id = format!("{}_{}_{}", item.merchant_id, item.payment_id, "1");
        let metadata_txn_uuid = Uuid::new_v4().to_string();

        //Only cards supported for mandates
        let pm_type = StripePaymentMethodType::Card;
        let payment_data = StripePaymentMethodData::try_from((
            item.request.payment_method_data.clone(),
            item.auth_type,
            pm_type,
        ))?;

        Ok(Self {
            confirm: true,
            metadata_order_id,
            metadata_txn_id,
            metadata_txn_uuid,
            payment_data,
            return_url: item.return_url.clone(),
            off_session: item.request.off_session,
            usage: item.request.setup_future_usage,
            payment_method_options: None,
            customer: item.connector_customer.to_owned(),
        })
    }
}

impl TryFrom<&types::TokenizationRouterData> for TokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        let payment_data = create_stripe_payment_method(
            None,
            None,
            &item.request.payment_method_data,
            item.auth_type,
        )?;
        Ok(Self {
            token_data: payment_data.0,
        })
    }
}

impl TryFrom<&types::ConnectorCustomerRouterData> for CustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            description: item.request.description.to_owned(),
            email: item.request.email.to_owned(),
            phone: item.request.phone.to_owned(),
            name: item.request.name.to_owned(),
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeMetadata {
    pub order_id: String,
    pub txn_id: String,
    pub txn_uuid: String,
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    #[serde(rename = "requires_action")]
    RequiresCustomerAction,
    #[serde(rename = "requires_payment_method")]
    RequiresPaymentMethod,
    RequiresConfirmation,
    Canceled,
    RequiresCapture,
    // This is the case in
    Pending,
}

impl From<StripePaymentStatus> for enums::AttemptStatus {
    fn from(item: StripePaymentStatus) -> Self {
        match item {
            StripePaymentStatus::Succeeded => Self::Charged,
            StripePaymentStatus::Failed => Self::Failure,
            StripePaymentStatus::Processing => Self::Authorizing,
            StripePaymentStatus::RequiresCustomerAction => Self::AuthenticationPending,
            StripePaymentStatus::RequiresPaymentMethod => Self::PaymentMethodAwaited,
            StripePaymentStatus::RequiresConfirmation => Self::ConfirmationAwaited,
            StripePaymentStatus::Canceled => Self::Voided,
            StripePaymentStatus::RequiresCapture => Self::Authorized,
            StripePaymentStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct PaymentIntentResponse {
    pub id: String,
    pub object: String,
    pub amount: i64,
    pub amount_received: i64,
    pub amount_capturable: i64,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub client_secret: Secret<String>,
    pub created: i32,
    pub customer: Option<String>,
    pub payment_method: Option<String>,
    pub description: Option<String>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: StripeMetadata,
    pub next_action: Option<StripeNextActionResponse>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
    pub last_payment_error: Option<ErrorDetails>,
    pub latest_attempt: Option<LatestAttempt>, //need a merchant to test this
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct PaymentSyncResponse {
    #[serde(flatten)]
    pub intent_fields: PaymentIntentResponse,
    pub last_payment_error: Option<ErrorDetails>,
}

impl std::ops::Deref for PaymentSyncResponse {
    type Target = PaymentIntentResponse;

    fn deref(&self) -> &Self::Target {
        &self.intent_fields
    }
}

#[derive(Serialize, Deserialize)]
pub struct LastPaymentError {
    code: String,
    message: String,
}

#[derive(Deserialize)]
pub struct PaymentIntentSyncResponse {
    #[serde(flatten)]
    payment_intent_fields: PaymentIntentResponse,
    pub last_payment_error: Option<LastPaymentError>,
}

impl std::ops::Deref for PaymentIntentSyncResponse {
    type Target = PaymentIntentResponse;

    fn deref(&self) -> &Self::Target {
        &self.payment_intent_fields
    }
}

#[derive(Deserialize)]
pub struct SetupIntentSyncResponse {
    #[serde(flatten)]
    setup_intent_fields: SetupIntentResponse,
    pub last_payment_error: Option<LastPaymentError>,
}

impl std::ops::Deref for SetupIntentSyncResponse {
    type Target = SetupIntentResponse;

    fn deref(&self) -> &Self::Target {
        &self.setup_intent_fields
    }
}

impl From<SetupIntentSyncResponse> for PaymentIntentSyncResponse {
    fn from(value: SetupIntentSyncResponse) -> Self {
        Self {
            payment_intent_fields: value.setup_intent_fields.into(),
            last_payment_error: value.last_payment_error,
        }
    }
}

impl From<SetupIntentResponse> for PaymentIntentResponse {
    fn from(value: SetupIntentResponse) -> Self {
        Self {
            id: value.id,
            object: value.object,
            status: value.status,
            client_secret: value.client_secret,
            customer: value.customer,
            description: None,
            statement_descriptor: value.statement_descriptor,
            statement_descriptor_suffix: value.statement_descriptor_suffix,
            metadata: value.metadata,
            next_action: value.next_action,
            payment_method_options: value.payment_method_options,
            last_payment_error: None,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct SetupIntentResponse {
    pub id: String,
    pub object: String,
    pub status: StripePaymentStatus, // Change to SetupStatus
    pub client_secret: Secret<String>,
    pub customer: Option<String>,
    pub payment_method: Option<String>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: StripeMetadata,
    pub next_action: Option<StripeNextActionResponse>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
    pub latest_attempt: Option<LatestAttempt>,
}

impl ForeignFrom<(Option<StripePaymentMethodOptions>, String)> for types::MandateReference {
    fn foreign_from(
        (payment_method_options, payment_method_id): (Option<StripePaymentMethodOptions>, String),
    ) -> Self {
        Self {
            connector_mandate_id: payment_method_options.and_then(|options| match options {
                StripePaymentMethodOptions::Card {
                    mandate_options, ..
                } => mandate_options.map(|mandate_options| mandate_options.reference),
                StripePaymentMethodOptions::Klarna {}
                | StripePaymentMethodOptions::Affirm {}
                | StripePaymentMethodOptions::AfterpayClearpay {}
                | StripePaymentMethodOptions::Eps {}
                | StripePaymentMethodOptions::Giropay {}
                | StripePaymentMethodOptions::Ideal {}
                | StripePaymentMethodOptions::Sofort {}
                | StripePaymentMethodOptions::Ach {}
                | StripePaymentMethodOptions::Bacs {}
                | StripePaymentMethodOptions::Becs {}
                | StripePaymentMethodOptions::WechatPay {}
                | StripePaymentMethodOptions::Alipay {}
                | StripePaymentMethodOptions::Sepa {} => None,
            }),
            payment_method_id: Some(payment_method_id),
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PaymentIntentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymentIntentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item.response.next_action.map(|next_action_response| {
            services::RedirectForm::from((next_action_response.get_url(), services::Method::Get))
        });

        let mandate_reference = item.response.payment_method.map(|pm| {
            types::MandateReference::foreign_from((item.response.payment_method_options, pm))
        });

        //Note: we might have to call retrieve_setup_intent to get the network_transaction_id in case its not sent in PaymentIntentResponse
        // Or we identify the mandate txns before hand and always call SetupIntent in case of mandate payment call
        let network_txn_id = Option::foreign_from(item.response.latest_attempt);

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            // client_secret: Some(item.response.client_secret.clone().as_str()),
            // description: item.response.description.map(|x| x.as_str()),
            // statement_descriptor_suffix: item.response.statement_descriptor_suffix.map(|x| x.as_str()),
            // three_ds_form,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference,
                connector_metadata: None,
                network_txn_id,
            }),
            amount_captured: Some(item.response.amount_received),
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PaymentIntentSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PaymentIntentSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .next_action
            .as_ref()
            .map(|next_action_response| {
                services::RedirectForm::from((
                    next_action_response.get_url(),
                    services::Method::Get,
                ))
            });

        let mandate_reference = item.response.payment_method.clone().map(|pm| {
            types::MandateReference::foreign_from((
                item.response.payment_method_options.clone(),
                pm,
            ))
        });
        let error_res =
            item.response
                .last_payment_error
                .as_ref()
                .map(|error| types::ErrorResponse {
                    code: error.code.to_owned(),
                    message: error.message.to_owned(),
                    reason: None,
                    status_code: item.http_code,
                });

        let response = error_res.map_or(
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data,
                mandate_reference,
                connector_metadata: None,
                network_txn_id: None,
            }),
            Err,
        );

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.to_owned()),
            response,
            amount_captured: Some(item.response.amount_received),
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, SetupIntentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, SetupIntentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item.response.next_action.map(|next_action_response| {
            services::RedirectForm::from((next_action_response.get_url(), services::Method::Get))
        });

        let mandate_reference = item.response.payment_method.map(|pm| {
            types::MandateReference::foreign_from((item.response.payment_method_options, pm))
        });

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference,
                connector_metadata: None,
                network_txn_id: Option::foreign_from(item.response.latest_attempt),
            }),
            ..item.data
        })
    }
}

impl ForeignFrom<Option<LatestAttempt>> for Option<String> {
    fn foreign_from(latest_attempt: Option<LatestAttempt>) -> Self {
        match latest_attempt {
            Some(LatestAttempt::PaymentIntentAttempt(attempt)) => attempt
                .payment_method_options
                .and_then(|payment_method_options| match payment_method_options {
                    StripePaymentMethodOptions::Card {
                        network_transaction_id,
                        ..
                    } => network_transaction_id,
                    _ => None,
                }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", remote = "Self")]
pub enum StripeNextActionResponse {
    RedirectToUrl(StripeRedirectToUrlResponse),
    AlipayHandleRedirect(StripeRedirectToUrlResponse),
    VerifyWithMicrodeposits(StripeVerifyWithMicroDepositsResponse),
    WechatPayDisplayQrCode(StripeRedirectToQr),
}

impl StripeNextActionResponse {
    fn get_url(&self) -> Url {
        match self {
            Self::RedirectToUrl(redirect_to_url) | Self::AlipayHandleRedirect(redirect_to_url) => {
                redirect_to_url.url.to_owned()
            }
            Self::WechatPayDisplayQrCode(redirect_to_url) => redirect_to_url.data.to_owned(),
            Self::VerifyWithMicrodeposits(verify_with_microdeposits) => {
                verify_with_microdeposits.hosted_verification_url.to_owned()
            }
        }
    }
}

// This impl is required because Stripe's response is of the below format, which is externally
// tagged, but also with an extra 'type' field specifying the enum variant name:
// "next_action": {
//   "redirect_to_url": { "return_url": "...", "url": "..." },
//   "type": "redirect_to_url"
// },
// Reference: https://github.com/serde-rs/serde/issues/1343#issuecomment-409698470
impl<'de> Deserialize<'de> for StripeNextActionResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(rename = "type")]
            _ignore: String,
            #[serde(flatten, with = "StripeNextActionResponse")]
            inner: StripeNextActionResponse,
        }
        Wrapper::deserialize(deserializer).map(|w| w.inner)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeRedirectToUrlResponse {
    return_url: String,
    url: Url,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeRedirectToQr {
    data: Url,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct StripeVerifyWithMicroDepositsResponse {
    hosted_verification_url: Url,
}

// REFUND :
// Type definition for Stripe RefundRequest

#[derive(Default, Debug, Serialize)]
pub struct RefundRequest {
    pub amount: Option<i64>, //amount in cents, hence passed as integer
    pub payment_intent: String,
    #[serde(rename = "metadata[order_id]")]
    pub metadata_order_id: String,
    #[serde(rename = "metadata[txn_id]")]
    pub metadata_txn_id: String,
    #[serde(rename = "metadata[txn_uuid]")]
    pub metadata_txn_uuid: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let amount = item.request.refund_amount;
        let metadata_txn_id = "Fetch txn_id from DB".to_string();
        let metadata_txn_uuid = "Fetch txn_id from DB".to_string();
        let payment_intent = item.request.connector_transaction_id.clone();
        Ok(Self {
            amount: Some(amount),
            payment_intent,
            metadata_order_id: item.payment_id.clone(),
            metadata_txn_id,
            metadata_txn_uuid,
        })
    }
}

// Type definition for Stripe Refund Response

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    RequiresAction,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            self::RefundStatus::Succeeded => Self::Success,
            self::RefundStatus::Failed => Self::Failure,
            self::RefundStatus::Pending => Self::Pending,
            self::RefundStatus::RequiresAction => Self::ManualReview,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub object: String,
    pub amount: i64,
    pub currency: String,
    pub metadata: StripeMetadata,
    pub payment_intent: String,
    pub status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeShippingAddress {
    #[serde(rename = "shipping[address][city]")]
    pub city: Option<String>,
    #[serde(rename = "shipping[address][country]")]
    pub country: Option<api_enums::CountryAlpha2>,
    #[serde(rename = "shipping[address][line1]")]
    pub line1: Option<Secret<String>>,
    #[serde(rename = "shipping[address][line2]")]
    pub line2: Option<Secret<String>>,
    #[serde(rename = "shipping[address][postal_code]")]
    pub zip: Option<Secret<String>>,
    #[serde(rename = "shipping[address][state]")]
    pub state: Option<Secret<String>>,
    #[serde(rename = "shipping[name]")]
    pub name: Option<Secret<String>>,
    #[serde(rename = "shipping[phone]")]
    pub phone: Option<Secret<String>>,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeBillingAddress {
    #[serde(rename = "payment_method_data[billing_details][email]")]
    pub email: Option<Email>,
    #[serde(rename = "payment_method_data[billing_details][address][country]")]
    pub country: Option<api_enums::CountryAlpha2>,
    #[serde(rename = "payment_method_data[billing_details][name]")]
    pub name: Option<Secret<String>>,
    #[serde(rename = "payment_method_data[billing_details][address][city]")]
    pub city: Option<String>,
    #[serde(rename = "payment_method_data[billing_details][address][line1]")]
    pub address_line1: Option<Secret<String>>,
    #[serde(rename = "payment_method_data[billing_details][address][line2]")]
    pub address_line2: Option<Secret<String>>,
    #[serde(rename = "payment_method_data[billing_details][address][postal_code]")]
    pub zip_code: Option<Secret<String>>,
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
pub struct StripeRedirectResponse {
    pub payment_intent: Option<String>,
    pub payment_intent_client_secret: Option<String>,
    pub source_redirect_slug: Option<String>,
    pub redirect_status: Option<StripePaymentStatus>,
    pub source_type: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct CancelRequest {
    cancellation_reason: Option<String>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for CancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            cancellation_reason: item.request.cancellation_reason.clone(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodOptions {
    Card {
        mandate_options: Option<StripeMandateOptions>,
        network_transaction_id: Option<String>,
        mit_exemption: Option<MitExemption>, // To be used for MIT mandate txns
    },
    Klarna {},
    Affirm {},
    AfterpayClearpay {},
    Eps {},
    Giropay {},
    Ideal {},
    Sofort {},
    #[serde(rename = "us_bank_account")]
    Ach {},
    #[serde(rename = "sepa_debit")]
    Sepa {},
    #[serde(rename = "au_becs_debit")]
    Becs {},
    #[serde(rename = "bacs_debit")]
    Bacs {},
    WechatPay {},
    Alipay {},
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MitExemption {
    pub network_transaction_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum LatestAttempt {
    PaymentIntentAttempt(LatestPaymentAttempt),
    SetupAttempt(String),
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct LatestPaymentAttempt {
    pub payment_method_options: Option<StripePaymentMethodOptions>,
}
// #[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
// pub struct Card
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct StripeMandateOptions {
    reference: String, // Extendable, But only important field to be captured
}
/// Represents the capture request body for stripe connector.
#[derive(Debug, Serialize, Clone, Copy)]
pub struct CaptureRequest {
    /// If amount_to_capture is None stripe captures the amount in the payment intent.
    amount_to_capture: Option<i64>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for CaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_to_capture: Some(item.request.amount_to_capture),
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StripeTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StripeTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.id,
            }),
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StripeCustomerResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StripeCustomerResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.id,
            }),
            ..item.data
        })
    }
}

// #[cfg(test)]
// mod test_stripe_transformers {
//     use super::*;

//     #[test]
//     fn verify_transform_from_router_to_stripe_req() {
//         let router_req = PaymentsRequest {
//             amount: 100.0,
//             currency: "USD".to_string(),
//             ..Default::default()
//         };

//         let stripe_req = PaymentIntentRequest::from(router_req);

//         //metadata is generated everytime. So use the transformed struct to copy uuid

//         let stripe_req_expected = PaymentIntentRequest {
//             amount: 10000,
//             currency: "USD".to_string(),
//             statement_descriptor_suffix: None,
//             metadata_order_id: "Auto generate Order ID".to_string(),
//             metadata_txn_id: "Fetch from Merchant Account_Auto generate Order ID_1".to_string(),
//             metadata_txn_uuid: stripe_req.metadata_txn_uuid.clone(),
//             return_url: "Fetch Url from Merchant Account".to_string(),
//             confirm: false,
//             payment_method_types: "card".to_string(),
//             payment_method_data_type: "card".to_string(),
//             payment_method_data_card_number: None,
//             payment_method_data_card_exp_month: None,
//             payment_method_data_card_exp_year: None,
//             payment_method_data_card_cvc: None,
//             description: None,
//         };
//         assert_eq!(stripe_req_expected, stripe_req);
//     }
// }

#[derive(Debug, Deserialize)]
pub struct WebhookEventDataResource {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEventObjectResource {
    pub data: WebhookEventDataResource,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEvent {
    #[serde(rename = "type")]
    pub event_type: WebhookEventType,
    #[serde(rename = "data")]
    pub event_data: WebhookEventData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEventData {
    #[serde(rename = "object")]
    pub event_object: WebhookEventObjectData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEventObjectData {
    pub id: String,
    pub object: WebhookEventObjectType,
    pub amount: i32,
    pub currency: String,
    pub payment_intent: Option<String>,
    pub reason: Option<String>,
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub created: PrimitiveDateTime,
    pub evidence_details: Option<EvidenceDetails>,
    pub status: Option<WebhookEventStatus>,
}

#[derive(Debug, Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventObjectType {
    PaymentIntent,
    Dispute,
    Charge,
}

#[derive(Debug, Deserialize)]
pub enum WebhookEventType {
    #[serde(rename = "payment_intent.payment_failed")]
    PaymentIntentFailed,
    #[serde(rename = "payment_intent.succeeded")]
    PaymentIntentSucceed,
    #[serde(rename = "charge.dispute.captured")]
    ChargeDisputeCaptured,
    #[serde(rename = "charge.dispute.created")]
    DisputeCreated,
    #[serde(rename = "charge.dispute.closed")]
    DisputeClosed,
    #[serde(rename = "charge.dispute.updated")]
    DisputeUpdated,
    #[serde(rename = "charge.dispute.funds_reinstated")]
    ChargeDisputeFundsReinstated,
    #[serde(rename = "charge.dispute.funds_withdrawn")]
    ChargeDisputeFundsWithdrawn,
    #[serde(rename = "charge.expired")]
    ChargeExpired,
    #[serde(rename = "charge.failed")]
    ChargeFailed,
    #[serde(rename = "charge.pending")]
    ChargePending,
    #[serde(rename = "charge.captured")]
    ChargeCaptured,
    #[serde(rename = "charge.succeeded")]
    ChargeSucceeded,
    #[serde(rename = "charge.updated")]
    ChargeUpdated,
    #[serde(rename = "charge.refunded")]
    ChanrgeRefunded,
    #[serde(rename = "payment_intent.canceled")]
    PaymentIntentCanceled,
    #[serde(rename = "payment_intent.created")]
    PaymentIntentCreated,
    #[serde(rename = "payment_intent.processing")]
    PaymentIntentProcessing,
    #[serde(rename = "payment_intent.requires_action")]
    PaymentIntentRequiresAction,
    #[serde(rename = "amount_capturable_updated")]
    PaymentIntentAmountCapturableUpdated,
}

#[derive(Debug, Serialize, strum::Display, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventStatus {
    WarningNeedsResponse,
    WarningClosed,
    WarningUnderReview,
    Won,
    Lost,
    NeedsResponse,
    UnderReview,
    ChargeRefunded,
    Succeeded,
    RequiresPaymentMethod,
    RequiresConfirmation,
    RequiresAction,
    Processing,
    RequiresCapture,
    Canceled,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct EvidenceDetails {
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub due_by: PrimitiveDateTime,
}

impl
    TryFrom<(
        api::PaymentMethodData,
        enums::AuthenticationType,
        StripePaymentMethodType,
    )> for StripePaymentMethodData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (pm_data, auth_type, pm_type): (
            api::PaymentMethodData,
            enums::AuthenticationType,
            StripePaymentMethodType,
        ),
    ) -> Result<Self, Self::Error> {
        match pm_data {
            api::PaymentMethodData::Card(ref ccard) => Ok(Self::Card({
                let payment_method_auth_type = match auth_type {
                    enums::AuthenticationType::ThreeDs => Auth3ds::Any,
                    enums::AuthenticationType::NoThreeDs => Auth3ds::Automatic,
                };
                StripeCardData {
                    payment_method_types: StripePaymentMethodType::Card,
                    payment_method_data_type: StripePaymentMethodType::Card,
                    payment_method_data_card_number: ccard.card_number.clone(),
                    payment_method_data_card_exp_month: ccard.card_exp_month.clone(),
                    payment_method_data_card_exp_year: ccard.card_exp_year.clone(),
                    payment_method_data_card_cvc: ccard.card_cvc.clone(),
                    payment_method_auth_type,
                }
            })),
            api::PaymentMethodData::PayLater(_) => Ok(Self::PayLater(StripePayLaterData {
                payment_method_types: pm_type,
                payment_method_data_type: pm_type,
            })),
            api::PaymentMethodData::BankRedirect(_) => {
                Ok(Self::BankRedirect(StripeBankRedirectData {
                    payment_method_types: pm_type,
                    payment_method_data_type: pm_type,
                    bank_name: None,
                    bank_specific_data: None,
                }))
            }
            api::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                payments::WalletData::ApplePay(data) => {
                    let wallet_info = StripeWallet::ApplepayToken(StripeApplePay {
                        pk_token: String::from_utf8(
                            consts::BASE64_ENGINE
                                .decode(data.payment_data)
                                .into_report()
                                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                        )
                        .into_report()
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                        pk_token_instrument_name: data.payment_method.pm_type,
                        pk_token_payment_network: data.payment_method.network,
                        pk_token_transaction_id: data.transaction_identifier,
                    });
                    Ok(Self::Wallet(wallet_info))
                }

                payments::WalletData::WeChatPayRedirect(_) => {
                    let wallet_info = StripeWallet::WechatpayPayment(WechatpayPayment {
                        client: WechatClient::Web,
                        payment_method_types: StripePaymentMethodType::Wechatpay,
                        payment_method_data_type: StripePaymentMethodType::Wechatpay,
                    });
                    Ok(Self::Wallet(wallet_info))
                }
                payments::WalletData::AliPay(_) => {
                    let wallet_info = StripeWallet::AlipayPayment(AlipayPayment {
                        payment_method_types: StripePaymentMethodType::Alipay,
                        payment_method_data_type: StripePaymentMethodType::Alipay,
                    });
                    Ok(Self::Wallet(wallet_info))
                }
                payments::WalletData::GooglePay(gpay_data) => Self::try_from(&gpay_data),
                _ => Err(errors::ConnectorError::InvalidWallet.into()),
            },
            api::PaymentMethodData::BankDebit(bank_debit_data) => {
                let (pm_type, bank_data, _) = get_bank_debit_data(&bank_debit_data);

                Ok(Self::BankDebit(StripeBankDebitEnum::Data(
                    StripeBankDebitData {
                        payment_method_types: pm_type,
                        bank_specific_data: bank_data,
                    },
                )))
            }
            api::PaymentMethodData::MandatePayment | api::PaymentMethodData::Crypto(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: format!("{pm_type:?}"),
                    connector: "Stripe",
                    payment_experience: api_models::enums::PaymentExperience::RedirectToUrl
                        .to_string(),
                })?
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StripeGpayToken {
    pub id: String,
}
pub fn construct_file_upload_request(
    file_upload_router_data: types::UploadFileRouterData,
) -> CustomResult<reqwest::multipart::Form, errors::ConnectorError> {
    let request = file_upload_router_data.request;
    let mut multipart = reqwest::multipart::Form::new();
    multipart = multipart.text("purpose", "dispute_evidence");
    let file_data = reqwest::multipart::Part::bytes(request.file)
        .file_name(request.file_key)
        .mime_str(request.file_type.as_ref())
        .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;
    multipart = multipart.part("file", file_data);
    Ok(multipart)
}

#[derive(Debug, Deserialize)]
pub struct FileUploadResponse {
    #[serde(rename = "id")]
    pub file_id: String,
}

#[derive(Debug, Serialize)]
pub struct Evidence {
    #[serde(rename = "evidence[access_activity_log]")]
    pub access_activity_log: Option<String>,
    #[serde(rename = "evidence[billing_address]")]
    pub billing_address: Option<String>,
    #[serde(rename = "evidence[cancellation_policy]")]
    pub cancellation_policy: Option<String>,
    #[serde(rename = "evidence[cancellation_policy_disclosure]")]
    pub cancellation_policy_disclosure: Option<String>,
    #[serde(rename = "evidence[cancellation_rebuttal]")]
    pub cancellation_rebuttal: Option<String>,
    #[serde(rename = "evidence[customer_communication]")]
    pub customer_communication: Option<String>,
    #[serde(rename = "evidence[customer_email_address]")]
    pub customer_email_address: Option<String>,
    #[serde(rename = "evidence[customer_name]")]
    pub customer_name: Option<String>,
    #[serde(rename = "evidence[customer_purchase_ip]")]
    pub customer_purchase_ip: Option<String>,
    #[serde(rename = "evidence[customer_signature]")]
    pub customer_signature: Option<String>,
    #[serde(rename = "evidence[product_description]")]
    pub product_description: Option<String>,
    #[serde(rename = "evidence[receipt]")]
    pub receipt: Option<String>,
    #[serde(rename = "evidence[refund_policy]")]
    pub refund_policy: Option<String>,
    #[serde(rename = "evidence[refund_policy_disclosure]")]
    pub refund_policy_disclosure: Option<String>,
    #[serde(rename = "evidence[refund_refusal_explanation]")]
    pub refund_refusal_explanation: Option<String>,
    #[serde(rename = "evidence[service_date]")]
    pub service_date: Option<String>,
    #[serde(rename = "evidence[service_documentation]")]
    pub service_documentation: Option<String>,
    #[serde(rename = "evidence[shipping_address]")]
    pub shipping_address: Option<String>,
    #[serde(rename = "evidence[shipping_carrier]")]
    pub shipping_carrier: Option<String>,
    #[serde(rename = "evidence[shipping_date]")]
    pub shipping_date: Option<String>,
    #[serde(rename = "evidence[shipping_documentation]")]
    pub shipping_documentation: Option<String>,
    #[serde(rename = "evidence[shipping_tracking_number]")]
    pub shipping_tracking_number: Option<String>,
    #[serde(rename = "evidence[uncategorized_file]")]
    pub uncategorized_file: Option<String>,
    #[serde(rename = "evidence[uncategorized_text]")]
    pub uncategorized_text: Option<String>,
    pub submit: bool,
}

impl TryFrom<&types::SubmitEvidenceRouterData> for Evidence {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SubmitEvidenceRouterData) -> Result<Self, Self::Error> {
        let submit_evidence_request_data = item.request.clone();
        Ok(Self {
            access_activity_log: submit_evidence_request_data.access_activity_log,
            billing_address: submit_evidence_request_data.billing_address,
            cancellation_policy: submit_evidence_request_data.cancellation_policy_provider_file_id,
            cancellation_policy_disclosure: submit_evidence_request_data
                .cancellation_policy_disclosure,
            cancellation_rebuttal: submit_evidence_request_data.cancellation_rebuttal,
            customer_communication: submit_evidence_request_data
                .customer_communication_provider_file_id,
            customer_email_address: submit_evidence_request_data.customer_email_address,
            customer_name: submit_evidence_request_data.customer_name,
            customer_purchase_ip: submit_evidence_request_data.customer_purchase_ip,
            customer_signature: submit_evidence_request_data.customer_signature_provider_file_id,
            product_description: submit_evidence_request_data.product_description,
            receipt: submit_evidence_request_data.receipt_provider_file_id,
            refund_policy: submit_evidence_request_data.refund_policy_provider_file_id,
            refund_policy_disclosure: submit_evidence_request_data.refund_policy_disclosure,
            refund_refusal_explanation: submit_evidence_request_data.refund_refusal_explanation,
            service_date: submit_evidence_request_data.service_date,
            service_documentation: submit_evidence_request_data
                .service_documentation_provider_file_id,
            shipping_address: submit_evidence_request_data.shipping_address,
            shipping_carrier: submit_evidence_request_data.shipping_carrier,
            shipping_date: submit_evidence_request_data.shipping_date,
            shipping_documentation: submit_evidence_request_data
                .shipping_documentation_provider_file_id,
            shipping_tracking_number: submit_evidence_request_data.shipping_tracking_number,
            uncategorized_file: submit_evidence_request_data.uncategorized_file_provider_file_id,
            uncategorized_text: submit_evidence_request_data.uncategorized_text,
            submit: true,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct DisputeObj {
    #[serde(rename = "id")]
    pub dispute_id: String,
    pub status: String,
}

#[cfg(test)]
mod test_validate_shipping_address_against_payment_method {
    #![allow(clippy::unwrap_used)]
    use api_models::enums::CountryAlpha2;
    use masking::Secret;

    use crate::{
        connector::stripe::transformers::{
            validate_shipping_address_against_payment_method, StripePaymentMethodType,
            StripeShippingAddress,
        },
        core::errors,
    };

    #[test]
    fn should_return_ok() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            Some("name".to_string()),
            Some("line1".to_string()),
            Some(CountryAlpha2::AD),
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            payment_method,
        );

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn should_return_err_for_empty_name() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            None,
            Some("line1".to_string()),
            Some(CountryAlpha2::AD),
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            payment_method,
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(missing_fields[0], "shipping.address.first_name");
    }

    #[test]
    fn should_return_err_for_empty_line1() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            Some("name".to_string()),
            None,
            Some(CountryAlpha2::AD),
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            payment_method,
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(missing_fields[0], "shipping.address.line1");
    }

    #[test]
    fn should_return_err_for_empty_country() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            Some("name".to_string()),
            Some("line1".to_string()),
            None,
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            payment_method,
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(missing_fields[0], "shipping.address.country");
    }

    #[test]
    fn should_return_err_for_empty_zip() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            Some("name".to_string()),
            Some("line1".to_string()),
            Some(CountryAlpha2::AD),
            None,
        );
        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            payment_method,
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(missing_fields[0], "shipping.address.zip");
    }

    #[test]
    fn should_return_error_when_missing_multiple_fields() {
        // Arrange
        let expected_missing_field_names: Vec<&'static str> =
            vec!["shipping.address.zip", "shipping.address.country"];
        let stripe_shipping_address = create_stripe_shipping_address(
            Some("name".to_string()),
            Some("line1".to_string()),
            None,
            None,
        );
        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            payment_method,
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        for field in missing_fields {
            assert!(expected_missing_field_names.contains(&field));
        }
    }

    fn get_missing_fields(connector_error: &errors::ConnectorError) -> Vec<&'static str> {
        if let errors::ConnectorError::MissingRequiredFields { field_names } = connector_error {
            return field_names.to_vec();
        }

        vec![]
    }

    fn create_stripe_shipping_address(
        name: Option<String>,
        line1: Option<String>,
        country: Option<CountryAlpha2>,
        zip: Option<String>,
    ) -> StripeShippingAddress {
        StripeShippingAddress {
            name: name.map(Secret::new),
            line1: line1.map(Secret::new),
            country,
            zip: zip.map(Secret::new),
            city: Some(String::from("city")),
            line2: Some(Secret::new(String::from("line2"))),
            state: Some(Secret::new(String::from("state"))),
            phone: Some(Secret::new(String::from("pbone number"))),
        }
    }
}
