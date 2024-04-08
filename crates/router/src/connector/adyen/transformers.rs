#[cfg(feature = "payouts")]
use api_models::payouts::PayoutMethodData;
use api_models::{enums, payments, webhooks};
use cards::CardNumber;
use common_utils::{ext_traits::Encode, pii};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use crate::{
    connector::utils::{
        self, AddressDetailsData, BrowserInformationData, CardData, MandateReferenceData,
        PaymentsAuthorizeRequestData, RouterData,
    },
    consts,
    core::errors,
    pii::{Email, Secret},
    services,
    types::{
        self,
        api::{self, enums as api_enums},
        domain,
        storage::enums as storage_enums,
        transformers::{ForeignFrom, ForeignTryFrom},
        PaymentsAuthorizeData,
    },
    utils as crate_utils,
};
#[cfg(feature = "payouts")]
use crate::{types::api::payouts, utils::OptionExt};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct AdyenRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for AdyenRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AdyenConnectorMetadataObject {
    pub endpoint_prefix: Option<String>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for AdyenConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

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
    manual_capture: Option<String>,
    execute_three_d: Option<String>,
    pub recurring_processing_model: Option<AdyenRecurringModel>,
    /// Enable recurring details in dashboard to receive this ID, https://docs.adyen.com/online-payments/tokenization/create-and-use-tokens#test-and-go-live
    #[serde(rename = "recurring.recurringDetailReference")]
    recurring_detail_reference: Option<Secret<String>>,
    #[serde(rename = "recurring.shopperReference")]
    recurring_shopper_reference: Option<String>,
    network_tx_reference: Option<Secret<String>>,
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
    city: String,
    country: api_enums::CountryAlpha2,
    house_number_or_name: Secret<String>,
    postal_code: Secret<String>,
    state_or_province: Option<Secret<String>>,
    street: Secret<String>,
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
    #[serde(rename = "shopperIP")]
    shopper_ip: Option<Secret<String, pii::IpAddress>>,
    shopper_locale: Option<String>,
    shopper_email: Option<Email>,
    shopper_statement: Option<String>,
    social_security_number: Option<Secret<String>>,
    telephone_number: Option<Secret<String>>,
    billing_address: Option<Address>,
    delivery_address: Option<Address>,
    country_code: Option<api_enums::CountryAlpha2>,
    line_items: Option<Vec<LineItem>>,
    channel: Option<Channel>,
    metadata: Option<pii::SecretSerdeValue>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenBalanceRequest<'a> {
    pub payment_method: AdyenPaymentMethod<'a>,
    pub merchant_account: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenBalanceResponse {
    pub psp_reference: String,
    pub balance: Amount,
}

/// This implementation will be used only in Authorize, Automatic capture flow.
/// It is also being used in Psync flow, However Psync will be called only after create payment call that too in redirect flow.
impl ForeignFrom<(bool, AdyenStatus, Option<common_enums::PaymentMethodType>)>
    for storage_enums::AttemptStatus
{
    fn foreign_from(
        (is_manual_capture, adyen_status, pmt): (
            bool,
            AdyenStatus,
            Option<common_enums::PaymentMethodType>,
        ),
    ) -> Self {
        match adyen_status {
            AdyenStatus::AuthenticationFinished => Self::AuthenticationSuccessful,
            AdyenStatus::AuthenticationNotRequired | AdyenStatus::Received => Self::Pending,
            AdyenStatus::Authorised => match is_manual_capture {
                true => Self::Authorized,
                // In case of Automatic capture Authorized is the final status of the payment
                false => Self::Charged,
            },
            AdyenStatus::Cancelled => Self::Voided,
            AdyenStatus::ChallengeShopper
            | AdyenStatus::RedirectShopper
            | AdyenStatus::PresentToShopper => Self::AuthenticationPending,
            AdyenStatus::Error | AdyenStatus::Refused => Self::Failure,
            AdyenStatus::Pending => match pmt {
                Some(common_enums::PaymentMethodType::Pix) => Self::AuthenticationPending,
                _ => Self::Pending,
            },
            #[cfg(feature = "payouts")]
            AdyenStatus::PayoutConfirmReceived => Self::Started,
            #[cfg(feature = "payouts")]
            AdyenStatus::PayoutSubmitReceived => Self::Pending,
            #[cfg(feature = "payouts")]
            AdyenStatus::PayoutDeclineReceived => Self::Voided,
        }
    }
}

impl ForeignTryFrom<(bool, AdyenWebhookStatus)> for storage_enums::AttemptStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (is_manual_capture, adyen_webhook_status): (bool, AdyenWebhookStatus),
    ) -> Result<Self, Self::Error> {
        match adyen_webhook_status {
            AdyenWebhookStatus::Authorised => match is_manual_capture {
                true => Ok(Self::Authorized),
                // In case of Automatic capture Authorized is the final status of the payment
                false => Ok(Self::Charged),
            },
            AdyenWebhookStatus::AuthorisationFailed => Ok(Self::Failure),
            AdyenWebhookStatus::Cancelled => Ok(Self::Voided),
            AdyenWebhookStatus::CancelFailed => Ok(Self::VoidFailed),
            AdyenWebhookStatus::Captured => Ok(Self::Charged),
            AdyenWebhookStatus::CaptureFailed => Ok(Self::CaptureFailed),
            //If Unexpected Event is received, need to understand how it reached this point
            //Webhooks with Payment Events only should try to conume this resource object.
            AdyenWebhookStatus::UnexpectedEvent => {
                Err(report!(errors::ConnectorError::WebhookBodyDecodingFailed))
            }
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
    AdyenRefusal(AdyenRefusal),
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefusal {
    pub payload: String,
    #[serde(rename = "type")]
    pub type_of_redirection_result: Option<String>,
    pub result_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirection {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AdyenPaymentResponse {
    Response(Box<Response>),
    PresentToShopper(Box<PresentToShopperResponse>),
    QrCodeResponse(Box<QrCodeResponseResponse>),
    RedirectionResponse(Box<RedirectionResponse>),
    RedirectionErrorResponse(Box<RedirectionErrorResponse>),
    WebhookResponse(Box<AdyenWebhookResponse>),
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
pub enum AdyenWebhookStatus {
    Authorised,
    AuthorisationFailed,
    Cancelled,
    CancelFailed,
    Captured,
    CaptureFailed,
    UnexpectedEvent,
}

//Creating custom struct which can be consumed in Psync Handler triggered from Webhooks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenWebhookResponse {
    transaction_id: String,
    payment_reference: Option<String>,
    status: AdyenWebhookStatus,
    amount: Option<Amount>,
    merchant_reference_id: String,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    event_code: WebhookEventCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectionErrorResponse {
    result_code: AdyenStatus,
    refusal_reason: String,
    psp_reference: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectionResponse {
    result_code: AdyenStatus,
    action: AdyenRedirectAction,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    psp_reference: Option<String>,
    merchant_reference: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentToShopperResponse {
    psp_reference: Option<String>,
    result_code: AdyenStatus,
    action: AdyenPtsAction,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    merchant_reference: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeResponseResponse {
    result_code: AdyenStatus,
    action: AdyenQrCodeAction,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
    additional_data: Option<QrCodeAdditionalData>,
    psp_reference: Option<String>,
    merchant_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenQrCodeAction {
    payment_method_type: PaymentType,
    #[serde(rename = "type")]
    type_of_response: ActionType,
    #[serde(rename = "url")]
    qr_code_url: Option<Url>,
    qr_code_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodeAdditionalData {
    #[serde(rename = "pix.expirationDate")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pix_expiration_date: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPtsAction {
    reference: String,
    download_url: Option<Url>,
    payment_method_type: PaymentType,
    #[serde(rename = "expiresAt")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option_without_timezone"
    )]
    expires_at: Option<PrimitiveDateTime>,
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
    pub currency: storage_enums::Currency,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum AdyenPaymentMethod<'a> {
    AdyenAffirm(Box<PmdForPaymentType>),
    AdyenCard(Box<AdyenCard>),
    AdyenKlarna(Box<PmdForPaymentType>),
    AdyenPaypal(Box<PmdForPaymentType>),
    #[serde(rename = "afterpaytouch")]
    AfterPay(Box<PmdForPaymentType>),
    AlmaPayLater(Box<PmdForPaymentType>),
    AliPay(Box<PmdForPaymentType>),
    AliPayHk(Box<PmdForPaymentType>),
    ApplePay(Box<AdyenApplePay>),
    #[serde(rename = "atome")]
    Atome,
    BancontactCard(Box<BancontactCardData>),
    Bizum(Box<PmdForPaymentType>),
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
    Giropay(Box<PmdForPaymentType>),
    Gpay(Box<AdyenGPay>),
    #[serde(rename = "gopay_wallet")]
    GoPay(Box<GoPayData>),
    Ideal(Box<BankRedirectionWithIssuer<'a>>),
    #[serde(rename = "kakaopay")]
    Kakaopay(Box<KakaoPayData>),
    Mandate(Box<AdyenMandate>),
    Mbway(Box<MbwayData>),
    MobilePay(Box<PmdForPaymentType>),
    #[serde(rename = "momo_wallet")]
    Momo(Box<MomoData>),
    #[serde(rename = "momo_atm")]
    MomoAtm,
    #[serde(rename = "touchngo")]
    TouchNGo(Box<TouchNGoData>),
    OnlineBankingCzechRepublic(Box<OnlineBankingCzechRepublicData>),
    OnlineBankingFinland(Box<PmdForPaymentType>),
    OnlineBankingPoland(Box<OnlineBankingPolandData>),
    OnlineBankingSlovakia(Box<OnlineBankingSlovakiaData>),
    #[serde(rename = "molpay_ebanking_fpx_MY")]
    OnlineBankingFpx(Box<OnlineBankingFpxData>),
    #[serde(rename = "molpay_ebanking_TH")]
    OnlineBankingThailand(Box<OnlineBankingThailandData>),
    #[serde(rename = "paybybank")]
    OpenBankingUK(Box<OpenBankingUKData>),
    #[serde(rename = "oxxo")]
    Oxxo,
    #[serde(rename = "paysafecard")]
    PaySafeCard,
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
    PaymentMethodBalance(Box<BalancePmData>),
    AdyenGiftCard(Box<GiftCardData>),
    #[serde(rename = "swish")]
    Swish,
    #[serde(rename = "benefit")]
    Benefit,
    #[serde(rename = "knet")]
    Knet,
    #[serde(rename = "econtext_seven_eleven")]
    SevenEleven(Box<JCSVoucherData>),
    #[serde(rename = "econtext_stores")]
    Lawson(Box<JCSVoucherData>),
    #[serde(rename = "econtext_stores")]
    MiniStop(Box<JCSVoucherData>),
    #[serde(rename = "econtext_stores")]
    FamilyMart(Box<JCSVoucherData>),
    #[serde(rename = "econtext_stores")]
    Seicomart(Box<JCSVoucherData>),
    #[serde(rename = "econtext_stores")]
    PayEasy(Box<JCSVoucherData>),
    Pix(Box<PmdForPaymentType>),
}

#[derive(Debug, Clone, Serialize)]
pub struct PmdForPaymentType {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JCSVoucherData {
    first_name: Secret<String>,
    last_name: Option<Secret<String>>,
    shopper_email: Email,
    telephone_number: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalancePmData {
    #[serde(rename = "type")]
    payment_type: GiftCardBrand,
    number: Secret<String>,
    cvc: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GiftCardData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    brand: GiftCardBrand,
    number: Secret<String>,
    cvc: Secret<String>,
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

impl TryFrom<&Box<payments::JCSVoucherData>> for JCSVoucherData {
    type Error = Error;
    fn try_from(jcs_data: &Box<payments::JCSVoucherData>) -> Result<Self, Self::Error> {
        Ok(Self {
            first_name: jcs_data.first_name.clone(),
            last_name: jcs_data.last_name.clone(),
            shopper_email: jcs_data.email.clone(),
            telephone_number: Secret::new(jcs_data.phone_number.clone()),
        })
    }
}

impl TryFrom<&api_enums::BankNames> for OnlineBankingCzechRepublicBanks {
    type Error = Error;
    fn try_from(bank_name: &api_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            api::enums::BankNames::KomercniBanka => Ok(Self::KB),
            api::enums::BankNames::CeskaSporitelna => Ok(Self::CS),
            api::enums::BankNames::PlatnoscOnlineKartaPlatnicza => Ok(Self::C),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
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
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
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
pub struct OpenBankingUKData {
    issuer: OpenBankingUKIssuer,
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
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
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
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
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
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        }
    }
}

impl TryFrom<&api_enums::BankNames> for OpenBankingUKIssuer {
    type Error = Error;
    fn try_from(bank_name: &api_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            api::enums::BankNames::OpenBankSuccess => Ok(Self::RedirectSuccess),
            api::enums::BankNames::OpenBankFailure => Ok(Self::RedirectFailure),
            api::enums::BankNames::OpenBankCancelled => Ok(Self::RedirectCancelled),
            api::enums::BankNames::Aib => Ok(Self::Aib),
            api::enums::BankNames::BankOfScotland => Ok(Self::BankOfScotland),
            api::enums::BankNames::Barclays => Ok(Self::Barclays),
            api::enums::BankNames::DanskeBank => Ok(Self::DanskeBank),
            api::enums::BankNames::FirstDirect => Ok(Self::FirstDirect),
            api::enums::BankNames::FirstTrust => Ok(Self::FirstTrust),
            api::enums::BankNames::HsbcBank => Ok(Self::HsbcBank),
            api::enums::BankNames::Halifax => Ok(Self::Halifax),
            api::enums::BankNames::Lloyds => Ok(Self::Lloyds),
            api::enums::BankNames::Monzo => Ok(Self::Monzo),
            api::enums::BankNames::NatWest => Ok(Self::NatWest),
            api::enums::BankNames::NationwideBank => Ok(Self::NationwideBank),
            api::enums::BankNames::Revolut => Ok(Self::Revolut),
            api::enums::BankNames::RoyalBankOfScotland => Ok(Self::RoyalBankOfScotland),
            api::enums::BankNames::SantanderPrzelew24 => Ok(Self::SantanderPrzelew24),
            api::enums::BankNames::Starling => Ok(Self::Starling),
            api::enums::BankNames::TsbBank => Ok(Self::TsbBank),
            api::enums::BankNames::TescoBank => Ok(Self::TescoBank),
            api::enums::BankNames::UlsterBank => Ok(Self::UlsterBank),
            enums::BankNames::AmericanExpress
            | enums::BankNames::AffinBank
            | enums::BankNames::AgroBank
            | enums::BankNames::AllianceBank
            | enums::BankNames::AmBank
            | enums::BankNames::BankOfAmerica
            | enums::BankNames::BankIslam
            | enums::BankNames::BankMuamalat
            | enums::BankNames::BankRakyat
            | enums::BankNames::BankSimpananNasional
            | enums::BankNames::BlikPSP
            | enums::BankNames::CapitalOne
            | enums::BankNames::Chase
            | enums::BankNames::Citi
            | enums::BankNames::CimbBank
            | enums::BankNames::Discover
            | enums::BankNames::NavyFederalCreditUnion
            | enums::BankNames::PentagonFederalCreditUnion
            | enums::BankNames::SynchronyBank
            | enums::BankNames::WellsFargo
            | enums::BankNames::AbnAmro
            | enums::BankNames::AsnBank
            | enums::BankNames::Bunq
            | enums::BankNames::Handelsbanken
            | enums::BankNames::HongLeongBank
            | enums::BankNames::Ing
            | enums::BankNames::Knab
            | enums::BankNames::KuwaitFinanceHouse
            | enums::BankNames::Moneyou
            | enums::BankNames::Rabobank
            | enums::BankNames::Regiobank
            | enums::BankNames::SnsBank
            | enums::BankNames::TriodosBank
            | enums::BankNames::VanLanschot
            | enums::BankNames::ArzteUndApothekerBank
            | enums::BankNames::AustrianAnadiBankAg
            | enums::BankNames::BankAustria
            | enums::BankNames::Bank99Ag
            | enums::BankNames::BankhausCarlSpangler
            | enums::BankNames::BankhausSchelhammerUndSchatteraAg
            | enums::BankNames::BankMillennium
            | enums::BankNames::BankPEKAOSA
            | enums::BankNames::BawagPskAg
            | enums::BankNames::BksBankAg
            | enums::BankNames::BrullKallmusBankAg
            | enums::BankNames::BtvVierLanderBank
            | enums::BankNames::CapitalBankGraweGruppeAg
            | enums::BankNames::CeskaSporitelna
            | enums::BankNames::Dolomitenbank
            | enums::BankNames::EasybankAg
            | enums::BankNames::EPlatbyVUB
            | enums::BankNames::ErsteBankUndSparkassen
            | enums::BankNames::FrieslandBank
            | enums::BankNames::HypoAlpeadriabankInternationalAg
            | enums::BankNames::HypoNoeLbFurNiederosterreichUWien
            | enums::BankNames::HypoOberosterreichSalzburgSteiermark
            | enums::BankNames::HypoTirolBankAg
            | enums::BankNames::HypoVorarlbergBankAg
            | enums::BankNames::HypoBankBurgenlandAktiengesellschaft
            | enums::BankNames::KomercniBanka
            | enums::BankNames::MBank
            | enums::BankNames::MarchfelderBank
            | enums::BankNames::Maybank
            | enums::BankNames::OberbankAg
            | enums::BankNames::OsterreichischeArzteUndApothekerbank
            | enums::BankNames::OcbcBank
            | enums::BankNames::PayWithING
            | enums::BankNames::PlaceZIPKO
            | enums::BankNames::PlatnoscOnlineKartaPlatnicza
            | enums::BankNames::PosojilnicaBankEGen
            | enums::BankNames::PostovaBanka
            | enums::BankNames::PublicBank
            | enums::BankNames::RaiffeisenBankengruppeOsterreich
            | enums::BankNames::RhbBank
            | enums::BankNames::SchelhammerCapitalBankAg
            | enums::BankNames::StandardCharteredBank
            | enums::BankNames::SchoellerbankAg
            | enums::BankNames::SpardaBankWien
            | enums::BankNames::SporoPay
            | enums::BankNames::TatraPay
            | enums::BankNames::Viamo
            | enums::BankNames::VolksbankGruppe
            | enums::BankNames::VolkskreditbankAg
            | enums::BankNames::VrBankBraunau
            | enums::BankNames::UobBank
            | enums::BankNames::PayWithAliorBank
            | enums::BankNames::BankiSpoldzielcze
            | enums::BankNames::PayWithInteligo
            | enums::BankNames::BNPParibasPoland
            | enums::BankNames::BankNowySA
            | enums::BankNames::CreditAgricole
            | enums::BankNames::PayWithBOS
            | enums::BankNames::PayWithCitiHandlowy
            | enums::BankNames::PayWithPlusBank
            | enums::BankNames::ToyotaBank
            | enums::BankNames::VeloBank
            | enums::BankNames::ETransferPocztowy24
            | enums::BankNames::PlusBank
            | enums::BankNames::EtransferPocztowy24
            | enums::BankNames::BankiSpbdzielcze
            | enums::BankNames::BankNowyBfgSa
            | enums::BankNames::GetinBank
            | enums::BankNames::Blik
            | enums::BankNames::NoblePay
            | enums::BankNames::IdeaBank
            | enums::BankNames::EnveloBank
            | enums::BankNames::NestPrzelew
            | enums::BankNames::MbankMtransfer
            | enums::BankNames::Inteligo
            | enums::BankNames::PbacZIpko
            | enums::BankNames::BnpParibas
            | enums::BankNames::BankPekaoSa
            | enums::BankNames::VolkswagenBank
            | enums::BankNames::AliorBank
            | enums::BankNames::Boz
            | enums::BankNames::BangkokBank
            | enums::BankNames::KrungsriBank
            | enums::BankNames::KrungThaiBank
            | enums::BankNames::TheSiamCommercialBank
            | enums::BankNames::Yoursafe
            | enums::BankNames::N26
            | enums::BankNames::NationaleNederlanden
            | enums::BankNames::KasikornBank => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlikRedirectionData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    blik_code: Secret<String>,
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
    stored_payment_method_id: Secret<String>,
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
    holder_name: Option<Secret<String>>,
    brand: Option<CardBrand>, //Mandatory for mandate using network_txns_id
    network_payment_reference: Option<Secret<String>>,
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

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelResponse {
    payment_psp_reference: String,
    status: CancelStatus,
    reference: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CancelStatus {
    Received,
    #[default]
    Processing,
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
    merchant_account: Secret<String>,
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
    #[serde(rename = "momo_atm")]
    MomoAtm,
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
    #[serde(rename = "paybybank")]
    OpenBankingUK,
    #[serde(rename = "oxxo")]
    Oxxo,
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
    Giftcard,
    Knet,
    Benefit,
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
    #[serde(rename = "econtext_seven_eleven")]
    SevenEleven,
    #[serde(rename = "econtext_stores")]
    Lawson,
    #[serde(rename = "econtext_stores")]
    MiniStop,
    #[serde(rename = "econtext_stores")]
    FamilyMart,
    #[serde(rename = "econtext_stores")]
    Seicomart,
    #[serde(rename = "econtext_stores")]
    PayEasy,
    Pix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GiftCardBrand {
    Givex,
    Auriga,
    Babygiftcard,
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

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
pub enum OpenBankingUKIssuer {
    #[serde(rename = "uk-test-open-banking-redirect")]
    RedirectSuccess,
    #[serde(rename = "uk-test-open-banking-redirect-failed")]
    RedirectFailure,
    #[serde(rename = "uk-test-open-banking-redirect-cancelled")]
    RedirectCancelled,
    #[serde(rename = "uk-aib-oauth2")]
    Aib,
    #[serde(rename = "uk-bankofscotland-oauth2")]
    BankOfScotland,
    #[serde(rename = "uk-barclays-oauth2")]
    Barclays,
    #[serde(rename = "uk-danskebank-oauth2")]
    DanskeBank,
    #[serde(rename = "uk-firstdirect-oauth2")]
    FirstDirect,
    #[serde(rename = "uk-firsttrust-oauth2")]
    FirstTrust,
    #[serde(rename = "uk-hsbc-oauth2")]
    HsbcBank,
    #[serde(rename = "uk-halifax-oauth2")]
    Halifax,
    #[serde(rename = "uk-lloyds-oauth2")]
    Lloyds,
    #[serde(rename = "uk-monzo-oauth2")]
    Monzo,
    #[serde(rename = "uk-natwest-oauth2")]
    NatWest,
    #[serde(rename = "uk-nationwide-oauth2")]
    NationwideBank,
    #[serde(rename = "uk-revolut-oauth2")]
    Revolut,
    #[serde(rename = "uk-rbs-oauth2")]
    RoyalBankOfScotland,
    #[serde(rename = "uk-santander-oauth2")]
    SantanderPrzelew24,
    #[serde(rename = "uk-starling-oauth2")]
    Starling,
    #[serde(rename = "uk-tsb-oauth2")]
    TsbBank,
    #[serde(rename = "uk-tesco-oauth2")]
    TescoBank,
    #[serde(rename = "uk-ulster-oauth2")]
    UlsterBank,
}

pub struct AdyenTestBankNames<'a>(&'a str);

impl<'a> TryFrom<&api_enums::BankNames> for AdyenTestBankNames<'a> {
    type Error = Error;
    fn try_from(bank: &api_enums::BankNames) -> Result<Self, Self::Error> {
        Ok(match bank {
            api_models::enums::BankNames::AbnAmro => Self("1121"),
            api_models::enums::BankNames::AsnBank => Self("1151"),
            api_models::enums::BankNames::Bunq => Self("1152"),
            api_models::enums::BankNames::Ing => Self("1154"),
            api_models::enums::BankNames::Knab => Self("1155"),
            api_models::enums::BankNames::N26 => Self("1156"),
            api_models::enums::BankNames::NationaleNederlanden => Self("1157"),
            api_models::enums::BankNames::Rabobank => Self("1157"),
            api_models::enums::BankNames::Regiobank => Self("1158"),
            api_models::enums::BankNames::Revolut => Self("1159"),
            api_models::enums::BankNames::SnsBank => Self("1159"),
            api_models::enums::BankNames::TriodosBank => Self("1159"),
            api_models::enums::BankNames::VanLanschot => Self("1159"),
            api_models::enums::BankNames::Yoursafe => Self("1159"),
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
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            ))?,
        })
    }
}

pub struct AdyenBankNames<'a>(&'a str);

impl<'a> TryFrom<&api_enums::BankNames> for AdyenBankNames<'a> {
    type Error = Error;
    fn try_from(bank: &api_enums::BankNames) -> Result<Self, Self::Error> {
        Ok(match bank {
            api_models::enums::BankNames::AbnAmro => Self("0031"),
            api_models::enums::BankNames::AsnBank => Self("0761"),
            api_models::enums::BankNames::Bunq => Self("0802"),
            api_models::enums::BankNames::Ing => Self("0721"),
            api_models::enums::BankNames::Knab => Self("0801"),
            api_models::enums::BankNames::N26 => Self("0807"),
            api_models::enums::BankNames::NationaleNederlanden => Self("0808"),
            api_models::enums::BankNames::Rabobank => Self("0021"),
            api_models::enums::BankNames::Regiobank => Self("0771"),
            api_models::enums::BankNames::Revolut => Self("0805"),
            api_models::enums::BankNames::SnsBank => Self("0751"),
            api_models::enums::BankNames::TriodosBank => Self("0511"),
            api_models::enums::BankNames::VanLanschot => Self("0161"),
            api_models::enums::BankNames::Yoursafe => Self("0806"),
            _ => Err(errors::ConnectorError::NotSupported {
                message: String::from("BankRedirect"),
                connector: "Adyen",
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

impl<'a> TryFrom<&AdyenRouterData<&types::PaymentsAuthorizeRouterData>>
    for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        item: &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item
            .router_data
            .request
            .mandate_id
            .to_owned()
            .and_then(|mandate_ids| mandate_ids.mandate_reference_id)
        {
            Some(mandate_ref) => AdyenPaymentRequest::try_from((item, mandate_ref)),
            None => match item.router_data.request.payment_method_data {
                domain::PaymentMethodData::Card(ref card) => {
                    AdyenPaymentRequest::try_from((item, card))
                }
                domain::PaymentMethodData::Wallet(ref wallet) => {
                    AdyenPaymentRequest::try_from((item, wallet))
                }
                domain::PaymentMethodData::PayLater(ref pay_later) => {
                    AdyenPaymentRequest::try_from((item, pay_later))
                }
                domain::PaymentMethodData::BankRedirect(ref bank_redirect) => {
                    AdyenPaymentRequest::try_from((item, bank_redirect))
                }
                domain::PaymentMethodData::BankDebit(ref bank_debit) => {
                    AdyenPaymentRequest::try_from((item, bank_debit))
                }
                domain::PaymentMethodData::BankTransfer(ref bank_transfer) => {
                    AdyenPaymentRequest::try_from((item, bank_transfer.as_ref()))
                }
                domain::PaymentMethodData::CardRedirect(ref card_redirect_data) => {
                    AdyenPaymentRequest::try_from((item, card_redirect_data))
                }
                domain::PaymentMethodData::Voucher(ref voucher_data) => {
                    AdyenPaymentRequest::try_from((item, voucher_data))
                }
                domain::PaymentMethodData::GiftCard(ref gift_card_data) => {
                    AdyenPaymentRequest::try_from((item, gift_card_data.as_ref()))
                }
                domain::PaymentMethodData::Crypto(_)
                | domain::PaymentMethodData::MandatePayment
                | domain::PaymentMethodData::Reward
                | domain::PaymentMethodData::Upi(_)
                | domain::PaymentMethodData::CardToken(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("Adyen"),
                    ))?
                }
            },
        }
    }
}

impl<'a> TryFrom<&types::PaymentsPreProcessingRouterData> for AdyenBalanceRequest<'a> {
    type Error = Error;
    fn try_from(item: &types::PaymentsPreProcessingRouterData) -> Result<Self, Self::Error> {
        let payment_method = match &item.request.payment_method_data {
            Some(domain::PaymentMethodData::GiftCard(gift_card_data)) => {
                match gift_card_data.as_ref() {
                    payments::GiftCardData::Givex(gift_card_data) => {
                        let balance_pm = BalancePmData {
                            payment_type: GiftCardBrand::Givex,
                            number: gift_card_data.number.clone(),
                            cvc: gift_card_data.cvc.clone(),
                        };
                        Ok(AdyenPaymentMethod::PaymentMethodBalance(Box::new(
                            balance_pm,
                        )))
                    }
                    payments::GiftCardData::PaySafeCard {} => {
                        Err(errors::ConnectorError::FlowNotSupported {
                            flow: "Balance".to_string(),
                            connector: "adyen".to_string(),
                        })
                    }
                }
            }
            _ => Err(errors::ConnectorError::FlowNotSupported {
                flow: "Balance".to_string(),
                connector: "adyen".to_string(),
            }),
        }?;
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            payment_method,
            merchant_account: auth_type.merchant_account,
        })
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
    let (authorisation_type, manual_capture) = match item.request.capture_method {
        Some(diesel_models::enums::CaptureMethod::Manual)
        | Some(diesel_models::enums::CaptureMethod::ManualMultiple) => {
            (Some(AuthType::PreAuth), Some("true".to_string()))
        }
        _ => (None, None),
    };
    let execute_three_d = if matches!(item.auth_type, enums::AuthenticationType::ThreeDs) {
        Some("true".to_string())
    } else {
        None
    };
    Some(AdditionalData {
        authorisation_type,
        manual_capture,
        execute_three_d,
        network_tx_reference: None,
        recurring_detail_reference: None,
        recurring_shopper_reference: None,
        recurring_processing_model: None,
        ..AdditionalData::default()
    })
}

fn get_channel_type(pm_type: &Option<storage_enums::PaymentMethodType>) -> Option<Channel> {
    pm_type.as_ref().and_then(|pmt| match pmt {
        storage_enums::PaymentMethodType::GoPay => Some(Channel::Web),
        _ => None,
    })
}

fn get_amount_data(item: &AdyenRouterData<&types::PaymentsAuthorizeRouterData>) -> Amount {
    Amount {
        currency: item.router_data.request.currency,
        value: item.amount.to_owned(),
    }
}

fn get_address_info(
    address: Option<&api_models::payments::Address>,
) -> Option<Result<Address, error_stack::Report<errors::ConnectorError>>> {
    address.and_then(|add| {
        add.address.as_ref().map(
            |a| -> Result<Address, error_stack::Report<errors::ConnectorError>> {
                Ok(Address {
                    city: a.get_city()?.to_owned(),
                    country: a.get_country()?.to_owned(),
                    house_number_or_name: a.get_line1()?.to_owned(),
                    postal_code: a.get_zip()?.to_owned(),
                    state_or_province: a.state.clone(),
                    street: a.get_line2()?.to_owned(),
                })
            },
        )
    })
}

fn get_line_items(item: &AdyenRouterData<&types::PaymentsAuthorizeRouterData>) -> Vec<LineItem> {
    let order_details: Option<Vec<payments::OrderDetailsWithAmount>> =
        item.router_data.request.order_details.clone();
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
                amount_including_tax: Some(item.amount.to_owned()),
                amount_excluding_tax: Some(item.amount.to_owned()),
                description: item.router_data.description.clone(),
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
        .get_optional_billing()
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
        | payments::VoucherData::Oxxo
        | payments::VoucherData::RedPagos
        | payments::VoucherData::SevenEleven { .. }
        | payments::VoucherData::Lawson { .. }
        | payments::VoucherData::MiniStop { .. }
        | payments::VoucherData::FamilyMart { .. }
        | payments::VoucherData::Seicomart { .. }
        | payments::VoucherData::PayEasy { .. } => None,
    }
}

fn build_shopper_reference(customer_id: &Option<String>, merchant_id: String) -> Option<String> {
    customer_id
        .clone()
        .map(|c_id| format!("{}_{}", merchant_id, c_id))
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
            payments::BankDebitData::BecsBankDebit { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Adyen"),
                )
                .into())
            }
        }
    }
}

impl<'a> TryFrom<&api_models::payments::VoucherData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(voucher_data: &api_models::payments::VoucherData) -> Result<Self, Self::Error> {
        match voucher_data {
            payments::VoucherData::Boleto { .. } => Ok(AdyenPaymentMethod::BoletoBancario),
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
            payments::VoucherData::Oxxo => Ok(AdyenPaymentMethod::Oxxo),
            payments::VoucherData::SevenEleven(jcs_data) => Ok(AdyenPaymentMethod::SevenEleven(
                Box::new(JCSVoucherData::try_from(jcs_data)?),
            )),
            payments::VoucherData::Lawson(jcs_data) => Ok(AdyenPaymentMethod::Lawson(Box::new(
                JCSVoucherData::try_from(jcs_data)?,
            ))),
            payments::VoucherData::MiniStop(jcs_data) => Ok(AdyenPaymentMethod::MiniStop(
                Box::new(JCSVoucherData::try_from(jcs_data)?),
            )),
            payments::VoucherData::FamilyMart(jcs_data) => Ok(AdyenPaymentMethod::FamilyMart(
                Box::new(JCSVoucherData::try_from(jcs_data)?),
            )),
            payments::VoucherData::Seicomart(jcs_data) => Ok(AdyenPaymentMethod::Seicomart(
                Box::new(JCSVoucherData::try_from(jcs_data)?),
            )),
            payments::VoucherData::PayEasy(jcs_data) => Ok(AdyenPaymentMethod::PayEasy(Box::new(
                JCSVoucherData::try_from(jcs_data)?,
            ))),
            payments::VoucherData::Efecty
            | payments::VoucherData::PagoEfectivo
            | payments::VoucherData::RedCompra
            | payments::VoucherData::RedPagos => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

impl<'a> TryFrom<&api_models::payments::GiftCardData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(gift_card_data: &api_models::payments::GiftCardData) -> Result<Self, Self::Error> {
        match gift_card_data {
            payments::GiftCardData::PaySafeCard {} => Ok(AdyenPaymentMethod::PaySafeCard),
            payments::GiftCardData::Givex(givex_data) => {
                let gift_card_pm = GiftCardData {
                    payment_type: PaymentType::Giftcard,
                    brand: GiftCardBrand::Givex,
                    number: givex_data.number.clone(),
                    cvc: givex_data.cvc.clone(),
                };
                Ok(AdyenPaymentMethod::AdyenGiftCard(Box::new(gift_card_pm)))
            }
        }
    }
}

impl<'a> TryFrom<&domain::Card> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(card: &domain::Card) -> Result<Self, Self::Error> {
        let adyen_card = AdyenCard {
            payment_type: PaymentType::Scheme,
            number: card.card_number.clone(),
            expiry_month: card.card_exp_month.clone(),
            expiry_year: card.get_expiry_year_4_digit(),
            cvc: Some(card.card_cvc.clone()),
            holder_name: card.card_holder_name.clone(),
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
                utils::get_unimplemented_payment_method_error_message("Adyen"),
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

impl<'a> TryFrom<&domain::WalletData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(wallet_data: &domain::WalletData) -> Result<Self, Self::Error> {
        match wallet_data {
            domain::WalletData::GooglePay(data) => {
                let gpay_data = AdyenGPay {
                    payment_type: PaymentType::Googlepay,
                    google_pay_token: Secret::new(data.tokenization_data.token.to_owned()),
                };
                Ok(AdyenPaymentMethod::Gpay(Box::new(gpay_data)))
            }
            domain::WalletData::ApplePay(data) => {
                let apple_pay_data = AdyenApplePay {
                    payment_type: PaymentType::Applepay,
                    apple_pay_token: Secret::new(data.payment_data.to_string()),
                };

                Ok(AdyenPaymentMethod::ApplePay(Box::new(apple_pay_data)))
            }
            domain::WalletData::PaypalRedirect(_) => {
                let wallet = PmdForPaymentType {
                    payment_type: PaymentType::Paypal,
                };
                Ok(AdyenPaymentMethod::AdyenPaypal(Box::new(wallet)))
            }
            domain::WalletData::AliPayRedirect(_) => {
                let alipay_data = PmdForPaymentType {
                    payment_type: PaymentType::Alipay,
                };
                Ok(AdyenPaymentMethod::AliPay(Box::new(alipay_data)))
            }
            domain::WalletData::AliPayHkRedirect(_) => {
                let alipay_hk_data = PmdForPaymentType {
                    payment_type: PaymentType::AlipayHk,
                };
                Ok(AdyenPaymentMethod::AliPayHk(Box::new(alipay_hk_data)))
            }
            domain::WalletData::GoPayRedirect(_) => {
                let go_pay_data = GoPayData {};
                Ok(AdyenPaymentMethod::GoPay(Box::new(go_pay_data)))
            }
            domain::WalletData::KakaoPayRedirect(_) => {
                let kakao_pay_data = KakaoPayData {};
                Ok(AdyenPaymentMethod::Kakaopay(Box::new(kakao_pay_data)))
            }
            domain::WalletData::GcashRedirect(_) => {
                let gcash_data = GcashData {};
                Ok(AdyenPaymentMethod::Gcash(Box::new(gcash_data)))
            }
            domain::WalletData::MomoRedirect(_) => {
                let momo_data = MomoData {};
                Ok(AdyenPaymentMethod::Momo(Box::new(momo_data)))
            }
            domain::WalletData::TouchNGoRedirect(_) => {
                let touch_n_go_data = TouchNGoData {};
                Ok(AdyenPaymentMethod::TouchNGo(Box::new(touch_n_go_data)))
            }
            domain::WalletData::MbWayRedirect(data) => {
                let mbway_data = MbwayData {
                    payment_type: PaymentType::Mbway,
                    telephone_number: data.telephone_number.clone(),
                };
                Ok(AdyenPaymentMethod::Mbway(Box::new(mbway_data)))
            }
            domain::WalletData::MobilePayRedirect(_) => {
                let data = PmdForPaymentType {
                    payment_type: PaymentType::MobilePay,
                };
                Ok(AdyenPaymentMethod::MobilePay(Box::new(data)))
            }
            domain::WalletData::WeChatPayRedirect(_) => Ok(AdyenPaymentMethod::WeChatPayWeb),
            domain::WalletData::SamsungPay(samsung_data) => {
                let data = SamsungPayPmData {
                    payment_type: PaymentType::Samsungpay,
                    samsung_pay_token: samsung_data.token.to_owned(),
                };
                Ok(AdyenPaymentMethod::SamsungPay(Box::new(data)))
            }
            domain::WalletData::TwintRedirect { .. } => Ok(AdyenPaymentMethod::Twint),
            domain::WalletData::VippsRedirect { .. } => Ok(AdyenPaymentMethod::Vipps),
            domain::WalletData::DanaRedirect { .. } => Ok(AdyenPaymentMethod::Dana),
            domain::WalletData::SwishQr(_) => Ok(AdyenPaymentMethod::Swish),
            domain::WalletData::AliPayQr(_)
            | domain::WalletData::ApplePayRedirect(_)
            | domain::WalletData::ApplePayThirdPartySdk(_)
            | domain::WalletData::GooglePayRedirect(_)
            | domain::WalletData::GooglePayThirdPartySdk(_)
            | domain::WalletData::PaypalSdk(_)
            | domain::WalletData::WeChatPayQr(_)
            | domain::WalletData::CashappQr(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

pub fn check_required_field<'a, T>(
    field: &'a Option<T>,
    message: &'static str,
) -> Result<&'a T, errors::ConnectorError> {
    field
        .as_ref()
        .ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: message,
        })
}

impl<'a>
    TryFrom<(
        &domain::PayLaterData,
        &Option<api_enums::CountryAlpha2>,
        &Option<Email>,
        &Option<String>,
        &Option<ShopperName>,
        &Option<Secret<String>>,
        &Option<Address>,
        &Option<Address>,
    )> for AdyenPaymentMethod<'a>
{
    type Error = Error;
    fn try_from(
        value: (
            &domain::PayLaterData,
            &Option<api_enums::CountryAlpha2>,
            &Option<Email>,
            &Option<String>,
            &Option<ShopperName>,
            &Option<Secret<String>>,
            &Option<Address>,
            &Option<Address>,
        ),
    ) -> Result<Self, Self::Error> {
        let (
            pay_later_data,
            country_code,
            shopper_email,
            shopper_reference,
            shopper_name,
            telephone_number,
            billing_address,
            delivery_address,
        ) = value;
        match pay_later_data {
            domain::payments::PayLaterData::KlarnaRedirect { .. } => {
                let klarna = PmdForPaymentType {
                    payment_type: PaymentType::Klarna,
                };
                check_required_field(shopper_email, "email")?;
                check_required_field(shopper_reference, "customer_id")?;
                check_required_field(country_code, "billing.country")?;

                Ok(AdyenPaymentMethod::AdyenKlarna(Box::new(klarna)))
            }
            domain::payments::PayLaterData::AffirmRedirect { .. } => {
                check_required_field(shopper_email, "email")?;
                check_required_field(shopper_name, "billing.first_name, billing.last_name")?;
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(billing_address, "billing")?;

                Ok(AdyenPaymentMethod::AdyenAffirm(Box::new(
                    PmdForPaymentType {
                        payment_type: PaymentType::Affirm,
                    },
                )))
            }
            domain::payments::PayLaterData::AfterpayClearpayRedirect { .. } => {
                check_required_field(shopper_email, "email")?;
                check_required_field(shopper_name, "billing.first_name, billing.last_name")?;
                check_required_field(delivery_address, "shipping")?;
                check_required_field(billing_address, "billing")?;

                if let Some(country) = country_code {
                    match country {
                        api_enums::CountryAlpha2::IT
                        | api_enums::CountryAlpha2::FR
                        | api_enums::CountryAlpha2::ES
                        | api_enums::CountryAlpha2::GB => Ok(AdyenPaymentMethod::ClearPay),
                        _ => Ok(AdyenPaymentMethod::AfterPay(Box::new(PmdForPaymentType {
                            payment_type: PaymentType::Afterpaytouch,
                        }))),
                    }
                } else {
                    Err(errors::ConnectorError::MissingRequiredField {
                        field_name: "country",
                    })?
                }
            }
            domain::payments::PayLaterData::PayBrightRedirect { .. } => {
                check_required_field(shopper_name, "billing.first_name, billing.last_name")?;
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(shopper_email, "email")?;
                check_required_field(billing_address, "billing")?;
                check_required_field(delivery_address, "shipping")?;
                check_required_field(country_code, "billing.country")?;
                Ok(AdyenPaymentMethod::PayBright)
            }
            domain::payments::PayLaterData::WalleyRedirect { .. } => {
                //[TODO: Line items specific sub-fields are mandatory]
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(shopper_email, "email")?;
                Ok(AdyenPaymentMethod::Walley)
            }
            domain::payments::PayLaterData::AlmaRedirect { .. } => {
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(shopper_email, "email")?;
                check_required_field(billing_address, "billing")?;
                check_required_field(delivery_address, "shipping")?;
                Ok(AdyenPaymentMethod::AlmaPayLater(Box::new(
                    PmdForPaymentType {
                        payment_type: PaymentType::Alma,
                    },
                )))
            }
            domain::payments::PayLaterData::AtomeRedirect { .. } => {
                check_required_field(shopper_email, "email")?;
                check_required_field(shopper_name, "billing.first_name, billing.last_name")?;
                check_required_field(telephone_number, "billing.phone")?;
                check_required_field(billing_address, "billing")?;
                Ok(AdyenPaymentMethod::Atome)
            }
            domain::payments::PayLaterData::KlarnaSdk { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Adyen"),
                )
                .into())
            }
        }
    }
}

impl<'a> TryFrom<(&api_models::payments::BankRedirectData, Option<bool>)>
    for AdyenPaymentMethod<'a>
{
    type Error = Error;
    fn try_from(
        (bank_redirect_data, test_mode): (&api_models::payments::BankRedirectData, Option<bool>),
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
                Ok(AdyenPaymentMethod::Bizum(Box::new(PmdForPaymentType {
                    payment_type: PaymentType::Bizum,
                })))
            }
            api_models::payments::BankRedirectData::Blik { blik_code } => {
                Ok(AdyenPaymentMethod::Blik(Box::new(BlikRedirectionData {
                    payment_type: PaymentType::Blik,
                    blik_code: Secret::new(blik_code.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "blik_code",
                        },
                    )?),
                })))
            }
            api_models::payments::BankRedirectData::Eps { bank_name, .. } => Ok(
                AdyenPaymentMethod::Eps(Box::new(BankRedirectionWithIssuer {
                    payment_type: PaymentType::Eps,
                    issuer: Some(
                        AdyenTestBankNames::try_from(&bank_name.ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "eps.bank_name",
                            },
                        )?)?
                        .0,
                    ),
                })),
            ),
            api_models::payments::BankRedirectData::Giropay { .. } => {
                Ok(AdyenPaymentMethod::Giropay(Box::new(PmdForPaymentType {
                    payment_type: PaymentType::Giropay,
                })))
            }
            api_models::payments::BankRedirectData::Ideal { bank_name, .. } => {
                let issuer = if test_mode.unwrap_or(true) {
                    Some(
                        AdyenTestBankNames::try_from(&bank_name.ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "ideal.bank_name",
                            },
                        )?)?
                        .0,
                    )
                } else {
                    Some(
                        AdyenBankNames::try_from(&bank_name.ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "ideal.bank_name",
                            },
                        )?)?
                        .0,
                    )
                };
                Ok(AdyenPaymentMethod::Ideal(Box::new(
                    BankRedirectionWithIssuer {
                        payment_type: PaymentType::Ideal,
                        issuer,
                    },
                )))
            }
            api_models::payments::BankRedirectData::OnlineBankingCzechRepublic { issuer } => {
                Ok(AdyenPaymentMethod::OnlineBankingCzechRepublic(Box::new(
                    OnlineBankingCzechRepublicData {
                        payment_type: PaymentType::OnlineBankingCzechRepublic,
                        issuer: OnlineBankingCzechRepublicBanks::try_from(issuer)?,
                    },
                )))
            }
            api_models::payments::BankRedirectData::OnlineBankingFinland { .. } => Ok(
                AdyenPaymentMethod::OnlineBankingFinland(Box::new(PmdForPaymentType {
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
            api_models::payments::BankRedirectData::OpenBankingUk { issuer, .. } => Ok(
                AdyenPaymentMethod::OpenBankingUK(Box::new(OpenBankingUKData {
                    issuer: match issuer {
                        Some(bank_name) => OpenBankingUKIssuer::try_from(bank_name)?,
                        None => Err(errors::ConnectorError::MissingRequiredField {
                            field_name: "issuer",
                        })?,
                    },
                })),
            ),
            api_models::payments::BankRedirectData::Sofort { .. } => Ok(AdyenPaymentMethod::Sofort),
            api_models::payments::BankRedirectData::Trustly { .. } => {
                Ok(AdyenPaymentMethod::Trustly)
            }
            payments::BankRedirectData::Interac { .. }
            | payments::BankRedirectData::Przelewy24 { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Adyen"),
                )
                .into())
            }
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
            api_models::payments::BankTransferData::Pix {} => {
                Ok(AdyenPaymentMethod::Pix(Box::new(PmdForPaymentType {
                    payment_type: PaymentType::Pix,
                })))
            }
            api_models::payments::BankTransferData::AchBankTransfer { .. }
            | api_models::payments::BankTransferData::SepaBankTransfer { .. }
            | api_models::payments::BankTransferData::BacsBankTransfer { .. }
            | api_models::payments::BankTransferData::MultibancoBankTransfer { .. }
            | payments::BankTransferData::Pse {} => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyen"),
            )
            .into()),
        }
    }
}

