use common_utils::pii::Email;
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// We need to derive Serialize and Deserialize because some parts of payment method data are being
// stored in the database as serde_json::Value
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum PaymentMethodData {
    Card(Card),
    CardRedirect(CardRedirectData),
    Wallet(WalletData),
    PayLater(api_models::payments::PayLaterData),
    BankRedirect(api_models::payments::BankRedirectData),
    BankDebit(api_models::payments::BankDebitData),
    BankTransfer(Box<api_models::payments::BankTransferData>),
    Crypto(api_models::payments::CryptoData),
    MandatePayment,
    Reward,
    Upi(api_models::payments::UpiData),
    Voucher(api_models::payments::VoucherData),
    GiftCard(Box<api_models::payments::GiftCardData>),
    CardToken(api_models::payments::CardToken),
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

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]

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

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]

pub struct SamsungPayWalletData {
    /// The encrypted payment token from Samsung
    pub token: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]

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

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GooglePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GooglePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WeChatPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WeChatPay {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WeChatPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CashappQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaypalRedirection {
    /// paypal's email address
    pub email: Option<Email>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AliPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AliPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AliPayHkRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MomoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct KakaoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GcashRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MobilePayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MbWayRedirection {
    /// Telephone number of the shopper. Should be Portuguese phone number.
    pub telephone_number: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]

pub struct GooglePayPaymentMethodInfo {
    /// The name of the card network
    pub card_network: String,
    /// The details of the card
    pub card_details: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PayPalWalletData {
    /// Token generated for the Apple pay
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct TouchNGoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SwishQrData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GpayTokenizationData {
    /// The type of the token
    pub token_type: String,
    /// Token generated for the wallet
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayWalletData {
    /// The payment data of Apple pay
    pub payment_data: String,
    /// The payment method of Apple pay
    pub payment_method: ApplepayPaymentMethod,
    /// The unique identifier for the transaction
    pub transaction_identifier: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
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
                Self::Crypto(crypto_data)
            }
            api_models::payments::PaymentMethodData::MandatePayment => Self::MandatePayment,
            api_models::payments::PaymentMethodData::Reward => Self::Reward,
            api_models::payments::PaymentMethodData::Upi(upi_data) => Self::Upi(upi_data),
            api_models::payments::PaymentMethodData::Voucher(voucher_data) => {
                Self::Voucher(voucher_data)
            }
            api_models::payments::PaymentMethodData::GiftCard(gift_card) => {
                Self::GiftCard(gift_card)
            }
            api_models::payments::PaymentMethodData::CardToken(card_token) => {
                Self::CardToken(card_token)
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
            api_models::payments::WalletData::ApplePay(data) => {
                let apple_pay_data = data;
                Self::ApplePay(ApplePayWalletData {
                    payment_data: apple_pay_data.payment_data,
                    payment_method: ApplepayPaymentMethod {
                        display_name: apple_pay_data.payment_method.display_name,
                        network: apple_pay_data.payment_method.network,
                        pm_type: apple_pay_data.payment_method.pm_type,
                    },
                    transaction_identifier: apple_pay_data.transaction_identifier,
                })
            }
            api_models::payments::WalletData::ApplePayRedirect(_) => {
                Self::ApplePayRedirect(Box::new(ApplePayRedirectData {}))
            }
            api_models::payments::WalletData::ApplePayThirdPartySdk(_) => {
                Self::ApplePayThirdPartySdk(Box::new(ApplePayThirdPartySdkData {}))
            }
            api_models::payments::WalletData::DanaRedirect {} => Self::DanaRedirect {},
            api_models::payments::WalletData::GooglePay(data) => {
                let google_pay_data = data;
                Self::GooglePay(GooglePayWalletData {
                    pm_type: google_pay_data.pm_type,
                    description: google_pay_data.description,
                    info: GooglePayPaymentMethodInfo {
                        card_network: google_pay_data.info.card_network,
                        card_details: google_pay_data.info.card_details,
                    },
                    tokenization_data: GpayTokenizationData {
                        token_type: google_pay_data.tokenization_data.token_type,
                        token: google_pay_data.tokenization_data.token,
                    },
                })
            }
            api_models::payments::WalletData::GooglePayRedirect(_) => {
                Self::GooglePayRedirect(Box::new(GooglePayRedirectData {}))
            }
            api_models::payments::WalletData::GooglePayThirdPartySdk(_) => {
                Self::GooglePayThirdPartySdk(Box::new(GooglePayThirdPartySdkData {}))
            }
            api_models::payments::WalletData::MbWayRedirect(data) => {
                let mbway_redirect_data = data;
                Self::MbWayRedirect(Box::new(MbWayRedirection {
                    telephone_number: mbway_redirect_data.telephone_number,
                }))
            }
            api_models::payments::WalletData::MobilePayRedirect(_) => {
                Self::MobilePayRedirect(Box::new(MobilePayRedirection {}))
            }
            api_models::payments::WalletData::PaypalRedirect(data) => {
                let paypal_redirect_data = data;
                Self::PaypalRedirect(PaypalRedirection {
                    email: paypal_redirect_data.email,
                })
            }
            api_models::payments::WalletData::PaypalSdk(data) => {
                let paypal_sdk_data = data;
                Self::PaypalSdk(PayPalWalletData {
                    token: paypal_sdk_data.token,
                })
            }
            api_models::payments::WalletData::SamsungPay(data) => {
                let samsung_pay_data = data;
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
