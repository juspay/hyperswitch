#[cfg(feature = "payouts")]
use api_models::payouts::PayoutMethodData;
use api_models::{enums, payments, webhooks};
use cards::CardNumber;
use error_stack::ResultExt;
use masking::PeekInterface;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

#[cfg(feature = "payouts")]
use crate::connector::utils::AddressDetailsData;
#[cfg(feature = "payouts")]
use crate::types::api::payouts;
use crate::{
    connector::utils::{
        self, BrowserInformationData, CardData, MandateReferenceData, PaymentsAuthorizeRequestData,
        RouterData,
    },
    consts,
    core::errors,
    pii::{Email, Secret},
    services,
    types::{
        self,
        api::{self, enums as api_enums},
        storage::enums as storage_enums,
        transformers::ForeignFrom,
        PaymentsAuthorizeData,
    },
    utils as crate_utils,
};

type Error = error_stack::Report<errors::ConnectorError>;

// Adyen Types Definition
// Payments Request and Response Types
#[derive(Default, Debug, Serialize, Deserialize)]
pub enum AdyenShopperInteraction {
    #[default]
    Ecommerce,
    #[serde(rename = "ContAuth")]
    ContinuedAuthentication,
    Moto,
    #[serde(rename = "POS")]
    Pos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AdyenRecurringModel {
    UnscheduledCardOnFile,
    CardOnFile,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum AuthType {
    #[default]
    PreAuth,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalData {
    authorisation_type: Option<AuthType>,
    manual_capture: Option<bool>,
    pub recurring_processing_model: Option<AdyenRecurringModel>,
    /// Enable recurring details in dashboard to receive this ID, https://docs.adyen.com/online-payments/tokenization/create-and-use-tokens#test-and-go-live
    #[serde(rename = "recurring.recurringDetailReference")]
    recurring_detail_reference: Option<String>,
    #[serde(rename = "recurring.shopperReference")]
    recurring_shopper_reference: Option<String>,
    network_tx_reference: Option<String>,
    #[cfg(feature = "payouts")]
    payout_eligible: Option<PayoutEligibility>,
    funds_availability: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopperName {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    city: Option<String>,
    country: Option<api_enums::CountryAlpha2>,
    house_number_or_name: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    state_or_province: Option<Secret<String>>,
    street: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineItem {
    amount_excluding_tax: Option<i64>,
    amount_including_tax: Option<i64>,
    description: Option<String>,
    id: Option<String>,
    tax_amount: Option<i64>,
    quantity: Option<u16>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPaymentRequest<'a> {
    amount: Amount,
    merchant_account: Secret<String>,
    payment_method: AdyenPaymentMethod<'a>,
    reference: String,
    return_url: String,
    browser_info: Option<AdyenBrowserInfo>,
    shopper_interaction: AdyenShopperInteraction,
    recurring_processing_model: Option<AdyenRecurringModel>,
    additional_data: Option<AdditionalData>,
    shopper_reference: Option<String>,
    store_payment_method: Option<bool>,
    shopper_name: Option<ShopperName>,
    shopper_locale: Option<String>,
    shopper_email: Option<Email>,
    social_security_number: Option<Secret<String>>,
    telephone_number: Option<Secret<String>>,
    billing_address: Option<Address>,
    delivery_address: Option<Address>,
    country_code: Option<api_enums::CountryAlpha2>,
    line_items: Option<Vec<LineItem>>,
    channel: Option<Channel>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdyenBrowserInfo {
    user_agent: String,
    accept_header: String,
    language: String,
    color_depth: u8,
    screen_height: u32,
    screen_width: u32,
    time_zone_offset: i32,
    java_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdyenStatus {
    AuthenticationFinished,
    AuthenticationNotRequired,
    Authorised,
    Cancelled,
    ChallengeShopper,
    Error,
    Pending,
    Received,
    RedirectShopper,
    Refused,
    PresentToShopper,
    #[cfg(feature = "payouts")]
    #[serde(rename = "[payout-confirm-received]")]
    PayoutConfirmReceived,
    #[cfg(feature = "payouts")]
    #[serde(rename = "[payout-decline-received]")]
    PayoutDeclineReceived,
    #[cfg(feature = "payouts")]
    #[serde(rename = "[payout-submit-received]")]
    PayoutSubmitReceived,
}

#[derive(Debug, Clone, Serialize)]
pub enum Channel {
    Web,
}

/// This implementation will be used only in Authorize, Automatic capture flow.
/// It is also being used in Psync flow, However Psync will be called only after create payment call that too in redirect flow.
impl ForeignFrom<(bool, AdyenStatus)> for storage_enums::AttemptStatus {
    fn foreign_from((is_manual_capture, adyen_status): (bool, AdyenStatus)) -> Self {
        match adyen_status {
            AdyenStatus::AuthenticationFinished => Self::AuthenticationSuccessful,
            AdyenStatus::AuthenticationNotRequired | AdyenStatus::PresentToShopper => Self::Pending,
            AdyenStatus::Authorised => match is_manual_capture {
                true => Self::Authorized,
                // In case of Automatic capture Authorized is the final status of the payment
                false => Self::Charged,
            },
            AdyenStatus::Cancelled => Self::Voided,
            AdyenStatus::ChallengeShopper | AdyenStatus::RedirectShopper => {
                Self::AuthenticationPending
            }
            AdyenStatus::Error | AdyenStatus::Refused => Self::Failure,
            AdyenStatus::Pending => Self::Pending,
            AdyenStatus::Received => Self::Started,
            #[cfg(feature = "payouts")]
            AdyenStatus::PayoutConfirmReceived => Self::Started,
            #[cfg(feature = "payouts")]
            AdyenStatus::PayoutSubmitReceived => Self::Pending,
            #[cfg(feature = "payouts")]
            AdyenStatus::PayoutDeclineReceived => Self::Voided,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AdyenRedirectRequest {
    pub details: AdyenRedirectRequestTypes,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum AdyenRedirectRequestTypes {
    AdyenRedirection(AdyenRedirection),
    AdyenThreeDS(AdyenThreeDS),
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirection {
    #[serde(rename = "redirectResult")]
    pub redirect_result: String,
    #[serde(rename = "type")]
    pub type_of_redirection_result: Option<String>,
    pub result_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdyenThreeDS {
    #[serde(rename = "threeDSResult")]
    pub three_ds_result: String,
    #[serde(rename = "type")]
    pub type_of_redirection_result: Option<String>,
    pub result_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AdyenPaymentResponse {
    Response(Box<Response>),
    PresentToShopper(Box<PresentToShopperResponse>),
    QrCodeResponse(Box<QrCodeResponseResponse>),
    RedirectionResponse(Box<RedirectionResponse>),
    RedirectionErrorResponse(Box<RedirectionErrorResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    psp_reference: String,
    result_code: AdyenStatus,
    amount: Option<Amount>,
    merchant_reference: String,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    additional_data: Option<AdditionalData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectionErrorResponse {
    result_code: AdyenStatus,
    refusal_reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectionResponse {
    result_code: AdyenStatus,
    action: AdyenRedirectAction,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentToShopperResponse {
    psp_reference: Option<String>,
    result_code: AdyenStatus,
    action: AdyenPtsAction,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeResponseResponse {
    result_code: AdyenStatus,
    action: AdyenQrCodeAction,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenQrCodeAction {
    payment_method_type: PaymentType,
    #[serde(rename = "type")]
    type_of_response: ActionType,
    #[serde(rename = "url")]
    mobile_redirection_url: Option<Url>,
    qr_code_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPtsAction {
    reference: String,
    download_url: Option<Url>,
    payment_method_type: PaymentType,
    expires_at: Option<String>,
    initial_amount: Option<Amount>,
    pass_creation_token: Option<String>,
    total_amount: Option<Amount>,
    #[serde(rename = "type")]
    type_of_response: ActionType,
    instructions_url: Option<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirectAction {
    payment_method_type: PaymentType,
    url: Option<Url>,
    method: Option<services::Method>,
    #[serde(rename = "type")]
    type_of_response: ActionType,
    data: Option<std::collections::HashMap<String, String>>,
    payment_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Redirect,
    Await,
    #[serde(rename = "qrCode")]
    QrCode,
    Voucher,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    currency: String,
    value: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum AdyenPaymentMethod<'a> {
    AdyenAffirm(Box<AdyenPayLaterData>),
    AdyenCard(Box<AdyenCard>),
    AdyenKlarna(Box<AdyenPayLaterData>),
    AdyenPaypal(Box<AdyenPaypal>),
    #[serde(rename = "afterpaytouch")]
    AfterPay(Box<AdyenPayLaterData>),
    AlmaPayLater(Box<AdyenPayLaterData>),
    AliPay(Box<AliPayData>),
    AliPayHk(Box<AliPayHkData>),
    ApplePay(Box<AdyenApplePay>),
    #[serde(rename = "atome")]
    Atome,
    BancontactCard(Box<BancontactCardData>),
    Bizum(Box<BankRedirectionPMData>),
    Blik(Box<BlikRedirectionData>),
    #[serde(rename = "boletobancario")]
    BoletoBancario,
    #[serde(rename = "clearpay")]
    ClearPay,
    #[serde(rename = "dana")]
    Dana,
    Eps(Box<BankRedirectionWithIssuer<'a>>),
    #[serde(rename = "gcash")]
    Gcash(Box<GcashData>),
    Giropay(Box<BankRedirectionPMData>),
    Gpay(Box<AdyenGPay>),
    #[serde(rename = "gopay_wallet")]
    GoPay(Box<GoPayData>),
    Ideal(Box<BankRedirectionWithIssuer<'a>>),
    #[serde(rename = "kakaopay")]
    Kakaopay(Box<KakaoPayData>),
    Mandate(Box<AdyenMandate>),
    Mbway(Box<MbwayData>),
    MobilePay(Box<MobilePayData>),
    #[serde(rename = "momo_wallet")]
    Momo(Box<MomoData>),
    #[serde(rename = "touchngo")]
    TouchNGo(Box<TouchNGoData>),
    OnlineBankingCzechRepublic(Box<OnlineBankingCzechRepublicData>),
    OnlineBankingFinland(Box<OnlineBankingFinlandData>),
    OnlineBankingPoland(Box<OnlineBankingPolandData>),
    OnlineBankingSlovakia(Box<OnlineBankingSlovakiaData>),
    #[serde(rename = "molpay_ebanking_fpx_MY")]
    OnlineBankingFpx(Box<OnlineBankingFpxData>),
    #[serde(rename = "molpay_ebanking_TH")]
    OnlineBankingThailand(Box<OnlineBankingThailandData>),
    #[serde(rename = "paysafecard")]
    PaySafeCard(Box<PaySafeCardData>),
    #[serde(rename = "paybright")]
    PayBright,
    #[serde(rename = "doku_permata_lite_atm")]
    PermataBankTransfer(Box<DokuBankData>),
    #[serde(rename = "directEbanking")]
    Sofort,
    #[serde(rename = "trustly")]
    Trustly,
    #[serde(rename = "walley")]
    Walley,
    #[serde(rename = "wechatpayWeb")]
    WeChatPayWeb,
    AchDirectDebit(Box<AchDirectDebitData>),
    #[serde(rename = "sepadirectdebit")]
    SepaDirectDebit(Box<SepaDirectDebitData>),
    BacsDirectDebit(Box<BacsDirectDebitData>),
    SamsungPay(Box<SamsungPayPmData>),
    #[serde(rename = "doku_bca_va")]
    BcaBankTransfer(Box<DokuBankData>),
    #[serde(rename = "doku_bni_va")]
    BniVa(Box<DokuBankData>),
    #[serde(rename = "doku_bri_va")]
    BriVa(Box<DokuBankData>),
    #[serde(rename = "doku_cimb_va")]
    CimbVa(Box<DokuBankData>),
    #[serde(rename = "doku_danamon_va")]
    DanamonVa(Box<DokuBankData>),
    #[serde(rename = "doku_mandiri_va")]
    MandiriVa(Box<DokuBankData>),
    #[serde(rename = "twint")]
    Twint,
    #[serde(rename = "vipps")]
    Vipps,
    #[serde(rename = "doku_indomaret")]
    Indomaret(Box<DokuBankData>),
    #[serde(rename = "doku_alfamart")]
    Alfamart(Box<DokuBankData>),
    #[serde(rename = "swish")]
    Swish,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchDirectDebitData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    bank_account_number: Secret<String>,
    bank_location_id: Secret<String>,
    owner_name: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SepaDirectDebitData {
    #[serde(rename = "sepa.ownerName")]
    owner_name: Secret<String>,
    #[serde(rename = "sepa.ibanNumber")]
    iban_number: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacsDirectDebitData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    bank_account_number: Secret<String>,
    bank_location_id: Secret<String>,
    holder_name: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandateData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    stored_payment_method_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BancontactCardData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    brand: String,
    number: CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    holder_name: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobilePayData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MbwayData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    telephone_number: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WalleyData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize)]
pub struct SamsungPayPmData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    #[serde(rename = "samsungPayToken")]
    samsung_pay_token: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PayBrightData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaySafeCardData {}

#[derive(Debug, Clone, Serialize)]
pub struct OnlineBankingFinlandData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}
#[derive(Debug, Clone, Serialize)]
pub struct OnlineBankingCzechRepublicData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    issuer: OnlineBankingCzechRepublicBanks,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OnlineBankingCzechRepublicBanks {
    KB,
    CS,
    C,
}

impl TryFrom<&api_enums::BankNames> for OnlineBankingCzechRepublicBanks {
    type Error = Error;
    fn try_from(bank_name: &api_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            api::enums::BankNames::KomercniBanka => Ok(Self::KB),
            api::enums::BankNames::CeskaSporitelna => Ok(Self::CS),
            api::enums::BankNames::PlatnoscOnlineKartaPlatnicza => Ok(Self::C),
            _ => Err(errors::ConnectorError::NotSupported {
                message: String::from("BankRedirect"),
                connector: "Adyen",
                payment_experience: api_enums::PaymentExperience::RedirectToUrl.to_string(),
            })?,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OnlineBankingPolandData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    issuer: OnlineBankingPolandBanks,
}

#[derive(Debug, Clone, Serialize)]
pub enum OnlineBankingPolandBanks {
    #[serde(rename = "154")]
    BlikPSP,
    #[serde(rename = "31")]
    PlaceZIPKO,
    #[serde(rename = "243")]
    MBank,
    #[serde(rename = "112")]
    PayWithING,
    #[serde(rename = "20")]
    SantanderPrzelew24,
    #[serde(rename = "65")]
    BankPEKAOSA,
    #[serde(rename = "85")]
    BankMillennium,
    #[serde(rename = "88")]
    PayWithAliorBank,
    #[serde(rename = "143")]
    BankiSpoldzielcze,
    #[serde(rename = "26")]
    PayWithInteligo,
    #[serde(rename = "33")]
    BNPParibasPoland,
    #[serde(rename = "144")]
    BankNowySA,
    #[serde(rename = "45")]
    CreditAgricole,
    #[serde(rename = "99")]
    PayWithBOS,
    #[serde(rename = "119")]
    PayWithCitiHandlowy,
    #[serde(rename = "131")]
    PayWithPlusBank,
    #[serde(rename = "64")]
    ToyotaBank,
    #[serde(rename = "153")]
    VeloBank,
    #[serde(rename = "141")]
    ETransferPocztowy24,
}

impl TryFrom<&api_enums::BankNames> for OnlineBankingPolandBanks {
    type Error = Error;
    fn try_from(bank_name: &api_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            api_models::enums::BankNames::BlikPSP => Ok(Self::BlikPSP),
            api_models::enums::BankNames::PlaceZIPKO => Ok(Self::PlaceZIPKO),
            api_models::enums::BankNames::MBank => Ok(Self::MBank),
            api_models::enums::BankNames::PayWithING => Ok(Self::PayWithING),
            api_models::enums::BankNames::SantanderPrzelew24 => Ok(Self::SantanderPrzelew24),
            api_models::enums::BankNames::BankPEKAOSA => Ok(Self::BankPEKAOSA),
            api_models::enums::BankNames::BankMillennium => Ok(Self::BankMillennium),
            api_models::enums::BankNames::PayWithAliorBank => Ok(Self::PayWithAliorBank),
            api_models::enums::BankNames::BankiSpoldzielcze => Ok(Self::BankiSpoldzielcze),
            api_models::enums::BankNames::PayWithInteligo => Ok(Self::PayWithInteligo),
            api_models::enums::BankNames::BNPParibasPoland => Ok(Self::BNPParibasPoland),
            api_models::enums::BankNames::BankNowySA => Ok(Self::BankNowySA),
            api_models::enums::BankNames::CreditAgricole => Ok(Self::CreditAgricole),
            api_models::enums::BankNames::PayWithBOS => Ok(Self::PayWithBOS),
            api_models::enums::BankNames::PayWithCitiHandlowy => Ok(Self::PayWithCitiHandlowy),
            api_models::enums::BankNames::PayWithPlusBank => Ok(Self::PayWithPlusBank),
            api_models::enums::BankNames::ToyotaBank => Ok(Self::ToyotaBank),
            api_models::enums::BankNames::VeloBank => Ok(Self::VeloBank),
            api_models::enums::BankNames::ETransferPocztowy24 => Ok(Self::ETransferPocztowy24),
            _ => Err(errors::ConnectorError::NotSupported {
                message: String::from("BankRedirect"),
                connector: "Adyen",
                payment_experience: api_enums::PaymentExperience::RedirectToUrl.to_string(),
            })?,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineBankingSlovakiaData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    issuer: OnlineBankingSlovakiaBanks,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineBankingFpxData {
    issuer: OnlineBankingFpxIssuer,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineBankingThailandData {
    issuer: OnlineBankingThailandIssuer,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OnlineBankingSlovakiaBanks {
    Vub,
    Posto,
    Sporo,
    Tatra,
    Viamo,
}

impl TryFrom<&api_enums::BankNames> for OnlineBankingSlovakiaBanks {
    type Error = Error;
    fn try_from(bank_name: &api_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            api::enums::BankNames::EPlatbyVUB => Ok(Self::Vub),
            api::enums::BankNames::PostovaBanka => Ok(Self::Posto),
            api::enums::BankNames::SporoPay => Ok(Self::Sporo),
            api::enums::BankNames::TatraPay => Ok(Self::Tatra),
            api::enums::BankNames::Viamo => Ok(Self::Viamo),
            _ => Err(errors::ConnectorError::NotSupported {
                message: String::from("BankRedirect"),
                connector: "Adyen",
                payment_experience: api_enums::PaymentExperience::RedirectToUrl.to_string(),
            })?,
        }
    }
}

impl TryFrom<&api_enums::BankNames> for OnlineBankingFpxIssuer {
    type Error = Error;
    fn try_from(bank_name: &api_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            api::enums::BankNames::AffinBank => Ok(Self::FpxAbb),
            api::enums::BankNames::AgroBank => Ok(Self::FpxAgrobank),
            api::enums::BankNames::AllianceBank => Ok(Self::FpxAbmb),
            api::enums::BankNames::AmBank => Ok(Self::FpxAmb),
            api::enums::BankNames::BankIslam => Ok(Self::FpxBimb),
            api::enums::BankNames::BankMuamalat => Ok(Self::FpxBmmb),
            api::enums::BankNames::BankRakyat => Ok(Self::FpxBkrm),
            api::enums::BankNames::BankSimpananNasional => Ok(Self::FpxBsn),
            api::enums::BankNames::CimbBank => Ok(Self::FpxCimbclicks),
            api::enums::BankNames::HongLeongBank => Ok(Self::FpxHlb),
            api::enums::BankNames::HsbcBank => Ok(Self::FpxHsbc),
            api::enums::BankNames::KuwaitFinanceHouse => Ok(Self::FpxKfh),
            api::enums::BankNames::Maybank => Ok(Self::FpxMb2u),
            api::enums::BankNames::OcbcBank => Ok(Self::FpxOcbc),
            api::enums::BankNames::PublicBank => Ok(Self::FpxPbb),
            api::enums::BankNames::RhbBank => Ok(Self::FpxRhb),
            api::enums::BankNames::StandardCharteredBank => Ok(Self::FpxScb),
            api::enums::BankNames::UobBank => Ok(Self::FpxUob),
            _ => Err(errors::ConnectorError::NotSupported {
                message: String::from("BankRedirect"),
                connector: "Adyen",
                payment_experience: api_enums::PaymentExperience::RedirectToUrl.to_string(),
            })?,
        }
    }
}

impl TryFrom<&api_enums::BankNames> for OnlineBankingThailandIssuer {
    type Error = Error;
    fn try_from(bank_name: &api_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            api::enums::BankNames::BangkokBank => Ok(Self::Bangkokbank),
            api::enums::BankNames::KrungsriBank => Ok(Self::Krungsribank),
            api::enums::BankNames::KrungThaiBank => Ok(Self::Krungthaibank),
            api::enums::BankNames::TheSiamCommercialBank => Ok(Self::Siamcommercialbank),
            api::enums::BankNames::KasikornBank => Ok(Self::Kbank),
            _ => Err(errors::ConnectorError::NotSupported {
                message: String::from("BankRedirect"),
                connector: "Adyen",
                payment_experience: api_enums::PaymentExperience::RedirectToUrl.to_string(),
            })?,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlikRedirectionData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    blik_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankRedirectionPMData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankRedirectionWithIssuer<'a> {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    issuer: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenMandate {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    stored_payment_method_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCard {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    number: CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Option<Secret<String>>,
    brand: Option<CardBrand>, //Mandatory for mandate using network_txns_id
    network_payment_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardBrand {
    Visa,
    MC,
    Amex,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelRequest {
    merchant_account: Secret<String>,
    reference: String,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelResponse {
    psp_reference: String,
    status: CancelStatus,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CancelStatus {
    Received,
    #[default]
    Processing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenPaypal {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliPayData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliPayHkData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoPayData {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KakaoPayData {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcashData {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomoData {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchNGoData {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenGPay {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    #[serde(rename = "googlePayToken")]
    google_pay_token: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenApplePay {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    #[serde(rename = "applePayToken")]
    apple_pay_token: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenPayLaterData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DokuBankData {
    first_name: Secret<String>,
    last_name: Option<Secret<String>>,
    shopper_email: Email,
}
// Refunds Request and Response
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefundRequest {
    merchant_account: Secret<String>,
    amount: Amount,
    merchant_refund_reason: Option<String>,
    reference: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefundResponse {
    merchant_account: String,
    psp_reference: String,
    payment_psp_reference: String,
    reference: String,
    status: String,
}

pub struct AdyenAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    #[allow(dead_code)]
    pub(super) review_key: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentType {
    Affirm,
    Afterpaytouch,
    Alipay,
    #[serde(rename = "alipay_hk")]
    AlipayHk,
    #[serde(rename = "doku_alfamart")]
    Alfamart,
    Alma,
    Applepay,
    Bizum,
    Atome,
    Blik,
    #[serde(rename = "boletobancario")]
    BoletoBancario,
    ClearPay,
    Dana,
    Eps,
    Gcash,
    Giropay,
    Googlepay,
    #[serde(rename = "gopay_wallet")]
    GoPay,
    Ideal,
    #[serde(rename = "doku_indomaret")]
    Indomaret,
    Klarna,
    Kakaopay,
    Mbway,
    MobilePay,
    #[serde(rename = "momo_wallet")]
    Momo,
    #[serde(rename = "onlineBanking_CZ")]
    OnlineBankingCzechRepublic,
    #[serde(rename = "ebanking_FI")]
    OnlineBankingFinland,
    #[serde(rename = "onlineBanking_PL")]
    OnlineBankingPoland,
    #[serde(rename = "onlineBanking_SK")]
    OnlineBankingSlovakia,
    #[serde(rename = "molpay_ebanking_fpx_MY")]
    OnlineBankingFpx,
    #[serde(rename = "molpay_ebanking_TH")]
    OnlineBankingThailand,
    #[serde(rename = "paysafecard")]
    PaySafeCard,
    PayBright,
    Paypal,
    Scheme,
    #[serde(rename = "directEbanking")]
    Sofort,
    #[serde(rename = "networkToken")]
    NetworkToken,
    Trustly,
    #[serde(rename = "touchngo")]
    TouchNGo,
    Walley,
    #[serde(rename = "wechatpayWeb")]
    WeChatPayWeb,
    #[serde(rename = "ach")]
    AchDirectDebit,
    SepaDirectDebit,
    #[serde(rename = "directdebit_GB")]
    BacsDirectDebit,
    Samsungpay,
    Twint,
    Vipps,
    Swish,
    #[serde(rename = "doku_permata_lite_atm")]
    PermataBankTransfer,
    #[serde(rename = "doku_bca_va")]
    BcaBankTransfer,
    #[serde(rename = "doku_bni_va")]
    BniVa,
    #[serde(rename = "doku_bri_va")]
    BriVa,
    #[serde(rename = "doku_cimb_va")]
    CimbVa,
    #[serde(rename = "doku_danamon_va")]
    DanamonVa,
    #[serde(rename = "doku_mandiri_va")]
    MandiriVa,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum OnlineBankingFpxIssuer {
    FpxAbb,
    FpxAgrobank,
    FpxAbmb,
    FpxAmb,
    FpxBimb,
    FpxBmmb,
    FpxBkrm,
    FpxBsn,
    FpxCimbclicks,
    FpxHlb,
    FpxHsbc,
    FpxKfh,
    FpxMb2u,
    FpxOcbc,
    FpxPbb,
    FpxRhb,
    FpxScb,
    FpxUob,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
pub enum OnlineBankingThailandIssuer {
    #[serde(rename = "molpay_bangkokbank")]
    Bangkokbank,
    #[serde(rename = "molpay_krungsribank")]
    Krungsribank,
    #[serde(rename = "molpay_krungthaibank")]
    Krungthaibank,
    #[serde(rename = "molpay_siamcommercialbank")]
    Siamcommercialbank,
    #[serde(rename = "molpay_kbank")]
    Kbank,
}

pub struct AdyenTestBankNames<'a>(&'a str);

impl<'a> TryFrom<&api_enums::BankNames> for AdyenTestBankNames<'a> {
    type Error = Error;
    fn try_from(bank: &api_enums::BankNames) -> Result<Self, Self::Error> {
        Ok(match bank {
            api_models::enums::BankNames::AbnAmro => Self("1121"),
            api_models::enums::BankNames::AsnBank => Self("1151"),
            api_models::enums::BankNames::Bunq => Self("1152"),
            api_models::enums::BankNames::Handelsbanken => Self("1153"),
            api_models::enums::BankNames::Ing => Self("1154"),
            api_models::enums::BankNames::Knab => Self("1155"),
            api_models::enums::BankNames::Moneyou => Self("1156"),
            api_models::enums::BankNames::Rabobank => Self("1157"),
            api_models::enums::BankNames::Regiobank => Self("1158"),
            api_models::enums::BankNames::Revolut => Self("1159"),
            api_models::enums::BankNames::SnsBank => Self("1159"),
            api_models::enums::BankNames::TriodosBank => Self("1159"),
            api_models::enums::BankNames::VanLanschot => Self("1159"),
            api_models::enums::BankNames::BankAustria => {
                Self("e6819e7a-f663-414b-92ec-cf7c82d2f4e5")
            }
            api_models::enums::BankNames::BawagPskAg => {
                Self("ba7199cc-f057-42f2-9856-2378abf21638")
            }
            api_models::enums::BankNames::Dolomitenbank => {
                Self("d5d5b133-1c0d-4c08-b2be-3c9b116dc326")
            }
            api_models::enums::BankNames::EasybankAg => {
                Self("eff103e6-843d-48b7-a6e6-fbd88f511b11")
            }
            api_models::enums::BankNames::ErsteBankUndSparkassen => {
                Self("3fdc41fc-3d3d-4ee3-a1fe-cd79cfd58ea3")
            }
            api_models::enums::BankNames::HypoTirolBankAg => {
                Self("6765e225-a0dc-4481-9666-e26303d4f221")
            }
            api_models::enums::BankNames::PosojilnicaBankEGen => {
                Self("65ef4682-4944-499f-828f-5d74ad288376")
            }
            api_models::enums::BankNames::RaiffeisenBankengruppeOsterreich => {
                Self("ee9fc487-ebe0-486c-8101-17dce5141a67")
            }
            api_models::enums::BankNames::SchoellerbankAg => {
                Self("1190c4d1-b37a-487e-9355-e0a067f54a9f")
            }
            api_models::enums::BankNames::SpardaBankWien => {
                Self("8b0bfeea-fbb0-4337-b3a1-0e25c0f060fc")
            }
            api_models::enums::BankNames::VolksbankGruppe => {
                Self("e2e97aaa-de4c-4e18-9431-d99790773433")
            }
            api_models::enums::BankNames::VolkskreditbankAg => {
                Self("4a0a975b-0594-4b40-9068-39f77b3a91f9")
            }
            _ => Err(errors::ConnectorError::NotSupported {
                message: String::from("BankRedirect"),
                connector: "Adyen",
                payment_experience: api_enums::PaymentExperience::RedirectToUrl.to_string(),
            })?,
        })
    }
}

impl TryFrom<&types::ConnectorAuthType> for AdyenAuthType {
    type Error = Error;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                review_key: None,
            }),
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                review_key: Some(api_secret.to_owned()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

impl<'a> TryFrom<&types::PaymentsAuthorizeRouterData> for AdyenPaymentRequest<'a> {
    type Error = Error;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item
            .request
            .mandate_id
            .to_owned()
            .and_then(|mandate_ids| mandate_ids.mandate_reference_id)
        {
            Some(mandate_ref) => AdyenPaymentRequest::try_from((item, mandate_ref)),
            None => match item.request.payment_method_data {
                api_models::payments::PaymentMethodData::Card(ref card) => {
                    AdyenPaymentRequest::try_from((item, card))
                }
                api_models::payments::PaymentMethodData::Wallet(ref wallet) => {
                    AdyenPaymentRequest::try_from((item, wallet))
                }
                api_models::payments::PaymentMethodData::PayLater(ref pay_later) => {
                    AdyenPaymentRequest::try_from((item, pay_later))
                }
                api_models::payments::PaymentMethodData::BankRedirect(ref bank_redirect) => {
                    AdyenPaymentRequest::try_from((item, bank_redirect))
                }
                api_models::payments::PaymentMethodData::BankDebit(ref bank_debit) => {
                    AdyenPaymentRequest::try_from((item, bank_debit))
                }
                api_models::payments::PaymentMethodData::BankTransfer(ref bank_transfer) => {
                    AdyenPaymentRequest::try_from((item, bank_transfer.as_ref()))
                }
                api_models::payments::PaymentMethodData::Voucher(ref voucher_data) => {
                    AdyenPaymentRequest::try_from((item, voucher_data))
                }
                api_models::payments::PaymentMethodData::GiftCard(ref gift_card) => {
                    AdyenPaymentRequest::try_from((item, gift_card.as_ref()))
                }
                _ => Err(errors::ConnectorError::NotSupported {
                    message: format!("{:?}", item.request.payment_method_type),
                    connector: "Adyen",
                    payment_experience: api_models::enums::PaymentExperience::RedirectToUrl
                        .to_string(),
                })?,
            },
        }
    }
}

impl From<&types::PaymentsAuthorizeRouterData> for AdyenShopperInteraction {
    fn from(item: &types::PaymentsAuthorizeRouterData) -> Self {
        match item.request.off_session {
            Some(true) => Self::ContinuedAuthentication,
            _ => Self::Ecommerce,
        }
    }
}
type RecurringDetails = (Option<AdyenRecurringModel>, Option<bool>, Option<String>);

fn get_recurring_processing_model(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<RecurringDetails, Error> {
    match (item.request.setup_future_usage, item.request.off_session) {
        (Some(storage_enums::FutureUsage::OffSession), _) => {
            let customer_id = item.get_customer_id()?;
            let shopper_reference = format!("{}_{}", item.merchant_id, customer_id);
            let store_payment_method = item.request.is_mandate_payment();
            Ok((
                Some(AdyenRecurringModel::UnscheduledCardOnFile),
                Some(store_payment_method),
                Some(shopper_reference),
            ))
        }
        (_, Some(true)) => Ok((
            Some(AdyenRecurringModel::UnscheduledCardOnFile),
            None,
            Some(format!("{}_{}", item.merchant_id, item.get_customer_id()?)),
        )),
        _ => Ok((None, None, None)),
    }
}

fn get_browser_info(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<AdyenBrowserInfo>, Error> {
    if item.auth_type == storage_enums::AuthenticationType::ThreeDs
        || item.payment_method == storage_enums::PaymentMethod::BankRedirect
        || item.request.payment_method_type == Some(storage_enums::PaymentMethodType::GoPay)
    {
        let info = item.request.get_browser_info()?;
        Ok(Some(AdyenBrowserInfo {
            accept_header: info.get_accept_header()?,
            language: info.get_language()?,
            screen_height: info.get_screen_height()?,
            screen_width: info.get_screen_width()?,
            color_depth: info.get_color_depth()?,
            user_agent: info.get_user_agent()?,
            time_zone_offset: info.get_time_zone()?,
            java_enabled: info.get_java_enabled()?,
        }))
    } else {
        Ok(None)
    }
}

fn get_additional_data(item: &types::PaymentsAuthorizeRouterData) -> Option<AdditionalData> {
    match item.request.capture_method {
        Some(diesel_models::enums::CaptureMethod::Manual) => Some(AdditionalData {
            authorisation_type: Some(AuthType::PreAuth),
            manual_capture: Some(true),
            network_tx_reference: None,
            recurring_detail_reference: None,
            recurring_shopper_reference: None,
            recurring_processing_model: Some(AdyenRecurringModel::UnscheduledCardOnFile),
            ..AdditionalData::default()
        }),
        _ => None,
    }
}

fn get_channel_type(pm_type: &Option<storage_enums::PaymentMethodType>) -> Option<Channel> {
    pm_type.as_ref().and_then(|pmt| match pmt {
        storage_enums::PaymentMethodType::GoPay => Some(Channel::Web),
        _ => None,
    })
}

fn get_amount_data(item: &types::PaymentsAuthorizeRouterData) -> Amount {
    Amount {
        currency: item.request.currency.to_string(),
        value: item.request.amount,
    }
}

fn get_address_info(address: Option<&api_models::payments::Address>) -> Option<Address> {
    address.and_then(|add| {
        add.address.as_ref().map(|a| Address {
            city: a.city.clone(),
            country: a.country,
            house_number_or_name: a.line1.clone(),
            postal_code: a.zip.clone(),
            state_or_province: a.state.clone(),
            street: a.line2.clone(),
        })
    })
}

fn get_line_items(item: &types::PaymentsAuthorizeRouterData) -> Vec<LineItem> {
    let order_details: Option<Vec<payments::OrderDetailsWithAmount>> =
        item.request.order_details.clone();
    match order_details {
        Some(od) => od
            .iter()
            .enumerate()
            .map(|(i, data)| LineItem {
                amount_including_tax: Some(data.amount),
                amount_excluding_tax: Some(data.amount),
                description: Some(data.product_name.clone()),
                id: Some(format!("Items #{i}")),
                tax_amount: None,
                quantity: Some(data.quantity),
            })
            .collect(),
        None => {
            let line_item = LineItem {
                amount_including_tax: Some(item.request.amount),
                amount_excluding_tax: Some(item.request.amount),
                description: item.description.clone(),
                id: Some(String::from("Items #1")),
                tax_amount: None,
                quantity: Some(1),
            };
            vec![line_item]
        }
    }
}

fn get_telephone_number(item: &types::PaymentsAuthorizeRouterData) -> Option<Secret<String>> {
    let phone = item
        .address
        .billing
        .as_ref()
        .and_then(|billing| billing.phone.as_ref());
    phone.as_ref().and_then(|phone| {
        phone.number.as_ref().and_then(|number| {
            phone
                .country_code
                .as_ref()
                .map(|cc| Secret::new(format!("{}{}", cc, number.peek())))
        })
    })
}

fn get_shopper_name(address: Option<&api_models::payments::Address>) -> Option<ShopperName> {
    let billing = address.and_then(|billing| billing.address.as_ref());
    Some(ShopperName {
        first_name: billing.and_then(|a| a.first_name.clone()),
        last_name: billing.and_then(|a| a.last_name.clone()),
    })
}

fn get_country_code(
    address: Option<&api_models::payments::Address>,
) -> Option<api_enums::CountryAlpha2> {
    address.and_then(|billing| billing.address.as_ref().and_then(|address| address.country))
}

#[cfg(feature = "payouts")]
fn get_payout_card_details(payout_method_data: &PayoutMethodData) -> Option<PayoutCardDetails> {
    match payout_method_data {
        PayoutMethodData::Card(card) => Some(PayoutCardDetails {
            _type: "scheme".to_string(), // FIXME: Remove hardcoding
            number: card.card_number.peek().to_string(),
            expiry_month: card.expiry_month.peek().to_string(),
            expiry_year: card.expiry_year.peek().to_string(),
            holder_name: card.card_holder_name.peek().to_string(),
        }),
        _ => None,
    }
}

fn get_social_security_number(
    voucher_data: &api_models::payments::VoucherData,
) -> Option<Secret<String>> {
    match voucher_data {
        payments::VoucherData::Boleto(boleto_data) => boleto_data.social_security_number.clone(),
        payments::VoucherData::Alfamart { .. }
        | payments::VoucherData::Indomaret { .. }
        | payments::VoucherData::Efecty
        | payments::VoucherData::PagoEfectivo
        | payments::VoucherData::RedCompra
        | payments::VoucherData::RedPagos => None,
    }
}

impl<'a> TryFrom<&api_models::payments::BankDebitData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(
        bank_debit_data: &api_models::payments::BankDebitData,
    ) -> Result<Self, Self::Error> {
        match bank_debit_data {
            payments::BankDebitData::AchBankDebit {
                account_number,
                routing_number,
                card_holder_name,
                ..
            } => Ok(AdyenPaymentMethod::AchDirectDebit(Box::new(
                AchDirectDebitData {
                    payment_type: PaymentType::AchDirectDebit,
                    bank_account_number: account_number.clone(),
                    bank_location_id: routing_number.clone(),
                    owner_name: card_holder_name.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "card_holder_name",
                        },
                    )?,
                },
            ))),
            payments::BankDebitData::SepaBankDebit {
                iban,
                bank_account_holder_name,
                ..
            } => Ok(AdyenPaymentMethod::SepaDirectDebit(Box::new(
                SepaDirectDebitData {
                    owner_name: bank_account_holder_name.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "bank_account_holder_name",
                        },
                    )?,
                    iban_number: iban.clone(),
                },
            ))),
            payments::BankDebitData::BacsBankDebit {
                account_number,
                sort_code,
                bank_account_holder_name,
                ..
            } => Ok(AdyenPaymentMethod::BacsDirectDebit(Box::new(
                BacsDirectDebitData {
                    payment_type: PaymentType::BacsDirectDebit,
                    bank_account_number: account_number.clone(),
                    bank_location_id: sort_code.clone(),
                    holder_name: bank_account_holder_name.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "bank_account_holder_name",
                        },
                    )?,
                },
            ))),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl<'a> TryFrom<&api_models::payments::VoucherData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(voucher_data: &api_models::payments::VoucherData) -> Result<Self, Self::Error> {
        match voucher_data {
            payments::VoucherData::Boleto { .. } => Ok(AdyenPaymentMethod::BoletoBancario(
                Box::new(AdyenVoucherData {
                    payment_type: PaymentType::BoletoBancario,
                }),
            )),
            payments::VoucherData::Alfamart(alfarmart_data) => {
                Ok(AdyenPaymentMethod::Alfamart(Box::new(DokuBankData {
                    first_name: alfarmart_data.first_name.clone(),
                    last_name: alfarmart_data.last_name.clone(),
                    shopper_email: alfarmart_data.email.clone(),
                })))
            }
            payments::VoucherData::Indomaret(indomaret_data) => {
                Ok(AdyenPaymentMethod::Indomaret(Box::new(DokuBankData {
                    first_name: indomaret_data.first_name.clone(),
                    last_name: indomaret_data.last_name.clone(),
                    shopper_email: indomaret_data.email.clone(),
                })))
            }
            payments::VoucherData::Efecty
            | payments::VoucherData::PagoEfectivo
            | payments::VoucherData::RedCompra
            | payments::VoucherData::RedPagos => Err(errors::ConnectorError::NotImplemented(
                "this payment method".to_string(),
            )
            .into()),
        }
    }
}

impl<'a> TryFrom<&api_models::payments::GiftCardData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(gift_card_data: &api_models::payments::GiftCardData) -> Result<Self, Self::Error> {
        match gift_card_data {
            payments::GiftCardData::PaySafeCard {} => Ok(AdyenPaymentMethod::PaySafeCard(
                Box::new(PaySafeCardData {}),
            )),
            payments::GiftCardData::BabyGiftCard {..} => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl<'a> TryFrom<&api::Card> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(card: &api::Card) -> Result<Self, Self::Error> {
        let adyen_card = AdyenCard {
            payment_type: PaymentType::Scheme,
            number: card.card_number.clone(),
            expiry_month: card.card_exp_month.clone(),
            expiry_year: card.get_expiry_year_4_digit(),
            cvc: Some(card.card_cvc.clone()),
            brand: None,
            network_payment_reference: None,
        };
        Ok(AdyenPaymentMethod::AdyenCard(Box::new(adyen_card)))
    }
}

impl TryFrom<&storage_enums::PaymentMethodType> for PaymentType {
    type Error = Error;
    fn try_from(item: &storage_enums::PaymentMethodType) -> Result<Self, Self::Error> {
        match item {
            storage_enums::PaymentMethodType::Credit
            | storage_enums::PaymentMethodType::Debit
            | storage_enums::PaymentMethodType::Klarna
            | storage_enums::PaymentMethodType::Ach
            | storage_enums::PaymentMethodType::Sepa
            | storage_enums::PaymentMethodType::Bacs
            | storage_enums::PaymentMethodType::BancontactCard
            | storage_enums::PaymentMethodType::Blik
            | storage_enums::PaymentMethodType::Eps
            | storage_enums::PaymentMethodType::Giropay
            | storage_enums::PaymentMethodType::Ideal
            | storage_enums::PaymentMethodType::OnlineBankingCzechRepublic
            | storage_enums::PaymentMethodType::OnlineBankingFinland
            | storage_enums::PaymentMethodType::OnlineBankingPoland
            | storage_enums::PaymentMethodType::OnlineBankingSlovakia
            | storage_enums::PaymentMethodType::Sofort
            | storage_enums::PaymentMethodType::Trustly
            | storage_enums::PaymentMethodType::GooglePay
            | storage_enums::PaymentMethodType::AliPay
            | storage_enums::PaymentMethodType::ApplePay
            | storage_enums::PaymentMethodType::AliPayHk
            | storage_enums::PaymentMethodType::MbWay
            | storage_enums::PaymentMethodType::MobilePay
            | storage_enums::PaymentMethodType::WeChatPay
            | storage_enums::PaymentMethodType::SamsungPay
            | storage_enums::PaymentMethodType::Affirm
            | storage_enums::PaymentMethodType::AfterpayClearpay
            | storage_enums::PaymentMethodType::PayBright
            | storage_enums::PaymentMethodType::Walley => Ok(Self::Scheme),
            storage_enums::PaymentMethodType::Paypal => Ok(Self::Paypal),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method Type".to_string(),
            ))?,
        }
    }
}

impl TryFrom<&utils::CardIssuer> for CardBrand {
    type Error = Error;
    fn try_from(card_issuer: &utils::CardIssuer) -> Result<Self, Self::Error> {
        match card_issuer {
            utils::CardIssuer::AmericanExpress => Ok(Self::Amex),
            utils::CardIssuer::Master => Ok(Self::MC),
            utils::CardIssuer::Visa => Ok(Self::Visa),
            _ => Err(errors::ConnectorError::NotImplemented("CardBrand".to_string()).into()),
        }
    }
}

impl<'a> TryFrom<&api::WalletData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(wallet_data: &api::WalletData) -> Result<Self, Self::Error> {
        match wallet_data {
            api_models::payments::WalletData::GooglePay(data) => {
                let gpay_data = AdyenGPay {
                    payment_type: PaymentType::Googlepay,
                    google_pay_token: Secret::new(data.tokenization_data.token.to_owned()),
                };
                Ok(AdyenPaymentMethod::Gpay(Box::new(gpay_data)))
            }
            api_models::payments::WalletData::ApplePay(data) => {
                let apple_pay_data = AdyenApplePay {
                    payment_type: PaymentType::Applepay,
                    apple_pay_token: Secret::new(data.payment_data.to_string()),
                };

                Ok(AdyenPaymentMethod::ApplePay(Box::new(apple_pay_data)))
            }
            api_models::payments::WalletData::PaypalRedirect(_) => {
                let wallet = AdyenPaypal {
                    payment_type: PaymentType::Paypal,
                };
                Ok(AdyenPaymentMethod::AdyenPaypal(Box::new(wallet)))
            }
            api_models::payments::WalletData::AliPayRedirect(_) => {
                let alipay_data = AliPayData {
                    payment_type: PaymentType::Alipay,
                };
                Ok(AdyenPaymentMethod::AliPay(Box::new(alipay_data)))
            }
            api_models::payments::WalletData::AliPayHkRedirect(_) => {
                let alipay_hk_data = AliPayHkData {
                    payment_type: PaymentType::AlipayHk,
                };
                Ok(AdyenPaymentMethod::AliPayHk(Box::new(alipay_hk_data)))
            }
            api_models::payments::WalletData::GoPayRedirect(_) => {
                let go_pay_data = GoPayData {};
                Ok(AdyenPaymentMethod::GoPay(Box::new(go_pay_data)))
            }
            api_models::payments::WalletData::KakaoPayRedirect(_) => {
                let kakao_pay_data = KakaoPayData {};
                Ok(AdyenPaymentMethod::Kakaopay(Box::new(kakao_pay_data)))
            }
            api_models::payments::WalletData::GcashRedirect(_) => {
                let gcash_data = GcashData {};
                Ok(AdyenPaymentMethod::Gcash(Box::new(gcash_data)))
            }
            api_models::payments::WalletData::MomoRedirect(_) => {
                let momo_data = MomoData {};
                Ok(AdyenPaymentMethod::Momo(Box::new(momo_data)))
            }
            api_models::payments::WalletData::TouchNGoRedirect(_) => {
                let touch_n_go_data = TouchNGoData {};
                Ok(AdyenPaymentMethod::TouchNGo(Box::new(touch_n_go_data)))
            }
            api_models::payments::WalletData::MbWayRedirect(data) => {
                let mbway_data = MbwayData {
                    payment_type: PaymentType::Mbway,
                    telephone_number: data.telephone_number.clone(),
                };
                Ok(AdyenPaymentMethod::Mbway(Box::new(mbway_data)))
            }
            api_models::payments::WalletData::MobilePayRedirect(_) => {
                let data = MobilePayData {
                    payment_type: PaymentType::MobilePay,
                };
                Ok(AdyenPaymentMethod::MobilePay(Box::new(data)))
            }
            api_models::payments::WalletData::WeChatPayRedirect(_) => {
                Ok(AdyenPaymentMethod::WeChatPayWeb)
            }
            api_models::payments::WalletData::SamsungPay(samsung_data) => {
                let data = SamsungPayPmData {
                    payment_type: PaymentType::Samsungpay,
                    samsung_pay_token: samsung_data.token.to_owned(),
                };
                Ok(AdyenPaymentMethod::SamsungPay(Box::new(data)))
            }
            api_models::payments::WalletData::TwintRedirect { .. } => Ok(AdyenPaymentMethod::Twint),
            api_models::payments::WalletData::VippsRedirect { .. } => Ok(AdyenPaymentMethod::Vipps),
            api_models::payments::WalletData::DanaRedirect { .. } => Ok(AdyenPaymentMethod::Dana),
            api_models::payments::WalletData::SwishQr(_) => Ok(AdyenPaymentMethod::Swish),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl<'a> TryFrom<(&api::PayLaterData, Option<api_enums::CountryAlpha2>)>
    for AdyenPaymentMethod<'a>
{
    type Error = Error;
    fn try_from(
        value: (&api::PayLaterData, Option<api_enums::CountryAlpha2>),
    ) -> Result<Self, Self::Error> {
        let (pay_later_data, country_code) = value;
        match pay_later_data {
            api_models::payments::PayLaterData::KlarnaRedirect { .. } => {
                let klarna = AdyenPayLaterData {
                    payment_type: PaymentType::Klarna,
                };
                Ok(AdyenPaymentMethod::AdyenKlarna(Box::new(klarna)))
            }
            api_models::payments::PayLaterData::AffirmRedirect { .. } => Ok(
                AdyenPaymentMethod::AdyenAffirm(Box::new(AdyenPayLaterData {
                    payment_type: PaymentType::Affirm,
                })),
            ),
            api_models::payments::PayLaterData::AfterpayClearpayRedirect { .. } => {
                if let Some(country) = country_code {
                    match country {
                        api_enums::CountryAlpha2::IT
                        | api_enums::CountryAlpha2::FR
                        | api_enums::CountryAlpha2::ES
                        | api_enums::CountryAlpha2::GB => Ok(AdyenPaymentMethod::ClearPay),
                        _ => Ok(AdyenPaymentMethod::AfterPay(Box::new(AdyenPayLaterData {
                            payment_type: PaymentType::Afterpaytouch,
                        }))),
                    }
                } else {
                    Err(errors::ConnectorError::MissingRequiredField {
                        field_name: "country",
                    })?
                }
            }
            api_models::payments::PayLaterData::PayBrightRedirect { .. } => {
                Ok(AdyenPaymentMethod::PayBright)
            }
            api_models::payments::PayLaterData::WalleyRedirect { .. } => {
                Ok(AdyenPaymentMethod::Walley)
            }
            api_models::payments::PayLaterData::AlmaRedirect { .. } => Ok(
                AdyenPaymentMethod::AlmaPayLater(Box::new(AdyenPayLaterData {
                    payment_type: PaymentType::Alma,
                })),
            ),
            api_models::payments::PayLaterData::AtomeRedirect { .. } => {
                Ok(AdyenPaymentMethod::Atome)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl<'a> TryFrom<&api_models::payments::BankRedirectData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(
        bank_redirect_data: &api_models::payments::BankRedirectData,
    ) -> Result<Self, Self::Error> {
        match bank_redirect_data {
            api_models::payments::BankRedirectData::BancontactCard {
                card_number,
                card_exp_month,
                card_exp_year,
                card_holder_name,
                ..
            } => Ok(AdyenPaymentMethod::BancontactCard(Box::new(
                BancontactCardData {
                    payment_type: PaymentType::Scheme,
                    brand: "bcmc".to_string(),
                    number: card_number
                        .as_ref()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "bancontact_card.card_number",
                        })?
                        .clone(),
                    expiry_month: card_exp_month
                        .as_ref()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "bancontact_card.card_exp_month",
                        })?
                        .clone(),
                    expiry_year: card_exp_year
                        .as_ref()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "bancontact_card.card_exp_year",
                        })?
                        .clone(),
                    holder_name: card_holder_name
                        .as_ref()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "bancontact_card.card_holder_name",
                        })?
                        .clone(),
                },
            ))),
            api_models::payments::BankRedirectData::Bizum { .. } => {
                Ok(AdyenPaymentMethod::Bizum(Box::new(BankRedirectionPMData {
                    payment_type: PaymentType::Bizum,
                })))
            }
            api_models::payments::BankRedirectData::Blik { blik_code } => {
                Ok(AdyenPaymentMethod::Blik(Box::new(BlikRedirectionData {
                    payment_type: PaymentType::Blik,
                    blik_code: blik_code.to_string(),
                })))
            }
            api_models::payments::BankRedirectData::Eps { bank_name, .. } => Ok(
                AdyenPaymentMethod::Eps(Box::new(BankRedirectionWithIssuer {
                    payment_type: PaymentType::Eps,
                    issuer: bank_name
                        .map(|bank_name| AdyenTestBankNames::try_from(&bank_name))
                        .transpose()?
                        .map(|adyen_bank_name| adyen_bank_name.0),
                })),
            ),
            api_models::payments::BankRedirectData::Giropay { .. } => Ok(
                AdyenPaymentMethod::Giropay(Box::new(BankRedirectionPMData {
                    payment_type: PaymentType::Giropay,
                })),
            ),
            api_models::payments::BankRedirectData::Ideal { bank_name, .. } => Ok(
                AdyenPaymentMethod::Ideal(Box::new(BankRedirectionWithIssuer {
                    payment_type: PaymentType::Ideal,
                    issuer: bank_name
                        .map(|bank_name| AdyenTestBankNames::try_from(&bank_name))
                        .transpose()?
                        .map(|adyen_bank_name| adyen_bank_name.0),
                })),
            ),
            api_models::payments::BankRedirectData::OnlineBankingCzechRepublic { issuer } => {
                Ok(AdyenPaymentMethod::OnlineBankingCzechRepublic(Box::new(
                    OnlineBankingCzechRepublicData {
                        payment_type: PaymentType::OnlineBankingCzechRepublic,
                        issuer: OnlineBankingCzechRepublicBanks::try_from(issuer)?,
                    },
                )))
            }
            api_models::payments::BankRedirectData::OnlineBankingFinland { .. } => Ok(
                AdyenPaymentMethod::OnlineBankingFinland(Box::new(OnlineBankingFinlandData {
                    payment_type: PaymentType::OnlineBankingFinland,
                })),
            ),
            api_models::payments::BankRedirectData::OnlineBankingPoland { issuer } => Ok(
                AdyenPaymentMethod::OnlineBankingPoland(Box::new(OnlineBankingPolandData {
                    payment_type: PaymentType::OnlineBankingPoland,
                    issuer: OnlineBankingPolandBanks::try_from(issuer)?,
                })),
            ),
            api_models::payments::BankRedirectData::OnlineBankingSlovakia { issuer } => Ok(
                AdyenPaymentMethod::OnlineBankingSlovakia(Box::new(OnlineBankingSlovakiaData {
                    payment_type: PaymentType::OnlineBankingSlovakia,
                    issuer: OnlineBankingSlovakiaBanks::try_from(issuer)?,
                })),
            ),
            api_models::payments::BankRedirectData::OnlineBankingFpx { issuer } => Ok(
                AdyenPaymentMethod::OnlineBankingFpx(Box::new(OnlineBankingFpxData {
                    issuer: OnlineBankingFpxIssuer::try_from(issuer)?,
                })),
            ),
            api_models::payments::BankRedirectData::OnlineBankingThailand { issuer } => Ok(
                AdyenPaymentMethod::OnlineBankingThailand(Box::new(OnlineBankingThailandData {
                    issuer: OnlineBankingThailandIssuer::try_from(issuer)?,
                })),
            ),
            api_models::payments::BankRedirectData::Sofort { .. } => Ok(AdyenPaymentMethod::Sofort),
            api_models::payments::BankRedirectData::Trustly { .. } => {
                Ok(AdyenPaymentMethod::Trustly)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl<'a> TryFrom<&api_models::payments::BankTransferData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(
        bank_transfer_data: &api_models::payments::BankTransferData,
    ) -> Result<Self, Self::Error> {
        match bank_transfer_data {
            payments::BankTransferData::PermataBankTransfer {
                ref billing_details,
            } => Ok(AdyenPaymentMethod::PermataBankTransfer(Box::new(
                DokuBankData {
                    first_name: billing_details.first_name.clone(),
                    last_name: billing_details.last_name.clone(),
                    shopper_email: billing_details.email.clone(),
                },
            ))),
            payments::BankTransferData::BcaBankTransfer {
                ref billing_details,
            } => Ok(AdyenPaymentMethod::BcaBankTransfer(Box::new(
                DokuBankData {
                    first_name: billing_details.first_name.clone(),
                    last_name: billing_details.last_name.clone(),
                    shopper_email: billing_details.email.clone(),
                },
            ))),
            payments::BankTransferData::BniVaBankTransfer {
                ref billing_details,
            } => Ok(AdyenPaymentMethod::BniVa(Box::new(DokuBankData {
                first_name: billing_details.first_name.clone(),
                last_name: billing_details.last_name.clone(),
                shopper_email: billing_details.email.clone(),
            }))),
            payments::BankTransferData::BriVaBankTransfer {
                ref billing_details,
            } => Ok(AdyenPaymentMethod::BriVa(Box::new(DokuBankData {
                first_name: billing_details.first_name.clone(),
                last_name: billing_details.last_name.clone(),
                shopper_email: billing_details.email.clone(),
            }))),
            payments::BankTransferData::CimbVaBankTransfer {
                ref billing_details,
            } => Ok(AdyenPaymentMethod::CimbVa(Box::new(DokuBankData {
                first_name: billing_details.first_name.clone(),
                last_name: billing_details.last_name.clone(),
                shopper_email: billing_details.email.clone(),
            }))),
            payments::BankTransferData::DanamonVaBankTransfer {
                ref billing_details,
            } => Ok(AdyenPaymentMethod::DanamonVa(Box::new(DokuBankData {
                first_name: billing_details.first_name.clone(),
                last_name: billing_details.last_name.clone(),
                shopper_email: billing_details.email.clone(),
            }))),
            payments::BankTransferData::MandiriVaBankTransfer {
                ref billing_details,
            } => Ok(AdyenPaymentMethod::MandiriVa(Box::new(DokuBankData {
                first_name: billing_details.first_name.clone(),
                last_name: billing_details.last_name.clone(),
                shopper_email: billing_details.email.clone(),
            }))),
            api_models::payments::BankTransferData::Pix {}
            | api_models::payments::BankTransferData::AchBankTransfer { .. }
            | api_models::payments::BankTransferData::SepaBankTransfer { .. }
            | api_models::payments::BankTransferData::BacsBankTransfer { .. }
            | api_models::payments::BankTransferData::MultibancoBankTransfer { .. }
            | payments::BankTransferData::Pse {} => {
                Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into())
            }
        }
    }
}

impl<'a>
    TryFrom<(
        &types::PaymentsAuthorizeRouterData,
        payments::MandateReferenceId,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (
            &types::PaymentsAuthorizeRouterData,
            payments::MandateReferenceId,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, mandate_ref_id) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item)?;
        let browser_info = get_browser_info(item)?;
        let additional_data = get_additional_data(item);
        let return_url = item.request.get_return_url()?;
        let payment_method_type = item
            .request
            .payment_method_type
            .as_ref()
            .ok_or(errors::ConnectorError::MissingPaymentMethodType)?;
        let payment_method = match mandate_ref_id {
            payments::MandateReferenceId::ConnectorMandateId(connector_mandate_ids) => {
                let adyen_mandate = AdyenMandate {
                    payment_type: PaymentType::try_from(payment_method_type)?,
                    stored_payment_method_id: connector_mandate_ids.get_connector_mandate_id()?,
                };
                Ok::<AdyenPaymentMethod<'_>, Self::Error>(AdyenPaymentMethod::Mandate(Box::new(
                    adyen_mandate,
                )))
            }
            payments::MandateReferenceId::NetworkMandateId(network_mandate_id) => {
                match item.request.payment_method_data {
                    api::PaymentMethodData::Card(ref card) => {
                        let card_issuer = card.get_card_issuer()?;
                        let brand = CardBrand::try_from(&card_issuer)?;
                        let adyen_card = AdyenCard {
                            payment_type: PaymentType::Scheme,
                            number: card.card_number.clone(),
                            expiry_month: card.card_exp_month.clone(),
                            expiry_year: card.card_exp_year.clone(),
                            cvc: None,
                            brand: Some(brand),
                            network_payment_reference: Some(network_mandate_id),
                        };
                        Ok(AdyenPaymentMethod::AdyenCard(Box::new(adyen_card)))
                    }
                    _ => Err(errors::ConnectorError::NotSupported {
                        message: format!("mandate_{:?}", item.payment_method),
                        connector: "Adyen",
                        payment_experience: api_models::enums::PaymentExperience::RedirectToUrl
                            .to_string(),
                    })?,
                }
            }
        }?;
        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            telephone_number: None,
            shopper_name: None,
            shopper_email: None,
            shopper_locale: None,
            social_security_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference,
            store_payment_method,
            channel: None,
        })
    }
}
impl<'a> TryFrom<(&types::PaymentsAuthorizeRouterData, &api::Card)> for AdyenPaymentRequest<'a> {
    type Error = Error;
    fn try_from(
        value: (&types::PaymentsAuthorizeRouterData, &api::Card),
    ) -> Result<Self, Self::Error> {
        let (item, card_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item)?;
        let browser_info = get_browser_info(item)?;
        let additional_data = get_additional_data(item);
        let return_url = item.request.get_return_url()?;
        let payment_method = AdyenPaymentMethod::try_from(card_data)?;
        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            telephone_number: None,
            shopper_name: None,
            shopper_email: None,
            shopper_locale: None,
            social_security_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference,
            store_payment_method,
            channel: None,
        })
    }
}

impl<'a>
    TryFrom<(
        &types::PaymentsAuthorizeRouterData,
        &api_models::payments::BankDebitData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;

    fn try_from(
        value: (
            &types::PaymentsAuthorizeRouterData,
            &api_models::payments::BankDebitData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_debit_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item);
        let recurring_processing_model = get_recurring_processing_model(item)?.0;
        let browser_info = get_browser_info(item)?;
        let additional_data = get_additional_data(item);
        let return_url = item.request.get_return_url()?;
        let payment_method = AdyenPaymentMethod::try_from(bank_debit_data)?;
        let country_code = get_country_code(item.address.billing.as_ref());
        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.connector_request_reference_id.clone(),
            return_url,
            browser_info,
            shopper_interaction,
            recurring_processing_model,
            additional_data,
            shopper_name: None,
            shopper_locale: None,
            shopper_email: item.request.email.clone(),
            social_security_number: None,
            telephone_number: None,
            billing_address: None,
            delivery_address: None,
            country_code,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
        };
        Ok(request)
    }
}

impl<'a>
    TryFrom<(
        &types::PaymentsAuthorizeRouterData,
        &api_models::payments::VoucherData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;

    fn try_from(
        value: (
            &types::PaymentsAuthorizeRouterData,
            &api_models::payments::VoucherData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, voucher_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item);
        let recurring_processing_model = get_recurring_processing_model(item)?.0;
        let browser_info = get_browser_info(item)?;
        let additional_data = get_additional_data(item);
        let payment_method = AdyenPaymentMethod::try_from(voucher_data)?;
        let return_url = item.request.get_return_url()?;
        let social_security_number = get_social_security_number(voucher_data);
        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.payment_id.to_string(),
            return_url,
            browser_info,
            shopper_interaction,
            recurring_processing_model,
            additional_data,
            shopper_name: None,
            shopper_locale: None,
            shopper_email: item.request.email.clone(),
            social_security_number,
            telephone_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
        };
        Ok(request)
    }
}

impl<'a>
    TryFrom<(
        &types::PaymentsAuthorizeRouterData,
        &api_models::payments::BankTransferData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;

    fn try_from(
        value: (
            &types::PaymentsAuthorizeRouterData,
            &api_models::payments::BankTransferData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_transfer_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item);
        let payment_method = AdyenPaymentMethod::try_from(bank_transfer_data)?;
        let return_url = item.request.get_return_url()?;
        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.payment_id.to_string(),
            return_url,
            browser_info: None,
            shopper_interaction,
            recurring_processing_model: None,
            additional_data: None,
            shopper_name: None,
            shopper_locale: None,
            shopper_email: item.request.email.clone(),
            social_security_number: None,
            telephone_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
        };
        Ok(request)
    }
}

impl<'a>
    TryFrom<(
        &types::PaymentsAuthorizeRouterData,
        &api_models::payments::GiftCardData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;

    fn try_from(
        value: (
            &types::PaymentsAuthorizeRouterData,
            &api_models::payments::GiftCardData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, gift_card_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item);
        let return_url = item.request.get_router_return_url()?;
        let payment_method = AdyenPaymentMethod::try_from(gift_card_data)?;
        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.payment_id.to_string(),
            return_url,
            browser_info: None,
            shopper_interaction,
            recurring_processing_model: None,
            additional_data: None,
            shopper_name: None,
            shopper_locale: None,
            shopper_email: item.request.email.clone(),
            telephone_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            social_security_number: None,
        };
        Ok(request)
    }
}

impl<'a>
    TryFrom<(
        &types::PaymentsAuthorizeRouterData,
        &api_models::payments::BankRedirectData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (
            &types::PaymentsAuthorizeRouterData,
            &api_models::payments::BankRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_redirect_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item)?;
        let browser_info = get_browser_info(item)?;
        let additional_data = get_additional_data(item);
        let return_url = item.request.get_return_url()?;
        let payment_method = AdyenPaymentMethod::try_from(bank_redirect_data)?;
        let (shopper_locale, country) = get_sofort_extra_details(item);
        let line_items = Some(get_line_items(item));

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            telephone_number: None,
            shopper_name: None,
            shopper_email: item.request.email.clone(),
            shopper_locale,
            social_security_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: country,
            line_items,
            shopper_reference,
            store_payment_method,
            channel: None,
        })
    }
}

fn get_sofort_extra_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> (Option<String>, Option<api_enums::CountryAlpha2>) {
    match item.request.payment_method_data {
        api_models::payments::PaymentMethodData::BankRedirect(ref b) => {
            if let api_models::payments::BankRedirectData::Sofort {
                country,
                preferred_language,
                ..
            } = b
            {
                (
                    Some(preferred_language.to_string()),
                    Some(country.to_owned()),
                )
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    }
}

fn get_shopper_email(
    item: &PaymentsAuthorizeData,
    is_mandate_payment: bool,
) -> errors::CustomResult<Option<Email>, errors::ConnectorError> {
    if is_mandate_payment {
        let payment_method_type = item
            .payment_method_type
            .as_ref()
            .ok_or(errors::ConnectorError::MissingPaymentMethodType)?;
        match payment_method_type {
            storage_enums::PaymentMethodType::Paypal => Ok(Some(item.get_email()?)),
            _ => Ok(item.email.clone()),
        }
    } else {
        Ok(item.email.clone())
    }
}

impl<'a> TryFrom<(&types::PaymentsAuthorizeRouterData, &api::WalletData)>
    for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (&types::PaymentsAuthorizeRouterData, &api::WalletData),
    ) -> Result<Self, Self::Error> {
        let (item, wallet_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let browser_info = get_browser_info(item)?;
        let additional_data = get_additional_data(item);
        let payment_method = AdyenPaymentMethod::try_from(wallet_data)?;
        let shopper_interaction = AdyenShopperInteraction::from(item);
        let channel = get_channel_type(&item.request.payment_method_type);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item)?;
        let return_url = item.request.get_router_return_url()?;
        let shopper_email = get_shopper_email(&item.request, store_payment_method.is_some())?;
        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            telephone_number: None,
            shopper_name: None,
            shopper_email,
            shopper_locale: None,
            social_security_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference,
            store_payment_method,
            channel,
        })
    }
}

impl<'a> TryFrom<(&types::PaymentsAuthorizeRouterData, &api::PayLaterData)>
    for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (&types::PaymentsAuthorizeRouterData, &api::PayLaterData),
    ) -> Result<Self, Self::Error> {
        let (item, paylater_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let browser_info = get_browser_info(item)?;
        let additional_data = get_additional_data(item);
        let country_code = get_country_code(item.address.billing.as_ref());
        let payment_method = AdyenPaymentMethod::try_from((paylater_data, country_code))?;
        let shopper_interaction = AdyenShopperInteraction::from(item);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item)?;
        let return_url = item.request.get_return_url()?;
        let shopper_name: Option<ShopperName> = get_shopper_name(item.address.billing.as_ref());
        let shopper_email = item.request.email.clone();
        let billing_address = get_address_info(item.address.billing.as_ref());
        let delivery_address = get_address_info(item.address.shipping.as_ref());
        let line_items = Some(get_line_items(item));
        let telephone_number = get_telephone_number(item);
        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            telephone_number,
            shopper_name,
            shopper_email,
            shopper_locale: None,
            social_security_number: None,
            billing_address,
            delivery_address,
            country_code,
            line_items,
            shopper_reference,
            store_payment_method,
            channel: None,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for AdyenCancelRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            reference: item.connector_request_reference_id.clone(),
        })
    }
}

impl From<CancelStatus> for storage_enums::AttemptStatus {
    fn from(status: CancelStatus) -> Self {
        match status {
            CancelStatus::Received => Self::Voided,
            CancelStatus::Processing => Self::Pending,
        }
    }
}

impl TryFrom<types::PaymentsCancelResponseRouterData<AdyenCancelResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = Error;
    fn try_from(
        item: types::PaymentsCancelResponseRouterData<AdyenCancelResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: item.response.status.into(),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.psp_reference),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

pub fn get_adyen_response(
    response: Response,
    is_capture_manual: bool,
    status_code: u16,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status =
        storage_enums::AttemptStatus::foreign_from((is_capture_manual, response.result_code));
    let error = if response.refusal_reason.is_some() || response.refusal_reason_code.is_some() {
        Some(types::ErrorResponse {
            code: response
                .refusal_reason_code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.refusal_reason,
            status_code,
        })
    } else {
        None
    };
    let mandate_reference = response
        .additional_data
        .as_ref()
        .and_then(|data| data.recurring_detail_reference.to_owned())
        .map(|mandate_id| types::MandateReference {
            connector_mandate_id: Some(mandate_id),
            payment_method_id: None,
        });
    let network_txn_id = response
        .additional_data
        .and_then(|additional_data| additional_data.network_tx_reference);

    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(response.psp_reference),
        redirection_data: None,
        mandate_reference,
        connector_metadata: None,
        network_txn_id,
        connector_response_reference_id: Some(response.merchant_reference),
    };
    Ok((status, error, payments_response_data))
}

pub fn get_redirection_response(
    response: RedirectionResponse,
    is_manual_capture: bool,
    status_code: u16,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = storage_enums::AttemptStatus::foreign_from((
        is_manual_capture,
        response.result_code.clone(),
    ));
    let error = if response.refusal_reason.is_some() || response.refusal_reason_code.is_some() {
        Some(types::ErrorResponse {
            code: response
                .refusal_reason_code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
            status_code,
        })
    } else {
        None
    };

    let redirection_data = response.action.url.clone().map(|url| {
        let form_fields = response.action.data.clone().unwrap_or_else(|| {
            std::collections::HashMap::from_iter(
                url.query_pairs()
                    .map(|(key, value)| (key.to_string(), value.to_string())),
            )
        });
        services::RedirectForm::Form {
            endpoint: url.to_string(),
            method: response.action.method.unwrap_or(services::Method::Get),
            form_fields,
        }
    });

    let connector_metadata = get_wait_screen_metadata(&response)?;

    // We don't get connector transaction id for redirections in Adyen.
    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::NoResponseId,
        redirection_data,
        mandate_reference: None,
        connector_metadata,
        network_txn_id: None,
        connector_response_reference_id: None,
    };
    Ok((status, error, payments_response_data))
}

pub fn get_present_to_shopper_response(
    response: PresentToShopperResponse,
    is_manual_capture: bool,
    status_code: u16,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = storage_enums::AttemptStatus::foreign_from((
        is_manual_capture,
        response.result_code.clone(),
    ));
    let error = if response.refusal_reason.is_some() || response.refusal_reason_code.is_some() {
        Some(types::ErrorResponse {
            code: response
                .refusal_reason_code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
            status_code,
        })
    } else {
        None
    };

    let connector_metadata = get_present_to_shopper_metadata(&response)?;
    // We don't get connector transaction id for redirections in Adyen.
    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: match response.psp_reference.as_ref() {
            Some(psp) => types::ResponseId::ConnectorTransactionId(psp.to_string()),
            None => types::ResponseId::NoResponseId,
        },
        redirection_data: None,
        mandate_reference: None,
        connector_metadata,
        network_txn_id: None,
        connector_response_reference_id: None,
    };
    Ok((status, error, payments_response_data))
}

pub fn get_qr_code_response(
    response: QrCodeResponseResponse,
    is_manual_capture: bool,
    status_code: u16,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = storage_enums::AttemptStatus::foreign_from((
        is_manual_capture,
        response.result_code.clone(),
    ));
    let error = if response.refusal_reason.is_some() || response.refusal_reason_code.is_some() {
        Some(types::ErrorResponse {
            code: response
                .refusal_reason_code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
            status_code,
        })
    } else {
        None
    };

    let connector_metadata = get_qr_metadata(&response)?;
    // We don't get connector transaction id for redirections in Adyen.
    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::NoResponseId,
        redirection_data: None,
        mandate_reference: None,
        connector_metadata,
        network_txn_id: None,
        connector_response_reference_id: None,
    };
    Ok((status, error, payments_response_data))
}

pub fn get_redirection_error_response(
    response: RedirectionErrorResponse,
    is_manual_capture: bool,
    status_code: u16,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status =
        storage_enums::AttemptStatus::foreign_from((is_manual_capture, response.result_code));
    let error = Some(types::ErrorResponse {
        code: status.to_string(),
        message: response.refusal_reason.clone(),
        reason: Some(response.refusal_reason),
        status_code,
    });
    // We don't get connector transaction id for redirections in Adyen.
    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::NoResponseId,
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: None,
    };

    Ok((status, error, payments_response_data))
}

pub fn get_qr_metadata(
    response: &QrCodeResponseResponse,
) -> errors::CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let image_data = crate_utils::QrImage::new_from_data(response.action.qr_code_data.to_owned())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.as_str())
        .ok()
        .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

