use api_models::payments;
use common_utils::{
    crypto::{self, GenerateDigest},
    date_time,
    ext_traits::Encode,
    fp_utils,
    pii::{Email, IpAddress},
};
use data_models::mandates::MandateDataType;
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, BrowserInformationData, PaymentsAuthorizeRequestData,
        PaymentsCancelRequestData, RouterData,
    },
    consts,
    core::errors,
    services,
    types::{self, api, domain, storage::enums, transformers::ForeignTryFrom, BrowserInformation},
    utils::OptionExt,
};

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct NuveiMeta {
    pub session_token: Secret<String>,
}

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct NuveiMandateMeta {
    pub frequency: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiSessionRequest {
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: String,
    pub time_stamp: date_time::DateTime<date_time::YYYYMMDDHHmmss>,
    pub checksum: Secret<String>,
}

#[derive(Debug, Serialize, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiSessionResponse {
    pub session_token: Secret<String>,
    pub internal_request_id: i64,
    pub status: String,
    pub err_code: i64,
    pub reason: String,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub version: String,
    pub client_request_id: String,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentsRequest {
    pub time_stamp: String,
    pub session_token: Secret<String>,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: Secret<String>,
    pub amount: String,
    pub currency: diesel_models::enums::Currency,
    /// This ID uniquely identifies your consumer/user in your system.
    pub user_token_id: Option<Email>,
    pub client_unique_id: String,
    pub transaction_type: TransactionType,
    pub is_rebilling: Option<String>,
    pub payment_option: PaymentOption,
    pub device_details: Option<DeviceDetails>,
    pub checksum: Secret<String>,
    pub billing_address: Option<BillingAddress>,
    pub related_transaction_id: Option<String>,
    pub url_details: Option<UrlDetails>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UrlDetails {
    pub success_url: String,
    pub failure_url: String,
    pub pending_url: String,
}

#[derive(Debug, Serialize, Default)]
pub struct NuveiInitPaymentRequest {
    pub session_token: Secret<String>,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: String,
    pub amount: String,
    pub currency: String,
    pub payment_option: PaymentOption,
    pub checksum: Secret<String>,
}

/// Handles payment request for capture, void and refund flows
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentFlowRequest {
    pub time_stamp: String,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: String,
    pub amount: String,
    pub currency: diesel_models::enums::Currency,
    pub related_transaction_id: Option<String>,
    pub checksum: Secret<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentSyncRequest {
    pub session_token: Secret<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum TransactionType {
    Auth,
    #[default]
    Sale,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentOption {
    pub card: Option<Card>,
    pub redirect_url: Option<Url>,
    pub user_payment_option_id: Option<String>,
    pub alternative_payment_method: Option<AlternativePaymentMethod>,
    pub billing_address: Option<BillingAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NuveiBIC {
    #[serde(rename = "ABNANL2A")]
    Abnamro,
    #[serde(rename = "ASNBNL21")]
    ASNBank,
    #[serde(rename = "BUNQNL2A")]
    Bunq,
    #[serde(rename = "INGBNL2A")]
    Ing,
    #[serde(rename = "KNABNL2H")]
    Knab,
    #[serde(rename = "RABONL2U")]
    Rabobank,
    #[serde(rename = "RBRBNL21")]
    RegioBank,
    #[serde(rename = "SNSBNL2A")]
    SNSBank,
    #[serde(rename = "TRIONL2U")]
    TriodosBank,
    #[serde(rename = "FVLBNL22")]
    VanLanschotBankiers,
    #[serde(rename = "MOYONL21")]
    Moneyou,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternativePaymentMethod {
    pub payment_method: AlternativePaymentMethodType,
    #[serde(rename = "BIC")]
    pub bank_id: Option<NuveiBIC>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlternativePaymentMethodType {
    #[default]
    #[serde(rename = "apmgw_expresscheckout")]
    Expresscheckout,
    #[serde(rename = "apmgw_Giropay")]
    Giropay,
    #[serde(rename = "apmgw_Sofort")]
    Sofort,
    #[serde(rename = "apmgw_iDeal")]
    Ideal,
    #[serde(rename = "apmgw_EPS")]
    Eps,
    #[serde(rename = "apmgw_Afterpay")]
    AfterPay,
    #[serde(rename = "apmgw_Klarna")]
    Klarna,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    pub email: Email,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub country: api_models::enums::CountryAlpha2,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub card_number: Option<cards::CardNumber>,
    pub card_holder_name: Option<Secret<String>>,
    pub expiration_month: Option<Secret<String>>,
    pub expiration_year: Option<Secret<String>>,
    #[serde(rename = "CVV")]
    pub cvv: Option<Secret<String>>,
    pub three_d: Option<ThreeD>,
    pub cc_card_number: Option<Secret<String>>,
    pub bin: Option<Secret<String>>,
    pub last4_digits: Option<Secret<String>>,
    pub cc_exp_month: Option<Secret<String>>,
    pub cc_exp_year: Option<Secret<String>>,
    pub acquirer_id: Option<Secret<String>>,
    pub cvv2_reply: Option<String>,
    pub avs_code: Option<String>,
    pub card_type: Option<String>,
    pub card_brand: Option<String>,
    pub issuer_bank_name: Option<String>,
    pub issuer_country: Option<String>,
    pub is_prepaid: Option<String>,
    pub external_token: Option<ExternalToken>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalToken {
    pub external_token_provider: ExternalTokenProvider,
    pub mobile_token: Secret<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ExternalTokenProvider {
    #[default]
    GooglePay,
    ApplePay,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeD {
    pub method_completion_ind: Option<MethodCompletion>,
    pub browser_details: Option<BrowserDetails>,
    pub version: Option<String>,
    #[serde(rename = "notificationURL")]
    pub notification_url: Option<String>,
    #[serde(rename = "merchantURL")]
    pub merchant_url: Option<String>,
    pub acs_url: Option<String>,
    pub c_req: Option<Secret<String>>,
    pub platform_type: Option<PlatformType>,
    pub v2supported: Option<String>,
    pub v2_additional_params: Option<V2AdditionalParams>,
    pub is_liability_on_issuer: Option<LiabilityShift>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum MethodCompletion {
    #[serde(rename = "Y")]
    Success,
    #[serde(rename = "N")]
    Failure,
    #[serde(rename = "U")]
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum PlatformType {
    #[serde(rename = "01")]
    App,
    #[serde(rename = "02")]
    #[default]
    Browser,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDetails {
    pub accept_header: String,
    pub ip: Secret<String, IpAddress>,
    pub java_enabled: String,
    pub java_script_enabled: String,
    pub language: String,
    pub color_depth: u8,
    pub screen_height: u32,
    pub screen_width: u32,
    pub time_zone: i32,
    pub user_agent: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AdditionalParams {
    pub challenge_window_size: Option<String>,
    /// Recurring Expiry in format YYYYMMDD. REQUIRED if isRebilling = 0, We recommend setting rebillExpiry to a value of no more than 5 years from the date of the initial transaction processing date.
    pub rebill_expiry: Option<String>,
    /// Recurring Frequency in days
    pub rebill_frequency: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDetails {
    pub ip_address: Secret<String, IpAddress>,
}

impl From<enums::CaptureMethod> for TransactionType {
    fn from(value: enums::CaptureMethod) -> Self {
        match value {
            enums::CaptureMethod::Manual => Self::Auth,
            _ => Self::Sale,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NuveiRedirectionResponse {
    pub cres: Secret<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiACSResponse {
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: Secret<String>,
    #[serde(rename = "acsTransID")]
    pub acs_trans_id: Secret<String>,
    pub message_type: String,
    pub message_version: String,
    pub trans_status: Option<LiabilityShift>,
    pub message_extension: Vec<MessageExtension>,
    pub acs_signed_content: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageExtension {
    pub name: String,
    pub id: String,
    pub criticality_indicator: bool,
    pub data: MessageExtensionData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageExtensionData {
    pub value_one: String,
    pub value_two: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiabilityShift {
    #[serde(rename = "Y", alias = "1")]
    Success,
    #[serde(rename = "N", alias = "0")]
    Failed,
}

fn encode_payload(payload: &[&str]) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let data = payload.join("");
    let digest = crypto::Sha256
        .generate_digest(data.as_bytes())
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("error encoding the payload")?;
    Ok(hex::encode(digest))
}

impl TryFrom<&types::PaymentsAuthorizeSessionTokenRouterData> for NuveiSessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &types::PaymentsAuthorizeSessionTokenRouterData,
    ) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.connector_request_reference_id.clone();
        let time_stamp = date_time::DateTime::<date_time::YYYYMMDDHHmmss>::from(date_time::now());
        let merchant_secret = connector_meta.merchant_secret;
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: client_request_id.clone(),
            time_stamp: time_stamp.clone(),
            checksum: Secret::new(encode_payload(&[
                merchant_id.peek(),
                merchant_site_id.peek(),
                &client_request_id,
                &time_stamp.to_string(),
                merchant_secret.peek(),
            ])?),
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NuveiSessionResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, NuveiSessionResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Pending,
            session_token: Some(item.response.session_token.clone().expose()),
            response: Ok(types::PaymentsResponseData::SessionTokenResponse {
                session_token: item.response.session_token.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug)]
pub struct NuveiCardDetails {
    card: domain::Card,
    three_d: Option<ThreeD>,
}

impl TryFrom<payments::GooglePayWalletData> for NuveiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(gpay_data: payments::GooglePayWalletData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_option: PaymentOption {
                card: Some(Card {
                    external_token: Some(ExternalToken {
                        external_token_provider: ExternalTokenProvider::GooglePay,
                        mobile_token: Secret::new(
                            utils::GooglePayWalletData::from(gpay_data)
                                .encode_to_string_of_json()
                                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                        ),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }
}
impl From<payments::ApplePayWalletData> for NuveiPaymentsRequest {
    fn from(apple_pay_data: payments::ApplePayWalletData) -> Self {
        Self {
            payment_option: PaymentOption {
                card: Some(Card {
                    external_token: Some(ExternalToken {
                        external_token_provider: ExternalTokenProvider::ApplePay,
                        mobile_token: Secret::new(apple_pay_data.payment_data),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl TryFrom<common_enums::enums::BankNames> for NuveiBIC {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(bank: common_enums::enums::BankNames) -> Result<Self, Self::Error> {
        match bank {
            common_enums::enums::BankNames::AbnAmro => Ok(Self::Abnamro),
            common_enums::enums::BankNames::AsnBank => Ok(Self::ASNBank),
            common_enums::enums::BankNames::Bunq => Ok(Self::Bunq),
            common_enums::enums::BankNames::Ing => Ok(Self::Ing),
            common_enums::enums::BankNames::Knab => Ok(Self::Knab),
            common_enums::enums::BankNames::Rabobank => Ok(Self::Rabobank),
            common_enums::enums::BankNames::SnsBank => Ok(Self::SNSBank),
            common_enums::enums::BankNames::TriodosBank => Ok(Self::TriodosBank),
            common_enums::enums::BankNames::VanLanschot => Ok(Self::VanLanschotBankiers),
            common_enums::enums::BankNames::Moneyou => Ok(Self::Moneyou),

            common_enums::enums::BankNames::AmericanExpress
            | common_enums::enums::BankNames::AffinBank
            | common_enums::enums::BankNames::AgroBank
            | common_enums::enums::BankNames::AllianceBank
            | common_enums::enums::BankNames::AmBank
            | common_enums::enums::BankNames::BankOfAmerica
            | common_enums::enums::BankNames::BankIslam
            | common_enums::enums::BankNames::BankMuamalat
            | common_enums::enums::BankNames::BankRakyat
            | common_enums::enums::BankNames::BankSimpananNasional
            | common_enums::enums::BankNames::Barclays
            | common_enums::enums::BankNames::BlikPSP
            | common_enums::enums::BankNames::CapitalOne
            | common_enums::enums::BankNames::Chase
            | common_enums::enums::BankNames::Citi
            | common_enums::enums::BankNames::CimbBank
            | common_enums::enums::BankNames::Discover
            | common_enums::enums::BankNames::NavyFederalCreditUnion
            | common_enums::enums::BankNames::PentagonFederalCreditUnion
            | common_enums::enums::BankNames::SynchronyBank
            | common_enums::enums::BankNames::WellsFargo
            | common_enums::enums::BankNames::Handelsbanken
            | common_enums::enums::BankNames::HongLeongBank
            | common_enums::enums::BankNames::HsbcBank
            | common_enums::enums::BankNames::KuwaitFinanceHouse
            | common_enums::enums::BankNames::Regiobank
            | common_enums::enums::BankNames::Revolut
            | common_enums::enums::BankNames::ArzteUndApothekerBank
            | common_enums::enums::BankNames::AustrianAnadiBankAg
            | common_enums::enums::BankNames::BankAustria
            | common_enums::enums::BankNames::Bank99Ag
            | common_enums::enums::BankNames::BankhausCarlSpangler
            | common_enums::enums::BankNames::BankhausSchelhammerUndSchatteraAg
            | common_enums::enums::BankNames::BankMillennium
            | common_enums::enums::BankNames::BankPEKAOSA
            | common_enums::enums::BankNames::BawagPskAg
            | common_enums::enums::BankNames::BksBankAg
            | common_enums::enums::BankNames::BrullKallmusBankAg
            | common_enums::enums::BankNames::BtvVierLanderBank
            | common_enums::enums::BankNames::CapitalBankGraweGruppeAg
            | common_enums::enums::BankNames::CeskaSporitelna
            | common_enums::enums::BankNames::Dolomitenbank
            | common_enums::enums::BankNames::EasybankAg
            | common_enums::enums::BankNames::EPlatbyVUB
            | common_enums::enums::BankNames::ErsteBankUndSparkassen
            | common_enums::enums::BankNames::FrieslandBank
            | common_enums::enums::BankNames::HypoAlpeadriabankInternationalAg
            | common_enums::enums::BankNames::HypoNoeLbFurNiederosterreichUWien
            | common_enums::enums::BankNames::HypoOberosterreichSalzburgSteiermark
            | common_enums::enums::BankNames::HypoTirolBankAg
            | common_enums::enums::BankNames::HypoVorarlbergBankAg
            | common_enums::enums::BankNames::HypoBankBurgenlandAktiengesellschaft
            | common_enums::enums::BankNames::KomercniBanka
            | common_enums::enums::BankNames::MBank
            | common_enums::enums::BankNames::MarchfelderBank
            | common_enums::enums::BankNames::Maybank
            | common_enums::enums::BankNames::OberbankAg
            | common_enums::enums::BankNames::OsterreichischeArzteUndApothekerbank
            | common_enums::enums::BankNames::OcbcBank
            | common_enums::enums::BankNames::PayWithING
            | common_enums::enums::BankNames::PlaceZIPKO
            | common_enums::enums::BankNames::PlatnoscOnlineKartaPlatnicza
            | common_enums::enums::BankNames::PosojilnicaBankEGen
            | common_enums::enums::BankNames::PostovaBanka
            | common_enums::enums::BankNames::PublicBank
            | common_enums::enums::BankNames::RaiffeisenBankengruppeOsterreich
            | common_enums::enums::BankNames::RhbBank
            | common_enums::enums::BankNames::SchelhammerCapitalBankAg
            | common_enums::enums::BankNames::StandardCharteredBank
            | common_enums::enums::BankNames::SchoellerbankAg
            | common_enums::enums::BankNames::SpardaBankWien
            | common_enums::enums::BankNames::SporoPay
            | common_enums::enums::BankNames::SantanderPrzelew24
            | common_enums::enums::BankNames::TatraPay
            | common_enums::enums::BankNames::Viamo
            | common_enums::enums::BankNames::VolksbankGruppe
            | common_enums::enums::BankNames::VolkskreditbankAg
            | common_enums::enums::BankNames::VrBankBraunau
            | common_enums::enums::BankNames::UobBank
            | common_enums::enums::BankNames::PayWithAliorBank
            | common_enums::enums::BankNames::BankiSpoldzielcze
            | common_enums::enums::BankNames::PayWithInteligo
            | common_enums::enums::BankNames::BNPParibasPoland
            | common_enums::enums::BankNames::BankNowySA
            | common_enums::enums::BankNames::CreditAgricole
            | common_enums::enums::BankNames::PayWithBOS
            | common_enums::enums::BankNames::PayWithCitiHandlowy
            | common_enums::enums::BankNames::PayWithPlusBank
            | common_enums::enums::BankNames::ToyotaBank
            | common_enums::enums::BankNames::VeloBank
            | common_enums::enums::BankNames::ETransferPocztowy24
            | common_enums::enums::BankNames::PlusBank
            | common_enums::enums::BankNames::EtransferPocztowy24
            | common_enums::enums::BankNames::BankiSpbdzielcze
            | common_enums::enums::BankNames::BankNowyBfgSa
            | common_enums::enums::BankNames::GetinBank
            | common_enums::enums::BankNames::Blik
            | common_enums::enums::BankNames::NoblePay
            | common_enums::enums::BankNames::IdeaBank
            | common_enums::enums::BankNames::EnveloBank
            | common_enums::enums::BankNames::NestPrzelew
            | common_enums::enums::BankNames::MbankMtransfer
            | common_enums::enums::BankNames::Inteligo
            | common_enums::enums::BankNames::PbacZIpko
            | common_enums::enums::BankNames::BnpParibas
            | common_enums::enums::BankNames::BankPekaoSa
            | common_enums::enums::BankNames::VolkswagenBank
            | common_enums::enums::BankNames::AliorBank
            | common_enums::enums::BankNames::Boz
            | common_enums::enums::BankNames::BangkokBank
            | common_enums::enums::BankNames::KrungsriBank
            | common_enums::enums::BankNames::KrungThaiBank
            | common_enums::enums::BankNames::TheSiamCommercialBank
            | common_enums::enums::BankNames::KasikornBank
            | common_enums::enums::BankNames::OpenBankSuccess
            | common_enums::enums::BankNames::OpenBankFailure
            | common_enums::enums::BankNames::OpenBankCancelled
            | common_enums::enums::BankNames::Aib
            | common_enums::enums::BankNames::BankOfScotland
            | common_enums::enums::BankNames::DanskeBank
            | common_enums::enums::BankNames::FirstDirect
            | common_enums::enums::BankNames::FirstTrust
            | common_enums::enums::BankNames::Halifax
            | common_enums::enums::BankNames::Lloyds
            | common_enums::enums::BankNames::Monzo
            | common_enums::enums::BankNames::NatWest
            | common_enums::enums::BankNames::NationwideBank
            | common_enums::enums::BankNames::RoyalBankOfScotland
            | common_enums::enums::BankNames::Starling
            | common_enums::enums::BankNames::TsbBank
            | common_enums::enums::BankNames::TescoBank
            | common_enums::enums::BankNames::Yoursafe
            | common_enums::enums::BankNames::N26
            | common_enums::enums::BankNames::NationaleNederlanden
            | common_enums::enums::BankNames::UlsterBank => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Nuvei"),
                ))?
            }
        }
    }
}

impl<F>
    ForeignTryFrom<(
        AlternativePaymentMethodType,
        Option<domain::BankRedirectData>,
        &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    )> for NuveiPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        data: (
            AlternativePaymentMethodType,
            Option<domain::BankRedirectData>,
            &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        ),
    ) -> Result<Self, Self::Error> {
        let (payment_method, redirect, item) = data;
        let (billing_address, bank_id) = match (&payment_method, redirect) {
            (AlternativePaymentMethodType::Expresscheckout, _) => (
                Some(BillingAddress {
                    email: item.request.get_email()?,
                    country: item.get_billing_country()?,
                    ..Default::default()
                }),
                None,
            ),
            (AlternativePaymentMethodType::Giropay, _) => (
                Some(BillingAddress {
                    email: item.request.get_email()?,
                    country: item.get_billing_country()?,
                    ..Default::default()
                }),
                None,
            ),
            (AlternativePaymentMethodType::Sofort, _) | (AlternativePaymentMethodType::Eps, _) => {
                let address = item.get_billing_address()?;
                (
                    Some(BillingAddress {
                        first_name: Some(address.get_first_name()?.clone()),
                        last_name: Some(address.get_last_name()?.clone()),
                        email: item.request.get_email()?,
                        country: item.get_billing_country()?,
                    }),
                    None,
                )
            }
            (
                AlternativePaymentMethodType::Ideal,
                Some(domain::BankRedirectData::Ideal { bank_name, .. }),
            ) => {
                let address = item.get_billing_address()?;
                (
                    Some(BillingAddress {
                        first_name: Some(address.get_first_name()?.clone()),
                        last_name: Some(address.get_last_name()?.clone()),
                        email: item.request.get_email()?,
                        country: item.get_billing_country()?,
                    }),
                    bank_name.map(NuveiBIC::try_from).transpose()?,
                )
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Nuvei"),
            ))?,
        };
        Ok(Self {
            payment_option: PaymentOption {
                alternative_payment_method: Some(AlternativePaymentMethod {
                    payment_method,
                    bank_id,
                }),
                ..Default::default()
            },
            billing_address,
            ..Default::default()
        })
    }
}

fn get_pay_later_info<F>(
    payment_method_type: AlternativePaymentMethodType,
    item: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let address = item
        .get_billing()?
        .address
        .as_ref()
        .ok_or_else(utils::missing_field_err("billing.address"))?;
    let payment_method = payment_method_type;
    Ok(NuveiPaymentsRequest {
        payment_option: PaymentOption {
            alternative_payment_method: Some(AlternativePaymentMethod {
                payment_method,
                ..Default::default()
            }),
            billing_address: Some(BillingAddress {
                email: item.request.get_email()?,
                first_name: Some(address.get_first_name()?.to_owned()),
                last_name: Some(address.get_last_name()?.to_owned()),
                country: address.get_country()?.to_owned(),
            }),
            ..Default::default()
        },
        ..Default::default()
    })
}

impl<F>
    TryFrom<(
        &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        String,
    )> for NuveiPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
            String,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let request_data = match item.request.payment_method_data.clone() {
            domain::PaymentMethodData::Card(card) => get_card_info(item, &card),
            domain::PaymentMethodData::MandatePayment => Self::try_from(item),
            domain::PaymentMethodData::Wallet(wallet) => match wallet {
                payments::WalletData::GooglePay(gpay_data) => Self::try_from(gpay_data),
                payments::WalletData::ApplePay(apple_pay_data) => Ok(Self::from(apple_pay_data)),
                payments::WalletData::PaypalRedirect(_) => Self::foreign_try_from((
                    AlternativePaymentMethodType::Expresscheckout,
                    None,
                    item,
                )),
                payments::WalletData::AliPayQr(_)
                | payments::WalletData::AliPayRedirect(_)
                | payments::WalletData::AliPayHkRedirect(_)
                | payments::WalletData::MomoRedirect(_)
                | payments::WalletData::KakaoPayRedirect(_)
                | payments::WalletData::GoPayRedirect(_)
                | payments::WalletData::GcashRedirect(_)
                | payments::WalletData::ApplePayRedirect(_)
                | payments::WalletData::ApplePayThirdPartySdk(_)
                | payments::WalletData::DanaRedirect {}
                | payments::WalletData::GooglePayRedirect(_)
                | payments::WalletData::GooglePayThirdPartySdk(_)
                | payments::WalletData::MbWayRedirect(_)
                | payments::WalletData::MobilePayRedirect(_)
                | payments::WalletData::PaypalSdk(_)
                | payments::WalletData::SamsungPay(_)
                | payments::WalletData::TwintRedirect {}
                | payments::WalletData::VippsRedirect {}
                | payments::WalletData::TouchNGoRedirect(_)
                | payments::WalletData::WeChatPayRedirect(_)
                | payments::WalletData::CashappQr(_)
                | payments::WalletData::SwishQr(_)
                | payments::WalletData::WeChatPayQr(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("nuvei"),
                    )
                    .into())
                }
            },
            domain::PaymentMethodData::BankRedirect(redirect) => match redirect {
                domain::BankRedirectData::Eps { .. } => Self::foreign_try_from((
                    AlternativePaymentMethodType::Eps,
                    Some(redirect),
                    item,
                )),
                domain::BankRedirectData::Giropay { .. } => Self::foreign_try_from((
                    AlternativePaymentMethodType::Giropay,
                    Some(redirect),
                    item,
                )),
                domain::BankRedirectData::Ideal { .. } => Self::foreign_try_from((
                    AlternativePaymentMethodType::Ideal,
                    Some(redirect),
                    item,
                )),
                domain::BankRedirectData::Sofort { .. } => Self::foreign_try_from((
                    AlternativePaymentMethodType::Sofort,
                    Some(redirect),
                    item,
                )),
                domain::BankRedirectData::BancontactCard { .. }
                | domain::BankRedirectData::Bizum {}
                | domain::BankRedirectData::Blik { .. }
                | domain::BankRedirectData::Interac { .. }
                | domain::BankRedirectData::OnlineBankingCzechRepublic { .. }
                | domain::BankRedirectData::OnlineBankingFinland { .. }
                | domain::BankRedirectData::OnlineBankingPoland { .. }
                | domain::BankRedirectData::OnlineBankingSlovakia { .. }
                | domain::BankRedirectData::Przelewy24 { .. }
                | domain::BankRedirectData::Trustly { .. }
                | domain::BankRedirectData::OnlineBankingFpx { .. }
                | domain::BankRedirectData::OnlineBankingThailand { .. }
                | domain::BankRedirectData::OpenBankingUk { .. } => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("nuvei"),
                    )
                    .into())
                }
            },
            domain::PaymentMethodData::PayLater(pay_later_data) => match pay_later_data {
                payments::PayLaterData::KlarnaRedirect { .. } => {
                    get_pay_later_info(AlternativePaymentMethodType::Klarna, item)
                }
                payments::PayLaterData::AfterpayClearpayRedirect { .. } => {
                    get_pay_later_info(AlternativePaymentMethodType::AfterPay, item)
                }
                payments::PayLaterData::KlarnaSdk { .. }
                | payments::PayLaterData::AffirmRedirect {}
                | payments::PayLaterData::PayBrightRedirect {}
                | payments::PayLaterData::WalleyRedirect {}
                | payments::PayLaterData::AlmaRedirect {}
                | payments::PayLaterData::AtomeRedirect {} => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("nuvei"),
                    )
                    .into())
                }
            },
            domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("nuvei"),
                )
                .into())
            }
        }?;
        let request = Self::try_from(NuveiPaymentRequestData {
            amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            currency: item.request.currency,
            connector_auth_type: item.connector_auth_type.clone(),
            client_request_id: item.connector_request_reference_id.clone(),
            session_token: Secret::new(data.1),
            capture_method: item.request.capture_method,
            ..Default::default()
        })?;
        let return_url = item.request.get_return_url()?;
        Ok(Self {
            is_rebilling: request_data.is_rebilling,
            user_token_id: request_data.user_token_id,
            related_transaction_id: request_data.related_transaction_id,
            payment_option: request_data.payment_option,
            billing_address: request_data.billing_address,
            device_details: request_data.device_details,
            url_details: Some(UrlDetails {
                success_url: return_url.clone(),
                failure_url: return_url.clone(),
                pending_url: return_url,
            }),
            ..request
        })
    }
}

fn get_card_info<F>(
    item: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    card_details: &domain::Card,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let browser_information = item.request.browser_info.clone();
    let related_transaction_id = if item.is_three_ds() {
        item.request.related_transaction_id.clone()
    } else {
        None
    };

    let address = item
        .get_optional_billing()
        .and_then(|billing_details| billing_details.address.as_ref());

    let billing_address = match address {
        Some(address) => Some(BillingAddress {
            first_name: Some(address.get_first_name()?.clone()),
            last_name: Some(address.get_last_name()?.clone()),
            email: item.request.get_email()?,
            country: item.get_billing_country()?,
        }),
        None => None,
    };
    let (is_rebilling, additional_params, user_token_id) =
        match item.request.setup_mandate_details.clone() {
            Some(mandate_data) => {
                let details = match mandate_data
                    .mandate_type
                    .get_required_value("mandate_type")
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "mandate_type",
                    })? {
                    MandateDataType::SingleUse(details) => details,
                    MandateDataType::MultiUse(details) => {
                        details.ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "mandate_data.mandate_type.multi_use",
                        })?
                    }
                };
                let mandate_meta: NuveiMandateMeta = utils::to_connector_meta_from_secret(Some(
                    details.get_metadata().ok_or_else(utils::missing_field_err(
                        "mandate_data.mandate_type.{multi_use|single_use}.metadata",
                    ))?,
                ))?;
                (
                    Some("0".to_string()), // In case of first installment, rebilling should be 0
                    Some(V2AdditionalParams {
                        rebill_expiry: Some(
                            details
                                .get_end_date(date_time::DateFormat::YYYYMMDD)
                                .change_context(errors::ConnectorError::DateFormattingFailed)?
                                .ok_or_else(utils::missing_field_err(
                                    "mandate_data.mandate_type.{multi_use|single_use}.end_date",
                                ))?,
                        ),
                        rebill_frequency: Some(mandate_meta.frequency),
                        challenge_window_size: None,
                    }),
                    Some(item.request.get_email()?),
                )
            }
            _ => (None, None, None),
        };
    let three_d = if item.is_three_ds() {
        let browser_details = match &browser_information {
            Some(browser_info) => Some(BrowserDetails {
                accept_header: browser_info.get_accept_header()?,
                ip: browser_info.get_ip_address()?,
                java_enabled: browser_info.get_java_enabled()?.to_string().to_uppercase(),
                java_script_enabled: browser_info
                    .get_java_script_enabled()?
                    .to_string()
                    .to_uppercase(),
                language: browser_info.get_language()?,
                screen_height: browser_info.get_screen_height()?,
                screen_width: browser_info.get_screen_width()?,
                color_depth: browser_info.get_color_depth()?,
                user_agent: browser_info.get_user_agent()?,
                time_zone: browser_info.get_time_zone()?,
            }),
            None => None,
        };
        Some(ThreeD {
            browser_details,
            v2_additional_params: additional_params,
            notification_url: item.request.complete_authorize_url.clone(),
            merchant_url: item.return_url.clone(),
            platform_type: Some(PlatformType::Browser),
            method_completion_ind: Some(MethodCompletion::Unavailable),
            ..Default::default()
        })
    } else {
        None
    };

    Ok(NuveiPaymentsRequest {
        related_transaction_id,
        is_rebilling,
        user_token_id,
        device_details: Option::<DeviceDetails>::foreign_try_from(
            &item.request.browser_info.clone(),
        )?,
        payment_option: PaymentOption::from(NuveiCardDetails {
            card: card_details.clone(),
            three_d,
        }),
        billing_address,
        ..Default::default()
    })
}
impl From<NuveiCardDetails> for PaymentOption {
    fn from(card_details: NuveiCardDetails) -> Self {
        let card = card_details.card;
        Self {
            card: Some(Card {
                card_number: Some(card.card_number),
                card_holder_name: card.card_holder_name,
                expiration_month: Some(card.card_exp_month),
                expiration_year: Some(card.card_exp_year),
                three_d: card_details.three_d,
                cvv: Some(card.card_cvc),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl TryFrom<(&types::PaymentsCompleteAuthorizeRouterData, Secret<String>)>
    for NuveiPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (&types::PaymentsCompleteAuthorizeRouterData, Secret<String>),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let request_data = match item.request.payment_method_data.clone() {
            Some(domain::PaymentMethodData::Card(card)) => Ok(Self {
                payment_option: PaymentOption::from(NuveiCardDetails {
                    card,
                    three_d: None,
                }),
                ..Default::default()
            }),
            Some(domain::PaymentMethodData::Wallet(..))
            | Some(domain::PaymentMethodData::PayLater(..))
            | Some(domain::PaymentMethodData::BankDebit(..))
            | Some(domain::PaymentMethodData::BankRedirect(..))
            | Some(domain::PaymentMethodData::BankTransfer(..))
            | Some(domain::PaymentMethodData::Crypto(..))
            | Some(domain::PaymentMethodData::MandatePayment)
            | Some(domain::PaymentMethodData::GiftCard(..))
            | Some(domain::PaymentMethodData::Voucher(..))
            | Some(domain::PaymentMethodData::CardRedirect(..))
            | Some(domain::PaymentMethodData::Reward)
            | Some(domain::PaymentMethodData::Upi(..))
            | Some(domain::PaymentMethodData::CardToken(..))
            | None => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("nuvei"),
            )),
        }?;
        let request = Self::try_from(NuveiPaymentRequestData {
            amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            currency: item.request.currency,
            connector_auth_type: item.connector_auth_type.clone(),
            client_request_id: item.connector_request_reference_id.clone(),
            session_token: data.1,
            capture_method: item.request.capture_method,
            ..Default::default()
        })?;
        Ok(Self {
            related_transaction_id: request_data.related_transaction_id,
            payment_option: request_data.payment_option,
            ..request
        })
    }
}

impl TryFrom<NuveiPaymentRequestData> for NuveiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(request: NuveiPaymentRequestData) -> Result<Self, Self::Error> {
        let session_token = request.session_token;
        fp_utils::when(session_token.clone().expose().is_empty(), || {
            Err(errors::ConnectorError::FailedToObtainAuthType)
        })?;
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&request.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = request.client_request_id;
        let time_stamp =
            date_time::format_date(date_time::now(), date_time::DateFormat::YYYYMMDDHHmmss)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: Secret::new(client_request_id.clone()),
            time_stamp: time_stamp.clone(),
            session_token,
            transaction_type: request
                .capture_method
                .map(TransactionType::from)
                .unwrap_or_default(),
            checksum: Secret::new(encode_payload(&[
                merchant_id.peek(),
                merchant_site_id.peek(),
                &client_request_id,
                &request.amount.clone(),
                &request.currency.to_string(),
                &time_stamp,
                merchant_secret.peek(),
            ])?),
            amount: request.amount,
            currency: request.currency,
            ..Default::default()
        })
    }
}

impl TryFrom<NuveiPaymentRequestData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(request: NuveiPaymentRequestData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&request.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = request.client_request_id;
        let time_stamp =
            date_time::format_date(date_time::now(), date_time::DateFormat::YYYYMMDDHHmmss)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        Ok(Self {
            merchant_id: merchant_id.to_owned(),
            merchant_site_id: merchant_site_id.to_owned(),
            client_request_id: client_request_id.clone(),
            time_stamp: time_stamp.clone(),
            checksum: Secret::new(encode_payload(&[
                merchant_id.peek(),
                merchant_site_id.peek(),
                &client_request_id,
                &request.amount.clone(),
                &request.currency.to_string(),
                &request.related_transaction_id.clone().unwrap_or_default(),
                &time_stamp,
                merchant_secret.peek(),
            ])?),
            amount: request.amount,
            currency: request.currency,
            related_transaction_id: request.related_transaction_id,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct NuveiPaymentRequestData {
    pub amount: String,
    pub currency: diesel_models::enums::Currency,
    pub related_transaction_id: Option<String>,
    pub client_request_id: String,
    pub connector_auth_type: types::ConnectorAuthType,
    pub session_token: Secret<String>,
    pub capture_method: Option<diesel_models::enums::CaptureMethod>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Self::try_from(NuveiPaymentRequestData {
            client_request_id: item.connector_request_reference_id.clone(),
            connector_auth_type: item.connector_auth_type.clone(),
            amount: utils::to_currency_base_unit(
                item.request.amount_to_capture,
                item.request.currency,
            )?,
            currency: item.request.currency,
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            ..Default::default()
        })
    }
}
impl TryFrom<&types::RefundExecuteRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundExecuteRouterData) -> Result<Self, Self::Error> {
        Self::try_from(NuveiPaymentRequestData {
            client_request_id: item.connector_request_reference_id.clone(),
            connector_auth_type: item.connector_auth_type.clone(),
            amount: utils::to_currency_base_unit(
                item.request.refund_amount,
                item.request.currency,
            )?,
            currency: item.request.currency,
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            ..Default::default()
        })
    }
}

impl TryFrom<&types::PaymentsSyncRouterData> for NuveiPaymentSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let meta: NuveiMeta = utils::to_connector_meta(value.request.connector_meta.clone())?;
        Ok(Self {
            session_token: meta.session_token,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Self::try_from(NuveiPaymentRequestData {
            client_request_id: item.connector_request_reference_id.clone(),
            connector_auth_type: item.connector_auth_type.clone(),
            amount: utils::to_currency_base_unit(
                item.request.get_amount()?,
                item.request.get_currency()?,
            )?,
            currency: item.request.get_currency()?,
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            ..Default::default()
        })
    }
}

// Auth Struct
pub struct NuveiAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) merchant_site_id: Secret<String>,
    pub(super) merchant_secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for NuveiAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                merchant_id: api_key.to_owned(),
                merchant_site_id: key1.to_owned(),
                merchant_secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NuveiPaymentStatus {
    Success,
    Failed,
    Error,
    #[default]
    Processing,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NuveiTransactionStatus {
    Approved,
    Declined,
    Error,
    Redirect,
    Pending,
    #[default]
    Processing,
}

impl From<NuveiTransactionStatus> for enums::AttemptStatus {
    fn from(item: NuveiTransactionStatus) -> Self {
        match item {
            NuveiTransactionStatus::Approved => Self::Charged,
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => Self::Failure,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentsResponse {
    pub order_id: Option<String>,
    pub user_token_id: Option<Secret<String>>,
    pub payment_option: Option<PaymentOption>,
    pub transaction_status: Option<NuveiTransactionStatus>,
    pub gw_error_code: Option<i64>,
    pub gw_error_reason: Option<String>,
    pub gw_extended_error_code: Option<i64>,
    pub issuer_decline_code: Option<String>,
    pub issuer_decline_reason: Option<String>,
    pub transaction_type: Option<NuveiTransactionType>,
    pub transaction_id: Option<String>,
    pub external_transaction_id: Option<String>,
    pub auth_code: Option<String>,
    pub custom_data: Option<String>,
    pub fraud_details: Option<FraudDetails>,
    pub external_scheme_transaction_id: Option<Secret<String>>,
    pub session_token: Option<Secret<String>>,
    //The ID of the transaction in the merchant’s system.
    pub client_unique_id: Option<String>,
    pub internal_request_id: Option<i64>,
    pub status: NuveiPaymentStatus,
    pub err_code: Option<i64>,
    pub reason: Option<String>,
    pub merchant_id: Option<Secret<String>>,
    pub merchant_site_id: Option<Secret<String>>,
    pub version: Option<String>,
    pub client_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NuveiTransactionType {
    Auth,
    Sale,
    Credit,
    Auth3D,
    InitAuth3D,
    Settle,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FraudDetails {
    pub final_decision: String,
}

fn get_payment_status(response: &NuveiPaymentsResponse) -> enums::AttemptStatus {
    match response.transaction_status.clone() {
        Some(status) => match status {
            NuveiTransactionStatus::Approved => match response.transaction_type {
                Some(NuveiTransactionType::Auth) => enums::AttemptStatus::Authorized,
                Some(NuveiTransactionType::Sale) | Some(NuveiTransactionType::Settle) => {
                    enums::AttemptStatus::Charged
                }
                Some(NuveiTransactionType::Void) => enums::AttemptStatus::Voided,
                _ => enums::AttemptStatus::Pending,
            },
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => {
                match response.transaction_type {
                    Some(NuveiTransactionType::Auth) => enums::AttemptStatus::AuthorizationFailed,
                    Some(NuveiTransactionType::Void) => enums::AttemptStatus::VoidFailed,
                    Some(NuveiTransactionType::Auth3D) => {
                        enums::AttemptStatus::AuthenticationFailed
                    }
                    _ => enums::AttemptStatus::Failure,
                }
            }
            NuveiTransactionStatus::Processing | NuveiTransactionStatus::Pending => {
                enums::AttemptStatus::Pending
            }
            NuveiTransactionStatus::Redirect => enums::AttemptStatus::AuthenticationPending,
        },
        None => match response.status {
            NuveiPaymentStatus::Failed | NuveiPaymentStatus::Error => enums::AttemptStatus::Failure,
            _ => enums::AttemptStatus::Pending,
        },
    }
}

fn build_error_response<T>(
    response: &NuveiPaymentsResponse,
    http_code: u16,
) -> Option<Result<T, types::ErrorResponse>> {
    match response.status {
        NuveiPaymentStatus::Error => Some(get_error_response(
            response.err_code,
            &response.reason,
            http_code,
        )),
        _ => {
            let err = Some(get_error_response(
                response.gw_error_code,
                &response.gw_error_reason,
                http_code,
            ));
            match response.transaction_status {
                Some(NuveiTransactionStatus::Error) | Some(NuveiTransactionStatus::Declined) => err,
                _ => match response
                    .gw_error_reason
                    .as_ref()
                    .map(|r| r.eq("Missing argument"))
                {
                    Some(true) => err,
                    _ => None,
                },
            }
        }
    }
}

pub trait NuveiPaymentsGenericResponse {}

impl NuveiPaymentsGenericResponse for api::Authorize {}
impl NuveiPaymentsGenericResponse for api::CompleteAuthorize {}
impl NuveiPaymentsGenericResponse for api::Void {}
impl NuveiPaymentsGenericResponse for api::PSync {}
impl NuveiPaymentsGenericResponse for api::Capture {}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NuveiPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
where
    F: NuveiPaymentsGenericResponse,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, NuveiPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = match item.data.payment_method {
            diesel_models::enums::PaymentMethod::Wallet
            | diesel_models::enums::PaymentMethod::BankRedirect => item
                .response
                .payment_option
                .as_ref()
                .and_then(|po| po.redirect_url.clone())
                .map(|base_url| services::RedirectForm::from((base_url, services::Method::Get))),
            _ => item
                .response
                .payment_option
                .as_ref()
                .and_then(|o| o.card.clone())
                .and_then(|card| card.three_d)
                .and_then(|three_ds| three_ds.acs_url.zip(three_ds.c_req))
                .map(|(base_url, creq)| services::RedirectForm::Form {
                    endpoint: base_url,
                    method: services::Method::Post,
                    form_fields: std::collections::HashMap::from([(
                        "creq".to_string(),
                        creq.expose(),
                    )]),
                }),
        };

        let response = item.response;
        Ok(Self {
            status: get_payment_status(&response),
            response: if let Some(err) = build_error_response(&response, item.http_code) {
                err
            } else {
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: response
                        .transaction_id
                        .map_or(response.order_id.clone(), Some) // For paypal there will be no transaction_id, only order_id will be present
                        .map(types::ResponseId::ConnectorTransactionId)
                        .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
                    redirection_data,
                    mandate_reference: response
                        .payment_option
                        .and_then(|po| po.user_payment_option_id)
                        .map(|id| types::MandateReference {
                            connector_mandate_id: Some(id),
                            payment_method_id: None,
                        }),
                    // we don't need to save session token for capture, void flow so ignoring if it is not present
                    connector_metadata: if let Some(token) = response.session_token {
                        Some(
                            serde_json::to_value(NuveiMeta {
                                session_token: token,
                            })
                            .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
                        )
                    } else {
                        None
                    },
                    network_txn_id: None,
                    connector_response_reference_id: response.order_id,
                    incremental_authorization_allowed: None,
                })
            },
            ..item.data
        })
    }
}

impl TryFrom<types::PaymentsInitResponseRouterData<NuveiPaymentsResponse>>
    for types::PaymentsInitRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsInitResponseRouterData<NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let response = item.response;
        let is_enrolled_for_3ds = response
            .clone()
            .payment_option
            .and_then(|po| po.card)
            .and_then(|c| c.three_d)
            .and_then(|t| t.v2supported)
            .map(utils::to_boolean)
            .unwrap_or_default();
        Ok(Self {
            status: get_payment_status(&response),
            response: Ok(types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                enrolled_v2: is_enrolled_for_3ds,
                related_transaction_id: response.transaction_id,
            }),
            ..item.data
        })
    }
}

impl From<NuveiTransactionStatus> for enums::RefundStatus {
    fn from(item: NuveiTransactionStatus) -> Self {
        match item {
            NuveiTransactionStatus::Approved => Self::Success,
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => Self::Failure,
            NuveiTransactionStatus::Processing
            | NuveiTransactionStatus::Pending
            | NuveiTransactionStatus::Redirect => Self::Pending,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, NuveiPaymentsResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: get_refund_response(
                item.response.clone(),
                item.http_code,
                item.response
                    .transaction_id
                    .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
            ),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, NuveiPaymentsResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: get_refund_response(
                item.response.clone(),
                item.http_code,
                item.response
                    .transaction_id
                    .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
            ),
            ..item.data
        })
    }
}

impl<F> TryFrom<&types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>>
    for NuveiPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        {
            let item = data;
            let connector_mandate_id = &item.request.connector_mandate_id();
            let related_transaction_id = if item.is_three_ds() {
                item.request.related_transaction_id.clone()
            } else {
                None
            };
            Ok(Self {
                related_transaction_id,
                device_details: Option::<DeviceDetails>::foreign_try_from(
                    &item.request.browser_info.clone(),
                )?,
                is_rebilling: Some("1".to_string()), // In case of second installment, rebilling should be 1
                user_token_id: Some(item.request.get_email()?),
                payment_option: PaymentOption {
                    user_payment_option_id: connector_mandate_id.clone(),
                    ..Default::default()
                },
                ..Default::default()
            })
        }
    }
}

impl ForeignTryFrom<&Option<BrowserInformation>> for Option<DeviceDetails> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(browser_info: &Option<BrowserInformation>) -> Result<Self, Self::Error> {
        let device_details = match browser_info {
            Some(browser_info) => Some(DeviceDetails {
                ip_address: browser_info.get_ip_address()?,
            }),
            None => None,
        };
        Ok(device_details)
    }
}

fn get_refund_response(
    response: NuveiPaymentsResponse,
    http_code: u16,
    txn_id: String,
) -> Result<types::RefundsResponseData, types::ErrorResponse> {
    let refund_status = response
        .transaction_status
        .clone()
        .map(enums::RefundStatus::from)
        .unwrap_or(enums::RefundStatus::Failure);
    match response.status {
        NuveiPaymentStatus::Error => {
            get_error_response(response.err_code, &response.reason, http_code)
        }
        _ => match response.transaction_status {
            Some(NuveiTransactionStatus::Error) => {
                get_error_response(response.gw_error_code, &response.gw_error_reason, http_code)
            }
            _ => Ok(types::RefundsResponseData {
                connector_refund_id: txn_id,
                refund_status,
            }),
        },
    }
}

fn get_error_response<T>(
    error_code: Option<i64>,
    error_msg: &Option<String>,
    http_code: u16,
) -> Result<T, types::ErrorResponse> {
    Err(types::ErrorResponse {
        code: error_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
        message: error_msg
            .clone()
            .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
        reason: None,
        status_code: http_code,
        attempt_status: None,
        connector_transaction_id: None,
    })
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NuveiWebhookDetails {
    pub ppp_status: Option<String>,
    #[serde(rename = "ppp_TransactionID")]
    pub ppp_transaction_id: String,
    #[serde(rename = "TransactionId")]
    pub transaction_id: Option<String>,
    pub userid: Option<String>,
    pub merchant_unique_id: Option<String>,
    #[serde(rename = "customData")]
    pub custom_data: Option<String>,
    #[serde(rename = "productId")]
    pub product_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "totalAmount")]
    pub total_amount: String,
    pub currency: String,
    #[serde(rename = "responseTimeStamp")]
    pub response_time_stamp: String,
    #[serde(rename = "Status")]
    pub status: NuveiWebhookStatus,
    #[serde(rename = "transactionType")]
    pub transaction_type: Option<NuveiTransactionType>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NuveiWebhookTransactionId {
    #[serde(rename = "ppp_TransactionID")]
    pub ppp_transaction_id: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NuveiWebhookDataStatus {
    #[serde(rename = "Status")]
    pub status: NuveiWebhookStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NuveiWebhookStatus {
    Approved,
    Declined,
    #[default]
    Pending,
    Update,
    #[serde(other)]
    Unknown,
}

impl From<NuveiWebhookStatus> for NuveiTransactionStatus {
    fn from(status: NuveiWebhookStatus) -> Self {
        match status {
            NuveiWebhookStatus::Approved => Self::Approved,
            NuveiWebhookStatus::Declined => Self::Declined,
            _ => Self::Processing,
        }
    }
}

impl From<NuveiWebhookDetails> for NuveiPaymentsResponse {
    fn from(item: NuveiWebhookDetails) -> Self {
        Self {
            transaction_status: Some(NuveiTransactionStatus::from(item.status)),
            transaction_id: item.transaction_id,
            transaction_type: item.transaction_type,
            ..Default::default()
        }
    }
}
