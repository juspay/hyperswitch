use strum::VariantNames;
use utoipa::ToSchema;

use crate::enums::collect_variants;
pub use crate::enums::{
    AuthenticationType, CaptureMethod, CardNetwork, Country, Country as BusinessCountry,
    Country as BillingCountry, Country as IssuerCountry, Country as AcquirerCountry, CountryAlpha2,
    Currency as PaymentCurrency, MandateAcceptanceType, MandateType, PaymentMethod, PaymentType,
    RoutableConnectors, SetupFutureUsage,
};
#[cfg(feature = "payouts")]
pub use crate::enums::{PayoutBankTransferType, PayoutType, PayoutWalletType};

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CardType {
    Credit,
    Debit,
    #[cfg(feature = "v2")]
    Card,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PayLaterType {
    Affirm,
    AfterpayClearpay,
    Alma,
    Klarna,
    PayBright,
    Walley,
    Flexiti,
    Atome,
    Breadpay,
    Payjustnow,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum WalletType {
    Bluecode,
    GooglePay,
    AmazonPay,
    Skrill,
    Paysera,
    ApplePay,
    Paypal,
    AliPay,
    AliPayHk,
    MbWay,
    MobilePay,
    WeChatPay,
    SamsungPay,
    GoPay,
    KakaoPay,
    Twint,
    Gcash,
    Vipps,
    Momo,
    Dana,
    TouchNGo,
    Swish,
    Cashapp,
    Venmo,
    Mifinity,
    Paze,
    RevolutPay,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum VoucherType {
    Boleto,
    Efecty,
    PagoEfectivo,
    RedCompra,
    RedPagos,
    Alfamart,
    Indomaret,
    SevenEleven,
    Lawson,
    MiniStop,
    FamilyMart,
    Seicomart,
    PayEasy,
    Oxxo,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BankRedirectType {
    Bizum,
    Giropay,
    Ideal,
    Sofort,
    Eft,
    Eps,
    BancontactCard,
    Blik,
    Interac,
    LocalBankRedirect,
    OnlineBankingCzechRepublic,
    OnlineBankingFinland,
    OnlineBankingPoland,
    OnlineBankingSlovakia,
    OnlineBankingFpx,
    OnlineBankingThailand,
    OpenBankingUk,
    Przelewy24,
    Trustly,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OpenBankingType {
    OpenBankingPIS,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BankTransferType {
    Multibanco,
    Ach,
    SepaBankTransfer,
    Bacs,
    BcaBankTransfer,
    BniVa,
    BriVa,
    CimbVa,
    DanamonVa,
    MandiriVa,
    PermataBankTransfer,
    Pix,
    Pse,
    LocalBankTransfer,
    InstantBankTransfer,
    InstantBankTransferFinland,
    InstantBankTransferPoland,
    IndonesianBankTransfer,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum GiftCardType {
    PaySafeCard,
    Givex,
    BhnCardNetwork,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CardRedirectType {
    Benefit,
    Knet,
    MomoAtm,
    CardRedirect,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MobilePaymentType {
    DirectCarrierBilling,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CryptoType {
    CryptoCurrency,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RealTimePaymentType {
    Fps,
    DuitNow,
    PromptPay,
    VietQr,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum UpiType {
    UpiCollect,
    UpiIntent,
    UpiQr,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BankDebitType {
    Ach,
    Sepa,
    SepaGuarenteedDebit,
    Bacs,
    Becs,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RewardType {
    ClassicReward,
    Evoucher,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CustomerDevicePlatform {
    Web,
    Android,
    Ios,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CustomerDeviceType {
    Mobile,
    Tablet,
    Desktop,
    GamingConsole,
}

// Common display sizes for different device types
#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CustomerDeviceDisplaySize {
    // Mobile sizes
    Size320x568, // iPhone SE
    Size375x667, // iPhone 8
    Size390x844, // iPhone 12/13
    Size414x896, // iPhone XR/11
    Size428x926, // iPhone 12/13 Pro Max

    // Tablet sizes
    Size768x1024,  // iPad
    Size834x1112,  // iPad Pro 10.5
    Size834x1194,  // iPad Pro 11
    Size1024x1366, // iPad Pro 12.9

    // Desktop sizes
    Size1280x720,  // HD
    Size1366x768,  // Common laptop
    Size1440x900,  // MacBook Air
    Size1920x1080, // Full HD
    Size2560x1440, // QHD
    Size3840x2160, // 4K

    // Custom sizes
    Size500x600,
    Size600x400,

    // Other common sizes
    Size360x640,  // Common Android
    Size412x915,  // Pixel 6
    Size800x1280, // Common Android tablet
}

collect_variants!(CardType);
collect_variants!(PayLaterType);
collect_variants!(WalletType);
collect_variants!(BankRedirectType);
collect_variants!(BankDebitType);
collect_variants!(CryptoType);
collect_variants!(RewardType);
collect_variants!(RealTimePaymentType);
collect_variants!(UpiType);
collect_variants!(VoucherType);
collect_variants!(GiftCardType);
collect_variants!(BankTransferType);
collect_variants!(CardRedirectType);
collect_variants!(OpenBankingType);
collect_variants!(MobilePaymentType);
collect_variants!(CustomerDeviceType);
collect_variants!(CustomerDevicePlatform);
collect_variants!(CustomerDeviceDisplaySize);