impl<'a> TryFrom<&domain::payments::CardRedirectData> for AdyenPaymentMethod<'a> {
    type Error = Error;
    fn try_from(
        card_redirect_data: &domain::payments::CardRedirectData,
    ) -> Result<Self, Self::Error> {
        match card_redirect_data {
            domain::CardRedirectData::Knet {} => Ok(AdyenPaymentMethod::Knet),
            domain::CardRedirectData::Benefit {} => Ok(AdyenPaymentMethod::Benefit),
            domain::CardRedirectData::MomoAtm {} => Ok(AdyenPaymentMethod::MomoAtm),
            domain::CardRedirectData::CardRedirect {} => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Adyen"),
                )
                .into())
            }
        }
    }
}

impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        payments::MandateReferenceId,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            payments::MandateReferenceId,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, mandate_ref_id) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item.router_data)?;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let return_url = item.router_data.request.get_return_url()?;
        let payment_method_type = item
            .router_data
            .request
            .payment_method_type
            .as_ref()
            .ok_or(errors::ConnectorError::MissingPaymentMethodType)?;
        let payment_method = match mandate_ref_id {
            payments::MandateReferenceId::ConnectorMandateId(connector_mandate_ids) => {
                let adyen_mandate = AdyenMandate {
                    payment_type: PaymentType::try_from(payment_method_type)?,
                    stored_payment_method_id: Secret::new(
                        connector_mandate_ids.get_connector_mandate_id()?,
                    ),
                };
                Ok::<AdyenPaymentMethod<'_>, Self::Error>(AdyenPaymentMethod::Mandate(Box::new(
                    adyen_mandate,
                )))
            }
            payments::MandateReferenceId::NetworkMandateId(network_mandate_id) => {
                match item.router_data.request.payment_method_data {
                    domain::PaymentMethodData::Card(ref card) => {
                        let card_issuer = card.get_card_issuer()?;
                        let brand = CardBrand::try_from(&card_issuer)?;
                        let adyen_card = AdyenCard {
                            payment_type: PaymentType::Scheme,
                            number: card.card_number.clone(),
                            expiry_month: card.card_exp_month.clone(),
                            expiry_year: card.card_exp_year.clone(),
                            cvc: None,
                            holder_name: card.card_holder_name.clone(),
                            brand: Some(brand),
                            network_payment_reference: Some(Secret::new(network_mandate_id)),
                        };
                        Ok(AdyenPaymentMethod::AdyenCard(Box::new(adyen_card)))
                    }
                    domain::PaymentMethodData::CardRedirect(_)
                    | domain::PaymentMethodData::Wallet(_)
                    | domain::PaymentMethodData::PayLater(_)
                    | domain::PaymentMethodData::BankRedirect(_)
                    | domain::PaymentMethodData::BankDebit(_)
                    | domain::PaymentMethodData::BankTransfer(_)
                    | domain::PaymentMethodData::Crypto(_)
                    | domain::PaymentMethodData::MandatePayment
                    | domain::PaymentMethodData::Reward
                    | domain::PaymentMethodData::Upi(_)
                    | domain::PaymentMethodData::Voucher(_)
                    | domain::PaymentMethodData::GiftCard(_)
                    | domain::PaymentMethodData::CardToken(_) => {
                        Err(errors::ConnectorError::NotSupported {
                            message: "Network tokenization for payment method".to_string(),
                            connector: "Adyen",
                        })?
                    }
                }
            }
        }?;
        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
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
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
        })
    }
}
impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        &domain::Card,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            &domain::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, card_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let shopper_reference = build_shopper_reference(
            &item.router_data.customer_id,
            item.router_data.merchant_id.clone(),
        );
        let (recurring_processing_model, store_payment_method, _) =
            get_recurring_processing_model(item.router_data)?;
        let browser_info = get_browser_info(item.router_data)?;
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).transpose()?;
        let country_code = get_country_code(item.router_data.get_optional_billing());
        let additional_data = get_additional_data(item.router_data);
        let return_url = item.router_data.request.get_return_url()?;
        let payment_method = AdyenPaymentMethod::try_from(card_data)?;
        let shopper_email = item.router_data.request.email.clone();
        let shopper_name = get_shopper_name(item.router_data.get_optional_billing());

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            telephone_number: None,
            shopper_name,
            shopper_email,
            shopper_locale: None,
            social_security_number: None,
            billing_address,
            delivery_address: None,
            country_code,
            line_items: None,
            shopper_reference,
            store_payment_method,
            channel: None,
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
        })
    }
}

impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        &api_models::payments::BankDebitData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;

    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            &api_models::payments::BankDebitData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_debit_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let recurring_processing_model = get_recurring_processing_model(item.router_data)?.0;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let return_url = item.router_data.request.get_return_url()?;
        let payment_method = AdyenPaymentMethod::try_from(bank_debit_data)?;
        let country_code = get_country_code(item.router_data.get_optional_billing());
        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            browser_info,
            shopper_interaction,
            recurring_processing_model,
            additional_data,
            shopper_name: None,
            shopper_locale: None,
            shopper_email: item.router_data.request.email.clone(),
            social_security_number: None,
            telephone_number: None,
            billing_address: None,
            delivery_address: None,
            country_code,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
        };
        Ok(request)
    }
}

impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        &api_models::payments::VoucherData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;

    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            &api_models::payments::VoucherData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, voucher_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let recurring_processing_model = get_recurring_processing_model(item.router_data)?.0;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let payment_method = AdyenPaymentMethod::try_from(voucher_data)?;
        let return_url = item.router_data.request.get_return_url()?;
        let social_security_number = get_social_security_number(voucher_data);
        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.to_string(),
            return_url,
            browser_info,
            shopper_interaction,
            recurring_processing_model,
            additional_data,
            shopper_name: None,
            shopper_locale: None,
            shopper_email: item.router_data.request.email.clone(),
            social_security_number,
            telephone_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
        };
        Ok(request)
    }
}

impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        &api_models::payments::BankTransferData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;

    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            &api_models::payments::BankTransferData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_transfer_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let payment_method = AdyenPaymentMethod::try_from(bank_transfer_data)?;
        let return_url = item.router_data.request.get_return_url()?;
        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.to_string(),
            return_url,
            browser_info: None,
            shopper_interaction,
            recurring_processing_model: None,
            additional_data: None,
            shopper_name: None,
            shopper_locale: None,
            shopper_email: item.router_data.request.email.clone(),
            social_security_number: None,
            telephone_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
        };
        Ok(request)
    }
}

impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        &api_models::payments::GiftCardData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;

    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            &api_models::payments::GiftCardData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, gift_card_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let return_url = item.router_data.request.get_router_return_url()?;
        let payment_method = AdyenPaymentMethod::try_from(gift_card_data)?;
        let request = AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.to_string(),
            return_url,
            browser_info: None,
            shopper_interaction,
            recurring_processing_model: None,
            additional_data: None,
            shopper_name: None,
            shopper_locale: None,
            shopper_email: item.router_data.request.email.clone(),
            telephone_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            social_security_number: None,
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
        };
        Ok(request)
    }
}

impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        &api_models::payments::BankRedirectData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            &api_models::payments::BankRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_redirect_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item.router_data)?;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let return_url = item.router_data.request.get_return_url()?;
        let payment_method =
            AdyenPaymentMethod::try_from((bank_redirect_data, item.router_data.test_mode))?;
        let (shopper_locale, country) = get_redirect_extra_details(item.router_data)?;
        let line_items = Some(get_line_items(item));

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
            return_url,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
            additional_data,
            telephone_number: None,
            shopper_name: None,
            shopper_email: item.router_data.request.email.clone(),
            shopper_locale,
            social_security_number: None,
            billing_address: None,
            delivery_address: None,
            country_code: country,
            line_items,
            shopper_reference,
            store_payment_method,
            channel: None,
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
        })
    }
}

