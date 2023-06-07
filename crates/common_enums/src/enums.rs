use router_derive;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
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
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthenticationType {
    /// If the card is enrolled for 3DS authentication, the 3DS based authentication will be activated. The liability of chargeback shift to the issuer
    #[default]
    ThreeDs,
    /// 3DS based authentication will not be activated. The liability of chargeback stays with the merchant.
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
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CaptureMethod {
    /// Post the payment authorization, the capture will be executed on the full amount immediately
    #[default]
    Automatic,
    /// The capture will happen only if the merchant triggers a Capture API request
    Manual,
    /// The capture will happen only if the merchant triggers a Capture API request
    ManualMultiple,
    /// The capture can be scheduled to automatically get triggered at a specific date & time
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
    ToSchema,
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
    strum::EnumIter,
    ToSchema,
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
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
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
    ToSchema,
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
    //::LabelledGeneric,
    ToSchema,
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
    Eq,
    strum::EnumString,
    PartialEq,
    Hash,
    Copy,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    ToSchema,
    Default,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentExperience {
    /// The URL to which the customer needs to be redirected for completing the payment.
    #[default]
    RedirectToUrl,
    /// Contains the data for invoking the sdk client for completing the payment.
    InvokeSdkClient,
    /// The QR code data to be displayed to the customer.
    DisplayQrCode,
    /// Contains data to finish one click payment.
    OneClick,
    /// Redirect customer to link wallet
    LinkWallet,
    /// Contains the data for invoking the sdk client for completing the payment.
    InvokePaymentApp,
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
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
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
    Default,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
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
    strum::Display,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
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
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]

/// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
#[schema(example = "custom")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoutingAlgorithm {
    RoundRobin,
    MaxConversion,
    MinCost,
    Custom,
}

/// The status of the mandate, which indicates whether it can be used to initiate a payment
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
    ToSchema,
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
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    Hash,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Connector {
    Aci,
    Adyen,
    Airwallex,
    Authorizedotnet,
    Bitpay,
    Bluesnap,
    Braintree,
    Checkout,
    Coinbase,
    Cybersource,
    Iatapay,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "dummyconnector1")]
    #[strum(serialize = "dummyconnector1")]
    DummyConnector1,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "dummyconnector2")]
    #[strum(serialize = "dummyconnector2")]
    DummyConnector2,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "dummyconnector3")]
    #[strum(serialize = "dummyconnector3")]
    DummyConnector3,
    Opennode,
    Bambora,
    Dlocal,
    Fiserv,
    Forte,
    Globalpay,
    Klarna,
    Mollie,
    Multisafepay,
    Nexinets,
    Nmi,
    Noon,
    Nuvei,
    // Payeezy, As psync and rsync are not supported by this connector, it is added as template code for future usage
    Paypal,
    Payu,
    Rapyd,
    Shift4,
    Stripe,
    Trustpay,
    Worldline,
    Worldpay,
    Zen,
}

impl Connector {
    pub fn supports_access_token(&self, payment_method: PaymentMethod) -> bool {
        matches!(
            (self, payment_method),
            (Self::Airwallex, _)
                | (Self::Globalpay, _)
                | (Self::Paypal, _)
                | (Self::Payu, _)
                | (Self::Trustpay, PaymentMethod::BankRedirect)
                | (Self::Iatapay, _)
        )
    }
    pub fn supports_file_storage_module(&self) -> bool {
        matches!(self, Self::Stripe | Self::Checkout)
    }
    pub fn requires_defend_dispute(&self) -> bool {
        matches!(self, Self::Checkout)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoutableConnectors {
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "dummyconnector1")]
    #[strum(serialize = "dummyconnector1")]
    DummyConnector1,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "dummyconnector2")]
    #[strum(serialize = "dummyconnector2")]
    DummyConnector2,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "dummyconnector3")]
    #[strum(serialize = "dummyconnector3")]
    DummyConnector3,
    Aci,
    Adyen,
    Airwallex,
    Authorizedotnet,
    Bitpay,
    Bambora,
    Bluesnap,
    Braintree,
    Checkout,
    Coinbase,
    Cybersource,
    Dlocal,
    Fiserv,
    Forte,
    Globalpay,
    Iatapay,
    Klarna,
    Mollie,
    Multisafepay,
    Nexinets,
    Nmi,
    Noon,
    Nuvei,
    Opennode,
    // Payeezy, As psync and rsync are not supported by this connector, it is added as template code for future usage
    Paypal,
    Payu,
    Rapyd,
    Shift4,
    Stripe,
    Trustpay,
    Worldline,
    Worldpay,
    Zen,
}