    let qr_code_instructions = payments::QrCodeNextStepsInstruction {
        image_data_url,
        display_to_timestamp: None,
    };

    Some(common_utils::ext_traits::Encode::<
        payments::QrCodeNextStepsInstruction,
    >::encode_to_value(&qr_code_instructions))
    .transpose()
    .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitScreenData {
    display_from_timestamp: i128,
    display_to_timestamp: Option<i128>,
}

pub fn get_wait_screen_metadata(
    next_action: &RedirectionResponse,
) -> errors::CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    match next_action.action.payment_method_type {
        PaymentType::Blik => {
            let current_time = OffsetDateTime::now_utc().unix_timestamp_nanos();
            Ok(Some(serde_json::json!(WaitScreenData {
                display_from_timestamp: current_time,
                display_to_timestamp: Some(current_time + Duration::minutes(1).whole_nanoseconds())
            })))
        }
        PaymentType::Mbway => {
            let current_time = OffsetDateTime::now_utc().unix_timestamp_nanos();
            Ok(Some(serde_json::json!(WaitScreenData {
                display_from_timestamp: current_time,
                display_to_timestamp: None
            })))
        }
        PaymentType::Affirm
        | PaymentType::Afterpaytouch
        | PaymentType::Alipay
        | PaymentType::AlipayHk
        | PaymentType::Alfamart
        | PaymentType::Alma
        | PaymentType::Applepay
        | PaymentType::Bizum
        | PaymentType::Atome
        | PaymentType::BoletoBancario
        | PaymentType::ClearPay
        | PaymentType::Dana
        | PaymentType::Eps
        | PaymentType::Gcash
        | PaymentType::Giropay
        | PaymentType::Googlepay
        | PaymentType::GoPay
        | PaymentType::Ideal
        | PaymentType::Indomaret
        | PaymentType::Klarna
        | PaymentType::Kakaopay
        | PaymentType::MobilePay
        | PaymentType::Momo
        | PaymentType::OnlineBankingCzechRepublic
        | PaymentType::OnlineBankingFinland
        | PaymentType::OnlineBankingPoland
        | PaymentType::OnlineBankingSlovakia
        | PaymentType::OnlineBankingFpx
        | PaymentType::OnlineBankingThailand
        | PaymentType::PayBright
        | PaymentType::Paypal
        | PaymentType::Scheme
        | PaymentType::Sofort
        | PaymentType::NetworkToken
        | PaymentType::Trustly
        | PaymentType::TouchNGo
        | PaymentType::Walley
        | PaymentType::WeChatPayWeb
        | PaymentType::AchDirectDebit
        | PaymentType::SepaDirectDebit
        | PaymentType::BacsDirectDebit
        | PaymentType::Samsungpay
        | PaymentType::Twint
        | PaymentType::Vipps
        | PaymentType::Swish
        | PaymentType::PermataBankTransfer
        | PaymentType::BcaBankTransfer
        | PaymentType::BniVa
        | PaymentType::BriVa
        | PaymentType::CimbVa
        | PaymentType::DanamonVa
        | PaymentType::MandiriVa
        | PaymentType::PaySafeCard => Err(errors::ConnectorError::ResponseHandlingFailed.into()),
    }
}