fn get_redirect_extra_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<(Option<String>, Option<api_enums::CountryAlpha2>), errors::ConnectorError> {
    match item.request.payment_method_data {
        domain::PaymentMethodData::BankRedirect(ref redirect_data) => match redirect_data {
            api_models::payments::BankRedirectData::Sofort {
                country,
                preferred_language,
                ..
            } => Ok((preferred_language.clone(), *country)),
            api_models::payments::BankRedirectData::OpenBankingUk { country, .. } => {
                let country = country.ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "country",
                })?;
                Ok((None, Some(country)))
            }
            _ => Ok((None, None)),
        },
        _ => Ok((None, None)),
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

impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        &domain::WalletData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            &domain::WalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, wallet_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let payment_method = AdyenPaymentMethod::try_from(wallet_data)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let channel = get_channel_type(&item.router_data.request.payment_method_type);
        let (recurring_processing_model, store_payment_method, shopper_reference) =
            get_recurring_processing_model(item.router_data)?;
        let return_url = item.router_data.request.get_router_return_url()?;
        let shopper_email =
            get_shopper_email(&item.router_data.request, store_payment_method.is_some())?;
        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
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
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
        })
    }
}

impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        &domain::PayLaterData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            &domain::PayLaterData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, paylater_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let browser_info = get_browser_info(item.router_data)?;
        let additional_data = get_additional_data(item.router_data);
        let country_code = get_country_code(item.router_data.get_optional_billing());
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let shopper_reference = build_shopper_reference(
            &item.router_data.customer_id,
            item.router_data.merchant_id.clone(),
        );
        let (recurring_processing_model, store_payment_method, _) =
            get_recurring_processing_model(item.router_data)?;
        let return_url = item.router_data.request.get_return_url()?;
        let shopper_name: Option<ShopperName> =
            get_shopper_name(item.router_data.get_optional_billing());
        let shopper_email = item.router_data.request.email.clone();
        let billing_address =
            get_address_info(item.router_data.get_optional_billing()).transpose()?;
        let delivery_address =
            get_address_info(item.router_data.get_optional_shipping()).transpose()?;
        let line_items = Some(get_line_items(item));
        let telephone_number = get_telephone_number(item.router_data);
        let payment_method = AdyenPaymentMethod::try_from((
            paylater_data,
            &country_code,
            &shopper_email,
            &shopper_reference,
            &shopper_name,
            &telephone_number,
            &billing_address,
            &delivery_address,
        ))?;
        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.clone(),
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
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
        })
    }
}