/// Name of banks supported by Hyperswitch
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
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BankNames {
    AmericanExpress,
    BankOfAmerica,
    Barclays,
    #[serde(rename = "BLIK - PSP")]
    BlikPSP,
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
    #[serde(rename = "Bank Millennium")]
    BankMillennium,
    #[serde(rename = "Bank PEKAO S.A.")]
    BankPEKAOSA,
    BawagPskAg,
    BksBankAg,
    BrullKallmusBankAg,
    BtvVierLanderBank,
    CapitalBankGraweGruppeAg,
    #[serde(rename = "Česká spořitelna")]
    CeskaSporitelna,
    Dolomitenbank,
    EasybankAg,
    #[serde(rename = "ePlatby VÚB")]
    EPlatbyVUB,
    ErsteBankUndSparkassen,
    FrieslandBank,
    HypoAlpeadriabankInternationalAg,
    HypoNoeLbFurNiederosterreichUWien,
    HypoOberosterreichSalzburgSteiermark,
    HypoTirolBankAg,
    HypoVorarlbergBankAg,
    HypoBankBurgenlandAktiengesellschaft,
    #[serde(rename = "Komercní banka")]
    KomercniBanka,
    #[serde(rename = "mBank - mTransfer")]
    MBank,
    MarchfelderBank,
    OberbankAg,
    OsterreichischeArzteUndApothekerbank,
    #[serde(rename = "Pay with ING")]
    PayWithING,
    #[serde(rename = "Płacę z iPKO")]
    PlaceZIPKO,
    #[serde(rename = "Płatność online kartą płatniczą")]
    PlatnoscOnlineKartaPlatnicza,
    PosojilnicaBankEGen,
    #[serde(rename = "Poštová banka")]
    PostovaBanka,
    RaiffeisenBankengruppeOsterreich,
    SchelhammerCapitalBankAg,
    SchoellerbankAg,
    SpardaBankWien,
    SporoPay,
    #[serde(rename = "Santander-Przelew24")]
    SantanderPrzelew24,
    TatraPay,
    Viamo,
    VolksbankGruppe,
    VolkskreditbankAg,
    VrBankBraunau,
    #[serde(rename = "Pay with Alior Bank")]
    PayWithAliorBank,
    #[serde(rename = "Banki Spółdzielcze")]
    BankiSpoldzielcze,
    #[serde(rename = "Pay with Inteligo")]
    PayWithInteligo,
    #[serde(rename = "BNP Paribas Poland")]
    BNPParibasPoland,
    #[serde(rename = "Bank Nowy S.A.")]
    BankNowySA,
    #[serde(rename = "Credit Agricole")]
    CreditAgricole,
    #[serde(rename = "Pay with BOŚ")]
    PayWithBOS,
    #[serde(rename = "Pay with CitiHandlowy")]
    PayWithCitiHandlowy,
    #[serde(rename = "Pay with Plus Bank")]
    PayWithPlusBank,
    #[serde(rename = "Toyota Bank")]
    ToyotaBank,
    VeloBank,
    #[serde(rename = "e-transfer Pocztowy24")]
    ETransferPocztowy24,
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
    Clone,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
pub enum CardNetwork {
    Visa,
    Mastercard,
    AmericanExpress,
    JCB,
    DinersClub,
    Discover,
    CartesBancaires,
    UnionPay,
    Interac,
    RuPay,
    Maestro,
}

#[derive(
    Clone,
    Copy,
    Default,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
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
    Debug,
    Copy,
    Default,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
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
    // dispute has been successfully challenged by the merchant
    DisputeWon,
    // dispute has been unsuccessfully challenged
    DisputeLost,
}