pub fn get_present_to_shopper_metadata(
    response: &PresentToShopperResponse,
) -> errors::CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let reference = response.action.reference.clone();

    match response.action.payment_method_type {
        PaymentType::Alfamart | PaymentType::Indomaret | PaymentType::BoletoBancario => {
            let voucher_data = payments::VoucherNextStepData {
                expires_at: response.action.expires_at.clone(),
                reference,
                download_url: response.action.download_url.clone(),
            };

            Some(common_utils::ext_traits::Encode::<
                payments::VoucherNextStepData,
            >::encode_to_value(&voucher_data))
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
        }
        PaymentType::PermataBankTransfer
        | PaymentType::BcaBankTransfer
        | PaymentType::BniVa
        | PaymentType::BriVa
        | PaymentType::CimbVa
        | PaymentType::DanamonVa
        | PaymentType::MandiriVa => {
            let voucher_data = payments::BankTransferInstructions::DokuBankTransferInstructions(
                Box::new(payments::DokuBankTransferInstructions {
                    reference: Secret::new(response.action.reference.clone()),
                    instructions_url: response.action.instructions_url.clone(),
                    expires_at: response.action.expires_at.clone(),
                }),
            );

            Some(common_utils::ext_traits::Encode::<
                payments::DokuBankTransferInstructions,
            >::encode_to_value(&voucher_data))
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
        }
        PaymentType::Affirm
        | PaymentType::Afterpaytouch
        | PaymentType::Alipay
        | PaymentType::AlipayHk
        | PaymentType::Alma
        | PaymentType::Applepay
        | PaymentType::Bizum
        | PaymentType::Atome
        | PaymentType::Blik
        | PaymentType::ClearPay
        | PaymentType::Dana
        | PaymentType::Eps
        | PaymentType::Gcash
        | PaymentType::Giropay
        | PaymentType::Googlepay
        | PaymentType::GoPay
        | PaymentType::Ideal
        | PaymentType::Klarna
        | PaymentType::Kakaopay
        | PaymentType::Mbway
        | PaymentType::MobilePay
        | PaymentType::Momo
        | PaymentType::OnlineBankingCzechRepublic
        | PaymentType::OnlineBankingFinland
        | PaymentType::OnlineBankingPoland
        | PaymentType::OnlineBankingSlovakia
        | PaymentType::OnlineBankingFpx
        | PaymentType::OnlineBankingThailand
        | PaymentType::PayBright
        | PaymentType::Paypal
        | PaymentType::Scheme
        | PaymentType::Sofort
        | PaymentType::NetworkToken
        | PaymentType::Trustly
        | PaymentType::TouchNGo
        | PaymentType::Walley
        | PaymentType::WeChatPayWeb
        | PaymentType::AchDirectDebit
        | PaymentType::SepaDirectDebit
        | PaymentType::BacsDirectDebit
        | PaymentType::Samsungpay
        | PaymentType::Twint
        | PaymentType::Vipps
        | PaymentType::Swish
        | PaymentType::PaySafeCard => Err(errors::ConnectorError::ResponseHandlingFailed.into()),
    }
}

