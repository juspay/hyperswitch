use common_utils::pii::{self, Email};
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// We need to derive Serialize and Deserialize because some parts of payment method data are being
// stored in the database as serde_json::Value
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum PaymentMethodData {
    Card(Card),
    CardRedirect(CardRedirectData),
    Wallet(api_models::payments::WalletData),
    PayLater(api_models::payments::PayLaterData),
    BankRedirect(api_models::payments::BankRedirectData),
    BankDebit(api_models::payments::BankDebitData),
    BankTransfer(Box<api_models::payments::BankTransferData>),
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
    pub card_holder_name: Option<Secret<String>>,
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

#[derive(Eq, PartialEq, Clone, Debug)]
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

#[derive(Eq, PartialEq, Clone, Debug)]

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

#[derive(Eq, PartialEq, Clone, Debug)]

pub struct SamsungPayWalletData {
    /// The encrypted payment token from Samsung
    pub token: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug)]

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

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct ApplePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct GooglePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct GooglePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct ApplePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct WeChatPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct WeChatPay {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct WeChatPayQr {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct CashappQr {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct PaypalRedirection {
    /// paypal's email address
    pub email: Option<Email>,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct AliPayQr {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct AliPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct AliPayHkRedirection {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct MomoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct KakaoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct GoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct GcashRedirection {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct MobilePayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct MbWayRedirection {
    /// Telephone number of the shopper. Should be Portuguese phone number.
    pub telephone_number: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug)]

pub struct GooglePayPaymentMethodInfo {
    /// The name of the card network
    pub card_network: String,
    /// The details of the card
    pub card_details: String,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct PayPalWalletData {
    /// Token generated for the Apple pay
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct TouchNGoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct SwishQrData {}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct GpayTokenizationData {
    /// The type of the token
    pub token_type: String,
    /// Token generated for the wallet
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct ApplePayWalletData {
    /// The payment data of Apple pay
    pub payment_data: String,
    /// The payment method of Apple pay
    pub payment_method: ApplepayPaymentMethod,
    /// The unique identifier for the transaction
    pub transaction_identifier: String,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct ApplepayPaymentMethod {
    pub display_name: String,
    pub network: String,
    pub pm_type: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BankRedirectBilling {
    pub billing_name: Option<Secret<String>>,
    pub email: Option<Email>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CryptoData {
    pub pay_currency: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UpiData {
    #[schema(value_type = Option<String>, example = "successtest@iata")]
    pub vpa_id: Option<Secret<String, pii::UpiVpaMaskingStrategy>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BoletoVoucherData {
    /// The shopper's social security number
    #[schema(value_type = Option<String>)]
    pub social_security_number: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AlfamartVoucherData {
    /// The billing first name for Alfamart
    #[schema(value_type = String, example = "Jane")]
    pub first_name: Secret<String>,
    /// The billing second name for Alfamart
    #[schema(value_type = String, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Alfamart
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct IndomaretVoucherData {
    /// The billing first name for Alfamart
    #[schema(value_type = String, example = "Jane")]
    pub first_name: Secret<String>,
    /// The billing second name for Alfamart
    #[schema(value_type = String, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Alfamart
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct JCSVoucherData {
    /// The billing first name for Japanese convenience stores
    #[schema(value_type = String, example = "Jane")]
    pub first_name: Secret<String>,
    /// The billing second name Japanese convenience stores
    #[schema(value_type = String, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Japanese convenience stores
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
    /// The telephone number for Japanese convenience stores
    #[schema(value_type = String, example = "9999999999")]
    pub phone_number: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GiftCardData {
    Givex(GiftCardDetails),
    PaySafeCard {},
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GiftCardDetails {
    /// The gift card number
    #[schema(value_type = String)]
    pub number: Secret<String>,
    /// The card verification code.
    #[schema(value_type = String)]
    pub cvc: Secret<String>,
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub struct CardToken {
    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    pub card_holder_name: Option<Secret<String>>,

    /// The CVC number for the card
    #[schema(value_type = Option<String>)]
    pub card_cvc: Option<Secret<String>>,
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
                Self::Wallet(wallet_data)
            }
            api_models::payments::PaymentMethodData::PayLater(pay_later_data) => {
                Self::PayLater(pay_later_data)
            }
            api_models::payments::PaymentMethodData::BankRedirect(bank_redirect_data) => {
                Self::BankRedirect(bank_redirect_data)
            }
            api_models::payments::PaymentMethodData::BankDebit(bank_debit_data) => {
                Self::BankDebit(bank_debit_data)
            }
            api_models::payments::PaymentMethodData::BankTransfer(bank_transfer_data) => {
                Self::BankTransfer(bank_transfer_data)
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
            card_holder_name,
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
            api_models::payments::VoucherData::Alfamart(alfamart_data) => {
                Self::Alfamart(Box::new(AlfamartVoucherData {
                    first_name: alfamart_data.first_name,
                    last_name: alfamart_data.last_name,
                    email: alfamart_data.email,
                }))
            }
            api_models::payments::VoucherData::Indomaret(indomaret_data) => {
                Self::Indomaret(Box::new(IndomaretVoucherData {
                    first_name: indomaret_data.first_name,
                    last_name: indomaret_data.last_name,
                    email: indomaret_data.email,
                }))
            }
            api_models::payments::VoucherData::SevenEleven(jcs_data)
            | api_models::payments::VoucherData::Lawson(jcs_data)
            | api_models::payments::VoucherData::MiniStop(jcs_data)
            | api_models::payments::VoucherData::FamilyMart(jcs_data)
            | api_models::payments::VoucherData::Seicomart(jcs_data)
            | api_models::payments::VoucherData::PayEasy(jcs_data) => {
                Self::SevenEleven(Box::new(JCSVoucherData {
                    first_name: jcs_data.first_name,
                    last_name: jcs_data.last_name,
                    email: jcs_data.email,
                    phone_number: jcs_data.phone_number,
                }))
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
