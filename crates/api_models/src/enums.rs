use utoipa::ToSchema;

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
    #[default]
    Pending,
    Failure,
    PaymentMethodAwaited,
    ConfirmationAwaited,
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
    ToSchema,
)]
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
    frunk::LabelledGeneric,
    ToSchema,
)]
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
    frunk::LabelledGeneric,
    ToSchema,
)]
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
    ToSchema,
    frunk::LabelledGeneric,
)]
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
    frunk::LabelledGeneric,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventType {
    PaymentSucceeded,
    RefundSucceeded,
    RefundFailed,
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
    frunk::LabelledGeneric,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum IntentStatus {
    Succeeded,
    Failed,
    Cancelled,
    Processing,
    RequiresCustomerAction,
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
    ToSchema,
)]
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
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
    ToSchema,
)]
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
    frunk::LabelledGeneric,
)]
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
    frunk::LabelledGeneric,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PaymentMethodType {
    Credit,
    Debit,
    Giropay,
    Ideal,
    Sofort,
    Eps,
    Klarna,
    Affirm,
    AfterpayClearpay,
    GooglePay,
    ApplePay,
    Paypal,
    Evoucher,
    Classic,
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
    frunk::LabelledGeneric,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PaymentMethod {
    #[default]
    Card,
    PayLater,
    Wallet,
    BankRedirect,
    Reward,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WalletIssuer {
    GooglePay,
    ApplePay,
    Paypal,
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
    frunk::LabelledGeneric,
)]
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
    frunk::LabelledGeneric,
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
    frunk::LabelledGeneric,
    ToSchema,
)]
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
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    frunk::LabelledGeneric,
    Hash,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Connector {
    Aci,
    Adyen,
    Airwallex,
    Applepay,
    Authorizedotnet,
    Bluesnap,
    Braintree,
    Checkout,
    Cybersource,
    #[default]
    Dummy,
	Cashtocode,
    Bambora,
    Dlocal,
    Fiserv,
    Globalpay,
    Klarna,
    Mollie,
    Multisafepay,
    Nuvei,
    Payu,
    Rapyd,
    Shift4,
    Stripe,
    Worldline,
    Worldpay,
    Trustpay,
}

impl Connector {
    pub fn supports_access_token(&self, payment_method: PaymentMethod) -> bool {
        matches!(
            (self, payment_method),
            (Self::Airwallex, _)
                | (Self::Globalpay, _)
                | (Self::Payu, _)
                | (Self::Trustpay, PaymentMethod::BankRedirect)
        )
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
    frunk::LabelledGeneric,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoutableConnectors {
    Aci,
    Adyen,
    Airwallex,
    Authorizedotnet,
    Bambora,
    Bluesnap,
    Braintree,
    Cashtocode,
    Checkout,
    Cybersource,
    Dlocal,
    Fiserv,
    Globalpay,
    Klarna,
    Mollie,
    Multisafepay,
    Nuvei,
    Payu,
    Rapyd,
    Shift4,
    Stripe,
    Trustpay,
    Worldline,
    Worldpay,
}

/// Wallets which support obtaining session object
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SupportedWallets {
    Paypal,
    ApplePay,
    Klarna,
    Gpay,
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
    frunk::LabelledGeneric,
    ToSchema,
)]
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

impl From<AttemptStatus> for IntentStatus {
    fn from(s: AttemptStatus) -> Self {
        match s {
            AttemptStatus::Charged | AttemptStatus::AutoRefunded => Self::Succeeded,

            AttemptStatus::ConfirmationAwaited => Self::RequiresConfirmation,
            AttemptStatus::PaymentMethodAwaited => Self::RequiresPaymentMethod,

            AttemptStatus::Authorized => Self::RequiresCapture,
            AttemptStatus::AuthenticationPending => Self::RequiresCustomerAction,

            AttemptStatus::PartialCharged
            | AttemptStatus::Started
            | AttemptStatus::AuthenticationSuccessful
            | AttemptStatus::Authorizing
            | AttemptStatus::CodInitiated
            | AttemptStatus::VoidInitiated
            | AttemptStatus::CaptureInitiated
            | AttemptStatus::Pending => Self::Processing,

            AttemptStatus::AuthenticationFailed
            | AttemptStatus::AuthorizationFailed
            | AttemptStatus::VoidFailed
            | AttemptStatus::RouterDeclined
            | AttemptStatus::CaptureFailed
            | AttemptStatus::Failure => Self::Failed,
            AttemptStatus::Voided => Self::Cancelled,
        }
    }
}