impl<F, Req>
    TryFrom<(
        types::ResponseRouterData<F, AdyenPaymentResponse, Req, types::PaymentsResponseData>,
        bool,
    )> for types::RouterData<F, Req, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        items: (
            types::ResponseRouterData<F, AdyenPaymentResponse, Req, types::PaymentsResponseData>,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let item = items.0;
        let is_manual_capture = items.1;
        let (status, error, payment_response_data) = match item.response {
            AdyenPaymentResponse::Response(response) => {
                get_adyen_response(*response, is_manual_capture, item.http_code)?
            }
            AdyenPaymentResponse::PresentToShopper(response) => {
                get_present_to_shopper_response(*response, is_manual_capture, item.http_code)?
            }
            AdyenPaymentResponse::QrCodeResponse(response) => {
                get_qr_code_response(*response, is_manual_capture, item.http_code)?
            }
            AdyenPaymentResponse::RedirectionResponse(response) => {
                get_redirection_response(*response, is_manual_capture, item.http_code)?
            }
            AdyenPaymentResponse::RedirectionErrorResponse(response) => {
                get_redirection_error_response(*response, is_manual_capture, item.http_code)?
            }
        };

        Ok(Self {
            status,
            response: error.map_or_else(|| Ok(payment_response_data), Err),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCaptureRequest {
    merchant_account: Secret<String>,
    amount: Amount,
    reference: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for AdyenCaptureRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            reference: item.connector_request_reference_id.clone(),
            amount: Amount {
                currency: item.request.currency.to_string(),
                value: item.request.amount_to_capture,
            },
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCaptureResponse {
    merchant_account: String,
    payment_psp_reference: String,
    psp_reference: String,
    reference: String,
    status: String,
    amount: Amount,
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<AdyenCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = Error;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<AdyenCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // From the docs, the only value returned is "received", outcome of refund is available
            // through refund notification webhook
            // For more info: https://docs.adyen.com/online-payments/capture
            status: storage_enums::AttemptStatus::Pending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.psp_reference),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            amount_captured: Some(item.response.amount.value),
            ..item.data
        })
    }
}

