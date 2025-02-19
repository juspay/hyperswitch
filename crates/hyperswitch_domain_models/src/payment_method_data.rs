use api_models::{
    mandates,
    payment_methods::{self},
    payments::{
        additional_info as payment_additional_types, AmazonPayRedirectData, ExtendedCardInfo,
    },
};
use common_enums::enums as api_enums;
use common_utils::{
    id_type,
    new_type::{
        MaskedBankAccount, MaskedIban, MaskedRoutingNumber, MaskedSortCode, MaskedUpiVpaId,
    },
    pii::{self, Email},
};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::Date;

// We need to derive Serialize and Deserialize because some parts of payment method data are being
// stored in the database as serde_json::Value
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum PaymentMethodData {
    Card(Card),
    CardDetailsForNetworkTransactionId(CardDetailsForNetworkTransactionId),
    CardRedirect(CardRedirectData),
    Wallet(WalletData),
    PayLater(PayLaterData),
    BankRedirect(BankRedirectData),
    BankDebit(BankDebitData),
    BankTransfer(Box<BankTransferData>),
    Crypto(CryptoData),
    MandatePayment,
    Reward,
    RealTimePayment(Box<RealTimePaymentData>),
    Upi(UpiData),
    Voucher(VoucherData),
    GiftCard(Box<GiftCardData>),
    CardToken(CardToken),
    OpenBanking(OpenBankingData),
    NetworkToken(NetworkTokenData),
    MobilePayment(MobilePaymentData),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplePayFlow {
    Simplified(api_models::payments::PaymentProcessingDetails),
    Manual,
}

impl PaymentMethodData {
    pub fn get_payment_method(&self) -> Option<common_enums::PaymentMethod> {
        match self {
            Self::Card(_) | Self::NetworkToken(_) | Self::CardDetailsForNetworkTransactionId(_) => {
                Some(common_enums::PaymentMethod::Card)
            }
            Self::CardRedirect(_) => Some(common_enums::PaymentMethod::CardRedirect),
            Self::Wallet(_) => Some(common_enums::PaymentMethod::Wallet),
            Self::PayLater(_) => Some(common_enums::PaymentMethod::PayLater),
            Self::BankRedirect(_) => Some(common_enums::PaymentMethod::BankRedirect),
            Self::BankDebit(_) => Some(common_enums::PaymentMethod::BankDebit),
            Self::BankTransfer(_) => Some(common_enums::PaymentMethod::BankTransfer),
            Self::Crypto(_) => Some(common_enums::PaymentMethod::Crypto),
            Self::Reward => Some(common_enums::PaymentMethod::Reward),
            Self::RealTimePayment(_) => Some(common_enums::PaymentMethod::RealTimePayment),
            Self::Upi(_) => Some(common_enums::PaymentMethod::Upi),
            Self::Voucher(_) => Some(common_enums::PaymentMethod::Voucher),
            Self::GiftCard(_) => Some(common_enums::PaymentMethod::GiftCard),
            Self::OpenBanking(_) => Some(common_enums::PaymentMethod::OpenBanking),
            Self::MobilePayment(_) => Some(common_enums::PaymentMethod::MobilePayment),
            Self::CardToken(_) | Self::MandatePayment => None,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct Card {
    pub card_number: cards::CardNumber,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_cvc: Secret<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct CardDetailsForNetworkTransactionId {
    pub card_number: cards::CardNumber,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
}

impl CardDetailsForNetworkTransactionId {
    pub fn get_nti_and_card_details_for_mit_flow(
        recurring_details: mandates::RecurringDetails,
    ) -> Option<(api_models::payments::MandateReferenceId, Self)> {
        let network_transaction_id_and_card_details = match recurring_details {
            mandates::RecurringDetails::NetworkTransactionIdAndCardDetails(
                network_transaction_id_and_card_details,
            ) => Some(network_transaction_id_and_card_details),
            mandates::RecurringDetails::MandateId(_)
            | mandates::RecurringDetails::PaymentMethodId(_)
            | mandates::RecurringDetails::ProcessorPaymentToken(_) => None,
        }?;

        let mandate_reference_id = api_models::payments::MandateReferenceId::NetworkMandateId(
            network_transaction_id_and_card_details
                .network_transaction_id
                .peek()
                .to_string(),
        );

        Some((
            mandate_reference_id,
            network_transaction_id_and_card_details.clone().into(),
        ))
    }
}

impl From<mandates::NetworkTransactionIdAndCardDetails> for CardDetailsForNetworkTransactionId {
    fn from(card_details_for_nti: mandates::NetworkTransactionIdAndCardDetails) -> Self {
        Self {
            card_number: card_details_for_nti.card_number,
            card_exp_month: card_details_for_nti.card_exp_month,
            card_exp_year: card_details_for_nti.card_exp_year,
            card_issuer: card_details_for_nti.card_issuer,
            card_network: card_details_for_nti.card_network,
            card_type: card_details_for_nti.card_type,
            card_issuing_country: card_details_for_nti.card_issuing_country,
            bank_code: card_details_for_nti.bank_code,
            nick_name: card_details_for_nti.nick_name,
            card_holder_name: card_details_for_nti.card_holder_name,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum CardRedirectData {
    Knet {},
    Benefit {},
    MomoAtm {},
    CardRedirect {},
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum PayLaterData {
    KlarnaRedirect {},
    KlarnaSdk { token: String },
    AffirmRedirect {},
    AfterpayClearpayRedirect {},
    PayBrightRedirect {},
    WalleyRedirect {},
    AlmaRedirect {},
    AtomeRedirect {},
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum WalletData {
    AliPayQr(Box<AliPayQr>),
    AliPayRedirect(AliPayRedirection),
    AliPayHkRedirect(AliPayHkRedirection),
    AmazonPayRedirect(Box<AmazonPayRedirectData>),
    MomoRedirect(MomoRedirection),
    KakaoPayRedirect(KakaoPayRedirection),
    GoPayRedirect(GoPayRedirection),
    GcashRedirect(GcashRedirection),
    ApplePay(ApplePayWalletData),
    ApplePayRedirect(Box<ApplePayRedirectData>),
    ApplePayThirdPartySdk(Box<ApplePayThirdPartySdkData>),
    DanaRedirect {},
    GooglePay(GooglePayWalletData),
    GooglePayRedirect(Box<GooglePayRedirectData>),
    GooglePayThirdPartySdk(Box<GooglePayThirdPartySdkData>),
    MbWayRedirect(Box<MbWayRedirection>),
    MobilePayRedirect(Box<MobilePayRedirection>),
    PaypalRedirect(PaypalRedirection),
    PaypalSdk(PayPalWalletData),
    Paze(PazeWalletData),
    SamsungPay(Box<SamsungPayWalletData>),
    TwintRedirect {},
    VippsRedirect {},
    TouchNGoRedirect(Box<TouchNGoRedirection>),
    WeChatPayRedirect(Box<WeChatPayRedirection>),
    WeChatPayQr(Box<WeChatPayQr>),
    CashappQr(Box<CashappQr>),
    SwishQr(SwishQrData),
    Mifinity(MifinityData),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MifinityData {
    pub date_of_birth: Secret<Date>,
    pub language_preference: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PazeWalletData {
    pub complete_response: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayWalletData {
    pub payment_credential: SamsungPayWalletCredentials,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayWalletCredentials {
    pub method: Option<String>,
    pub recurring_payment: Option<bool>,
    pub card_brand: common_enums::SamsungPayCardBrand,
    pub dpan_last_four_digits: Option<String>,
    #[serde(rename = "card_last4digits")]
    pub card_last_four_digits: String,
    #[serde(rename = "3_d_s")]
    pub token_data: SamsungPayTokenData,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayTokenData {
    #[serde(rename = "type")]
    pub three_ds_type: Option<String>,
    pub version: String,
    pub data: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GooglePayWalletData {
    /// The type of payment method
    pub pm_type: String,
    /// User-facing message to describe the payment method that funds this transaction.
    pub description: String,
    /// The information of the payment method
    pub info: GooglePayPaymentMethodInfo,
    /// The tokenization data of Google pay
    pub tokenization_data: GpayTokenizationData,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ApplePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GooglePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GooglePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ApplePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WeChatPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WeChatPay {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WeChatPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CashappQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaypalRedirection {
    /// paypal's email address
    pub email: Option<Email>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AliPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AliPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AliPayHkRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MomoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct KakaoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GcashRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MobilePayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MbWayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GooglePayPaymentMethodInfo {
    /// The name of the card network
    pub card_network: String,
    /// The details of the card
    pub card_details: String,
    //assurance_details of the card
    pub assurance_details: Option<GooglePayAssuranceDetails>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GooglePayAssuranceDetails {
    ///indicates that Cardholder possession validation has been performed
    pub card_holder_authenticated: bool,
    /// indicates that identification and verifications (ID&V) was performed
    pub account_verified: bool,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PayPalWalletData {
    /// Token generated for the Apple pay
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TouchNGoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SwishQrData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GpayTokenizationData {
    /// The type of the token
    pub token_type: String,
    /// Token generated for the wallet
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ApplePayWalletData {
    /// The payment data of Apple pay
    pub payment_data: String,
    /// The payment method of Apple pay
    pub payment_method: ApplepayPaymentMethod,
    /// The unique identifier for the transaction
    pub transaction_identifier: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ApplepayPaymentMethod {
    pub display_name: String,
    pub network: String,
    pub pm_type: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RealTimePaymentData {
    DuitNow {},
    Fps {},
    PromptPay {},
    VietQr {},
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum BankRedirectData {
    BancontactCard {
        card_number: Option<cards::CardNumber>,
        card_exp_month: Option<Secret<String>>,
        card_exp_year: Option<Secret<String>>,
        card_holder_name: Option<Secret<String>>,
    },
    Bizum {},
    Blik {
        blik_code: Option<String>,
    },
    Eps {
        bank_name: Option<common_enums::BankNames>,
        country: Option<api_enums::CountryAlpha2>,
    },
    Giropay {
        bank_account_bic: Option<Secret<String>>,
        bank_account_iban: Option<Secret<String>>,
        country: Option<api_enums::CountryAlpha2>,
    },
    Ideal {
        bank_name: Option<common_enums::BankNames>,
    },
    Interac {
        country: Option<api_enums::CountryAlpha2>,
        email: Option<Email>,
    },
    OnlineBankingCzechRepublic {
        issuer: common_enums::BankNames,
    },
    OnlineBankingFinland {
        email: Option<Email>,
    },
    OnlineBankingPoland {
        issuer: common_enums::BankNames,
    },
    OnlineBankingSlovakia {
        issuer: common_enums::BankNames,
    },
    OpenBankingUk {
        issuer: Option<common_enums::BankNames>,
        country: Option<api_enums::CountryAlpha2>,
    },
    Przelewy24 {
        bank_name: Option<common_enums::BankNames>,
    },
    Sofort {
        country: Option<api_enums::CountryAlpha2>,
        preferred_language: Option<String>,
    },
    Trustly {
        country: Option<api_enums::CountryAlpha2>,
    },
    OnlineBankingFpx {
        issuer: common_enums::BankNames,
    },
    OnlineBankingThailand {
        issuer: common_enums::BankNames,
    },
    LocalBankRedirect {},
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenBankingData {
    OpenBankingPIS {},
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CryptoData {
    pub pay_currency: Option<String>,
    pub network: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpiData {
    UpiCollect(UpiCollectData),
    UpiIntent(UpiIntentData),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UpiCollectData {
    pub vpa_id: Option<Secret<String, pii::UpiVpaMaskingStrategy>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct UpiIntentData {}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoucherData {
    Boleto(Box<BoletoVoucherData>),
    Efecty,
    PagoEfectivo,
    RedCompra,
    RedPagos,
    Alfamart(Box<AlfamartVoucherData>),
    Indomaret(Box<IndomaretVoucherData>),
    Oxxo,
    SevenEleven(Box<JCSVoucherData>),
    Lawson(Box<JCSVoucherData>),
    MiniStop(Box<JCSVoucherData>),
    FamilyMart(Box<JCSVoucherData>),
    Seicomart(Box<JCSVoucherData>),
    PayEasy(Box<JCSVoucherData>),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BoletoVoucherData {
    /// The shopper's social security number
    pub social_security_number: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AlfamartVoucherData {}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IndomaretVoucherData {}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JCSVoucherData {}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GiftCardData {
    Givex(GiftCardDetails),
    PaySafeCard {},
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GiftCardDetails {
    /// The gift card number
    pub number: Secret<String>,
    /// The card verification code.
    pub cvc: Secret<String>,
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct CardToken {
    /// The card holder's name
    pub card_holder_name: Option<Secret<String>>,

    /// The CVC number for the card
    pub card_cvc: Option<Secret<String>>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BankDebitData {
    AchBankDebit {
        account_number: Secret<String>,
        routing_number: Secret<String>,
        card_holder_name: Option<Secret<String>>,
        bank_account_holder_name: Option<Secret<String>>,
        bank_name: Option<common_enums::BankNames>,
        bank_type: Option<common_enums::BankType>,
        bank_holder_type: Option<common_enums::BankHolderType>,
    },
    SepaBankDebit {
        iban: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
    BecsBankDebit {
        account_number: Secret<String>,
        bsb_number: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
    BacsBankDebit {
        account_number: Secret<String>,
        sort_code: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BankTransferData {
    AchBankTransfer {},
    SepaBankTransfer {},
    BacsBankTransfer {},
    MultibancoBankTransfer {},
    PermataBankTransfer {},
    BcaBankTransfer {},
    BniVaBankTransfer {},
    BriVaBankTransfer {},
    CimbVaBankTransfer {},
    DanamonVaBankTransfer {},
    MandiriVaBankTransfer {},
    Pix {
        /// Unique key for pix transfer
        pix_key: Option<Secret<String>>,
        /// CPF is a Brazilian tax identification number
        cpf: Option<Secret<String>>,
        /// CNPJ is a Brazilian company tax identification number
        cnpj: Option<Secret<String>>,
    },
    Pse {},
    LocalBankTransfer {
        bank_code: Option<String>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SepaAndBacsBillingDetails {
    /// The Email ID for SEPA and BACS billing
    pub email: Email,
    /// The billing name for SEPA and BACS billing
    pub name: Secret<String>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct NetworkTokenData {
    pub token_number: cards::CardNumber,
    pub token_exp_month: Secret<String>,
    pub token_exp_year: Secret<String>,
    pub token_cryptogram: Option<Secret<String>>,
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code: Option<String>,
    pub nick_name: Option<Secret<String>>,
    pub eci: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct NetworkTokenData {
    pub network_token: cards::NetworkToken,
    pub network_token_exp_month: Secret<String>,
    pub network_token_exp_year: Secret<String>,
    pub cryptogram: Option<Secret<String>>,
    pub card_issuer: Option<String>, //since network token is tied to card, so its issuer will be same as card issuer
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<payment_methods::CardType>,
    pub card_issuing_country: Option<String>,
    pub bank_code: Option<String>,
    pub card_holder_name: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub eci: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct NetworkTokenDetails {
    pub network_token: cards::NetworkToken,
    pub network_token_exp_month: Secret<String>,
    pub network_token_exp_year: Secret<String>,
    pub card_issuer: Option<String>, //since network token is tied to card, so its issuer will be same as card issuer
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<payment_methods::CardType>,
    pub card_issuing_country: Option<api_enums::CountryAlpha2>,
    pub card_holder_name: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobilePaymentData {
    DirectCarrierBilling {
        /// The phone number of the user
        msisdn: String,
        /// Unique user identifier
        client_uid: Option<String>,
    },
}

impl From<api_models::payments::PaymentMethodData> for PaymentMethodData {
    fn from(api_model_payment_method_data: api_models::payments::PaymentMethodData) -> Self {
        match api_model_payment_method_data {
            api_models::payments::PaymentMethodData::Card(card_data) => {
                Self::Card(Card::from(card_data))
            }
            api_models::payments::PaymentMethodData::CardRedirect(card_redirect) => {
                Self::CardRedirect(From::from(card_redirect))
            }
            api_models::payments::PaymentMethodData::Wallet(wallet_data) => {
                Self::Wallet(From::from(wallet_data))
            }
            api_models::payments::PaymentMethodData::PayLater(pay_later_data) => {
                Self::PayLater(From::from(pay_later_data))
            }
            api_models::payments::PaymentMethodData::BankRedirect(bank_redirect_data) => {
                Self::BankRedirect(From::from(bank_redirect_data))
            }
            api_models::payments::PaymentMethodData::BankDebit(bank_debit_data) => {
                Self::BankDebit(From::from(bank_debit_data))
            }
            api_models::payments::PaymentMethodData::BankTransfer(bank_transfer_data) => {
                Self::BankTransfer(Box::new(From::from(*bank_transfer_data)))
            }
            api_models::payments::PaymentMethodData::Crypto(crypto_data) => {
                Self::Crypto(From::from(crypto_data))
            }
            api_models::payments::PaymentMethodData::MandatePayment => Self::MandatePayment,
            api_models::payments::PaymentMethodData::Reward => Self::Reward,
            api_models::payments::PaymentMethodData::RealTimePayment(real_time_payment_data) => {
                Self::RealTimePayment(Box::new(From::from(*real_time_payment_data)))
            }
            api_models::payments::PaymentMethodData::Upi(upi_data) => {
                Self::Upi(From::from(upi_data))
            }
            api_models::payments::PaymentMethodData::Voucher(voucher_data) => {
                Self::Voucher(From::from(voucher_data))
            }
            api_models::payments::PaymentMethodData::GiftCard(gift_card) => {
                Self::GiftCard(Box::new(From::from(*gift_card)))
            }
            api_models::payments::PaymentMethodData::CardToken(card_token) => {
                Self::CardToken(From::from(card_token))
            }
            api_models::payments::PaymentMethodData::OpenBanking(ob_data) => {
                Self::OpenBanking(From::from(ob_data))
            }
            api_models::payments::PaymentMethodData::MobilePayment(mobile_payment_data) => {
                Self::MobilePayment(From::from(mobile_payment_data))
            }
        }
    }
}

impl From<api_models::payments::Card> for Card {
    fn from(value: api_models::payments::Card) -> Self {
        let api_models::payments::Card {
            card_number,
            card_exp_month,
            card_exp_year,
            card_holder_name,
            card_cvc,
            card_issuer,
            card_network,
            card_type,
            card_issuing_country,
            bank_code,
            nick_name,
        } = value;

        Self {
            card_number,
            card_exp_month,
            card_exp_year,
            card_cvc,
            card_issuer,
            card_network,
            card_type,
            card_issuing_country,
            bank_code,
            nick_name,
            card_holder_name,
        }
    }
}

impl From<api_models::payments::CardRedirectData> for CardRedirectData {
    fn from(value: api_models::payments::CardRedirectData) -> Self {
        match value {
            api_models::payments::CardRedirectData::Knet {} => Self::Knet {},
            api_models::payments::CardRedirectData::Benefit {} => Self::Benefit {},
            api_models::payments::CardRedirectData::MomoAtm {} => Self::MomoAtm {},
            api_models::payments::CardRedirectData::CardRedirect {} => Self::CardRedirect {},
        }
    }
}

impl From<CardRedirectData> for api_models::payments::CardRedirectData {
    fn from(value: CardRedirectData) -> Self {
        match value {
            CardRedirectData::Knet {} => Self::Knet {},
            CardRedirectData::Benefit {} => Self::Benefit {},
            CardRedirectData::MomoAtm {} => Self::MomoAtm {},
            CardRedirectData::CardRedirect {} => Self::CardRedirect {},
        }
    }
}

impl From<api_models::payments::WalletData> for WalletData {
    fn from(value: api_models::payments::WalletData) -> Self {
        match value {
            api_models::payments::WalletData::AliPayQr(_) => Self::AliPayQr(Box::new(AliPayQr {})),
            api_models::payments::WalletData::AliPayRedirect(_) => {
                Self::AliPayRedirect(AliPayRedirection {})
            }
            api_models::payments::WalletData::AliPayHkRedirect(_) => {
                Self::AliPayHkRedirect(AliPayHkRedirection {})
            }
            api_models::payments::WalletData::AmazonPayRedirect(_) => {
                Self::AmazonPayRedirect(Box::new(AmazonPayRedirectData {}))
            }
            api_models::payments::WalletData::MomoRedirect(_) => {
                Self::MomoRedirect(MomoRedirection {})
            }
            api_models::payments::WalletData::KakaoPayRedirect(_) => {
                Self::KakaoPayRedirect(KakaoPayRedirection {})
            }
            api_models::payments::WalletData::GoPayRedirect(_) => {
                Self::GoPayRedirect(GoPayRedirection {})
            }
            api_models::payments::WalletData::GcashRedirect(_) => {
                Self::GcashRedirect(GcashRedirection {})
            }
            api_models::payments::WalletData::ApplePay(apple_pay_data) => {
                Self::ApplePay(ApplePayWalletData::from(apple_pay_data))
            }
            api_models::payments::WalletData::ApplePayRedirect(_) => {
                Self::ApplePayRedirect(Box::new(ApplePayRedirectData {}))
            }
            api_models::payments::WalletData::ApplePayThirdPartySdk(_) => {
                Self::ApplePayThirdPartySdk(Box::new(ApplePayThirdPartySdkData {}))
            }
            api_models::payments::WalletData::DanaRedirect {} => Self::DanaRedirect {},
            api_models::payments::WalletData::GooglePay(google_pay_data) => {
                Self::GooglePay(GooglePayWalletData::from(google_pay_data))
            }
            api_models::payments::WalletData::GooglePayRedirect(_) => {
                Self::GooglePayRedirect(Box::new(GooglePayRedirectData {}))
            }
            api_models::payments::WalletData::GooglePayThirdPartySdk(_) => {
                Self::GooglePayThirdPartySdk(Box::new(GooglePayThirdPartySdkData {}))
            }
            api_models::payments::WalletData::MbWayRedirect(..) => {
                Self::MbWayRedirect(Box::new(MbWayRedirection {}))
            }
            api_models::payments::WalletData::MobilePayRedirect(_) => {
                Self::MobilePayRedirect(Box::new(MobilePayRedirection {}))
            }
            api_models::payments::WalletData::PaypalRedirect(paypal_redirect_data) => {
                Self::PaypalRedirect(PaypalRedirection {
                    email: paypal_redirect_data.email,
                })
            }
            api_models::payments::WalletData::PaypalSdk(paypal_sdk_data) => {
                Self::PaypalSdk(PayPalWalletData {
                    token: paypal_sdk_data.token,
                })
            }
            api_models::payments::WalletData::Paze(paze_data) => {
                Self::Paze(PazeWalletData::from(paze_data))
            }
            api_models::payments::WalletData::SamsungPay(samsung_pay_data) => {
                Self::SamsungPay(Box::new(SamsungPayWalletData::from(samsung_pay_data)))
            }
            api_models::payments::WalletData::TwintRedirect {} => Self::TwintRedirect {},
            api_models::payments::WalletData::VippsRedirect {} => Self::VippsRedirect {},
            api_models::payments::WalletData::TouchNGoRedirect(_) => {
                Self::TouchNGoRedirect(Box::new(TouchNGoRedirection {}))
            }
            api_models::payments::WalletData::WeChatPayRedirect(_) => {
                Self::WeChatPayRedirect(Box::new(WeChatPayRedirection {}))
            }
            api_models::payments::WalletData::WeChatPayQr(_) => {
                Self::WeChatPayQr(Box::new(WeChatPayQr {}))
            }
            api_models::payments::WalletData::CashappQr(_) => {
                Self::CashappQr(Box::new(CashappQr {}))
            }
            api_models::payments::WalletData::SwishQr(_) => Self::SwishQr(SwishQrData {}),
            api_models::payments::WalletData::Mifinity(mifinity_data) => {
                Self::Mifinity(MifinityData {
                    date_of_birth: mifinity_data.date_of_birth,
                    language_preference: mifinity_data.language_preference,
                })
            }
        }
    }
}

impl From<api_models::payments::GooglePayWalletData> for GooglePayWalletData {
    fn from(value: api_models::payments::GooglePayWalletData) -> Self {
        Self {
            pm_type: value.pm_type,
            description: value.description,
            info: GooglePayPaymentMethodInfo {
                card_network: value.info.card_network,
                card_details: value.info.card_details,
                assurance_details: value.info.assurance_details.map(|info| {
                    GooglePayAssuranceDetails {
                        card_holder_authenticated: info.card_holder_authenticated,
                        account_verified: info.account_verified,
                    }
                }),
            },
            tokenization_data: GpayTokenizationData {
                token_type: value.tokenization_data.token_type,
                token: value.tokenization_data.token,
            },
        }
    }
}

impl From<api_models::payments::ApplePayWalletData> for ApplePayWalletData {
    fn from(value: api_models::payments::ApplePayWalletData) -> Self {
        Self {
            payment_data: value.payment_data,
            payment_method: ApplepayPaymentMethod {
                display_name: value.payment_method.display_name,
                network: value.payment_method.network,
                pm_type: value.payment_method.pm_type,
            },
            transaction_identifier: value.transaction_identifier,
        }
    }
}

impl From<api_models::payments::SamsungPayTokenData> for SamsungPayTokenData {
    fn from(samsung_pay_token_data: api_models::payments::SamsungPayTokenData) -> Self {
        Self {
            three_ds_type: samsung_pay_token_data.three_ds_type,
            version: samsung_pay_token_data.version,
            data: samsung_pay_token_data.data,
        }
    }
}

impl From<api_models::payments::PazeWalletData> for PazeWalletData {
    fn from(value: api_models::payments::PazeWalletData) -> Self {
        Self {
            complete_response: value.complete_response,
        }
    }
}

impl From<Box<api_models::payments::SamsungPayWalletData>> for SamsungPayWalletData {
    fn from(value: Box<api_models::payments::SamsungPayWalletData>) -> Self {
        match value.payment_credential {
            api_models::payments::SamsungPayWalletCredentials::SamsungPayWalletDataForApp(
                samsung_pay_app_wallet_data,
            ) => Self {
                payment_credential: SamsungPayWalletCredentials {
                    method: samsung_pay_app_wallet_data.method,
                    recurring_payment: samsung_pay_app_wallet_data.recurring_payment,
                    card_brand: samsung_pay_app_wallet_data.payment_card_brand.into(),
                    dpan_last_four_digits: samsung_pay_app_wallet_data.payment_last4_dpan,
                    card_last_four_digits: samsung_pay_app_wallet_data.payment_last4_fpan,
                    token_data: samsung_pay_app_wallet_data.token_data.into(),
                },
            },
            api_models::payments::SamsungPayWalletCredentials::SamsungPayWalletDataForWeb(
                samsung_pay_web_wallet_data,
            ) => Self {
                payment_credential: SamsungPayWalletCredentials {
                    method: samsung_pay_web_wallet_data.method,
                    recurring_payment: samsung_pay_web_wallet_data.recurring_payment,
                    card_brand: samsung_pay_web_wallet_data.card_brand.into(),
                    dpan_last_four_digits: None,
                    card_last_four_digits: samsung_pay_web_wallet_data.card_last_four_digits,
                    token_data: samsung_pay_web_wallet_data.token_data.into(),
                },
            },
        }
    }
}

impl From<api_models::payments::PayLaterData> for PayLaterData {
    fn from(value: api_models::payments::PayLaterData) -> Self {
        match value {
            api_models::payments::PayLaterData::KlarnaRedirect { .. } => Self::KlarnaRedirect {},
            api_models::payments::PayLaterData::KlarnaSdk { token } => Self::KlarnaSdk { token },
            api_models::payments::PayLaterData::AffirmRedirect {} => Self::AffirmRedirect {},
            api_models::payments::PayLaterData::AfterpayClearpayRedirect { .. } => {
                Self::AfterpayClearpayRedirect {}
            }
            api_models::payments::PayLaterData::PayBrightRedirect {} => Self::PayBrightRedirect {},
            api_models::payments::PayLaterData::WalleyRedirect {} => Self::WalleyRedirect {},
            api_models::payments::PayLaterData::AlmaRedirect {} => Self::AlmaRedirect {},
            api_models::payments::PayLaterData::AtomeRedirect {} => Self::AtomeRedirect {},
        }
    }
}

impl From<api_models::payments::BankRedirectData> for BankRedirectData {
    fn from(value: api_models::payments::BankRedirectData) -> Self {
        match value {
            api_models::payments::BankRedirectData::BancontactCard {
                card_number,
                card_exp_month,
                card_exp_year,
                card_holder_name,
                ..
            } => Self::BancontactCard {
                card_number,
                card_exp_month,
                card_exp_year,
                card_holder_name,
            },
            api_models::payments::BankRedirectData::Bizum {} => Self::Bizum {},
            api_models::payments::BankRedirectData::Blik { blik_code } => Self::Blik { blik_code },
            api_models::payments::BankRedirectData::Eps {
                bank_name, country, ..
            } => Self::Eps { bank_name, country },
            api_models::payments::BankRedirectData::Giropay {
                bank_account_bic,
                bank_account_iban,
                country,
                ..
            } => Self::Giropay {
                bank_account_bic,
                bank_account_iban,
                country,
            },
            api_models::payments::BankRedirectData::Ideal { bank_name, .. } => {
                Self::Ideal { bank_name }
            }
            api_models::payments::BankRedirectData::Interac { country, email } => {
                Self::Interac { country, email }
            }
            api_models::payments::BankRedirectData::OnlineBankingCzechRepublic { issuer } => {
                Self::OnlineBankingCzechRepublic { issuer }
            }
            api_models::payments::BankRedirectData::OnlineBankingFinland { email } => {
                Self::OnlineBankingFinland { email }
            }
            api_models::payments::BankRedirectData::OnlineBankingPoland { issuer } => {
                Self::OnlineBankingPoland { issuer }
            }
            api_models::payments::BankRedirectData::OnlineBankingSlovakia { issuer } => {
                Self::OnlineBankingSlovakia { issuer }
            }
            api_models::payments::BankRedirectData::OpenBankingUk {
                country, issuer, ..
            } => Self::OpenBankingUk { country, issuer },
            api_models::payments::BankRedirectData::Przelewy24 { bank_name, .. } => {
                Self::Przelewy24 { bank_name }
            }
            api_models::payments::BankRedirectData::Sofort {
                preferred_language,
                country,
                ..
            } => Self::Sofort {
                country,
                preferred_language,
            },
            api_models::payments::BankRedirectData::Trustly { country } => Self::Trustly {
                country: Some(country),
            },
            api_models::payments::BankRedirectData::OnlineBankingFpx { issuer } => {
                Self::OnlineBankingFpx { issuer }
            }
            api_models::payments::BankRedirectData::OnlineBankingThailand { issuer } => {
                Self::OnlineBankingThailand { issuer }
            }
            api_models::payments::BankRedirectData::LocalBankRedirect { .. } => {
                Self::LocalBankRedirect {}
            }
        }
    }
}

impl From<api_models::payments::CryptoData> for CryptoData {
    fn from(value: api_models::payments::CryptoData) -> Self {
        let api_models::payments::CryptoData {
            pay_currency,
            network,
        } = value;
        Self {
            pay_currency,
            network,
        }
    }
}

impl From<CryptoData> for api_models::payments::CryptoData {
    fn from(value: CryptoData) -> Self {
        let CryptoData {
            pay_currency,
            network,
        } = value;
        Self {
            pay_currency,
            network,
        }
    }
}

impl From<api_models::payments::UpiData> for UpiData {
    fn from(value: api_models::payments::UpiData) -> Self {
        match value {
            api_models::payments::UpiData::UpiCollect(upi) => {
                Self::UpiCollect(UpiCollectData { vpa_id: upi.vpa_id })
            }
            api_models::payments::UpiData::UpiIntent(_) => Self::UpiIntent(UpiIntentData {}),
        }
    }
}

impl From<UpiData> for api_models::payments::additional_info::UpiAdditionalData {
    fn from(value: UpiData) -> Self {
        match value {
            UpiData::UpiCollect(upi) => Self::UpiCollect(Box::new(
                payment_additional_types::UpiCollectAdditionalData {
                    vpa_id: upi.vpa_id.map(MaskedUpiVpaId::from),
                },
            )),
            UpiData::UpiIntent(_) => {
                Self::UpiIntent(Box::new(api_models::payments::UpiIntentData {}))
            }
        }
    }
}

impl From<api_models::payments::VoucherData> for VoucherData {
    fn from(value: api_models::payments::VoucherData) -> Self {
        match value {
            api_models::payments::VoucherData::Boleto(boleto_data) => {
                Self::Boleto(Box::new(BoletoVoucherData {
                    social_security_number: boleto_data.social_security_number,
                }))
            }
            api_models::payments::VoucherData::Alfamart(_) => {
                Self::Alfamart(Box::new(AlfamartVoucherData {}))
            }
            api_models::payments::VoucherData::Indomaret(_) => {
                Self::Indomaret(Box::new(IndomaretVoucherData {}))
            }
            api_models::payments::VoucherData::SevenEleven(_)
            | api_models::payments::VoucherData::Lawson(_)
            | api_models::payments::VoucherData::MiniStop(_)
            | api_models::payments::VoucherData::FamilyMart(_)
            | api_models::payments::VoucherData::Seicomart(_)
            | api_models::payments::VoucherData::PayEasy(_) => {
                Self::SevenEleven(Box::new(JCSVoucherData {}))
            }
            api_models::payments::VoucherData::Efecty => Self::Efecty,
            api_models::payments::VoucherData::PagoEfectivo => Self::PagoEfectivo,
            api_models::payments::VoucherData::RedCompra => Self::RedCompra,
            api_models::payments::VoucherData::RedPagos => Self::RedPagos,
            api_models::payments::VoucherData::Oxxo => Self::Oxxo,
        }
    }
}

impl From<Box<BoletoVoucherData>> for Box<api_models::payments::BoletoVoucherData> {
    fn from(value: Box<BoletoVoucherData>) -> Self {
        Self::new(api_models::payments::BoletoVoucherData {
            social_security_number: value.social_security_number,
        })
    }
}

impl From<Box<AlfamartVoucherData>> for Box<api_models::payments::AlfamartVoucherData> {
    fn from(_value: Box<AlfamartVoucherData>) -> Self {
        Self::new(api_models::payments::AlfamartVoucherData {
            first_name: None,
            last_name: None,
            email: None,
        })
    }
}

impl From<Box<IndomaretVoucherData>> for Box<api_models::payments::IndomaretVoucherData> {
    fn from(_value: Box<IndomaretVoucherData>) -> Self {
        Self::new(api_models::payments::IndomaretVoucherData {
            first_name: None,
            last_name: None,
            email: None,
        })
    }
}

impl From<Box<JCSVoucherData>> for Box<api_models::payments::JCSVoucherData> {
    fn from(_value: Box<JCSVoucherData>) -> Self {
        Self::new(api_models::payments::JCSVoucherData {
            first_name: None,
            last_name: None,
            email: None,
            phone_number: None,
        })
    }
}

impl From<VoucherData> for api_models::payments::VoucherData {
    fn from(value: VoucherData) -> Self {
        match value {
            VoucherData::Boleto(boleto_data) => Self::Boleto(boleto_data.into()),
            VoucherData::Alfamart(alfa_mart) => Self::Alfamart(alfa_mart.into()),
            VoucherData::Indomaret(info_maret) => Self::Indomaret(info_maret.into()),
            VoucherData::SevenEleven(jcs_data)
            | VoucherData::Lawson(jcs_data)
            | VoucherData::MiniStop(jcs_data)
            | VoucherData::FamilyMart(jcs_data)
            | VoucherData::Seicomart(jcs_data)
            | VoucherData::PayEasy(jcs_data) => Self::SevenEleven(jcs_data.into()),
            VoucherData::Efecty => Self::Efecty,
            VoucherData::PagoEfectivo => Self::PagoEfectivo,
            VoucherData::RedCompra => Self::RedCompra,
            VoucherData::RedPagos => Self::RedPagos,
            VoucherData::Oxxo => Self::Oxxo,
        }
    }
}

impl From<api_models::payments::GiftCardData> for GiftCardData {
    fn from(value: api_models::payments::GiftCardData) -> Self {
        match value {
            api_models::payments::GiftCardData::Givex(details) => Self::Givex(GiftCardDetails {
                number: details.number,
                cvc: details.cvc,
            }),
            api_models::payments::GiftCardData::PaySafeCard {} => Self::PaySafeCard {},
        }
    }
}

impl From<GiftCardData> for payment_additional_types::GiftCardAdditionalData {
    fn from(value: GiftCardData) -> Self {
        match value {
            GiftCardData::Givex(details) => Self::Givex(Box::new(
                payment_additional_types::GivexGiftCardAdditionalData {
                    last4: details
                        .number
                        .peek()
                        .chars()
                        .rev()
                        .take(4)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>()
                        .into(),
                },
            )),
            GiftCardData::PaySafeCard {} => Self::PaySafeCard {},
        }
    }
}

impl From<api_models::payments::CardToken> for CardToken {
    fn from(value: api_models::payments::CardToken) -> Self {
        let api_models::payments::CardToken {
            card_holder_name,
            card_cvc,
        } = value;
        Self {
            card_holder_name,
            card_cvc,
        }
    }
}

impl From<CardToken> for payment_additional_types::CardTokenAdditionalData {
    fn from(value: CardToken) -> Self {
        let CardToken {
            card_holder_name, ..
        } = value;
        Self { card_holder_name }
    }
}

impl From<api_models::payments::BankDebitData> for BankDebitData {
    fn from(value: api_models::payments::BankDebitData) -> Self {
        match value {
            api_models::payments::BankDebitData::AchBankDebit {
                account_number,
                routing_number,
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
                ..
            } => Self::AchBankDebit {
                account_number,
                routing_number,
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            },
            api_models::payments::BankDebitData::SepaBankDebit {
                iban,
                bank_account_holder_name,
                ..
            } => Self::SepaBankDebit {
                iban,
                bank_account_holder_name,
            },
            api_models::payments::BankDebitData::BecsBankDebit {
                account_number,
                bsb_number,
                bank_account_holder_name,
                ..
            } => Self::BecsBankDebit {
                account_number,
                bsb_number,
                bank_account_holder_name,
            },
            api_models::payments::BankDebitData::BacsBankDebit {
                account_number,
                sort_code,
                bank_account_holder_name,
                ..
            } => Self::BacsBankDebit {
                account_number,
                sort_code,
                bank_account_holder_name,
            },
        }
    }
}

impl From<BankDebitData> for api_models::payments::additional_info::BankDebitAdditionalData {
    fn from(value: BankDebitData) -> Self {
        match value {
            BankDebitData::AchBankDebit {
                account_number,
                routing_number,
                bank_name,
                bank_type,
                bank_holder_type,
                card_holder_name,
                bank_account_holder_name,
            } => Self::Ach(Box::new(
                payment_additional_types::AchBankDebitAdditionalData {
                    account_number: MaskedBankAccount::from(account_number),
                    routing_number: MaskedRoutingNumber::from(routing_number),
                    bank_name,
                    bank_type,
                    bank_holder_type,
                    card_holder_name,
                    bank_account_holder_name,
                },
            )),
            BankDebitData::SepaBankDebit {
                iban,
                bank_account_holder_name,
            } => Self::Sepa(Box::new(
                payment_additional_types::SepaBankDebitAdditionalData {
                    iban: MaskedIban::from(iban),
                    bank_account_holder_name,
                },
            )),
            BankDebitData::BecsBankDebit {
                account_number,
                bsb_number,
                bank_account_holder_name,
            } => Self::Becs(Box::new(
                payment_additional_types::BecsBankDebitAdditionalData {
                    account_number: MaskedBankAccount::from(account_number),
                    bsb_number,
                    bank_account_holder_name,
                },
            )),
            BankDebitData::BacsBankDebit {
                account_number,
                sort_code,
                bank_account_holder_name,
            } => Self::Bacs(Box::new(
                payment_additional_types::BacsBankDebitAdditionalData {
                    account_number: MaskedBankAccount::from(account_number),
                    sort_code: MaskedSortCode::from(sort_code),
                    bank_account_holder_name,
                },
            )),
        }
    }
}

impl From<api_models::payments::BankTransferData> for BankTransferData {
    fn from(value: api_models::payments::BankTransferData) -> Self {
        match value {
            api_models::payments::BankTransferData::AchBankTransfer { .. } => {
                Self::AchBankTransfer {}
            }
            api_models::payments::BankTransferData::SepaBankTransfer { .. } => {
                Self::SepaBankTransfer {}
            }
            api_models::payments::BankTransferData::BacsBankTransfer { .. } => {
                Self::BacsBankTransfer {}
            }
            api_models::payments::BankTransferData::MultibancoBankTransfer { .. } => {
                Self::MultibancoBankTransfer {}
            }
            api_models::payments::BankTransferData::PermataBankTransfer { .. } => {
                Self::PermataBankTransfer {}
            }
            api_models::payments::BankTransferData::BcaBankTransfer { .. } => {
                Self::BcaBankTransfer {}
            }
            api_models::payments::BankTransferData::BniVaBankTransfer { .. } => {
                Self::BniVaBankTransfer {}
            }
            api_models::payments::BankTransferData::BriVaBankTransfer { .. } => {
                Self::BriVaBankTransfer {}
            }
            api_models::payments::BankTransferData::CimbVaBankTransfer { .. } => {
                Self::CimbVaBankTransfer {}
            }
            api_models::payments::BankTransferData::DanamonVaBankTransfer { .. } => {
                Self::DanamonVaBankTransfer {}
            }
            api_models::payments::BankTransferData::MandiriVaBankTransfer { .. } => {
                Self::MandiriVaBankTransfer {}
            }
            api_models::payments::BankTransferData::Pix { pix_key, cpf, cnpj } => {
                Self::Pix { pix_key, cpf, cnpj }
            }
            api_models::payments::BankTransferData::Pse {} => Self::Pse {},
            api_models::payments::BankTransferData::LocalBankTransfer { bank_code } => {
                Self::LocalBankTransfer { bank_code }
            }
        }
    }
}

impl From<BankTransferData> for api_models::payments::additional_info::BankTransferAdditionalData {
    fn from(value: BankTransferData) -> Self {
        match value {
            BankTransferData::AchBankTransfer {} => Self::Ach {},
            BankTransferData::SepaBankTransfer {} => Self::Sepa {},
            BankTransferData::BacsBankTransfer {} => Self::Bacs {},
            BankTransferData::MultibancoBankTransfer {} => Self::Multibanco {},
            BankTransferData::PermataBankTransfer {} => Self::Permata {},
            BankTransferData::BcaBankTransfer {} => Self::Bca {},
            BankTransferData::BniVaBankTransfer {} => Self::BniVa {},
            BankTransferData::BriVaBankTransfer {} => Self::BriVa {},
            BankTransferData::CimbVaBankTransfer {} => Self::CimbVa {},
            BankTransferData::DanamonVaBankTransfer {} => Self::DanamonVa {},
            BankTransferData::MandiriVaBankTransfer {} => Self::MandiriVa {},
            BankTransferData::Pix { pix_key, cpf, cnpj } => Self::Pix(Box::new(
                api_models::payments::additional_info::PixBankTransferAdditionalData {
                    pix_key: pix_key.map(MaskedBankAccount::from),
                    cpf: cpf.map(MaskedBankAccount::from),
                    cnpj: cnpj.map(MaskedBankAccount::from),
                },
            )),
            BankTransferData::Pse {} => Self::Pse {},
            BankTransferData::LocalBankTransfer { bank_code } => Self::LocalBankTransfer(Box::new(
                api_models::payments::additional_info::LocalBankTransferAdditionalData {
                    bank_code: bank_code.map(MaskedBankAccount::from),
                },
            )),
        }
    }
}

impl From<api_models::payments::RealTimePaymentData> for RealTimePaymentData {
    fn from(value: api_models::payments::RealTimePaymentData) -> Self {
        match value {
            api_models::payments::RealTimePaymentData::Fps {} => Self::Fps {},
            api_models::payments::RealTimePaymentData::DuitNow {} => Self::DuitNow {},
            api_models::payments::RealTimePaymentData::PromptPay {} => Self::PromptPay {},
            api_models::payments::RealTimePaymentData::VietQr {} => Self::VietQr {},
        }
    }
}

impl From<RealTimePaymentData> for api_models::payments::RealTimePaymentData {
    fn from(value: RealTimePaymentData) -> Self {
        match value {
            RealTimePaymentData::Fps {} => Self::Fps {},
            RealTimePaymentData::DuitNow {} => Self::DuitNow {},
            RealTimePaymentData::PromptPay {} => Self::PromptPay {},
            RealTimePaymentData::VietQr {} => Self::VietQr {},
        }
    }
}

impl From<api_models::payments::OpenBankingData> for OpenBankingData {
    fn from(value: api_models::payments::OpenBankingData) -> Self {
        match value {
            api_models::payments::OpenBankingData::OpenBankingPIS {} => Self::OpenBankingPIS {},
        }
    }
}

impl From<OpenBankingData> for api_models::payments::OpenBankingData {
    fn from(value: OpenBankingData) -> Self {
        match value {
            OpenBankingData::OpenBankingPIS {} => Self::OpenBankingPIS {},
        }
    }
}

impl From<api_models::payments::MobilePaymentData> for MobilePaymentData {
    fn from(value: api_models::payments::MobilePaymentData) -> Self {
        match value {
            api_models::payments::MobilePaymentData::DirectCarrierBilling {
                msisdn,
                client_uid,
            } => Self::DirectCarrierBilling { msisdn, client_uid },
        }
    }
}

impl From<MobilePaymentData> for api_models::payments::MobilePaymentData {
    fn from(value: MobilePaymentData) -> Self {
        match value {
            MobilePaymentData::DirectCarrierBilling { msisdn, client_uid } => {
                Self::DirectCarrierBilling { msisdn, client_uid }
            }
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCardValue1 {
    pub card_number: String,
    pub exp_year: String,
    pub exp_month: String,
    pub nickname: Option<String>,
    pub card_last_four: Option<String>,
    pub card_token: Option<String>,
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCardValue2 {
    pub card_security_code: Option<String>,
    pub card_fingerprint: Option<String>,
    pub external_id: Option<String>,
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue1 {
    pub data: WalletData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankTransferValue1 {
    pub data: BankTransferData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankTransferValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankRedirectValue1 {
    pub data: BankRedirectData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankRedirectValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankDebitValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankDebitValue1 {
    pub data: BankDebitData,
}

pub trait GetPaymentMethodType {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType;
}

impl GetPaymentMethodType for CardRedirectData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Knet {} => api_enums::PaymentMethodType::Knet,
            Self::Benefit {} => api_enums::PaymentMethodType::Benefit,
            Self::MomoAtm {} => api_enums::PaymentMethodType::MomoAtm,
            Self::CardRedirect {} => api_enums::PaymentMethodType::CardRedirect,
        }
    }
}

impl GetPaymentMethodType for WalletData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AliPayQr(_) | Self::AliPayRedirect(_) => api_enums::PaymentMethodType::AliPay,
            Self::AliPayHkRedirect(_) => api_enums::PaymentMethodType::AliPayHk,
            Self::AmazonPayRedirect(_) => api_enums::PaymentMethodType::AmazonPay,
            Self::MomoRedirect(_) => api_enums::PaymentMethodType::Momo,
            Self::KakaoPayRedirect(_) => api_enums::PaymentMethodType::KakaoPay,
            Self::GoPayRedirect(_) => api_enums::PaymentMethodType::GoPay,
            Self::GcashRedirect(_) => api_enums::PaymentMethodType::Gcash,
            Self::ApplePay(_) | Self::ApplePayRedirect(_) | Self::ApplePayThirdPartySdk(_) => {
                api_enums::PaymentMethodType::ApplePay
            }
            Self::DanaRedirect {} => api_enums::PaymentMethodType::Dana,
            Self::GooglePay(_) | Self::GooglePayRedirect(_) | Self::GooglePayThirdPartySdk(_) => {
                api_enums::PaymentMethodType::GooglePay
            }
            Self::MbWayRedirect(_) => api_enums::PaymentMethodType::MbWay,
            Self::MobilePayRedirect(_) => api_enums::PaymentMethodType::MobilePay,
            Self::PaypalRedirect(_) | Self::PaypalSdk(_) => api_enums::PaymentMethodType::Paypal,
            Self::Paze(_) => api_enums::PaymentMethodType::Paze,
            Self::SamsungPay(_) => api_enums::PaymentMethodType::SamsungPay,
            Self::TwintRedirect {} => api_enums::PaymentMethodType::Twint,
            Self::VippsRedirect {} => api_enums::PaymentMethodType::Vipps,
            Self::TouchNGoRedirect(_) => api_enums::PaymentMethodType::TouchNGo,
            Self::WeChatPayRedirect(_) | Self::WeChatPayQr(_) => {
                api_enums::PaymentMethodType::WeChatPay
            }
            Self::CashappQr(_) => api_enums::PaymentMethodType::Cashapp,
            Self::SwishQr(_) => api_enums::PaymentMethodType::Swish,
            Self::Mifinity(_) => api_enums::PaymentMethodType::Mifinity,
        }
    }
}

impl GetPaymentMethodType for PayLaterData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::KlarnaRedirect { .. } => api_enums::PaymentMethodType::Klarna,
            Self::KlarnaSdk { .. } => api_enums::PaymentMethodType::Klarna,
            Self::AffirmRedirect {} => api_enums::PaymentMethodType::Affirm,
            Self::AfterpayClearpayRedirect { .. } => api_enums::PaymentMethodType::AfterpayClearpay,
            Self::PayBrightRedirect {} => api_enums::PaymentMethodType::PayBright,
            Self::WalleyRedirect {} => api_enums::PaymentMethodType::Walley,
            Self::AlmaRedirect {} => api_enums::PaymentMethodType::Alma,
            Self::AtomeRedirect {} => api_enums::PaymentMethodType::Atome,
        }
    }
}

impl GetPaymentMethodType for BankRedirectData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::BancontactCard { .. } => api_enums::PaymentMethodType::BancontactCard,
            Self::Bizum {} => api_enums::PaymentMethodType::Bizum,
            Self::Blik { .. } => api_enums::PaymentMethodType::Blik,
            Self::Eps { .. } => api_enums::PaymentMethodType::Eps,
            Self::Giropay { .. } => api_enums::PaymentMethodType::Giropay,
            Self::Ideal { .. } => api_enums::PaymentMethodType::Ideal,
            Self::Interac { .. } => api_enums::PaymentMethodType::Interac,
            Self::OnlineBankingCzechRepublic { .. } => {
                api_enums::PaymentMethodType::OnlineBankingCzechRepublic
            }
            Self::OnlineBankingFinland { .. } => api_enums::PaymentMethodType::OnlineBankingFinland,
            Self::OnlineBankingPoland { .. } => api_enums::PaymentMethodType::OnlineBankingPoland,
            Self::OnlineBankingSlovakia { .. } => {
                api_enums::PaymentMethodType::OnlineBankingSlovakia
            }
            Self::OpenBankingUk { .. } => api_enums::PaymentMethodType::OpenBankingUk,
            Self::Przelewy24 { .. } => api_enums::PaymentMethodType::Przelewy24,
            Self::Sofort { .. } => api_enums::PaymentMethodType::Sofort,
            Self::Trustly { .. } => api_enums::PaymentMethodType::Trustly,
            Self::OnlineBankingFpx { .. } => api_enums::PaymentMethodType::OnlineBankingFpx,
            Self::OnlineBankingThailand { .. } => {
                api_enums::PaymentMethodType::OnlineBankingThailand
            }
            Self::LocalBankRedirect { .. } => api_enums::PaymentMethodType::LocalBankRedirect,
        }
    }
}

impl GetPaymentMethodType for BankDebitData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AchBankDebit { .. } => api_enums::PaymentMethodType::Ach,
            Self::SepaBankDebit { .. } => api_enums::PaymentMethodType::Sepa,
            Self::BecsBankDebit { .. } => api_enums::PaymentMethodType::Becs,
            Self::BacsBankDebit { .. } => api_enums::PaymentMethodType::Bacs,
        }
    }
}

impl GetPaymentMethodType for BankTransferData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AchBankTransfer { .. } => api_enums::PaymentMethodType::Ach,
            Self::SepaBankTransfer { .. } => api_enums::PaymentMethodType::Sepa,
            Self::BacsBankTransfer { .. } => api_enums::PaymentMethodType::Bacs,
            Self::MultibancoBankTransfer { .. } => api_enums::PaymentMethodType::Multibanco,
            Self::PermataBankTransfer { .. } => api_enums::PaymentMethodType::PermataBankTransfer,
            Self::BcaBankTransfer { .. } => api_enums::PaymentMethodType::BcaBankTransfer,
            Self::BniVaBankTransfer { .. } => api_enums::PaymentMethodType::BniVa,
            Self::BriVaBankTransfer { .. } => api_enums::PaymentMethodType::BriVa,
            Self::CimbVaBankTransfer { .. } => api_enums::PaymentMethodType::CimbVa,
            Self::DanamonVaBankTransfer { .. } => api_enums::PaymentMethodType::DanamonVa,
            Self::MandiriVaBankTransfer { .. } => api_enums::PaymentMethodType::MandiriVa,
            Self::Pix { .. } => api_enums::PaymentMethodType::Pix,
            Self::Pse {} => api_enums::PaymentMethodType::Pse,
            Self::LocalBankTransfer { .. } => api_enums::PaymentMethodType::LocalBankTransfer,
        }
    }
}

impl GetPaymentMethodType for CryptoData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        api_enums::PaymentMethodType::CryptoCurrency
    }
}

impl GetPaymentMethodType for RealTimePaymentData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Fps {} => api_enums::PaymentMethodType::Fps,
            Self::DuitNow {} => api_enums::PaymentMethodType::DuitNow,
            Self::PromptPay {} => api_enums::PaymentMethodType::PromptPay,
            Self::VietQr {} => api_enums::PaymentMethodType::VietQr,
        }
    }
}

impl GetPaymentMethodType for UpiData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::UpiCollect(_) => api_enums::PaymentMethodType::UpiCollect,
            Self::UpiIntent(_) => api_enums::PaymentMethodType::UpiIntent,
        }
    }
}
impl GetPaymentMethodType for VoucherData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Boleto(_) => api_enums::PaymentMethodType::Boleto,
            Self::Efecty => api_enums::PaymentMethodType::Efecty,
            Self::PagoEfectivo => api_enums::PaymentMethodType::PagoEfectivo,
            Self::RedCompra => api_enums::PaymentMethodType::RedCompra,
            Self::RedPagos => api_enums::PaymentMethodType::RedPagos,
            Self::Alfamart(_) => api_enums::PaymentMethodType::Alfamart,
            Self::Indomaret(_) => api_enums::PaymentMethodType::Indomaret,
            Self::Oxxo => api_enums::PaymentMethodType::Oxxo,
            Self::SevenEleven(_) => api_enums::PaymentMethodType::SevenEleven,
            Self::Lawson(_) => api_enums::PaymentMethodType::Lawson,
            Self::MiniStop(_) => api_enums::PaymentMethodType::MiniStop,
            Self::FamilyMart(_) => api_enums::PaymentMethodType::FamilyMart,
            Self::Seicomart(_) => api_enums::PaymentMethodType::Seicomart,
            Self::PayEasy(_) => api_enums::PaymentMethodType::PayEasy,
        }
    }
}
impl GetPaymentMethodType for GiftCardData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Givex(_) => api_enums::PaymentMethodType::Givex,
            Self::PaySafeCard {} => api_enums::PaymentMethodType::PaySafeCard,
        }
    }
}

impl GetPaymentMethodType for OpenBankingData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::OpenBankingPIS {} => api_enums::PaymentMethodType::OpenBankingPIS,
        }
    }
}

impl GetPaymentMethodType for MobilePaymentData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::DirectCarrierBilling { .. } => api_enums::PaymentMethodType::DirectCarrierBilling,
        }
    }
}

impl From<Card> for ExtendedCardInfo {
    fn from(value: Card) -> Self {
        Self {
            card_number: value.card_number,
            card_exp_month: value.card_exp_month,
            card_exp_year: value.card_exp_year,
            card_holder_name: None,
            card_cvc: value.card_cvc,
            card_issuer: value.card_issuer,
            card_network: value.card_network,
            card_type: value.card_type,
            card_issuing_country: value.card_issuing_country,
            bank_code: value.bank_code,
        }
    }
}

impl From<GooglePayWalletData> for payment_methods::PaymentMethodDataWalletInfo {
    fn from(item: GooglePayWalletData) -> Self {
        Self {
            last4: item.info.card_details,
            card_network: item.info.card_network,
            card_type: Some(item.pm_type),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PaymentMethodsData {
    Card(CardDetailsPaymentMethod),
    BankDetails(payment_methods::PaymentMethodDataBankCreds), //PaymentMethodDataBankCreds and its transformations should be moved to the domain models
    WalletDetails(payment_methods::PaymentMethodDataWalletInfo), //PaymentMethodDataWalletInfo and its transformations should be moved to the domain models
    NetworkToken(NetworkTokenDetailsPaymentMethod),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct NetworkTokenDetailsPaymentMethod {
    pub last4_digits: Option<String>,
    pub issuer_country: Option<String>,
    pub network_token_expiry_month: Option<Secret<String>>,
    pub network_token_expiry_year: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_type: Option<String>,
    #[serde(default = "saved_in_locker_default")]
    pub saved_to_locker: bool,
}

fn saved_in_locker_default() -> bool {
    true
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CardDetailsPaymentMethod {
    pub last4_digits: Option<String>,
    pub issuer_country: Option<String>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_type: Option<String>,
    #[serde(default = "saved_in_locker_default")]
    pub saved_to_locker: bool,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<payment_methods::CardDetail> for CardDetailsPaymentMethod {
    fn from(item: payment_methods::CardDetail) -> Self {
        Self {
            issuer_country: item.card_issuing_country.map(|c| c.to_string()),
            last4_digits: Some(item.card_number.get_last4()),
            expiry_month: Some(item.card_exp_month),
            expiry_year: Some(item.card_exp_year),
            card_holder_name: item.card_holder_name,
            nick_name: item.nick_name,
            card_isin: None,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type.map(|card| card.to_string()),
            saved_to_locker: true,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<NetworkTokenDetails> for NetworkTokenDetailsPaymentMethod {
    fn from(item: NetworkTokenDetails) -> Self {
        Self {
            issuer_country: item.card_issuing_country.map(|c| c.to_string()),
            last4_digits: Some(item.network_token.get_last4()),
            network_token_expiry_month: Some(item.network_token_exp_month),
            network_token_expiry_year: Some(item.network_token_exp_year),
            card_holder_name: item.card_holder_name,
            nick_name: item.nick_name,
            card_isin: None,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type.map(|card| card.to_string()),
            saved_to_locker: true,
        }
    }
}
