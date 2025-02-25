use strum::VariantNames;

use crate::enums::collect_variants;
pub use crate::enums::{
    AuthenticationType, CaptureMethod, CardNetwork, Country, Country as BusinessCountry,
    Country as BillingCountry, CountryAlpha2, Currency as PaymentCurrency, MandateAcceptanceType,
    MandateType, PaymentMethod, PaymentType, RoutableConnectors, SetupFutureUsage,
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
    Atome,
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
    GooglePay,
    AmazonPay,
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
    Sepa,
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