/*
// This is a repeated code block from Stripe inegration. Can we avoid the repetition in every integration
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdyenPaymentStatus {
    Succeeded,
    Failed,
    Processing,
    RequiresCustomerAction,
    RequiresPaymentMethod,
    RequiresConfirmation,
}

// Default always be Processing
impl Default for AdyenPaymentStatus {
    fn default() -> Self {
        AdyenPaymentStatus::Processing
    }
}

impl From<AdyenPaymentStatus> for enums::Status {
    fn from(item: AdyenPaymentStatus) -> Self {
        match item {
            AdyenPaymentStatus::Succeeded => enums::Status::Charged,
            AdyenPaymentStatus::Failed => enums::Status::Failure,
            AdyenPaymentStatus::Processing
            | AdyenPaymentStatus::RequiresCustomerAction
            | AdyenPaymentStatus::RequiresPaymentMethod
            | AdyenPaymentStatus::RequiresConfirmation => enums::Status::Pending,
        }
    }
}
*/
// Refund Request Transform
impl<F> TryFrom<&types::RefundsRouterData<F>> for AdyenRefundRequest {
    type Error = Error;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            amount: Amount {
                currency: item.request.currency.to_string(),
                value: item.request.refund_amount,
            },
            merchant_refund_reason: item.request.reason.clone(),
            reference: item.request.refund_id.clone(),
        })
    }
}

