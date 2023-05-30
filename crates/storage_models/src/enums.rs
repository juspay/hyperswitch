#[doc(hidden)]
pub mod diesel_exports {
    pub use super::{
        DbAttemptStatus as AttemptStatus, DbAuthenticationType as AuthenticationType,
        DbCaptureMethod as CaptureMethod, DbConnectorType as ConnectorType,
        DbCountryAlpha2 as CountryAlpha2, DbCurrency as Currency, DbDisputeStage as DisputeStage,
        DbDisputeStatus as DisputeStatus, DbEventClass as EventClass,
        DbEventObjectType as EventObjectType, DbEventType as EventType,
        DbFutureUsage as FutureUsage, DbIntentStatus as IntentStatus,
        DbMandateStatus as MandateStatus, DbMandateType as MandateType,
        DbMerchantStorageScheme as MerchantStorageScheme,
        DbPaymentMethodIssuerCode as PaymentMethodIssuerCode,
        DbProcessTrackerStatus as ProcessTrackerStatus, DbRefundStatus as RefundStatus,
        DbRefundType as RefundType,
    };
}

pub use common_enums::*;
use common_utils::pii;
use diesel::serialize::{Output, ToSql};
use time::PrimitiveDateTime;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AttemptStatus {
    Started,
    AuthenticationFailed,
    RouterDeclined,
    AuthenticationPending,
    AuthenticationSuccessful,
    Authorized,
    AuthorizationFailed,
    Charged,
    Authorizing,
    CodInitiated,
    Voided,
    VoidInitiated,
    CaptureInitiated,
    CaptureFailed,
    VoidFailed,
    AutoRefunded,
    PartialCharged,
    Unresolved,
    #[default]
    Pending,
    Failure,
    PaymentMethodAwaited,
    ConfirmationAwaited,
    DeviceDataCollectionPending,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthenticationType {
    #[default]
    ThreeDs,
    NoThreeDs,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CaptureMethod {
    #[default]
    Automatic,
    Manual,
    ManualMultiple,
    Scheduled,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    strum::Display,
    strum::EnumString,
    serde::Deserialize,
    serde::Serialize,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ConnectorType {
    /// PayFacs, Acquirers, Gateways, BNPL etc
    PaymentProcessor,
    /// Fraud, Currency Conversion, Crypto etc
    PaymentVas,
    /// Accounting, Billing, Invoicing, Tax etc
    FinOperations,
    /// Inventory, ERP, CRM, KYC etc
    FizOperations,
    /// Payment Networks like Visa, MasterCard etc
    Networks,
    /// All types of banks including corporate / commercial / personal / neo banks
    BankingEntities,
    /// All types of non-banking financial institutions including Insurance, Credit / Lending etc
    NonBankingFinance,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
pub enum Currency {
    AED,
    ALL,
    AMD,
    ANG,
    ARS,
    AUD,
    AWG,
    AZN,
    BBD,
    BDT,
    BHD,
    BMD,
    BND,
    BOB,
    BRL,
    BSD,
    BWP,
    BZD,
    CAD,
    CHF,
    CNY,
    COP,
    CRC,
    CUP,
    CZK,
    DKK,
    DOP,
    DZD,
    EGP,
    ETB,
    EUR,
    FJD,
    GBP,
    GHS,
    GIP,
    GMD,
    GTQ,
    GYD,
    HKD,
    HNL,
    HRK,
    HTG,
    HUF,
    IDR,
    ILS,
    INR,
    JMD,
    JOD,
    JPY,
    KES,
    KGS,
    KHR,
    KRW,
    KWD,
    KYD,
    KZT,
    LAK,
    LBP,
    LKR,
    LRD,
    LSL,
    MAD,
    MDL,
    MKD,
    MMK,
    MNT,
    MOP,
    MUR,
    MVR,
    MWK,
    MXN,
    MYR,
    NAD,
    NGN,
    NIO,
    NOK,
    NPR,
    NZD,
    OMR,
    PEN,
    PGK,
    PHP,
    PKR,
    PLN,
    QAR,
    RUB,
    SAR,
    SCR,
    SEK,
    SGD,
    SLL,
    SOS,
    SSP,
    SVC,
    SZL,
    THB,
    TTD,
    TWD,
    TZS,
    #[default]
    USD,
    UYU,
    UZS,
    YER,
    ZAR,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventClass {
    Payments,
    Refunds,
    Disputes,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventObjectType {
    PaymentDetails,
    RefundDetails,
    DisputeDetails,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventType {
    PaymentSucceeded,
    PaymentFailed,
    PaymentProcessing,
    ActionRequired,
    RefundSucceeded,
    RefundFailed,
    DisputeOpened,
    DisputeExpired,
    DisputeAccepted,
    DisputeCancelled,
    DisputeChallenged,
    DisputeWon,
    DisputeLost,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum IntentStatus {
    Succeeded,
    Failed,
    Cancelled,
    Processing,
    RequiresCustomerAction,
    RequiresMerchantAction,
    RequiresPaymentMethod,
    #[default]
    RequiresConfirmation,
    RequiresCapture,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FutureUsage {
    #[default]
    OffSession,
    OnSession,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MerchantStorageScheme {
    #[default]
    PostgresOnly,
    RedisKv,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodIssuerCode {
    JpHdfc,
    JpIcici,
    JpGooglepay,
    JpApplepay,
    JpPhonepay,
    JpWechat,
    JpSofort,
    JpGiropay,
    JpSepa,
    JpBacs,
}
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PaymentMethod {
    #[default]
    Card,
    PayLater,
    Wallet,
    BankRedirect,
    BankTransfer,
    Crypto,
    BankDebit,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WalletIssuer {
    GooglePay,
    ApplePay,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ProcessTrackerStatus {
    // Picked by the producer
    Processing,
    // State when the task is added
    New,
    // Send to retry
    Pending,
    // Picked by consumer
    ProcessStarted,
    // Finished by consumer
    Finish,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[strum(serialize_all = "snake_case")]
pub enum RefundStatus {
    Failure,
    ManualReview,
    #[default]
    Pending,
    Success,
    TransactionFailure,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[strum(serialize_all = "snake_case")]
pub enum RefundType {
    InstantRefund,
    #[default]
    RegularRefund,
    RetryRefund,
}

// Mandate
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Default,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MandateType {
    SingleUse,
    #[default]
    MultiUse,
}

use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    sql_types::Jsonb,
};
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
pub enum MandateDataType {
    SingleUse(MandateAmountData),
    MultiUse(Option<MandateAmountData>),
}

impl<DB: Backend> FromSql<Jsonb, DB> for MandateDataType
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
        Ok(serde_json::from_value(value)?)
    }
}
impl ToSql<Jsonb, diesel::pg::Pg> for MandateDataType
where
    serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = serde_json::to_value(self)?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct MandateAmountData {
    pub amount: i64,
    pub currency: Currency,
    pub start_date: Option<PrimitiveDateTime>,
    pub end_date: Option<PrimitiveDateTime>,
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Default,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MandateStatus {
    #[default]
    Active,
    Inactive,
    Pending,
    Revoked,
}

#[derive(
    Clone,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodType {
    Ach,
    Affirm,
    AfterpayClearpay,
    AliPay,
    ApplePay,
    Bacs,
    BancontactCard,
    Becs,
    Blik,
    Credit,
    CryptoCurrency,
    Debit,
    Eps,
    Giropay,
    GooglePay,
    Ideal,
    Interac,
    Klarna,
    MbWay,
    MobilePay,
    OnlineBankingCzechRepublic,
    OnlineBankingFinland,
    OnlineBankingPoland,
    OnlineBankingSlovakia,
    PayBright,
    Paypal,
    Przelewy24,
    Sepa,
    Sofort,
    Swish,
    Trustly,
    Walley,
    WeChatPay,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BankNames {
    AmericanExpress,
    BankOfAmerica,
    Barclays,
    CapitalOne,
    Chase,
    Citi,
    Discover,
    NavyFederalCreditUnion,
    PentagonFederalCreditUnion,
    SynchronyBank,
    WellsFargo,
    AbnAmro,
    AsnBank,
    Bunq,
    Handelsbanken,
    Ing,
    Knab,
    Moneyou,
    Rabobank,
    Regiobank,
    Revolut,
    SnsBank,
    TriodosBank,
    VanLanschot,
    ArzteUndApothekerBank,
    AustrianAnadiBankAg,
    BankAustria,
    Bank99Ag,
    BankhausCarlSpangler,
    BankhausSchelhammerUndSchatteraAg,
    BawagPskAg,
    BksBankAg,
    BrullKallmusBankAg,
    BtvVierLanderBank,
    CapitalBankGraweGruppeAg,
    Dolomitenbank,
    EasybankAg,
    ErsteBankUndSparkassen,
    HypoAlpeadriabankInternationalAg,
    HypoNoeLbFurNiederosterreichUWien,
    HypoOberosterreichSalzburgSteiermark,
    HypoTirolBankAg,
    HypoVorarlbergBankAg,
    HypoBankBurgenlandAktiengesellschaft,
    MarchfelderBank,
    OberbankAg,
    OsterreichischeArzteUndApothekerbank,
    PosojilnicaBankEGen,
    RaiffeisenBankengruppeOsterreich,
    SchelhammerCapitalBankAg,
    SchoellerbankAg,
    SpardaBankWien,
    VolksbankGruppe,
    VolkskreditbankAg,
    VrBankBraunau,
    PlusBank,
    EtransferPocztowy24,
    BankiSpbdzielcze,
    BankNowyBfgSa,
    GetinBank,
    Blik,
    NoblePay,
    IdeaBank,
    EnveloBank,
    NestPrzelew,
    MbankMtransfer,
    Inteligo,
    PbacZIpko,
    BnpParibas,
    BankPekaoSa,
    VolkswagenBank,
    AliorBank,
    Boz,
}

#[derive(
    Eq,
    PartialEq,
    Hash,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Default,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentExperience {
    #[default]
    RedirectToUrl,
    InvokeSdkClient,
    DisplayQrCode,
    OneClick,
    LinkWallet,
    InvokePaymentApp,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Default,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DisputeStage {
    PreDispute,
    #[default]
    Dispute,
    PreArbitration,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Default,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DisputeStatus {
    #[default]
    DisputeOpened,
    DisputeExpired,
    DisputeAccepted,
    DisputeCancelled,
    DisputeChallenged,
    DisputeWon,
    DisputeLost,
}