impl<'a>
    TryFrom<(
        &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
        &domain::payments::CardRedirectData,
    )> for AdyenPaymentRequest<'a>
{
    type Error = Error;
    fn try_from(
        value: (
            &AdyenRouterData<&types::PaymentsAuthorizeRouterData>,
            &domain::payments::CardRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, card_redirect_data) = value;
        let amount = get_amount_data(item);
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payment_method = AdyenPaymentMethod::try_from(card_redirect_data)?;
        let shopper_interaction = AdyenShopperInteraction::from(item.router_data);
        let return_url = item.router_data.request.get_return_url()?;
        let shopper_name = get_shopper_name(item.router_data.get_optional_billing());
        let shopper_email = item.router_data.request.email.clone();
        let telephone_number = item
            .router_data
            .get_billing_phone()
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "billing.phone",
            })?
            .number
            .to_owned();
        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference: item.router_data.connector_request_reference_id.to_string(),
            return_url,
            shopper_interaction,
            recurring_processing_model: None,
            browser_info: None,
            additional_data: None,
            telephone_number,
            shopper_name,
            shopper_email,
            shopper_locale: None,
            billing_address: None,
            delivery_address: None,
            country_code: None,
            line_items: None,
            shopper_reference: None,
            store_payment_method: None,
            channel: None,
            social_security_number: None,
            shopper_statement: item.router_data.request.statement_descriptor.clone(),
            shopper_ip: item.router_data.request.get_ip_address_as_optional(),
            metadata: item.router_data.request.metadata.clone(),
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

impl TryFrom<types::PaymentsCancelResponseRouterData<AdyenCancelResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = Error;
    fn try_from(
        item: types::PaymentsCancelResponseRouterData<AdyenCancelResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Pending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.payment_psp_reference,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.reference),
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            AdyenBalanceResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsPreProcessingData, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            AdyenBalanceResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.psp_reference),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            payment_method_balance: Some(types::PaymentMethodBalance {
                amount: item.response.balance.value,
                currency: item.response.balance.currency,
            }),
            ..item.data
        })
    }
}

pub fn get_adyen_response(
    response: Response,
    is_capture_manual: bool,
    status_code: u16,
    pmt: Option<enums::PaymentMethodType>,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status =
        storage_enums::AttemptStatus::foreign_from((is_capture_manual, response.result_code, pmt));
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
            attempt_status: None,
            connector_transaction_id: Some(response.psp_reference.clone()),
        })
    } else {
        None
    };
    let mandate_reference = response
        .additional_data
        .as_ref()
        .and_then(|data| data.recurring_detail_reference.to_owned())
        .map(|mandate_id| types::MandateReference {
            connector_mandate_id: Some(mandate_id.expose()),
            payment_method_id: None,
        });
    let network_txn_id = response.additional_data.and_then(|additional_data| {
        additional_data
            .network_tx_reference
            .map(|network_tx_id| network_tx_id.expose())
    });

    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(response.psp_reference),
        redirection_data: None,
        mandate_reference,
        connector_metadata: None,
        network_txn_id,
        connector_response_reference_id: Some(response.merchant_reference),
        incremental_authorization_allowed: None,
    };
    Ok((status, error, payments_response_data))
}

pub fn get_webhook_response(
    response: AdyenWebhookResponse,
    is_capture_manual: bool,
    is_multiple_capture_psync_flow: bool,
    status_code: u16,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = storage_enums::AttemptStatus::foreign_try_from((
        is_capture_manual,
        response.status.clone(),
    ))?;
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
            reason: response.refusal_reason.clone(),
            status_code,
            attempt_status: None,
            connector_transaction_id: Some(response.transaction_id.clone()),
        })
    } else {
        None
    };

    if is_multiple_capture_psync_flow {
        let capture_sync_response_list = utils::construct_captures_response_hashmap(vec![response]);
        Ok((
            status,
            error,
            types::PaymentsResponseData::MultipleCaptureResponse {
                capture_sync_response_list,
            },
        ))
    } else {
        let payments_response_data = types::PaymentsResponseData::TransactionResponse {
            resource_id: types::ResponseId::ConnectorTransactionId(
                response
                    .payment_reference
                    .unwrap_or(response.transaction_id),
            ),
            redirection_data: None,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(response.merchant_reference_id),
            incremental_authorization_allowed: None,
        };
        Ok((status, error, payments_response_data))
    }
}

pub fn get_redirection_response(
    response: RedirectionResponse,
    is_manual_capture: bool,
    status_code: u16,
    pmt: Option<enums::PaymentMethodType>,
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
        pmt,
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
            reason: response.refusal_reason.to_owned(),
            status_code,
            attempt_status: None,
            connector_transaction_id: response.psp_reference.clone(),
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

    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: match response.psp_reference.as_ref() {
            Some(psp) => types::ResponseId::ConnectorTransactionId(psp.to_string()),
            None => types::ResponseId::NoResponseId,
        },
        redirection_data,
        mandate_reference: None,
        connector_metadata,
        network_txn_id: None,
        connector_response_reference_id: response
            .merchant_reference
            .clone()
            .or(response.psp_reference),
        incremental_authorization_allowed: None,
    };
    Ok((status, error, payments_response_data))
}

pub fn get_present_to_shopper_response(
    response: PresentToShopperResponse,
    is_manual_capture: bool,
    status_code: u16,
    pmt: Option<enums::PaymentMethodType>,
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
        pmt,
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
            reason: response.refusal_reason.to_owned(),
            status_code,
            attempt_status: None,
            connector_transaction_id: response.psp_reference.clone(),
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
        connector_response_reference_id: response
            .merchant_reference
            .clone()
            .or(response.psp_reference),
        incremental_authorization_allowed: None,
    };
    Ok((status, error, payments_response_data))
}

pub fn get_qr_code_response(
    response: QrCodeResponseResponse,
    is_manual_capture: bool,
    status_code: u16,
    pmt: Option<enums::PaymentMethodType>,
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
        pmt,
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
            reason: response.refusal_reason.to_owned(),
            status_code,
            attempt_status: None,
            connector_transaction_id: response.psp_reference.clone(),
        })
    } else {
        None
    };

    let connector_metadata = get_qr_metadata(&response)?;
    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: match response.psp_reference.as_ref() {
            Some(psp) => types::ResponseId::ConnectorTransactionId(psp.to_string()),
            None => types::ResponseId::NoResponseId,
        },
        redirection_data: None,
        mandate_reference: None,
        connector_metadata,
        network_txn_id: None,
        connector_response_reference_id: response
            .merchant_reference
            .clone()
            .or(response.psp_reference),
        incremental_authorization_allowed: None,
    };
    Ok((status, error, payments_response_data))
}

pub fn get_redirection_error_response(
    response: RedirectionErrorResponse,
    is_manual_capture: bool,
    status_code: u16,
    pmt: Option<enums::PaymentMethodType>,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status =
        storage_enums::AttemptStatus::foreign_from((is_manual_capture, response.result_code, pmt));
    let error = Some(types::ErrorResponse {
        code: status.to_string(),
        message: response.refusal_reason.clone(),
        reason: Some(response.refusal_reason),
        status_code,
        attempt_status: None,
        connector_transaction_id: response.psp_reference.clone(),
    });
    // We don't get connector transaction id for redirections in Adyen.
    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::NoResponseId,
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: None,
        incremental_authorization_allowed: None,
    };

    Ok((status, error, payments_response_data))
}