// Refund Response Transform
impl<F> TryFrom<types::RefundsResponseRouterData<F, AdyenRefundResponse>>
    for types::RefundsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<F, AdyenRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.reference,
                // From the docs, the only value returned is "received", outcome of refund is available
                // through refund notification webhook
                // For more info: https://docs.adyen.com/online-payments/refund
                refund_status: storage_enums::RefundStatus::Pending,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub status: i32,
    pub error_code: String,
    pub message: String,
    pub error_type: String,
    pub psp_reference: Option<String>,
}

// #[cfg(test)]
// mod test_adyen_transformers {
//     use super::*;

//     #[test]
//     fn verify_transform_from_router_to_adyen_req() {
//         let router_req = PaymentsRequest {
//             amount: 0.0,
//             currency: "None".to_string(),
//             ..Default::default()
//         };
//         println!("{:#?}", &router_req);
//         let adyen_req = AdyenPaymentRequest::from(router_req);
//         println!("{:#?}", &adyen_req);
//         let adyen_req_json: String = serde_json::to_string(&adyen_req).unwrap();
//         println!("{}", adyen_req_json);
//         assert_eq!(true, true)
//     }
// }

#[derive(Debug, Deserialize)]
pub enum DisputeStatus {
    Undefended,
    Pending,
    Lost,
    Accepted,
    Won,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenAdditionalDataWH {
    pub hmac_signature: String,
    pub dispute_status: Option<DisputeStatus>,
    pub chargeback_reason_code: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub defense_period_ends_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct AdyenAmountWH {
    pub value: i64,
    pub currency: String,
}

#[derive(Clone, Debug, Deserialize, strum::Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum WebhookEventCode {
    Authorisation,
    Refund,
    CancelOrRefund,
    RefundFailed,
    NotificationOfChargeback,
    Chargeback,
    ChargebackReversed,
    SecondChargeback,
    PrearbitrationWon,
    PrearbitrationLost,
    #[serde(other)]
    Unknown,
}

pub fn is_transaction_event(event_code: &WebhookEventCode) -> bool {
    matches!(event_code, WebhookEventCode::Authorisation)
}

pub fn is_refund_event(event_code: &WebhookEventCode) -> bool {
    matches!(
        event_code,
        WebhookEventCode::Refund
            | WebhookEventCode::CancelOrRefund
            | WebhookEventCode::RefundFailed
    )
}

pub fn is_chargeback_event(event_code: &WebhookEventCode) -> bool {
    matches!(
        event_code,
        WebhookEventCode::NotificationOfChargeback
            | WebhookEventCode::Chargeback
            | WebhookEventCode::ChargebackReversed
            | WebhookEventCode::SecondChargeback
            | WebhookEventCode::PrearbitrationWon
            | WebhookEventCode::PrearbitrationLost
    )
}

impl ForeignFrom<(WebhookEventCode, Option<DisputeStatus>)> for webhooks::IncomingWebhookEvent {
    fn foreign_from((code, status): (WebhookEventCode, Option<DisputeStatus>)) -> Self {
        match (code, status) {
            (WebhookEventCode::Authorisation, _) => Self::PaymentIntentSuccess,
            (WebhookEventCode::Refund, _) => Self::RefundSuccess,
            (WebhookEventCode::CancelOrRefund, _) => Self::RefundSuccess,
            (WebhookEventCode::RefundFailed, _) => Self::RefundFailure,
            (WebhookEventCode::NotificationOfChargeback, _) => Self::DisputeOpened,
            (WebhookEventCode::Chargeback, None) => Self::DisputeLost,
            (WebhookEventCode::Chargeback, Some(DisputeStatus::Won)) => Self::DisputeWon,
            (WebhookEventCode::Chargeback, Some(DisputeStatus::Lost)) => Self::DisputeLost,
            (WebhookEventCode::Chargeback, Some(_)) => Self::DisputeOpened,
            (WebhookEventCode::ChargebackReversed, Some(DisputeStatus::Pending)) => {
                Self::DisputeChallenged
            }
            (WebhookEventCode::ChargebackReversed, _) => Self::DisputeWon,
            (WebhookEventCode::SecondChargeback, _) => Self::DisputeLost,
            (WebhookEventCode::PrearbitrationWon, Some(DisputeStatus::Pending)) => {
                Self::DisputeOpened
            }
            (WebhookEventCode::PrearbitrationWon, _) => Self::DisputeWon,
            (WebhookEventCode::PrearbitrationLost, _) => Self::DisputeLost,
            (WebhookEventCode::Unknown, _) => Self::EventNotSupported,
        }
    }
}

impl From<WebhookEventCode> for enums::DisputeStage {
    fn from(code: WebhookEventCode) -> Self {
        match code {
            WebhookEventCode::NotificationOfChargeback => Self::PreDispute,
            WebhookEventCode::SecondChargeback => Self::PreArbitration,
            WebhookEventCode::PrearbitrationWon => Self::PreArbitration,
            WebhookEventCode::PrearbitrationLost => Self::PreArbitration,
            _ => Self::Dispute,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenNotificationRequestItemWH {
    pub additional_data: AdyenAdditionalDataWH,
    pub amount: AdyenAmountWH,
    pub original_reference: Option<String>,
    pub psp_reference: String,
    pub event_code: WebhookEventCode,
    pub merchant_account_code: String,
    pub merchant_reference: String,
    pub success: String,
    pub reason: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub event_date: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdyenItemObjectWH {
    pub notification_request_item: AdyenNotificationRequestItemWH,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenIncomingWebhook {
    pub notification_items: Vec<AdyenItemObjectWH>,
}

impl From<AdyenNotificationRequestItemWH> for Response {
    fn from(notif: AdyenNotificationRequestItemWH) -> Self {
        Self {
            psp_reference: notif.psp_reference,
            merchant_reference: notif.merchant_reference,
            result_code: match notif.success.as_str() {
                "true" => AdyenStatus::Authorised,
                _ => AdyenStatus::Refused,
            },
            amount: Some(Amount {
                value: notif.amount.value,
                currency: notif.amount.currency,
            }),
            refusal_reason: None,
            refusal_reason_code: None,
            additional_data: None,
        }
    }
}

// Payouts
#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPayoutCreateRequest {
    amount: Amount,
    recurring: RecurringContract,
    merchant_account: Secret<String>,
    bank: PayoutBankDetails,
    reference: String,
    shopper_reference: String,
    shopper_email: Option<Email>,
    shopper_name: ShopperName,
    date_of_birth: Option<Secret<String>>,
    entity_type: Option<storage_enums::PayoutEntityType>,
    nationality: Option<storage_enums::CountryAlpha2>,
    billing_address: Option<Address>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayoutBankDetails {
    bank_name: String,
    bic: Option<Secret<String>>,
    country_code: storage_enums::CountryAlpha2,
    iban: Option<Secret<String>>,
    owner_name: Option<Secret<String>>,
    bank_city: String,
    tax_id: Option<Secret<String>>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecurringContract {
    contract: Contract,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum Contract {
    Oneclick,
    Recurring,
    Payout,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPayoutResponse {
    psp_reference: String,
    result_code: Option<AdyenStatus>,
    response: Option<AdyenStatus>,
    amount: Option<Amount>,
    merchant_reference: Option<String>,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    additional_data: Option<AdditionalData>,
    auth_code: Option<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPayoutEligibilityRequest {
    amount: Amount,
    merchant_account: Secret<String>,
    payment_method: PayoutCardDetails,
    reference: String,
    shopper_reference: String,
}

#[cfg(feature = "payouts")]
#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayoutCardDetails {
    #[serde(rename = "type")]
    _type: String,
    number: String,
    expiry_month: String,
    expiry_year: String,
    holder_name: String,
}

#[cfg(feature = "payouts")]
#[derive(Clone, Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum PayoutEligibility {
    #[serde(rename = "Y")]
    Yes,
    #[serde(rename = "N")]
    #[default]
    No,
    #[serde(rename = "D")]
    Domestic,
    #[serde(rename = "U")]
    Unknown,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdyenPayoutFulfillRequest {
    Bank(PayoutFulfillBankRequest),
    Card(Box<PayoutFulfillCardRequest>),
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutFulfillBankRequest {
    merchant_account: Secret<String>,
    original_reference: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutFulfillCardRequest {
    amount: Amount,
    card: PayoutCardDetails,
    billing_address: Option<Address>,
    merchant_account: Secret<String>,
    reference: String,
    shopper_name: ShopperName,
    nationality: Option<storage_enums::CountryAlpha2>,
    entity_type: Option<storage_enums::PayoutEntityType>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPayoutCancelRequest {
    original_reference: String,
    merchant_account: Secret<String>,
}

// Payouts eligibility request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for AdyenPayoutEligibilityRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let payout_method_data = get_payout_card_details(&item.get_payout_method_data()?).map_or(
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payout_method_data",
            }),
            Ok,
        )?;
        Ok(Self {
            amount: Amount {
                currency: item.request.destination_currency.to_string(),
                value: item.request.amount,
            },
            merchant_account: auth_type.merchant_account,
            payment_method: payout_method_data,
            reference: item.request.payout_id.clone(),
            shopper_reference: item.merchant_id.clone(),
        })
    }
}

// Payouts create request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for AdyenPayoutCancelRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;

        let merchant_account = auth_type.merchant_account;
        if let Some(id) = &item.request.connector_payout_id {
            Ok(Self {
                merchant_account,
                original_reference: id.to_string(),
            })
        } else {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_payout_id",
            })?
        }
    }
}

// Payouts cancel request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for AdyenPayoutCreateRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let merchant_account = auth_type.merchant_account;
        let (owner_name, customer_email) = item
            .request
            .customer_details
            .to_owned()
            .map_or((None, None), |c| (c.name, c.email));

        match item.get_payout_method_data()? {
            PayoutMethodData::Card(_) => Err(errors::ConnectorError::NotSupported {
                message: "Card payout creation is not supported".to_string(),
                connector: "Adyen",
                payment_experience: "".to_string(),
            })?,
            PayoutMethodData::Bank(bd) => {
                let bank_details = match bd {
                    payouts::BankPayout::Sepa(b) => PayoutBankDetails {
                        bank_name: b.bank_name,
                        country_code: b.bank_country_code,
                        bank_city: b.bank_city,
                        owner_name,
                        bic: b.bic,
                        iban: Some(b.iban),
                        tax_id: None,
                    },
                    _ => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfers via ACH or Bacs are not supported".to_string(),
                        connector: "Adyen",
                        payment_experience: "".to_string(),
                    })?,
                };
                let address: &payments::AddressDetails = item.get_billing_address()?;
                Ok(Self {
                    amount: Amount {
                        value: item.request.amount,
                        currency: item.request.destination_currency.to_string(),
                    },
                    recurring: RecurringContract {
                        contract: Contract::Payout,
                    },
                    merchant_account,
                    bank: bank_details,
                    reference: item.request.payout_id.to_owned(),
                    shopper_reference: item.merchant_id.to_owned(),
                    shopper_email: customer_email,
                    shopper_name: ShopperName {
                        first_name: address.get_first_name().ok().cloned(),
                        last_name: address.get_last_name().ok().cloned(),
                    },
                    date_of_birth: None,
                    entity_type: Some(item.request.entity_type),
                    nationality: get_country_code(item.address.billing.as_ref()),
                    billing_address: get_address_info(item.address.billing.as_ref()),
                })
            }
        }
    }
}