#[derive(
    Clone, Debug, serde::Deserialize, serde::Serialize, strum::Display, strum::EnumString, ToSchema,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FrmAction {
    CancelTxn,
    AutoRefund,
    ManualReview,
}

#[derive(
    Clone, Debug, serde::Deserialize, serde::Serialize, strum::Display, strum::EnumString, ToSchema,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FrmPreferredFlowTypes {
    Pre,
    Post,
}
#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnresolvedResponseReason {
    pub code: String,
    /// A message to merchant to give hint on next action he/she should do to resolve
    pub message: String,
}

#[derive(
    Clone,
    Debug,
    Eq,
    Default,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    utoipa::ToSchema,
    Copy
)]
#[router_derive::diesel_enum(storage_type = "pg_enum")]
#[rustfmt::skip]
pub enum CountryAlpha2 {
    AF, AX, AL, DZ, AS, AD, AO, AI, AQ, AG, AR, AM, AW, AU, AT,
    AZ, BS, BH, BD, BB, BY, BE, BZ, BJ, BM, BT, BO, BQ, BA, BW,
    BV, BR, IO, BN, BG, BF, BI, KH, CM, CA, CV, KY, CF, TD, CL,
    CN, CX, CC, CO, KM, CG, CD, CK, CR, CI, HR, CU, CW, CY, CZ,
    DK, DJ, DM, DO, EC, EG, SV, GQ, ER, EE, ET, FK, FO, FJ, FI,
    FR, GF, PF, TF, GA, GM, GE, DE, GH, GI, GR, GL, GD, GP, GU,
    GT, GG, GN, GW, GY, HT, HM, VA, HN, HK, HU, IS, IN, ID, IR,
    IQ, IE, IM, IL, IT, JM, JP, JE, JO, KZ, KE, KI, KP, KR, KW,
    KG, LA, LV, LB, LS, LR, LY, LI, LT, LU, MO, MK, MG, MW, MY,
    MV, ML, MT, MH, MQ, MR, MU, YT, MX, FM, MD, MC, MN, ME, MS,
    MA, MZ, MM, NA, NR, NP, NL, NC, NZ, NI, NE, NG, NU, NF, MP,
    NO, OM, PK, PW, PS, PA, PG, PY, PE, PH, PN, PL, PT, PR, QA,
    RE, RO, RU, RW, BL, SH, KN, LC, MF, PM, VC, WS, SM, ST, SA,
    SN, RS, SC, SL, SG, SX, SK, SI, SB, SO, ZA, GS, SS, ES, LK,
    SD, SR, SJ, SZ, SE, CH, SY, TW, TJ, TZ, TH, TL, TG, TK, TO,
    TT, TN, TR, TM, TC, TV, UG, UA, AE, GB, UM, UY, UZ, VU,
    VE, VN, VG, VI, WF, EH, YE, ZM, ZW,
    #[default]
    US
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[rustfmt::skip]
pub enum CountryAlpha3 {
    AFG, ALA, ALB, DZA, ASM, AND, AGO, AIA, ATA, ATG, ARG, ARM, ABW, AUS, AUT,
    AZE, BHS, BHR, BGD, BRB, BLR, BEL, BLZ, BEN, BMU, BTN, BOL, BES, BIH, BWA,
    BVT, BRA, IOT, BRN, BGR, BFA, BDI, CPV, KHM, CMR, CAN, CYM, CAF, TCD, CHL,
    CHN, CXR, CCK, COL, COM, COG, COD, COK, CRI, CIV, HRV, CUB, CUW, CYP, CZE,
    DNK, DJI, DMA, DOM, ECU, EGY, SLV, GNQ, ERI, EST, ETH, FLK, FRO, FJI, FIN,
    FRA, GUF, PYF, ATF, GAB, GMB, GEO, DEU, GHA, GIB, GRC, GRL, GRD, GLP, GUM,
    GTM, GGY, GIN, GNB, GUY, HTI, HMD, VAT, HND, HKG, HUN, ISL, IND, IDN, IRN,
    IRQ, IRL, IMN, ISR, ITA, JAM, JPN, JEY, JOR, KAZ, KEN, KIR, PRK, KOR, KWT,
    KGZ, LAO, LVA, LBN, LSO, LBR, LBY, LIE, LTU, LUX, MAC, MKD, MDG, MWI, MYS,
    MDV, MLI, MLT, MHL, MTQ, MRT, MUS, MYT, MEX, FSM, MDA, MCO, MNG, MNE, MSR,
    MAR, MOZ, MMR, NAM, NRU, NPL, NLD, NCL, NZL, NIC, NER, NGA, NIU, NFK, MNP,
    NOR, OMN, PAK, PLW, PSE, PAN, PNG, PRY, PER, PHL, PCN, POL, PRT, PRI, QAT,
    REU, ROU, RUS, RWA, BLM, SHN, KNA, LCA, MAF, SPM, VCT, WSM, SMR, STP, SAU,
    SEN, SRB, SYC, SLE, SGP, SXM, SVK, SVN, SLB, SOM, ZAF, SGS, SSD, ESP, LKA,
    SDN, SUR, SJM, SWZ, SWE, CHE, SYR, TWN, TJK, TZA, THA, TLS, TGO, TKL, TON,
    TTO, TUN, TUR, TKM, TCA, TUV, UGA, UKR, ARE, GBR, USA, UMI, URY, UZB, VUT,
    VEN, VNM, VGB, VIR, WLF, ESH, YEM, ZMB, ZWE
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum Country {
    Afghanistan,
    AlandIslands,
    Albania,
    Algeria,
    AmericanSamoa,
    Andorra,
    Angola,
    Anguilla,
    Antarctica,
    AntiguaAndBarbuda,
    Argentina,
    Armenia,
    Aruba,
    Australia,
    Austria,
    Azerbaijan,
    Bahamas,
    Bahrain,
    Bangladesh,
    Barbados,
    Belarus,
    Belgium,
    Belize,
    Benin,
    Bermuda,
    Bhutan,
    BoliviaPlurinationalState,
    BonaireSintEustatiusAndSaba,
    BosniaAndHerzegovina,
    Botswana,
    BouvetIsland,
    Brazil,
    BritishIndianOceanTerritory,
    BruneiDarussalam,
    Bulgaria,
    BurkinaFaso,
    Burundi,
    CaboVerde,
    Cambodia,
    Cameroon,
    Canada,
    CaymanIslands,
    CentralAfricanRepublic,
    Chad,
    Chile,
    China,
    ChristmasIsland,
    CocosKeelingIslands,
    Colombia,
    Comoros,
    Congo,
    CongoDemocraticRepublic,
    CookIslands,
    CostaRica,
    CotedIvoire,
    Croatia,
    Cuba,
    Curacao,
    Cyprus,
    Czechia,
    Denmark,
    Djibouti,
    Dominica,
    DominicanRepublic,
    Ecuador,
    Egypt,
    ElSalvador,
    EquatorialGuinea,
    Eritrea,
    Estonia,
    Ethiopia,
    FalklandIslandsMalvinas,
    FaroeIslands,
    Fiji,
    Finland,
    France,
    FrenchGuiana,
    FrenchPolynesia,
    FrenchSouthernTerritories,
    Gabon,
    Gambia,
    Georgia,
    Germany,
    Ghana,
    Gibraltar,
    Greece,
    Greenland,
    Grenada,
    Guadeloupe,
    Guam,
    Guatemala,
    Guernsey,
    Guinea,
    GuineaBissau,
    Guyana,
    Haiti,
    HeardIslandAndMcDonaldIslands,
    HolySee,
    Honduras,
    HongKong,
    Hungary,
    Iceland,
    India,
    Indonesia,
    IranIslamicRepublic,
    Iraq,
    Ireland,
    IsleOfMan,
    Israel,
    Italy,
    Jamaica,
    Japan,
    Jersey,
    Jordan,
    Kazakhstan,
    Kenya,
    Kiribati,
    KoreaDemocraticPeoplesRepublic,
    KoreaRepublic,
    Kuwait,
    Kyrgyzstan,
    LaoPeoplesDemocraticRepublic,
    Latvia,
    Lebanon,
    Lesotho,
    Liberia,
    Libya,
    Liechtenstein,
    Lithuania,
    Luxembourg,
    Macao,
    MacedoniaTheFormerYugoslavRepublic,
    Madagascar,
    Malawi,
    Malaysia,
    Maldives,
    Mali,
    Malta,
    MarshallIslands,
    Martinique,
    Mauritania,
    Mauritius,
    Mayotte,
    Mexico,
    MicronesiaFederatedStates,
    MoldovaRepublic,
    Monaco,
    Mongolia,
    Montenegro,
    Montserrat,
    Morocco,
    Mozambique,
    Myanmar,
    Namibia,
    Nauru,
    Nepal,
    Netherlands,
    NewCaledonia,
    NewZealand,
    Nicaragua,
    Niger,
    Nigeria,
    Niue,
    NorfolkIsland,
    NorthernMarianaIslands,
    Norway,
    Oman,
    Pakistan,
    Palau,
    PalestineState,
    Panama,
    PapuaNewGuinea,
    Paraguay,
    Peru,
    Philippines,
    Pitcairn,
    Poland,
    Portugal,
    PuertoRico,
    Qatar,
    Reunion,
    Romania,
    RussianFederation,
    Rwanda,
    SaintBarthelemy,
    SaintHelenaAscensionAndTristandaCunha,
    SaintKittsAndNevis,
    SaintLucia,
    SaintMartinFrenchpart,
    SaintPierreAndMiquelon,
    SaintVincentAndTheGrenadines,
    Samoa,
    SanMarino,
    SaoTomeAndPrincipe,
    SaudiArabia,
    Senegal,
    Serbia,
    Seychelles,
    SierraLeone,
    Singapore,
    SintMaartenDutchpart,
    Slovakia,
    Slovenia,
    SolomonIslands,
    Somalia,
    SouthAfrica,
    SouthGeorgiaAndTheSouthSandwichIslands,
    SouthSudan,
    Spain,
    SriLanka,
    Sudan,
    Suriname,
    SvalbardAndJanMayen,
    Swaziland,
    Sweden,
    Switzerland,
    SyrianArabRepublic,
    TaiwanProvinceOfChina,
    Tajikistan,
    TanzaniaUnitedRepublic,
    Thailand,
    TimorLeste,
    Togo,
    Tokelau,
    Tonga,
    TrinidadAndTobago,
    Tunisia,
    Turkey,
    Turkmenistan,
    TurksAndCaicosIslands,
    Tuvalu,
    Uganda,
    Ukraine,
    UnitedArabEmirates,
    UnitedKingdomOfGreatBritainAndNorthernIreland,
    UnitedStatesOfAmerica,
    UnitedStatesMinorOutlyingIslands,
    Uruguay,
    Uzbekistan,
    Vanuatu,
    VenezuelaBolivarianRepublic,
    Vietnam,
    VirginIslandsBritish,
    VirginIslandsUS,
    WallisAndFutuna,
    WesternSahara,
    Yemen,
    Zambia,
    Zimbabwe,
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
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FileUploadProvider {
    #[default]
    Router,
    Stripe,
    Checkout,
}