pub fn get_qr_metadata(
    response: &QrCodeResponseResponse,
) -> errors::CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let image_data = crate_utils::QrImage::new_from_data(response.action.qr_code_data.clone())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.clone().as_str()).ok();
    let qr_code_url = response.action.qr_code_url.clone();
    let display_to_timestamp = response
        .additional_data
        .clone()
        .and_then(|additional_data| additional_data.pix_expiration_date)
        .map(|time| utils::get_timestamp_in_milliseconds(&time));

    if let (Some(image_data_url), Some(qr_code_url)) = (image_data_url.clone(), qr_code_url.clone())
    {
        let qr_code_info = payments::QrCodeInformation::QrCodeUrl {
            image_data_url,
            qr_code_url,
            display_to_timestamp,
        };
        Some(qr_code_info.encode_to_value())
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    } else if let (None, Some(qr_code_url)) = (image_data_url.clone(), qr_code_url.clone()) {
        let qr_code_info = payments::QrCodeInformation::QrCodeImageUrl {
            qr_code_url,
            display_to_timestamp,
        };
        Some(qr_code_info.encode_to_value())
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    } else if let (Some(image_data_url), None) = (image_data_url, qr_code_url) {
        let qr_code_info = payments::QrCodeInformation::QrDataUrl {
            image_data_url,
            display_to_timestamp,
        };

        Some(qr_code_info.encode_to_value())
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    } else {
        Ok(None)
    }
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
        | PaymentType::Oxxo
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
        | PaymentType::MomoAtm
        | PaymentType::OnlineBankingCzechRepublic
        | PaymentType::OnlineBankingFinland
        | PaymentType::OnlineBankingPoland
        | PaymentType::OnlineBankingSlovakia
        | PaymentType::OnlineBankingFpx
        | PaymentType::OnlineBankingThailand
        | PaymentType::OpenBankingUK
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
        | PaymentType::Knet
        | PaymentType::Benefit
        | PaymentType::PermataBankTransfer
        | PaymentType::BcaBankTransfer
        | PaymentType::BniVa
        | PaymentType::BriVa
        | PaymentType::CimbVa
        | PaymentType::DanamonVa
        | PaymentType::Giftcard
        | PaymentType::MandiriVa
        | PaymentType::PaySafeCard
        | PaymentType::SevenEleven
        | PaymentType::Lawson
        | PaymentType::MiniStop
        | PaymentType::FamilyMart
        | PaymentType::Seicomart
        | PaymentType::PayEasy
        | PaymentType::Pix => Ok(None),
    }
}

pub fn get_present_to_shopper_metadata(
    response: &PresentToShopperResponse,
) -> errors::CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let reference = response.action.reference.clone();
    let expires_at = response
        .action
        .expires_at
        .map(|time| utils::get_timestamp_in_milliseconds(&time));

    match response.action.payment_method_type {
        PaymentType::Alfamart
        | PaymentType::Indomaret
        | PaymentType::BoletoBancario
        | PaymentType::Oxxo
        | PaymentType::Lawson
        | PaymentType::MiniStop
        | PaymentType::FamilyMart
        | PaymentType::Seicomart
        | PaymentType::PayEasy => {
            let voucher_data = payments::VoucherNextStepData {
                expires_at,
                reference,
                download_url: response.action.download_url.clone(),
                instructions_url: response.action.instructions_url.clone(),
            };

            Some(voucher_data.encode_to_value())
                .transpose()
                .change_context(errors::ConnectorError::ResponseHandlingFailed)
        }
        PaymentType::PermataBankTransfer
        | PaymentType::BcaBankTransfer
        | PaymentType::BniVa
        | PaymentType::BriVa
        | PaymentType::CimbVa
        | PaymentType::DanamonVa
        | PaymentType::Giftcard
        | PaymentType::MandiriVa => {
            let voucher_data = payments::BankTransferInstructions::DokuBankTransferInstructions(
                Box::new(payments::DokuBankTransferInstructions {
                    reference: Secret::new(response.action.reference.clone()),
                    instructions_url: response.action.instructions_url.clone(),
                    expires_at,
                }),
            );

            Some(voucher_data.encode_to_value())
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
        | PaymentType::Knet
        | PaymentType::Benefit
        | PaymentType::MobilePay
        | PaymentType::Momo
        | PaymentType::MomoAtm
        | PaymentType::OnlineBankingCzechRepublic
        | PaymentType::OnlineBankingFinland
        | PaymentType::OnlineBankingPoland
        | PaymentType::OnlineBankingSlovakia
        | PaymentType::OnlineBankingFpx
        | PaymentType::OnlineBankingThailand
        | PaymentType::OpenBankingUK
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
        | PaymentType::PaySafeCard
        | PaymentType::SevenEleven
        | PaymentType::Pix => Ok(None),
    }
}