// Payouts fulfill request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for AdyenPayoutFulfillRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let payout_type = item.request.payout_type.to_owned();
        let merchant_account = auth_type.merchant_account;
        match payout_type {
            storage_enums::PayoutType::Bank => Ok(Self::Bank(PayoutFulfillBankRequest {
                merchant_account,
                original_reference: item
                    .request
                    .connector_payout_id
                    .clone()
                    .unwrap_or("".to_string()),
            })),
            storage_enums::PayoutType::Card => {
                let address = item.get_billing_address()?;
                Ok(Self::Card(Box::new(PayoutFulfillCardRequest {
                    amount: Amount {
                        value: item.request.amount,
                        currency: item.request.destination_currency.to_string(),
                    },
                    card: get_payout_card_details(&item.get_payout_method_data()?).map_or(
                        Err(errors::ConnectorError::MissingRequiredField {
                            field_name: "payout_method_data",
                        }),
                        Ok,
                    )?,
                    billing_address: get_address_info(item.get_billing().ok()),
                    merchant_account,
                    reference: item.request.payout_id.clone(),
                    shopper_name: ShopperName {
                        first_name: address.get_first_name().ok().cloned(),
                        last_name: address.get_last_name().ok().cloned(),
                    },
                    nationality: get_country_code(item.address.billing.as_ref()),
                    entity_type: Some(item.request.entity_type),
                })))
            }
        }
    }
}

