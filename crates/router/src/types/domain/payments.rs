use common_utils::pii::{self, Email};
use masking::Secret;
use serde::{Deserialize, Serialize};

// We need to derive Serialize and Deserialize because some parts of payment method data are being
// stored in the database as serde_json::Value
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum PaymentMethodData {
    Card(Card),
    CardRedirect(CardRedirectData),
    Wallet(WalletData),
    PayLater(PayLaterData),
    BankRedirect(BankRedirectData),
    BankDebit(BankDebitData),
    BankTransfer(Box<BankTransferData>),
    Crypto(CryptoData),
    MandatePayment,
    Reward,
    Upi(UpiData),
    Voucher(VoucherData),
    GiftCard(Box<GiftCardData>),
    CardToken(CardToken),
}

impl PaymentMethodData {
    pub fn get_payment_method(&self) -> Option<common_enums::PaymentMethod> {
        match self {
            Self::Card(_) => Some(common_enums::PaymentMethod::Card),
            Self::CardRedirect(_) => Some(common_enums::PaymentMethod::CardRedirect),
            Self::Wallet(_) => Some(common_enums::PaymentMethod::Wallet),
            Self::PayLater(_) => Some(common_enums::PaymentMethod::PayLater),
            Self::BankRedirect(_) => Some(common_enums::PaymentMethod::BankRedirect),
            Self::BankDebit(_) => Some(common_enums::PaymentMethod::BankDebit),
            Self::BankTransfer(_) => Some(common_enums::PaymentMethod::BankTransfer),
            Self::Crypto(_) => Some(common_enums::PaymentMethod::Crypto),
            Self::Reward => Some(common_enums::PaymentMethod::Reward),
            Self::Upi(_) => Some(common_enums::PaymentMethod::Upi),
            Self::Voucher(_) => Some(common_enums::PaymentMethod::Voucher),
            Self::GiftCard(_) => Some(common_enums::PaymentMethod::GiftCard),
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
    KlarnaRedirect {
        billing_email: Email,
        billing_country: common_enums::CountryAlpha2,
    },
    KlarnaSdk {
        token: String,
    },
    AffirmRedirect {},
    AfterpayClearpayRedirect {
        billing_email: Email,
        billing_name: Secret<String>,
    },
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
    SamsungPay(Box<SamsungPayWalletData>),
    TwintRedirect {},
    VippsRedirect {},
    TouchNGoRedirect(Box<TouchNGoRedirection>),
    WeChatPayRedirect(Box<WeChatPayRedirection>),
    WeChatPayQr(Box<WeChatPayQr>),
    CashappQr(Box<CashappQr>),
    SwishQr(SwishQrData),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]

pub struct SamsungPayWalletData {
    /// The encrypted payment token from Samsung
    pub token: Secret<String>,
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

pub enum BankRedirectData {
    BancontactCard {
        card_number: Option<cards::CardNumber>,
        card_exp_month: Option<Secret<String>>,
        card_exp_year: Option<Secret<String>>,
        card_holder_name: Option<Secret<String>>,
        billing_details: Option<BankRedirectBilling>,
    },
    Bizum {},
    Blik {
        blik_code: Option<String>,
    },
    Eps {
        billing_details: Option<BankRedirectBilling>,
        bank_name: Option<common_enums::BankNames>,
        country: Option<common_enums::CountryAlpha2>,
    },
    Giropay {
        billing_details: Option<BankRedirectBilling>,
        bank_account_bic: Option<Secret<String>>,
        bank_account_iban: Option<Secret<String>>,
        country: Option<common_enums::CountryAlpha2>,
    },
    Ideal {
        billing_details: Option<BankRedirectBilling>,
        bank_name: Option<common_enums::BankNames>,
        country: Option<common_enums::CountryAlpha2>,
    },
    Interac {
        country: common_enums::CountryAlpha2,
        email: Email,
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
        country: Option<common_enums::CountryAlpha2>,
    },
    Przelewy24 {
        bank_name: Option<common_enums::BankNames>,
        billing_details: BankRedirectBilling,
    },
    Sofort {
        billing_details: Option<BankRedirectBilling>,
        country: Option<common_enums::CountryAlpha2>,
        preferred_language: Option<String>,
    },
    Trustly {
        country: common_enums::CountryAlpha2,
    },
    OnlineBankingFpx {
        issuer: common_enums::BankNames,
    },
    OnlineBankingThailand {
        issuer: common_enums::BankNames,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct BankRedirectBilling {
    pub billing_name: Option<Secret<String>>,
    pub email: Option<Email>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CryptoData {
    pub pay_currency: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UpiData {
    pub vpa_id: Option<Secret<String, pii::UpiVpaMaskingStrategy>>,
}

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
        billing_details: BankDebitBilling,
        account_number: Secret<String>,
        routing_number: Secret<String>,
        card_holder_name: Option<Secret<String>>,
        bank_account_holder_name: Option<Secret<String>>,
        bank_name: Option<common_enums::BankNames>,
        bank_type: Option<common_enums::BankType>,
        bank_holder_type: Option<common_enums::BankHolderType>,
    },
    SepaBankDebit {
        billing_details: BankDebitBilling,
        iban: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
    BecsBankDebit {
        billing_details: BankDebitBilling,
        account_number: Secret<String>,
        bsb_number: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
    BacsBankDebit {
        billing_details: BankDebitBilling,
        account_number: Secret<String>,
        sort_code: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
    },
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
pub struct BankDebitBilling {
    pub name: Secret<String>,
    pub email: Email,
    pub address: Option<api_models::payments::AddressDetails>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BankTransferData {
    AchBankTransfer {
        /// The billing details for ACH Bank Transfer
        billing_details: AchBillingDetails,
    },
    SepaBankTransfer {
        /// The billing details for SEPA
        billing_details: SepaAndBacsBillingDetails,

        /// The two-letter ISO country code for SEPA and BACS
        country: api_models::enums::CountryAlpha2,
    },
    BacsBankTransfer {
        /// The billing details for SEPA
        billing_details: SepaAndBacsBillingDetails,
    },
    MultibancoBankTransfer {
        /// The billing details for Multibanco
        billing_details: MultibancoBillingDetails,
    },
    PermataBankTransfer {
        /// The billing details for Permata Bank Transfer
        billing_details: DokuBillingDetails,
    },
    BcaBankTransfer {
        /// The billing details for BCA Bank Transfer
        billing_details: DokuBillingDetails,
    },
    BniVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    BriVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    CimbVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    DanamonVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    MandiriVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    Pix {},
    Pse {},
    LocalBankTransfer {
        bank_code: Option<String>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AchBillingDetails {
    /// The Email ID for ACH billing
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DokuBillingDetails {
    /// The billing first name for Doku
    pub first_name: Secret<String>,
    /// The billing second name for Doku
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Doku billing
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MultibancoBillingDetails {
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SepaAndBacsBillingDetails {
    /// The Email ID for SEPA and BACS billing
    pub email: Email,
    /// The billing name for SEPA and BACS billing
    pub name: Secret<String>,
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
        }
    }
}

impl From<api_models::payments::Card> for Card {
    fn from(value: api_models::payments::Card) -> Self {
        let api_models::payments::Card {
            card_number,
            card_exp_month,
            card_exp_year,
            card_holder_name: _,
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
            api_models::payments::WalletData::SamsungPay(samsung_pay_data) => {
                Self::SamsungPay(Box::new(SamsungPayWalletData {
                    token: samsung_pay_data.token,
                }))
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

impl From<api_models::payments::PayLaterData> for PayLaterData {
    fn from(value: api_models::payments::PayLaterData) -> Self {
        match value {
            api_models::payments::PayLaterData::KlarnaRedirect {
                billing_email,
                billing_country,
            } => Self::KlarnaRedirect {
                billing_email,
                billing_country,
            },
            api_models::payments::PayLaterData::KlarnaSdk { token } => Self::KlarnaSdk { token },
            api_models::payments::PayLaterData::AffirmRedirect {} => Self::AffirmRedirect {},
            api_models::payments::PayLaterData::AfterpayClearpayRedirect {
                billing_email,
                billing_name,
            } => Self::AfterpayClearpayRedirect {
                billing_email,
                billing_name,
            },
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
                billing_details,
            } => Self::BancontactCard {
                card_number,
                card_exp_month,
                card_exp_year,
                card_holder_name,
                billing_details: billing_details.map(BankRedirectBilling::from),
            },
            api_models::payments::BankRedirectData::Bizum {} => Self::Bizum {},
            api_models::payments::BankRedirectData::Blik { blik_code } => Self::Blik { blik_code },
            api_models::payments::BankRedirectData::Eps {
                billing_details,
                bank_name,
                country,
            } => Self::Eps {
                billing_details: billing_details.map(BankRedirectBilling::from),
                bank_name,
                country,
            },
            api_models::payments::BankRedirectData::Giropay {
                billing_details,
                bank_account_bic,
                bank_account_iban,
                country,
            } => Self::Giropay {
                billing_details: billing_details.map(BankRedirectBilling::from),
                bank_account_bic,
                bank_account_iban,
                country,
            },
            api_models::payments::BankRedirectData::Ideal {
                billing_details,
                bank_name,
                country,
            } => Self::Ideal {
                billing_details: billing_details.map(BankRedirectBilling::from),
                bank_name,
                country,
            },
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
            api_models::payments::BankRedirectData::OpenBankingUk { issuer, country } => {
                Self::OpenBankingUk { issuer, country }
            }
            api_models::payments::BankRedirectData::Przelewy24 {
                bank_name,
                billing_details,
            } => Self::Przelewy24 {
                bank_name,
                billing_details: BankRedirectBilling {
                    billing_name: billing_details.billing_name,
                    email: billing_details.email,
                },
            },
            api_models::payments::BankRedirectData::Sofort {
                billing_details,
                country,
                preferred_language,
            } => Self::Sofort {
                billing_details: billing_details.map(BankRedirectBilling::from),
                country,
                preferred_language,
            },
            api_models::payments::BankRedirectData::Trustly { country } => {
                Self::Trustly { country }
            }
            api_models::payments::BankRedirectData::OnlineBankingFpx { issuer } => {
                Self::OnlineBankingFpx { issuer }
            }
            api_models::payments::BankRedirectData::OnlineBankingThailand { issuer } => {
                Self::OnlineBankingThailand { issuer }
            }
        }
    }
}

impl From<api_models::payments::BankRedirectBilling> for BankRedirectBilling {
    fn from(billing: api_models::payments::BankRedirectBilling) -> Self {
        Self {
            billing_name: billing.billing_name,
            email: billing.email,
        }
    }
}

impl From<api_models::payments::CryptoData> for CryptoData {
    fn from(value: api_models::payments::CryptoData) -> Self {
        let api_models::payments::CryptoData { pay_currency } = value;
        Self { pay_currency }
    }
}

impl From<api_models::payments::UpiData> for UpiData {
    fn from(value: api_models::payments::UpiData) -> Self {
        let api_models::payments::UpiData { vpa_id } = value;
        Self { vpa_id }
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

impl From<api_models::payments::BankDebitData> for BankDebitData {
    fn from(value: api_models::payments::BankDebitData) -> Self {
        match value {
            api_models::payments::BankDebitData::AchBankDebit {
                billing_details,
                account_number,
                routing_number,
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            } => Self::AchBankDebit {
                billing_details: BankDebitBilling {
                    name: billing_details.name,
                    email: billing_details.email,
                    address: billing_details.address,
                },
                account_number,
                routing_number,
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            },
            api_models::payments::BankDebitData::SepaBankDebit {
                billing_details,
                iban,
                bank_account_holder_name,
            } => Self::SepaBankDebit {
                billing_details: BankDebitBilling {
                    name: billing_details.name,
                    email: billing_details.email,
                    address: billing_details.address,
                },
                iban,
                bank_account_holder_name,
            },
            api_models::payments::BankDebitData::BecsBankDebit {
                billing_details,
                account_number,
                bsb_number,
                bank_account_holder_name,
            } => Self::BecsBankDebit {
                billing_details: BankDebitBilling {
                    name: billing_details.name,
                    email: billing_details.email,
                    address: billing_details.address,
                },
                account_number,
                bsb_number,
                bank_account_holder_name,
            },
            api_models::payments::BankDebitData::BacsBankDebit {
                billing_details,
                account_number,
                sort_code,
                bank_account_holder_name,
            } => Self::BacsBankDebit {
                billing_details: BankDebitBilling {
                    name: billing_details.name,
                    email: billing_details.email,
                    address: billing_details.address,
                },
                account_number,
                sort_code,
                bank_account_holder_name,
            },
        }
    }
}

impl From<api_models::payments::BankTransferData> for BankTransferData {
    fn from(value: api_models::payments::BankTransferData) -> Self {
        match value {
            api_models::payments::BankTransferData::AchBankTransfer { billing_details } => {
                Self::AchBankTransfer {
                    billing_details: AchBillingDetails {
                        email: billing_details.email,
                    },
                }
            }
            api_models::payments::BankTransferData::SepaBankTransfer {
                billing_details,
                country,
            } => Self::SepaBankTransfer {
                billing_details: SepaAndBacsBillingDetails {
                    email: billing_details.email,
                    name: billing_details.name,
                },
                country,
            },
            api_models::payments::BankTransferData::BacsBankTransfer { billing_details } => {
                Self::BacsBankTransfer {
                    billing_details: SepaAndBacsBillingDetails {
                        email: billing_details.email,
                        name: billing_details.name,
                    },
                }
            }
            api_models::payments::BankTransferData::MultibancoBankTransfer { billing_details } => {
                Self::MultibancoBankTransfer {
                    billing_details: MultibancoBillingDetails {
                        email: billing_details.email,
                    },
                }
            }
            api_models::payments::BankTransferData::PermataBankTransfer { billing_details } => {
                Self::PermataBankTransfer {
                    billing_details: DokuBillingDetails::from(billing_details),
                }
            }
            api_models::payments::BankTransferData::BcaBankTransfer { billing_details } => {
                Self::BcaBankTransfer {
                    billing_details: DokuBillingDetails::from(billing_details),
                }
            }
            api_models::payments::BankTransferData::BniVaBankTransfer { billing_details } => {
                Self::BniVaBankTransfer {
                    billing_details: DokuBillingDetails::from(billing_details),
                }
            }
            api_models::payments::BankTransferData::BriVaBankTransfer { billing_details } => {
                Self::BriVaBankTransfer {
                    billing_details: DokuBillingDetails::from(billing_details),
                }
            }
            api_models::payments::BankTransferData::CimbVaBankTransfer { billing_details } => {
                Self::CimbVaBankTransfer {
                    billing_details: DokuBillingDetails::from(billing_details),
                }
            }
            api_models::payments::BankTransferData::DanamonVaBankTransfer { billing_details } => {
                Self::DanamonVaBankTransfer {
                    billing_details: DokuBillingDetails::from(billing_details),
                }
            }
            api_models::payments::BankTransferData::MandiriVaBankTransfer { billing_details } => {
                Self::MandiriVaBankTransfer {
                    billing_details: DokuBillingDetails::from(billing_details),
                }
            }
            api_models::payments::BankTransferData::Pix {} => Self::Pix {},
            api_models::payments::BankTransferData::Pse {} => Self::Pse {},
            api_models::payments::BankTransferData::LocalBankTransfer { bank_code } => {
                Self::LocalBankTransfer { bank_code }
            }
        }
    }
}

impl From<api_models::payments::DokuBillingDetails> for DokuBillingDetails {
    fn from(billing: api_models::payments::DokuBillingDetails) -> Self {
        Self {
            first_name: billing.first_name,
            last_name: billing.last_name,
            email: billing.email,
        }
    }
}