impl<F, Req>
    TryFrom<(
        types::ResponseRouterData<F, AdyenPaymentResponse, Req, types::PaymentsResponseData>,
        Option<storage_enums::CaptureMethod>,
        bool,
        Option<enums::PaymentMethodType>,
    )> for types::RouterData<F, Req, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        (item, capture_method, is_multiple_capture_psync_flow, pmt): (
            types::ResponseRouterData<F, AdyenPaymentResponse, Req, types::PaymentsResponseData>,
            Option<storage_enums::CaptureMethod>,
            bool,
            Option<enums::PaymentMethodType>,
        ),
    ) -> Result<Self, Self::Error> {
        let is_manual_capture = utils::is_manual_capture(capture_method);
        let (status, error, payment_response_data) = match item.response {
            AdyenPaymentResponse::Response(response) => {
                get_adyen_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::PresentToShopper(response) => {
                get_present_to_shopper_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::QrCodeResponse(response) => {
                get_qr_code_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::RedirectionResponse(response) => {
                get_redirection_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::RedirectionErrorResponse(response) => {
                get_redirection_error_response(*response, is_manual_capture, item.http_code, pmt)?
            }
            AdyenPaymentResponse::WebhookResponse(response) => get_webhook_response(
                *response,
                is_manual_capture,
                is_multiple_capture_psync_flow,
                item.http_code,
            )?,
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

impl TryFrom<&AdyenRouterData<&types::PaymentsCaptureRouterData>> for AdyenCaptureRequest {
    type Error = Error;
    fn try_from(
        item: &AdyenRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let reference = match item.router_data.request.multiple_capture_data.clone() {
            // if multiple capture request, send capture_id as our reference for the capture
            Some(multiple_capture_request_data) => multiple_capture_request_data.capture_reference,
            // if single capture request, send connector_request_reference_id(attempt_id)
            None => item.router_data.connector_request_reference_id.clone(),
        };
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            reference,
            amount: Amount {
                currency: item.router_data.request.currency,
                value: item.amount.to_owned(),
            },
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCaptureResponse {
    merchant_account: Secret<String>,
    payment_psp_reference: String,
    psp_reference: String,
    reference: String,
    status: String,
    amount: Amount,
    merchant_reference: Option<String>,
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<AdyenCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = Error;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<AdyenCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let connector_transaction_id = if item.data.request.multiple_capture_data.is_some() {
            item.response.psp_reference.clone()
        } else {
            item.response.payment_psp_reference
        };
        Ok(Self {
            // From the docs, the only value returned is "received", outcome of refund is available
            // through refund notification webhook
            // For more info: https://docs.adyen.com/online-payments/capture
            status: storage_enums::AttemptStatus::Pending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(connector_transaction_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.reference),
                incremental_authorization_allowed: None,
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
impl<F> TryFrom<&AdyenRouterData<&types::RefundsRouterData<F>>> for AdyenRefundRequest {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            amount: Amount {
                currency: item.router_data.request.currency,
                value: item.router_data.request.refund_amount,
            },
            merchant_refund_reason: item.router_data.request.reason.clone(),
            reference: item.router_data.request.refund_id.clone(),
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
                connector_refund_id: item.response.psp_reference,
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
    pub hmac_signature: Secret<String>,
    pub dispute_status: Option<DisputeStatus>,
    pub chargeback_reason_code: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub defense_period_ends_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct AdyenAmountWH {
    pub value: i64,
    pub currency: storage_enums::Currency,
}

#[derive(Clone, Debug, Deserialize, Serialize, strum::Display, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum WebhookEventCode {
    Authorisation,
    Refund,
    CancelOrRefund,
    Cancellation,
    Capture,
    CaptureFailed,
    RefundFailed,
    RefundReversed,
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

pub fn is_capture_or_cancel_event(event_code: &WebhookEventCode) -> bool {
    matches!(
        event_code,
        WebhookEventCode::Capture
            | WebhookEventCode::CaptureFailed
            | WebhookEventCode::Cancellation
    )
}

pub fn is_refund_event(event_code: &WebhookEventCode) -> bool {
    matches!(
        event_code,
        WebhookEventCode::Refund
            | WebhookEventCode::CancelOrRefund
            | WebhookEventCode::RefundFailed
            | WebhookEventCode::RefundReversed
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

fn is_success_scenario(is_success: String) -> bool {
    is_success.as_str() == "true"
}

impl ForeignFrom<(WebhookEventCode, String, Option<DisputeStatus>)>
    for webhooks::IncomingWebhookEvent
{
    fn foreign_from(
        (code, is_success, dispute_status): (WebhookEventCode, String, Option<DisputeStatus>),
    ) -> Self {
        match code {
            WebhookEventCode::Authorisation => {
                if is_success_scenario(is_success) {
                    Self::PaymentIntentSuccess
                } else {
                    Self::PaymentIntentFailure
                }
            }
            WebhookEventCode::Refund | WebhookEventCode::CancelOrRefund => {
                if is_success_scenario(is_success) {
                    Self::RefundSuccess
                } else {
                    Self::RefundFailure
                }
            }
            WebhookEventCode::Cancellation => {
                if is_success_scenario(is_success) {
                    Self::PaymentIntentCancelled
                } else {
                    Self::PaymentIntentCancelFailure
                }
            }
            WebhookEventCode::RefundFailed | WebhookEventCode::RefundReversed => {
                Self::RefundFailure
            }
            WebhookEventCode::NotificationOfChargeback => Self::DisputeOpened,
            WebhookEventCode::Chargeback => match dispute_status {
                Some(DisputeStatus::Won) => Self::DisputeWon,
                Some(DisputeStatus::Lost) | None => Self::DisputeLost,
                Some(_) => Self::DisputeOpened,
            },
            WebhookEventCode::ChargebackReversed => match dispute_status {
                Some(DisputeStatus::Pending) => Self::DisputeChallenged,
                _ => Self::DisputeWon,
            },
            WebhookEventCode::SecondChargeback => Self::DisputeLost,
            WebhookEventCode::PrearbitrationWon => match dispute_status {
                Some(DisputeStatus::Pending) => Self::DisputeOpened,
                _ => Self::DisputeWon,
            },
            WebhookEventCode::PrearbitrationLost => Self::DisputeLost,
            WebhookEventCode::Capture => {
                if is_success_scenario(is_success) {
                    Self::PaymentIntentCaptureSuccess
                } else {
                    Self::PaymentIntentCaptureFailure
                }
            }
            WebhookEventCode::CaptureFailed => Self::PaymentIntentCaptureFailure,
            WebhookEventCode::Unknown => Self::EventNotSupported,
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

impl From<AdyenNotificationRequestItemWH> for AdyenWebhookResponse {
    fn from(notif: AdyenNotificationRequestItemWH) -> Self {
        Self {
            transaction_id: notif.psp_reference,
            payment_reference: notif.original_reference,
            //Translating into custom status so that it can be clearly mapped to out attempt_status
            status: match notif.event_code {
                WebhookEventCode::Authorisation => {
                    if is_success_scenario(notif.success) {
                        AdyenWebhookStatus::Authorised
                    } else {
                        AdyenWebhookStatus::AuthorisationFailed
                    }
                }
                WebhookEventCode::Cancellation => {
                    if is_success_scenario(notif.success) {
                        AdyenWebhookStatus::Cancelled
                    } else {
                        AdyenWebhookStatus::CancelFailed
                    }
                }
                WebhookEventCode::Capture => {
                    if is_success_scenario(notif.success) {
                        AdyenWebhookStatus::Captured
                    } else {
                        AdyenWebhookStatus::CaptureFailed
                    }
                }
                WebhookEventCode::CaptureFailed => AdyenWebhookStatus::CaptureFailed,
                WebhookEventCode::CancelOrRefund
                | WebhookEventCode::Refund
                | WebhookEventCode::RefundFailed
                | WebhookEventCode::RefundReversed
                | WebhookEventCode::NotificationOfChargeback
                | WebhookEventCode::Chargeback
                | WebhookEventCode::ChargebackReversed
                | WebhookEventCode::SecondChargeback
                | WebhookEventCode::PrearbitrationWon
                | WebhookEventCode::PrearbitrationLost
                | WebhookEventCode::Unknown => AdyenWebhookStatus::UnexpectedEvent,
            },
            amount: Some(Amount {
                value: notif.amount.value,
                currency: notif.amount.currency,
            }),
            merchant_reference_id: notif.merchant_reference,
            refusal_reason: None,
            refusal_reason_code: None,
            event_code: notif.event_code,
        }
    }
}

//This will be triggered in Psync handler of webhook response
impl utils::MultipleCaptureSyncResponse for AdyenWebhookResponse {
    fn get_connector_capture_id(&self) -> String {
        self.transaction_id.clone()
    }

    fn get_capture_attempt_status(&self) -> enums::AttemptStatus {
        match self.status {
            AdyenWebhookStatus::Captured => enums::AttemptStatus::Charged,
            _ => enums::AttemptStatus::CaptureFailed,
        }
    }

    fn is_capture_response(&self) -> bool {
        matches!(
            self.event_code,
            WebhookEventCode::Capture | WebhookEventCode::CaptureFailed
        )
    }

    fn get_connector_reference_id(&self) -> Option<String> {
        Some(self.merchant_reference_id.clone())
    }

    fn get_amount_captured(&self) -> Option<i64> {
        self.amount
            .as_ref()
            .map(|amount_struct| amount_struct.value)
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
    #[serde(flatten)]
    payment_data: PayoutPaymentMethodData,
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
#[serde(untagged)]
pub enum PayoutPaymentMethodData {
    PayoutBankData(PayoutBankData),
    PayoutWalletData(PayoutWalletData),
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutBankData {
    bank: PayoutBankDetails,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutWalletData {
    selected_brand: PayoutBrand,
    additional_data: PayoutAdditionalData,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PayoutBrand {
    Paypal,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayoutAdditionalData {
    token_data_type: PayoutTokenDataType,
    email_id: Email,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
enum PayoutTokenDataType {
    PayPal,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayoutBankDetails {
    iban: Secret<String>,
    owner_name: Secret<String>,
    bank_city: Option<String>,
    bank_name: Option<String>,
    bic: Option<Secret<String>>,
    country_code: Option<storage_enums::CountryAlpha2>,
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
    payment_method_type: String,
    number: CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    holder_name: Secret<String>,
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
    GenericFulfillRequest(PayoutFulfillGenericRequest),
    Card(Box<PayoutFulfillCardRequest>),
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutFulfillGenericRequest {
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

#[cfg(feature = "payouts")]
impl TryFrom<&PayoutMethodData> for PayoutCardDetails {
    type Error = Error;
    fn try_from(item: &PayoutMethodData) -> Result<Self, Self::Error> {
        match item {
            PayoutMethodData::Card(card) => Ok(Self {
                payment_method_type: "scheme".to_string(), // FIXME: Remove hardcoding
                number: card.card_number.clone(),
                expiry_month: card.expiry_month.clone(),
                expiry_year: card.expiry_year.clone(),
                holder_name: card
                    .card_holder_name
                    .clone()
                    .get_required_value("card_holder_name")
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "payout_method_data.card.holder_name",
                    })?,
            }),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payout_method_data.card",
            })?,
        }
    }
}

// Payouts eligibility request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&AdyenRouterData<&types::PayoutsRouterData<F>>> for AdyenPayoutEligibilityRequest {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&types::PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payout_method_data =
            PayoutCardDetails::try_from(&item.router_data.get_payout_method_data()?)?;
        Ok(Self {
            amount: Amount {
                currency: item.router_data.request.destination_currency,
                value: item.amount.to_owned(),
            },
            merchant_account: auth_type.merchant_account,
            payment_method: payout_method_data,
            reference: item.router_data.request.payout_id.clone(),
            shopper_reference: item.router_data.merchant_id.clone(),
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
impl<F> TryFrom<&AdyenRouterData<&types::PayoutsRouterData<F>>> for AdyenPayoutCreateRequest {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&types::PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let merchant_account = auth_type.merchant_account;
        let (owner_name, customer_email) = item
            .router_data
            .request
            .customer_details
            .to_owned()
            .map_or((None, None), |c| (c.name, c.email));
        let owner_name = owner_name.get_required_value("owner_name").change_context(
            errors::ConnectorError::MissingRequiredField {
                field_name: "payout_method_data.bank.owner_name",
            },
        )?;

        match item.router_data.get_payout_method_data()? {
            PayoutMethodData::Card(_) => Err(errors::ConnectorError::NotSupported {
                message: "Card payout creation is not supported".to_string(),
                connector: "Adyen",
            })?,
            PayoutMethodData::Bank(bd) => {
                let bank_details = match bd {
                    payouts::BankPayout::Sepa(b) => PayoutBankDetails {
                        bank_name: b.bank_name,
                        country_code: b.bank_country_code,
                        bank_city: b.bank_city,
                        owner_name,
                        bic: b.bic,
                        iban: b.iban,
                        tax_id: None,
                    },
                    payouts::BankPayout::Ach(..) => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfer via ACH is not supported".to_string(),
                        connector: "Adyen",
                    })?,
                    payouts::BankPayout::Bacs(..) => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfer via Bacs is not supported".to_string(),
                        connector: "Adyen",
                    })?,
                    payouts::BankPayout::Pix(..) => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfer via Pix is not supported".to_string(),
                        connector: "Adyen",
                    })?,
                };
                let bank_data = PayoutBankData { bank: bank_details };
                let address: &payments::AddressDetails = item.router_data.get_billing_address()?;
                Ok(Self {
                    amount: Amount {
                        value: item.amount.to_owned(),
                        currency: item.router_data.request.destination_currency,
                    },
                    recurring: RecurringContract {
                        contract: Contract::Payout,
                    },
                    merchant_account,
                    payment_data: PayoutPaymentMethodData::PayoutBankData(bank_data),
                    reference: item.router_data.request.payout_id.to_owned(),
                    shopper_reference: item.router_data.merchant_id.to_owned(),
                    shopper_email: customer_email,
                    shopper_name: ShopperName {
                        first_name: Some(address.get_first_name()?.to_owned()), // it is a required field for payouts
                        last_name: Some(address.get_last_name()?.to_owned()), // it is a required field for payouts
                    },
                    date_of_birth: None,
                    entity_type: Some(item.router_data.request.entity_type),
                    nationality: get_country_code(item.router_data.get_optional_billing()),
                    billing_address: get_address_info(item.router_data.get_optional_billing())
                        .transpose()?,
                })
            }
            PayoutMethodData::Wallet(wallet_data) => {
                let additional_data = match wallet_data {
                    api_models::payouts::Wallet::Paypal(paypal_data) => PayoutAdditionalData {
                        token_data_type: PayoutTokenDataType::PayPal,
                        email_id: paypal_data.email.clone().ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "email_address",
                            },
                        )?,
                    },
                };
                let address: &payments::AddressDetails = item.router_data.get_billing_address()?;
                let payout_wallet = PayoutWalletData {
                    selected_brand: PayoutBrand::Paypal,
                    additional_data,
                };
                Ok(Self {
                    amount: Amount {
                        value: item.amount.to_owned(),
                        currency: item.router_data.request.destination_currency,
                    },
                    recurring: RecurringContract {
                        contract: Contract::Payout,
                    },
                    merchant_account,
                    payment_data: PayoutPaymentMethodData::PayoutWalletData(payout_wallet),
                    reference: item.router_data.request.payout_id.to_owned(),
                    shopper_reference: item.router_data.merchant_id.to_owned(),
                    shopper_email: customer_email,
                    shopper_name: ShopperName {
                        first_name: Some(address.get_first_name()?.to_owned()), // it is a required field for payouts
                        last_name: Some(address.get_last_name()?.to_owned()), // it is a required field for payouts
                    },
                    date_of_birth: None,
                    entity_type: Some(item.router_data.request.entity_type),
                    nationality: get_country_code(item.router_data.get_optional_billing()),
                    billing_address: get_address_info(item.router_data.get_optional_billing())
                        .transpose()?,
                })
            }
        }
    }
}

// Payouts fulfill request transform
#[cfg(feature = "payouts")]
impl<F> TryFrom<&AdyenRouterData<&types::PayoutsRouterData<F>>> for AdyenPayoutFulfillRequest {
    type Error = Error;
    fn try_from(item: &AdyenRouterData<&types::PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payout_type = item.router_data.request.payout_type.to_owned();
        let merchant_account = auth_type.merchant_account;
        match payout_type {
            storage_enums::PayoutType::Bank | storage_enums::PayoutType::Wallet => {
                Ok(Self::GenericFulfillRequest(PayoutFulfillGenericRequest {
                    merchant_account,
                    original_reference: item
                        .router_data
                        .request
                        .connector_payout_id
                        .clone()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "connector_payout_id",
                        })?,
                }))
            }
            storage_enums::PayoutType::Card => {
                let address = item.router_data.get_billing_address()?;
                Ok(Self::Card(Box::new(PayoutFulfillCardRequest {
                    amount: Amount {
                        value: item.amount.to_owned(),
                        currency: item.router_data.request.destination_currency,
                    },
                    card: PayoutCardDetails::try_from(&item.router_data.get_payout_method_data()?)?,
                    billing_address: get_address_info(item.router_data.get_billing().ok())
                        .transpose()?,
                    merchant_account,
                    reference: item.router_data.request.payout_id.clone(),
                    shopper_name: ShopperName {
                        first_name: Some(address.get_first_name()?.to_owned()), // it is a required field for payouts
                        last_name: Some(address.get_last_name()?.to_owned()), // it is a required field for payouts
                    },
                    nationality: get_country_code(item.router_data.get_optional_billing()),
                    entity_type: Some(item.router_data.request.entity_type),
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