// Payouts response transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, AdyenPayoutResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, AdyenPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        let response: AdyenPayoutResponse = item.response;
        let payout_eligible = response
            .additional_data
            .and_then(|pa| pa.payout_eligible)
            .map(|pe| pe == PayoutEligibility::Yes || pe == PayoutEligibility::Domestic);

        let status = payout_eligible.map_or(
            {
                response.result_code.map_or(
                    response
                        .response
                        .map(storage_enums::PayoutStatus::foreign_from),
                    |rc| Some(storage_enums::PayoutStatus::foreign_from(rc)),
                )
            },
            |pe| {
                if pe {
                    Some(storage_enums::PayoutStatus::RequiresFulfillment)
                } else {
                    Some(storage_enums::PayoutStatus::Ineligible)
                }
            },
        );

        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status,
                connector_payout_id: response.psp_reference,
                payout_eligible,
            }),
            ..item.data
        })
    }
}

#[cfg(feature = "payouts")]
impl ForeignFrom<AdyenStatus> for storage_enums::PayoutStatus {
    fn foreign_from(adyen_status: AdyenStatus) -> Self {
        match adyen_status {
            AdyenStatus::Authorised | AdyenStatus::PayoutConfirmReceived => Self::Success,
            AdyenStatus::Cancelled | AdyenStatus::PayoutDeclineReceived => Self::Cancelled,
            AdyenStatus::Error => Self::Failed,
            AdyenStatus::Pending => Self::Pending,
            AdyenStatus::PayoutSubmitReceived => Self::RequiresFulfillment,
            _ => Self::Ineligible,
        }
    }
}
