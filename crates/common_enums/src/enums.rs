mod payments;
mod ui;
use std::num::{ParseFloatError, TryFromIntError};

pub use payments::ProductType;
use serde::{Deserialize, Serialize};
pub use ui::*;
use utoipa::ToSchema;

pub use super::connector_enums::RoutableConnectors;

#[doc(hidden)]
pub mod diesel_exports {
    pub use super::{
        DbApiVersion as ApiVersion, DbAttemptStatus as AttemptStatus,
        DbAuthenticationType as AuthenticationType, DbBlocklistDataKind as BlocklistDataKind,
        DbCaptureMethod as CaptureMethod, DbCaptureStatus as CaptureStatus,
        DbConnectorType as ConnectorType, DbCountryAlpha2 as CountryAlpha2, DbCurrency as Currency,
        DbDeleteStatus as DeleteStatus, DbDisputeStage as DisputeStage,
        DbDisputeStatus as DisputeStatus, DbFraudCheckStatus as FraudCheckStatus,
        DbFutureUsage as FutureUsage, DbIntentStatus as IntentStatus,
        DbMandateStatus as MandateStatus, DbPaymentMethodIssuerCode as PaymentMethodIssuerCode,
        DbPaymentType as PaymentType, DbRefundStatus as RefundStatus,
        DbRequestIncrementalAuthorization as RequestIncrementalAuthorization,
        DbScaExemptionType as ScaExemptionType,
        DbSuccessBasedRoutingConclusiveState as SuccessBasedRoutingConclusiveState,
        DbWebhookDeliveryAttempt as WebhookDeliveryAttempt,
    };
}

pub type ApplicationResult<T> = Result<T, ApplicationError>;

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Application configuration error")]
    ConfigurationError,

    #[error("Invalid configuration value provided: {0}")]
    InvalidConfigurationValueError(String),

    #[error("Metrics error")]
    MetricsError,

    #[error("I/O: {0}")]
    IoError(std::io::Error),

    #[error("Error while constructing api client: {0}")]
    ApiClientError(ApiClientError),
}

#[derive(Debug, thiserror::Error, PartialEq, Clone)]
pub enum ApiClientError {
    #[error("Header map construction failed")]
    HeaderMapConstructionFailed,
    #[error("Invalid proxy configuration")]
    InvalidProxyConfiguration,
    #[error("Client construction failed")]
    ClientConstructionFailed,
    #[error("Certificate decode failed")]
    CertificateDecodeFailed,
    #[error("Request body serialization failed")]
    BodySerializationFailed,
    #[error("Unexpected state reached/Invariants conflicted")]
    UnexpectedState,

    #[error("Failed to parse URL")]
    UrlParsingFailed,
    #[error("URL encoding of request payload failed")]
    UrlEncodingFailed,
    #[error("Failed to send request to connector {0}")]
    RequestNotSent(String),
    #[error("Failed to decode response")]
    ResponseDecodingFailed,

    #[error("Server responded with Request Timeout")]
    RequestTimeoutReceived,

    #[error("connection closed before a message could complete")]
    ConnectionClosedIncompleteMessage,

    #[error("Server responded with Internal Server Error")]
    InternalServerErrorReceived,
    #[error("Server responded with Bad Gateway")]
    BadGatewayReceived,
    #[error("Server responded with Service Unavailable")]
    ServiceUnavailableReceived,
    #[error("Server responded with Gateway Timeout")]
    GatewayTimeoutReceived,
    #[error("Server responded with unexpected response")]
    UnexpectedServerResponse,
}
impl ApiClientError {
    pub fn is_upstream_timeout(&self) -> bool {
        self == &Self::RequestTimeoutReceived
    }
    pub fn is_connection_closed_before_message_could_complete(&self) -> bool {
        self == &Self::ConnectionClosedIncompleteMessage
    }
}

impl From<std::io::Error> for ApplicationError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

/// The status of the attempt
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Hash,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
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
    PartialChargedAndChargeable,
    Unresolved,
    #[default]
    Pending,
    Failure,
    PaymentMethodAwaited,
    ConfirmationAwaited,
    DeviceDataCollectionPending,
}

impl AttemptStatus {
    pub fn is_terminal_status(self) -> bool {
        match self {
            Self::RouterDeclined
            | Self::Charged
            | Self::AutoRefunded
            | Self::Voided
            | Self::VoidFailed
            | Self::CaptureFailed
            | Self::Failure
            | Self::PartialCharged => true,
            Self::Started
            | Self::AuthenticationFailed
            | Self::AuthenticationPending
            | Self::AuthenticationSuccessful
            | Self::Authorized
            | Self::AuthorizationFailed
            | Self::Authorizing
            | Self::CodInitiated
            | Self::VoidInitiated
            | Self::CaptureInitiated
            | Self::PartialChargedAndChargeable
            | Self::Unresolved
            | Self::Pending
            | Self::PaymentMethodAwaited
            | Self::ConfirmationAwaited
            | Self::DeviceDataCollectionPending => false,
        }
    }
}

/// Indicates the method by which a card is discovered during a payment
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Hash,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CardDiscovery {
    #[default]
    Manual,
    SavedCard,
    ClickToPay,
}

/// Pass this parameter to force 3DS or non 3DS auth for this payment. Some connectors will still force 3DS auth even in case of passing 'no_three_ds' here and vice versa. Default value is 'no_three_ds' if not set
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
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthenticationType {
    /// If the card is enrolled for 3DS authentication, the 3DS based authentication will be activated. The liability of chargeback shift to the issuer
    ThreeDs,
    /// 3DS based authentication will not be activated. The liability of chargeback stays with the merchant.
    #[default]
    NoThreeDs,
}

/// The status of the capture
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
pub enum FraudCheckStatus {
    Fraud,
    ManualReview,
    #[default]
    Pending,
    Legit,
    TransactionFailure,
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
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CaptureStatus {
    // Capture request initiated
    #[default]
    Started,
    // Capture request was successful
    Charged,
    // Capture is pending at connector side
    Pending,
    // Capture request failed
    Failed,
}

#[derive(
    Default,
    Clone,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthorizationStatus {
    Success,
    Failure,
    // Processing state is before calling connector
    #[default]
    Processing,
    // Requires merchant action
    Unresolved,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SessionUpdateStatus {
    Success,
    Failure,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BlocklistDataKind {
    PaymentMethod,
    CardBin,
    ExtendedCardBin,
}

/// Default value if not passed is set to 'automatic' which results in Auth and Capture in one single API request. Pass 'manual' or 'manual_multiple' in case you want do a separate Auth and Capture by first authorizing and placing a hold on your customer's funds so that you can use the Payments/Capture endpoint later to capture the authorized amount. Pass 'manual' if you want to only capture the amount later once or 'manual_multiple' if you want to capture the funds multiple times later. Both 'manual' and 'manual_multiple' are only supported by a specific list of processors
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
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
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
    /// Handles separate auth and capture sequentially; same as `Automatic` for most connectors.
    SequentialAutomatic,
}

/// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
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
    /// Acquirers, Gateways etc
    PayoutProcessor,
    /// PaymentMethods Auth Services
    PaymentMethodAuth,
    /// 3DS Authentication Service Providers
    AuthenticationProcessor,
    /// Tax Calculation Processor
    TaxProcessor,
}

#[derive(Debug, Eq, PartialEq)]
pub enum PaymentAction {
    PSync,
    CompleteAuthorize,
    PaymentAuthenticateCompleteAuthorize,
}

#[derive(Clone, PartialEq)]
pub enum CallConnectorAction {
    Trigger,
    Avoid,
    StatusUpdate {
        status: AttemptStatus,
        error_code: Option<String>,
        error_message: Option<String>,
    },
    HandleResponse(Vec<u8>),
}

/// The three letter ISO currency code in uppercase. Eg: 'USD' for the United States Dollar.
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
    strum::VariantNames,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
pub enum Currency {
    AED,
    AFN,
    ALL,
    AMD,
    ANG,
    AOA,
    ARS,
    AUD,
    AWG,
    AZN,
    BAM,
    BBD,
    BDT,
    BGN,
    BHD,
    BIF,
    BMD,
    BND,
    BOB,
    BRL,
    BSD,
    BTN,
    BWP,
    BYN,
    BZD,
    CAD,
    CDF,
    CHF,
    CLP,
    CNY,
    COP,
    CRC,
    CUP,
    CVE,
    CZK,
    DJF,
    DKK,
    DOP,
    DZD,
    EGP,
    ERN,
    ETB,
    EUR,
    FJD,
    FKP,
    GBP,
    GEL,
    GHS,
    GIP,
    GMD,
    GNF,
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
    IQD,
    IRR,
    ISK,
    JMD,
    JOD,
    JPY,
    KES,
    KGS,
    KHR,
    KMF,
    KPW,
    KRW,
    KWD,
    KYD,
    KZT,
    LAK,
    LBP,
    LKR,
    LRD,
    LSL,
    LYD,
    MAD,
    MDL,
    MGA,
    MKD,
    MMK,
    MNT,
    MOP,
    MRU,
    MUR,
    MVR,
    MWK,
    MXN,
    MYR,
    MZN,
    NAD,
    NGN,
    NIO,
    NOK,
    NPR,
    NZD,
    OMR,
    PAB,
    PEN,
    PGK,
    PHP,
    PKR,
    PLN,
    PYG,
    QAR,
    RON,
    RSD,
    RUB,
    RWF,
    SAR,
    SBD,
    SCR,
    SDG,
    SEK,
    SGD,
    SHP,
    SLE,
    SLL,
    SOS,
    SRD,
    SSP,
    STN,
    SVC,
    SYP,
    SZL,
    THB,
    TJS,
    TMT,
    TND,
    TOP,
    TRY,
    TTD,
    TWD,
    TZS,
    UAH,
    UGX,
    #[default]
    USD,
    UYU,
    UZS,
    VES,
    VND,
    VUV,
    WST,
    XAF,
    XCD,
    XOF,
    XPF,
    YER,
    ZAR,
    ZMW,
    ZWL,
}

impl Currency {
    /// Convert the amount to its base denomination based on Currency and return String
    pub fn to_currency_base_unit(self, amount: i64) -> Result<String, TryFromIntError> {
        let amount_f64 = self.to_currency_base_unit_asf64(amount)?;
        Ok(format!("{amount_f64:.2}"))
    }

    /// Convert the amount to its base denomination based on Currency and return f64
    pub fn to_currency_base_unit_asf64(self, amount: i64) -> Result<f64, TryFromIntError> {
        let amount_f64: f64 = u32::try_from(amount)?.into();
        let amount = if self.is_zero_decimal_currency() {
            amount_f64
        } else if self.is_three_decimal_currency() {
            amount_f64 / 1000.00
        } else {
            amount_f64 / 100.00
        };
        Ok(amount)
    }

    ///Convert the higher decimal amount to its base absolute units
    pub fn to_currency_lower_unit(self, amount: String) -> Result<String, ParseFloatError> {
        let amount_f64 = amount.parse::<f64>()?;
        let amount_string = if self.is_zero_decimal_currency() {
            amount_f64
        } else if self.is_three_decimal_currency() {
            amount_f64 * 1000.00
        } else {
            amount_f64 * 100.00
        };
        Ok(amount_string.to_string())
    }

    /// Convert the amount to its base denomination based on Currency and check for zero decimal currency and return String
    /// Paypal Connector accepts Zero and Two decimal currency but not three decimal and it should be updated as required for 3 decimal currencies.
    /// Paypal Ref - https://developer.paypal.com/docs/reports/reference/paypal-supported-currencies/
    pub fn to_currency_base_unit_with_zero_decimal_check(
        self,
        amount: i64,
    ) -> Result<String, TryFromIntError> {
        let amount_f64 = self.to_currency_base_unit_asf64(amount)?;
        if self.is_zero_decimal_currency() {
            Ok(amount_f64.to_string())
        } else {
            Ok(format!("{amount_f64:.2}"))
        }
    }

    pub fn iso_4217(self) -> &'static str {
        match self {
            Self::AED => "784",
            Self::AFN => "971",
            Self::ALL => "008",
            Self::AMD => "051",
            Self::ANG => "532",
            Self::AOA => "973",
            Self::ARS => "032",
            Self::AUD => "036",
            Self::AWG => "533",
            Self::AZN => "944",
            Self::BAM => "977",
            Self::BBD => "052",
            Self::BDT => "050",
            Self::BGN => "975",
            Self::BHD => "048",
            Self::BIF => "108",
            Self::BMD => "060",
            Self::BND => "096",
            Self::BOB => "068",
            Self::BRL => "986",
            Self::BSD => "044",
            Self::BTN => "064",
            Self::BWP => "072",
            Self::BYN => "933",
            Self::BZD => "084",
            Self::CAD => "124",
            Self::CDF => "976",
            Self::CHF => "756",
            Self::CLP => "152",
            Self::COP => "170",
            Self::CRC => "188",
            Self::CUP => "192",
            Self::CVE => "132",
            Self::CZK => "203",
            Self::DJF => "262",
            Self::DKK => "208",
            Self::DOP => "214",
            Self::DZD => "012",
            Self::EGP => "818",
            Self::ERN => "232",
            Self::ETB => "230",
            Self::EUR => "978",
            Self::FJD => "242",
            Self::FKP => "238",
            Self::GBP => "826",
            Self::GEL => "981",
            Self::GHS => "936",
            Self::GIP => "292",
            Self::GMD => "270",
            Self::GNF => "324",
            Self::GTQ => "320",
            Self::GYD => "328",
            Self::HKD => "344",
            Self::HNL => "340",
            Self::HTG => "332",
            Self::HUF => "348",
            Self::HRK => "191",
            Self::IDR => "360",
            Self::ILS => "376",
            Self::INR => "356",
            Self::IQD => "368",
            Self::IRR => "364",
            Self::ISK => "352",
            Self::JMD => "388",
            Self::JOD => "400",
            Self::JPY => "392",
            Self::KES => "404",
            Self::KGS => "417",
            Self::KHR => "116",
            Self::KMF => "174",
            Self::KPW => "408",
            Self::KRW => "410",
            Self::KWD => "414",
            Self::KYD => "136",
            Self::KZT => "398",
            Self::LAK => "418",
            Self::LBP => "422",
            Self::LKR => "144",
            Self::LRD => "430",
            Self::LSL => "426",
            Self::LYD => "434",
            Self::MAD => "504",
            Self::MDL => "498",
            Self::MGA => "969",
            Self::MKD => "807",
            Self::MMK => "104",
            Self::MNT => "496",
            Self::MOP => "446",
            Self::MRU => "929",
            Self::MUR => "480",
            Self::MVR => "462",
            Self::MWK => "454",
            Self::MXN => "484",
            Self::MYR => "458",
            Self::MZN => "943",
            Self::NAD => "516",
            Self::NGN => "566",
            Self::NIO => "558",
            Self::NOK => "578",
            Self::NPR => "524",
            Self::NZD => "554",
            Self::OMR => "512",
            Self::PAB => "590",
            Self::PEN => "604",
            Self::PGK => "598",
            Self::PHP => "608",
            Self::PKR => "586",
            Self::PLN => "985",
            Self::PYG => "600",
            Self::QAR => "634",
            Self::RON => "946",
            Self::CNY => "156",
            Self::RSD => "941",
            Self::RUB => "643",
            Self::RWF => "646",
            Self::SAR => "682",
            Self::SBD => "090",
            Self::SCR => "690",
            Self::SDG => "938",
            Self::SEK => "752",
            Self::SGD => "702",
            Self::SHP => "654",
            Self::SLE => "925",
            Self::SLL => "694",
            Self::SOS => "706",
            Self::SRD => "968",
            Self::SSP => "728",
            Self::STN => "930",
            Self::SVC => "222",
            Self::SYP => "760",
            Self::SZL => "748",
            Self::THB => "764",
            Self::TJS => "972",
            Self::TMT => "934",
            Self::TND => "788",
            Self::TOP => "776",
            Self::TRY => "949",
            Self::TTD => "780",
            Self::TWD => "901",
            Self::TZS => "834",
            Self::UAH => "980",
            Self::UGX => "800",
            Self::USD => "840",
            Self::UYU => "858",
            Self::UZS => "860",
            Self::VES => "928",
            Self::VND => "704",
            Self::VUV => "548",
            Self::WST => "882",
            Self::XAF => "950",
            Self::XCD => "951",
            Self::XOF => "952",
            Self::XPF => "953",
            Self::YER => "886",
            Self::ZAR => "710",
            Self::ZMW => "967",
            Self::ZWL => "932",
        }
    }

    pub fn is_zero_decimal_currency(self) -> bool {
        match self {
            Self::BIF
            | Self::CLP
            | Self::DJF
            | Self::GNF
            | Self::IRR
            | Self::JPY
            | Self::KMF
            | Self::KRW
            | Self::MGA
            | Self::PYG
            | Self::RWF
            | Self::UGX
            | Self::VND
            | Self::VUV
            | Self::XAF
            | Self::XOF
            | Self::XPF => true,
            Self::AED
            | Self::AFN
            | Self::ALL
            | Self::AMD
            | Self::ANG
            | Self::AOA
            | Self::ARS
            | Self::AUD
            | Self::AWG
            | Self::AZN
            | Self::BAM
            | Self::BBD
            | Self::BDT
            | Self::BGN
            | Self::BHD
            | Self::BMD
            | Self::BND
            | Self::BOB
            | Self::BRL
            | Self::BSD
            | Self::BTN
            | Self::BWP
            | Self::BYN
            | Self::BZD
            | Self::CAD
            | Self::CDF
            | Self::CHF
            | Self::CNY
            | Self::COP
            | Self::CRC
            | Self::CUP
            | Self::CVE
            | Self::CZK
            | Self::DKK
            | Self::DOP
            | Self::DZD
            | Self::EGP
            | Self::ERN
            | Self::ETB
            | Self::EUR
            | Self::FJD
            | Self::FKP
            | Self::GBP
            | Self::GEL
            | Self::GHS
            | Self::GIP
            | Self::GMD
            | Self::GTQ
            | Self::GYD
            | Self::HKD
            | Self::HNL
            | Self::HRK
            | Self::HTG
            | Self::HUF
            | Self::IDR
            | Self::ILS
            | Self::INR
            | Self::IQD
            | Self::ISK
            | Self::JMD
            | Self::JOD
            | Self::KES
            | Self::KGS
            | Self::KHR
            | Self::KPW
            | Self::KWD
            | Self::KYD
            | Self::KZT
            | Self::LAK
            | Self::LBP
            | Self::LKR
            | Self::LRD
            | Self::LSL
            | Self::LYD
            | Self::MAD
            | Self::MDL
            | Self::MKD
            | Self::MMK
            | Self::MNT
            | Self::MOP
            | Self::MRU
            | Self::MUR
            | Self::MVR
            | Self::MWK
            | Self::MXN
            | Self::MYR
            | Self::MZN
            | Self::NAD
            | Self::NGN
            | Self::NIO
            | Self::NOK
            | Self::NPR
            | Self::NZD
            | Self::OMR
            | Self::PAB
            | Self::PEN
            | Self::PGK
            | Self::PHP
            | Self::PKR
            | Self::PLN
            | Self::QAR
            | Self::RON
            | Self::RSD
            | Self::RUB
            | Self::SAR
            | Self::SBD
            | Self::SCR
            | Self::SDG
            | Self::SEK
            | Self::SGD
            | Self::SHP
            | Self::SLE
            | Self::SLL
            | Self::SOS
            | Self::SRD
            | Self::SSP
            | Self::STN
            | Self::SVC
            | Self::SYP
            | Self::SZL
            | Self::THB
            | Self::TJS
            | Self::TMT
            | Self::TND
            | Self::TOP
            | Self::TRY
            | Self::TTD
            | Self::TWD
            | Self::TZS
            | Self::UAH
            | Self::USD
            | Self::UYU
            | Self::UZS
            | Self::VES
            | Self::WST
            | Self::XCD
            | Self::YER
            | Self::ZAR
            | Self::ZMW
            | Self::ZWL => false,
        }
    }

    pub fn is_three_decimal_currency(self) -> bool {
        match self {
            Self::BHD | Self::IQD | Self::JOD | Self::KWD | Self::LYD | Self::OMR | Self::TND => {
                true
            }
            Self::AED
            | Self::AFN
            | Self::ALL
            | Self::AMD
            | Self::AOA
            | Self::ANG
            | Self::ARS
            | Self::AUD
            | Self::AWG
            | Self::AZN
            | Self::BAM
            | Self::BBD
            | Self::BDT
            | Self::BGN
            | Self::BIF
            | Self::BMD
            | Self::BND
            | Self::BOB
            | Self::BRL
            | Self::BSD
            | Self::BTN
            | Self::BWP
            | Self::BYN
            | Self::BZD
            | Self::CAD
            | Self::CDF
            | Self::CHF
            | Self::CLP
            | Self::CNY
            | Self::COP
            | Self::CRC
            | Self::CUP
            | Self::CVE
            | Self::CZK
            | Self::DJF
            | Self::DKK
            | Self::DOP
            | Self::DZD
            | Self::EGP
            | Self::ERN
            | Self::ETB
            | Self::EUR
            | Self::FJD
            | Self::FKP
            | Self::GBP
            | Self::GEL
            | Self::GHS
            | Self::GIP
            | Self::GMD
            | Self::GNF
            | Self::GTQ
            | Self::GYD
            | Self::HKD
            | Self::HNL
            | Self::HRK
            | Self::HTG
            | Self::HUF
            | Self::IDR
            | Self::ILS
            | Self::INR
            | Self::IRR
            | Self::ISK
            | Self::JMD
            | Self::JPY
            | Self::KES
            | Self::KGS
            | Self::KHR
            | Self::KMF
            | Self::KPW
            | Self::KRW
            | Self::KYD
            | Self::KZT
            | Self::LAK
            | Self::LBP
            | Self::LKR
            | Self::LRD
            | Self::LSL
            | Self::MAD
            | Self::MDL
            | Self::MGA
            | Self::MKD
            | Self::MMK
            | Self::MNT
            | Self::MOP
            | Self::MRU
            | Self::MUR
            | Self::MVR
            | Self::MWK
            | Self::MXN
            | Self::MYR
            | Self::MZN
            | Self::NAD
            | Self::NGN
            | Self::NIO
            | Self::NOK
            | Self::NPR
            | Self::NZD
            | Self::PAB
            | Self::PEN
            | Self::PGK
            | Self::PHP
            | Self::PKR
            | Self::PLN
            | Self::PYG
            | Self::QAR
            | Self::RON
            | Self::RSD
            | Self::RUB
            | Self::RWF
            | Self::SAR
            | Self::SBD
            | Self::SCR
            | Self::SDG
            | Self::SEK
            | Self::SGD
            | Self::SHP
            | Self::SLE
            | Self::SLL
            | Self::SOS
            | Self::SRD
            | Self::SSP
            | Self::STN
            | Self::SVC
            | Self::SYP
            | Self::SZL
            | Self::THB
            | Self::TJS
            | Self::TMT
            | Self::TOP
            | Self::TRY
            | Self::TTD
            | Self::TWD
            | Self::TZS
            | Self::UAH
            | Self::UGX
            | Self::USD
            | Self::UYU
            | Self::UZS
            | Self::VES
            | Self::VND
            | Self::VUV
            | Self::WST
            | Self::XAF
            | Self::XCD
            | Self::XPF
            | Self::XOF
            | Self::YER
            | Self::ZAR
            | Self::ZMW
            | Self::ZWL => false,
        }
    }

    pub fn number_of_digits_after_decimal_point(self) -> u8 {
        if self.is_zero_decimal_currency() {
            0
        } else if self.is_three_decimal_currency() {
            3
        } else {
            2
        }
    }
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventClass {
    Payments,
    Refunds,
    Disputes,
    Mandates,
    #[cfg(feature = "payouts")]
    Payouts,
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventType {
    /// Authorize + Capture success
    PaymentSucceeded,
    /// Authorize + Capture failed
    PaymentFailed,
    PaymentProcessing,
    PaymentCancelled,
    PaymentAuthorized,
    PaymentCaptured,
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
    MandateActive,
    MandateRevoked,
    PayoutSuccess,
    PayoutFailed,
    PayoutInitiated,
    PayoutProcessing,
    PayoutCancelled,
    PayoutExpired,
    PayoutReversed,
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum WebhookDeliveryAttempt {
    InitialAttempt,
    AutomaticRetry,
    ManualRetry,
}

// TODO: This decision about using KV mode or not,
// should be taken at a top level rather than pushing it down to individual functions via an enum.
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MerchantStorageScheme {
    #[default]
    PostgresOnly,
    RedisKv,
}

/// The status of the current payment that was made
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumIter,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum IntentStatus {
    /// The payment has succeeded. Refunds and disputes can be initiated.
    /// Manual retries are not allowed to be performed.
    Succeeded,
    /// The payment has failed. Refunds and disputes cannot be initiated.
    /// This payment can be retried manually with a new payment attempt.
    Failed,
    /// This payment has been cancelled.
    Cancelled,
    /// This payment is still being processed by the payment processor.
    /// The status update might happen through webhooks or polling with the connector.
    Processing,
    /// The payment is waiting on some action from the customer.
    RequiresCustomerAction,
    /// The payment is waiting on some action from the merchant
    /// This would be in case of manual fraud approval
    RequiresMerchantAction,
    /// The payment is waiting to be confirmed with the payment method by the customer.
    RequiresPaymentMethod,
    #[default]
    RequiresConfirmation,
    /// The payment has been authorized, and it waiting to be captured.
    RequiresCapture,
    /// The payment has been captured partially. The remaining amount is cannot be captured.
    PartiallyCaptured,
    /// The payment has been captured partially and the remaining amount is capturable
    PartiallyCapturedAndCapturable,
}

impl IntentStatus {
    /// Indicates whether the payment intent is in terminal state or not
    pub fn is_in_terminal_state(self) -> bool {
        match self {
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::PartiallyCaptured => true,
            Self::Processing
            | Self::RequiresCustomerAction
            | Self::RequiresMerchantAction
            | Self::RequiresPaymentMethod
            | Self::RequiresConfirmation
            | Self::RequiresCapture
            | Self::PartiallyCapturedAndCapturable => false,
        }
    }

    /// Indicates whether the syncing with the connector should be allowed or not
    pub fn should_force_sync_with_connector(self) -> bool {
        match self {
            // Confirm has not happened yet
            Self::RequiresConfirmation
            | Self::RequiresPaymentMethod
            // Once the status is success, failed or cancelled need not force sync with the connector
            | Self::Succeeded
            | Self::Failed
            | Self::Cancelled
            |  Self::PartiallyCaptured
            |  Self::RequiresCapture => false,
            Self::Processing
            | Self::RequiresCustomerAction
            | Self::RequiresMerchantAction
            | Self::PartiallyCapturedAndCapturable
            => true,
        }
    }
}

/// Indicates that you intend to make future payments with the payment methods used for this Payment. Providing this parameter will attach the payment method to the Customer, if present, after the Payment is confirmed and any required actions from the user are complete.
/// - On_session - Payment method saved only at hyperswitch when consent is provided by the user. CVV will asked during the returning user payment
/// - Off_session - Payment method saved at both hyperswitch and Processor when consent is provided by the user. No input is required during the returning user payment.
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
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FutureUsage {
    OffSession,
    #[default]
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
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
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

/// Payment Method Status
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodStatus {
    /// Indicates that the payment method is active and can be used for payments.
    Active,
    /// Indicates that the payment method is not active and hence cannot be used for payments.
    Inactive,
    /// Indicates that the payment method is awaiting some data or action before it can be marked
    /// as 'active'.
    Processing,
    /// Indicates that the payment method is awaiting some data before changing state to active
    AwaitingData,
}

impl From<AttemptStatus> for PaymentMethodStatus {
    fn from(attempt_status: AttemptStatus) -> Self {
        match attempt_status {
            AttemptStatus::Failure
            | AttemptStatus::Voided
            | AttemptStatus::Started
            | AttemptStatus::Pending
            | AttemptStatus::Unresolved
            | AttemptStatus::CodInitiated
            | AttemptStatus::Authorizing
            | AttemptStatus::VoidInitiated
            | AttemptStatus::AuthorizationFailed
            | AttemptStatus::RouterDeclined
            | AttemptStatus::AuthenticationSuccessful
            | AttemptStatus::PaymentMethodAwaited
            | AttemptStatus::AuthenticationFailed
            | AttemptStatus::AuthenticationPending
            | AttemptStatus::CaptureInitiated
            | AttemptStatus::CaptureFailed
            | AttemptStatus::VoidFailed
            | AttemptStatus::AutoRefunded
            | AttemptStatus::PartialCharged
            | AttemptStatus::PartialChargedAndChargeable
            | AttemptStatus::ConfirmationAwaited
            | AttemptStatus::DeviceDataCollectionPending => Self::Inactive,
            AttemptStatus::Charged | AttemptStatus::Authorized => Self::Active,
        }
    }
}

/// To indicate the type of payment experience that the customer would go through
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
    /// Contains the data for displaying wait screen
    DisplayWaitScreen,
    /// Represents that otp needs to be collect and contains if consent is required
    CollectOtp,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, strum::Display)]
#[serde(rename_all = "lowercase")]
pub enum SamsungPayCardBrand {
    Visa,
    MasterCard,
    Amex,
    Discover,
    Unknown,
}

/// Indicates the sub type of payment method. Eg: 'google_pay' & 'apple_pay' for wallets.
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
    strum::VariantNames,
    strum::EnumIter,
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
    Alfamart,
    AliPay,
    AliPayHk,
    Alma,
    AmazonPay,
    ApplePay,
    Atome,
    Bacs,
    BancontactCard,
    Becs,
    Benefit,
    Bizum,
    Blik,
    Boleto,
    BcaBankTransfer,
    BniVa,
    BriVa,
    CardRedirect,
    CimbVa,
    #[serde(rename = "classic")]
    ClassicReward,
    Credit,
    CryptoCurrency,
    Cashapp,
    Dana,
    DanamonVa,
    Debit,
    DuitNow,
    Efecty,
    Eps,
    Fps,
    Evoucher,
    Giropay,
    Givex,
    GooglePay,
    GoPay,
    Gcash,
    Ideal,
    Interac,
    Indomaret,
    Klarna,
    KakaoPay,
    LocalBankRedirect,
    MandiriVa,
    Knet,
    MbWay,
    MobilePay,
    Momo,
    MomoAtm,
    Multibanco,
    OnlineBankingThailand,
    OnlineBankingCzechRepublic,
    OnlineBankingFinland,
    OnlineBankingFpx,
    OnlineBankingPoland,
    OnlineBankingSlovakia,
    Oxxo,
    PagoEfectivo,
    PermataBankTransfer,
    OpenBankingUk,
    PayBright,
    Paypal,
    Paze,
    Pix,
    PaySafeCard,
    Przelewy24,
    PromptPay,
    Pse,
    RedCompra,
    RedPagos,
    SamsungPay,
    Sepa,
    Sofort,
    Swish,
    TouchNGo,
    Trustly,
    Twint,
    UpiCollect,
    UpiIntent,
    Vipps,
    VietQr,
    Venmo,
    Walley,
    WeChatPay,
    SevenEleven,
    Lawson,
    MiniStop,
    FamilyMart,
    Seicomart,
    PayEasy,
    LocalBankTransfer,
    Mifinity,
    #[serde(rename = "open_banking_pis")]
    OpenBankingPIS,
    DirectCarrierBilling,
}

impl masking::SerializableSecret for PaymentMethodType {}

/// Indicates the type of payment method. Eg: 'card', 'wallet', etc.
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
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PaymentMethod {
    #[default]
    Card,
    CardRedirect,
    PayLater,
    Wallet,
    BankRedirect,
    BankTransfer,
    Crypto,
    BankDebit,
    Reward,
    RealTimePayment,
    Upi,
    Voucher,
    GiftCard,
    OpenBanking,
    MobilePayment,
}

/// The type of the payment that differentiates between normal and various types of mandate payments. Use 'setup_mandate' in case of zero auth flow.
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PaymentType {
    #[default]
    Normal,
    NewMandate,
    SetupMandate,
    RecurringMandate,
}

/// SCA Exemptions types available for authentication
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ScaExemptionType {
    #[default]
    LowValue,
    TransactionRiskAnalysis,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    serde::Serialize,
    serde::Deserialize,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RefundStatus {
    #[serde(alias = "Failure")]
    Failure,
    #[serde(alias = "ManualReview")]
    ManualReview,
    #[default]
    #[serde(alias = "Pending")]
    Pending,
    #[serde(alias = "Success")]
    Success,
    #[serde(alias = "TransactionFailure")]
    TransactionFailure,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    serde::Serialize,
    serde::Deserialize,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RelayStatus {
    Created,
    #[default]
    Pending,
    Success,
    Failure,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    serde::Serialize,
    serde::Deserialize,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RelayType {
    Refund,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    serde::Serialize,
    serde::Deserialize,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
pub enum FrmTransactionType {
    #[default]
    PreFrm,
    PostFrm,
}

/// The status of the mandate, which indicates whether it can be used to initiate a payment.
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MandateStatus {
    #[default]
    Active,
    Inactive,
    Pending,
    Revoked,
}

/// Indicates the card network.
#[derive(
    Clone,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "text")]
pub enum CardNetwork {
    #[serde(alias = "VISA")]
    Visa,
    #[serde(alias = "MASTERCARD")]
    Mastercard,
    #[serde(alias = "AMERICANEXPRESS")]
    #[serde(alias = "AMEX")]
    AmericanExpress,
    JCB,
    #[serde(alias = "DINERSCLUB")]
    DinersClub,
    #[serde(alias = "DISCOVER")]
    Discover,
    #[serde(alias = "CARTESBANCAIRES")]
    CartesBancaires,
    #[serde(alias = "UNIONPAY")]
    UnionPay,
    #[serde(alias = "INTERAC")]
    Interac,
    #[serde(alias = "RUPAY")]
    RuPay,
    #[serde(alias = "MAESTRO")]
    Maestro,
}

/// Stage of the dispute
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
    strum::EnumIter,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DisputeStage {
    PreDispute,
    #[default]
    Dispute,
    PreArbitration,
}

/// Status of the dispute
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
    strum::EnumIter,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
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
    Clone,
    Debug,
    Eq,
    Default,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumIter,
    strum::EnumString,
    utoipa::ToSchema,
    Copy
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RequestIncrementalAuthorization {
    True,
    False,
    #[default]
    Default,
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

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    Deserialize,
    Serialize,
)]
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

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum UsStatesAbbreviation {
    AL,
    AK,
    AS,
    AZ,
    AR,
    CA,
    CO,
    CT,
    DE,
    DC,
    FM,
    FL,
    GA,
    GU,
    HI,
    ID,
    IL,
    IN,
    IA,
    KS,
    KY,
    LA,
    ME,
    MH,
    MD,
    MA,
    MI,
    MN,
    MS,
    MO,
    MT,
    NE,
    NV,
    NH,
    NJ,
    NM,
    NY,
    NC,
    ND,
    MP,
    OH,
    OK,
    OR,
    PW,
    PA,
    PR,
    RI,
    SC,
    SD,
    TN,
    TX,
    UT,
    VT,
    VI,
    VA,
    WA,
    WV,
    WI,
    WY,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum CanadaStatesAbbreviation {
    AB,
    BC,
    MB,
    NB,
    NL,
    NT,
    NS,
    NU,
    ON,
    PE,
    QC,
    SK,
    YT,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum AlbaniaStatesAbbreviation {
    #[strum(serialize = "01")]
    Berat,
    #[strum(serialize = "09")]
    Diber,
    #[strum(serialize = "02")]
    Durres,
    #[strum(serialize = "03")]
    Elbasan,
    #[strum(serialize = "04")]
    Fier,
    #[strum(serialize = "05")]
    Gjirokaster,
    #[strum(serialize = "06")]
    Korce,
    #[strum(serialize = "07")]
    Kukes,
    #[strum(serialize = "08")]
    Lezhe,
    #[strum(serialize = "10")]
    Shkoder,
    #[strum(serialize = "11")]
    Tirane,
    #[strum(serialize = "12")]
    Vlore,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum AndorraStatesAbbreviation {
    #[strum(serialize = "07")]
    AndorraLaVella,
    #[strum(serialize = "02")]
    Canillo,
    #[strum(serialize = "03")]
    Encamp,
    #[strum(serialize = "08")]
    EscaldesEngordany,
    #[strum(serialize = "04")]
    LaMassana,
    #[strum(serialize = "05")]
    Ordino,
    #[strum(serialize = "06")]
    SantJuliaDeLoria,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum AustriaStatesAbbreviation {
    #[strum(serialize = "1")]
    Burgenland,
    #[strum(serialize = "2")]
    Carinthia,
    #[strum(serialize = "3")]
    LowerAustria,
    #[strum(serialize = "5")]
    Salzburg,
    #[strum(serialize = "6")]
    Styria,
    #[strum(serialize = "7")]
    Tyrol,
    #[strum(serialize = "4")]
    UpperAustria,
    #[strum(serialize = "9")]
    Vienna,
    #[strum(serialize = "8")]
    Vorarlberg,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum BelarusStatesAbbreviation {
    #[strum(serialize = "BR")]
    BrestRegion,
    #[strum(serialize = "HO")]
    GomelRegion,
    #[strum(serialize = "HR")]
    GrodnoRegion,
    #[strum(serialize = "HM")]
    Minsk,
    #[strum(serialize = "MI")]
    MinskRegion,
    #[strum(serialize = "MA")]
    MogilevRegion,
    #[strum(serialize = "VI")]
    VitebskRegion,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum BosniaAndHerzegovinaStatesAbbreviation {
    #[strum(serialize = "05")]
    BosnianPodrinjeCanton,
    #[strum(serialize = "BRC")]
    BrckoDistrict,
    #[strum(serialize = "10")]
    Canton10,
    #[strum(serialize = "06")]
    CentralBosniaCanton,
    #[strum(serialize = "BIH")]
    FederationOfBosniaAndHerzegovina,
    #[strum(serialize = "07")]
    HerzegovinaNeretvaCanton,
    #[strum(serialize = "02")]
    PosavinaCanton,
    #[strum(serialize = "SRP")]
    RepublikaSrpska,
    #[strum(serialize = "09")]
    SarajevoCanton,
    #[strum(serialize = "03")]
    TuzlaCanton,
    #[strum(serialize = "01")]
    UnaSanaCanton,
    #[strum(serialize = "08")]
    WestHerzegovinaCanton,
    #[strum(serialize = "04")]
    ZenicaDobojCanton,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum BulgariaStatesAbbreviation {
    #[strum(serialize = "01")]
    BlagoevgradProvince,
    #[strum(serialize = "02")]
    BurgasProvince,
    #[strum(serialize = "08")]
    DobrichProvince,
    #[strum(serialize = "07")]
    GabrovoProvince,
    #[strum(serialize = "26")]
    HaskovoProvince,
    #[strum(serialize = "09")]
    KardzhaliProvince,
    #[strum(serialize = "10")]
    KyustendilProvince,
    #[strum(serialize = "11")]
    LovechProvince,
    #[strum(serialize = "12")]
    MontanaProvince,
    #[strum(serialize = "13")]
    PazardzhikProvince,
    #[strum(serialize = "14")]
    PernikProvince,
    #[strum(serialize = "15")]
    PlevenProvince,
    #[strum(serialize = "16")]
    PlovdivProvince,
    #[strum(serialize = "17")]
    RazgradProvince,
    #[strum(serialize = "18")]
    RuseProvince,
    #[strum(serialize = "27")]
    Shumen,
    #[strum(serialize = "19")]
    SilistraProvince,
    #[strum(serialize = "20")]
    SlivenProvince,
    #[strum(serialize = "21")]
    SmolyanProvince,
    #[strum(serialize = "22")]
    SofiaCityProvince,
    #[strum(serialize = "23")]
    SofiaProvince,
    #[strum(serialize = "24")]
    StaraZagoraProvince,
    #[strum(serialize = "25")]
    TargovishteProvince,
    #[strum(serialize = "03")]
    VarnaProvince,
    #[strum(serialize = "04")]
    VelikoTarnovoProvince,
    #[strum(serialize = "05")]
    VidinProvince,
    #[strum(serialize = "06")]
    VratsaProvince,
    #[strum(serialize = "28")]
    YambolProvince,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum CroatiaStatesAbbreviation {
    #[strum(serialize = "07")]
    BjelovarBilogoraCounty,
    #[strum(serialize = "12")]
    BrodPosavinaCounty,
    #[strum(serialize = "19")]
    DubrovnikNeretvaCounty,
    #[strum(serialize = "18")]
    IstriaCounty,
    #[strum(serialize = "06")]
    KoprivnicaKrizevciCounty,
    #[strum(serialize = "02")]
    KrapinaZagorjeCounty,
    #[strum(serialize = "09")]
    LikaSenjCounty,
    #[strum(serialize = "20")]
    MedimurjeCounty,
    #[strum(serialize = "14")]
    OsijekBaranjaCounty,
    #[strum(serialize = "11")]
    PozegaSlavoniaCounty,
    #[strum(serialize = "08")]
    PrimorjeGorskiKotarCounty,
    #[strum(serialize = "03")]
    SisakMoslavinaCounty,
    #[strum(serialize = "17")]
    SplitDalmatiaCounty,
    #[strum(serialize = "05")]
    VarazdinCounty,
    #[strum(serialize = "10")]
    ViroviticaPodravinaCounty,
    #[strum(serialize = "16")]
    VukovarSyrmiaCounty,
    #[strum(serialize = "13")]
    ZadarCounty,
    #[strum(serialize = "21")]
    Zagreb,
    #[strum(serialize = "01")]
    ZagrebCounty,
    #[strum(serialize = "15")]
    SibenikKninCounty,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum CzechRepublicStatesAbbreviation {
    #[strum(serialize = "201")]
    BenesovDistrict,
    #[strum(serialize = "202")]
    BerounDistrict,
    #[strum(serialize = "641")]
    BlanskoDistrict,
    #[strum(serialize = "642")]
    BrnoCityDistrict,
    #[strum(serialize = "643")]
    BrnoCountryDistrict,
    #[strum(serialize = "801")]
    BruntalDistrict,
    #[strum(serialize = "644")]
    BreclavDistrict,
    #[strum(serialize = "20")]
    CentralBohemianRegion,
    #[strum(serialize = "411")]
    ChebDistrict,
    #[strum(serialize = "422")]
    ChomutovDistrict,
    #[strum(serialize = "531")]
    ChrudimDistrict,
    #[strum(serialize = "321")]
    DomazliceDistrict,
    #[strum(serialize = "421")]
    DecinDistrict,
    #[strum(serialize = "802")]
    FrydekMistekDistrict,
    #[strum(serialize = "631")]
    HavlickuvBrodDistrict,
    #[strum(serialize = "645")]
    HodoninDistrict,
    #[strum(serialize = "120")]
    HorniPocernice,
    #[strum(serialize = "521")]
    HradecKraloveDistrict,
    #[strum(serialize = "52")]
    HradecKraloveRegion,
    #[strum(serialize = "512")]
    JablonecNadNisouDistrict,
    #[strum(serialize = "711")]
    JesenikDistrict,
    #[strum(serialize = "632")]
    JihlavaDistrict,
    #[strum(serialize = "313")]
    JindrichuvHradecDistrict,
    #[strum(serialize = "522")]
    JicinDistrict,
    #[strum(serialize = "412")]
    KarlovyVaryDistrict,
    #[strum(serialize = "41")]
    KarlovyVaryRegion,
    #[strum(serialize = "803")]
    KarvinaDistrict,
    #[strum(serialize = "203")]
    KladnoDistrict,
    #[strum(serialize = "322")]
    KlatovyDistrict,
    #[strum(serialize = "204")]
    KolinDistrict,
    #[strum(serialize = "721")]
    KromerizDistrict,
    #[strum(serialize = "513")]
    LiberecDistrict,
    #[strum(serialize = "51")]
    LiberecRegion,
    #[strum(serialize = "423")]
    LitomericeDistrict,
    #[strum(serialize = "424")]
    LounyDistrict,
    #[strum(serialize = "207")]
    MladaBoleslavDistrict,
    #[strum(serialize = "80")]
    MoravianSilesianRegion,
    #[strum(serialize = "425")]
    MostDistrict,
    #[strum(serialize = "206")]
    MelnikDistrict,
    #[strum(serialize = "804")]
    NovyJicinDistrict,
    #[strum(serialize = "208")]
    NymburkDistrict,
    #[strum(serialize = "523")]
    NachodDistrict,
    #[strum(serialize = "712")]
    OlomoucDistrict,
    #[strum(serialize = "71")]
    OlomoucRegion,
    #[strum(serialize = "805")]
    OpavaDistrict,
    #[strum(serialize = "806")]
    OstravaCityDistrict,
    #[strum(serialize = "532")]
    PardubiceDistrict,
    #[strum(serialize = "53")]
    PardubiceRegion,
    #[strum(serialize = "633")]
    PelhrimovDistrict,
    #[strum(serialize = "32")]
    PlzenRegion,
    #[strum(serialize = "323")]
    PlzenCityDistrict,
    #[strum(serialize = "325")]
    PlzenNorthDistrict,
    #[strum(serialize = "324")]
    PlzenSouthDistrict,
    #[strum(serialize = "315")]
    PrachaticeDistrict,
    #[strum(serialize = "10")]
    Prague,
    #[strum(serialize = "101")]
    Prague1,
    #[strum(serialize = "110")]
    Prague10,
    #[strum(serialize = "111")]
    Prague11,
    #[strum(serialize = "112")]
    Prague12,
    #[strum(serialize = "113")]
    Prague13,
    #[strum(serialize = "114")]
    Prague14,
    #[strum(serialize = "115")]
    Prague15,
    #[strum(serialize = "116")]
    Prague16,
    #[strum(serialize = "102")]
    Prague2,
    #[strum(serialize = "121")]
    Prague21,
    #[strum(serialize = "103")]
    Prague3,
    #[strum(serialize = "104")]
    Prague4,
    #[strum(serialize = "105")]
    Prague5,
    #[strum(serialize = "106")]
    Prague6,
    #[strum(serialize = "107")]
    Prague7,
    #[strum(serialize = "108")]
    Prague8,
    #[strum(serialize = "109")]
    Prague9,
    #[strum(serialize = "209")]
    PragueEastDistrict,
    #[strum(serialize = "20A")]
    PragueWestDistrict,
    #[strum(serialize = "713")]
    ProstejovDistrict,
    #[strum(serialize = "314")]
    PisekDistrict,
    #[strum(serialize = "714")]
    PrerovDistrict,
    #[strum(serialize = "20B")]
    PribramDistrict,
    #[strum(serialize = "20C")]
    RakovnikDistrict,
    #[strum(serialize = "326")]
    RokycanyDistrict,
    #[strum(serialize = "524")]
    RychnovNadKneznouDistrict,
    #[strum(serialize = "514")]
    SemilyDistrict,
    #[strum(serialize = "413")]
    SokolovDistrict,
    #[strum(serialize = "31")]
    SouthBohemianRegion,
    #[strum(serialize = "64")]
    SouthMoravianRegion,
    #[strum(serialize = "316")]
    StrakoniceDistrict,
    #[strum(serialize = "533")]
    SvitavyDistrict,
    #[strum(serialize = "327")]
    TachovDistrict,
    #[strum(serialize = "426")]
    TepliceDistrict,
    #[strum(serialize = "525")]
    TrutnovDistrict,
    #[strum(serialize = "317")]
    TaborDistrict,
    #[strum(serialize = "634")]
    TrebicDistrict,
    #[strum(serialize = "722")]
    UherskeHradisteDistrict,
    #[strum(serialize = "723")]
    VsetinDistrict,
    #[strum(serialize = "63")]
    VysocinaRegion,
    #[strum(serialize = "646")]
    VyskovDistrict,
    #[strum(serialize = "724")]
    ZlinDistrict,
    #[strum(serialize = "72")]
    ZlinRegion,
    #[strum(serialize = "647")]
    ZnojmoDistrict,
    #[strum(serialize = "427")]
    UstiNadLabemDistrict,
    #[strum(serialize = "42")]
    UstiNadLabemRegion,
    #[strum(serialize = "534")]
    UstiNadOrliciDistrict,
    #[strum(serialize = "511")]
    CeskaLipaDistrict,
    #[strum(serialize = "311")]
    CeskeBudejoviceDistrict,
    #[strum(serialize = "312")]
    CeskyKrumlovDistrict,
    #[strum(serialize = "715")]
    SumperkDistrict,
    #[strum(serialize = "635")]
    ZdarNadSazavouDistrict,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum DenmarkStatesAbbreviation {
    #[strum(serialize = "84")]
    CapitalRegionOfDenmark,
    #[strum(serialize = "82")]
    CentralDenmarkRegion,
    #[strum(serialize = "81")]
    NorthDenmarkRegion,
    #[strum(serialize = "85")]
    RegionZealand,
    #[strum(serialize = "83")]
    RegionOfSouthernDenmark,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum FinlandStatesAbbreviation {
    #[strum(serialize = "08")]
    CentralFinland,
    #[strum(serialize = "07")]
    CentralOstrobothnia,
    #[strum(serialize = "IS")]
    EasternFinlandProvince,
    #[strum(serialize = "19")]
    FinlandProper,
    #[strum(serialize = "05")]
    Kainuu,
    #[strum(serialize = "09")]
    Kymenlaakso,
    #[strum(serialize = "LL")]
    Lapland,
    #[strum(serialize = "13")]
    NorthKarelia,
    #[strum(serialize = "14")]
    NorthernOstrobothnia,
    #[strum(serialize = "15")]
    NorthernSavonia,
    #[strum(serialize = "12")]
    Ostrobothnia,
    #[strum(serialize = "OL")]
    OuluProvince,
    #[strum(serialize = "11")]
    Pirkanmaa,
    #[strum(serialize = "16")]
    PaijanneTavastia,
    #[strum(serialize = "17")]
    Satakunta,
    #[strum(serialize = "02")]
    SouthKarelia,
    #[strum(serialize = "03")]
    SouthernOstrobothnia,
    #[strum(serialize = "04")]
    SouthernSavonia,
    #[strum(serialize = "06")]
    TavastiaProper,
    #[strum(serialize = "18")]
    Uusimaa,
    #[strum(serialize = "01")]
    AlandIslands,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum FranceStatesAbbreviation {
    #[strum(serialize = "01")]
    Ain,
    #[strum(serialize = "02")]
    Aisne,
    #[strum(serialize = "03")]
    Allier,
    #[strum(serialize = "04")]
    AlpesDeHauteProvence,
    #[strum(serialize = "06")]
    AlpesMaritimes,
    #[strum(serialize = "6AE")]
    Alsace,
    #[strum(serialize = "07")]
    Ardeche,
    #[strum(serialize = "08")]
    Ardennes,
    #[strum(serialize = "09")]
    Ariege,
    #[strum(serialize = "10")]
    Aube,
    #[strum(serialize = "11")]
    Aude,
    #[strum(serialize = "ARA")]
    AuvergneRhoneAlpes,
    #[strum(serialize = "12")]
    Aveyron,
    #[strum(serialize = "67")]
    BasRhin,
    #[strum(serialize = "13")]
    BouchesDuRhone,
    #[strum(serialize = "BFC")]
    BourgogneFrancheComte,
    #[strum(serialize = "BRE")]
    Bretagne,
    #[strum(serialize = "14")]
    Calvados,
    #[strum(serialize = "15")]
    Cantal,
    #[strum(serialize = "CVL")]
    CentreValDeLoire,
    #[strum(serialize = "16")]
    Charente,
    #[strum(serialize = "17")]
    CharenteMaritime,
    #[strum(serialize = "18")]
    Cher,
    #[strum(serialize = "CP")]
    Clipperton,
    #[strum(serialize = "19")]
    Correze,
    #[strum(serialize = "20R")]
    Corse,
    #[strum(serialize = "2A")]
    CorseDuSud,
    #[strum(serialize = "21")]
    CoteDor,
    #[strum(serialize = "22")]
    CotesDarmor,
    #[strum(serialize = "23")]
    Creuse,
    #[strum(serialize = "79")]
    DeuxSevres,
    #[strum(serialize = "24")]
    Dordogne,
    #[strum(serialize = "25")]
    Doubs,
    #[strum(serialize = "26")]
    Drome,
    #[strum(serialize = "91")]
    Essonne,
    #[strum(serialize = "27")]
    Eure,
    #[strum(serialize = "28")]
    EureEtLoir,
    #[strum(serialize = "29")]
    Finistere,
    #[strum(serialize = "973")]
    FrenchGuiana,
    #[strum(serialize = "PF")]
    FrenchPolynesia,
    #[strum(serialize = "TF")]
    FrenchSouthernAndAntarcticLands,
    #[strum(serialize = "30")]
    Gard,
    #[strum(serialize = "32")]
    Gers,
    #[strum(serialize = "33")]
    Gironde,
    #[strum(serialize = "GES")]
    GrandEst,
    #[strum(serialize = "971")]
    Guadeloupe,
    #[strum(serialize = "68")]
    HautRhin,
    #[strum(serialize = "2B")]
    HauteCorse,
    #[strum(serialize = "31")]
    HauteGaronne,
    #[strum(serialize = "43")]
    HauteLoire,
    #[strum(serialize = "52")]
    HauteMarne,
    #[strum(serialize = "70")]
    HauteSaone,
    #[strum(serialize = "74")]
    HauteSavoie,
    #[strum(serialize = "87")]
    HauteVienne,
    #[strum(serialize = "05")]
    HautesAlpes,
    #[strum(serialize = "65")]
    HautesPyrenees,
    #[strum(serialize = "HDF")]
    HautsDeFrance,
    #[strum(serialize = "92")]
    HautsDeSeine,
    #[strum(serialize = "34")]
    Herault,
    #[strum(serialize = "IDF")]
    IleDeFrance,
    #[strum(serialize = "35")]
    IlleEtVilaine,
    #[strum(serialize = "36")]
    Indre,
    #[strum(serialize = "37")]
    IndreEtLoire,
    #[strum(serialize = "38")]
    Isere,
    #[strum(serialize = "39")]
    Jura,
    #[strum(serialize = "974")]
    LaReunion,
    #[strum(serialize = "40")]
    Landes,
    #[strum(serialize = "41")]
    LoirEtCher,
    #[strum(serialize = "42")]
    Loire,
    #[strum(serialize = "44")]
    LoireAtlantique,
    #[strum(serialize = "45")]
    Loiret,
    #[strum(serialize = "46")]
    Lot,
    #[strum(serialize = "47")]
    LotEtGaronne,
    #[strum(serialize = "48")]
    Lozere,
    #[strum(serialize = "49")]
    MaineEtLoire,
    #[strum(serialize = "50")]
    Manche,
    #[strum(serialize = "51")]
    Marne,
    #[strum(serialize = "972")]
    Martinique,
    #[strum(serialize = "53")]
    Mayenne,
    #[strum(serialize = "976")]
    Mayotte,
    #[strum(serialize = "69M")]
    MetropoleDeLyon,
    #[strum(serialize = "54")]
    MeurtheEtMoselle,
    #[strum(serialize = "55")]
    Meuse,
    #[strum(serialize = "56")]
    Morbihan,
    #[strum(serialize = "57")]
    Moselle,
    #[strum(serialize = "58")]
    Nievre,
    #[strum(serialize = "59")]
    Nord,
    #[strum(serialize = "NOR")]
    Normandie,
    #[strum(serialize = "NAQ")]
    NouvelleAquitaine,
    #[strum(serialize = "OCC")]
    Occitanie,
    #[strum(serialize = "60")]
    Oise,
    #[strum(serialize = "61")]
    Orne,
    #[strum(serialize = "75C")]
    Paris,
    #[strum(serialize = "62")]
    PasDeCalais,
    #[strum(serialize = "PDL")]
    PaysDeLaLoire,
    #[strum(serialize = "PAC")]
    ProvenceAlpesCoteDazur,
    #[strum(serialize = "63")]
    PuyDeDome,
    #[strum(serialize = "64")]
    PyreneesAtlantiques,
    #[strum(serialize = "66")]
    PyreneesOrientales,
    #[strum(serialize = "69")]
    Rhone,
    #[strum(serialize = "PM")]
    SaintPierreAndMiquelon,
    #[strum(serialize = "BL")]
    SaintBarthelemy,
    #[strum(serialize = "MF")]
    SaintMartin,
    #[strum(serialize = "71")]
    SaoneEtLoire,
    #[strum(serialize = "72")]
    Sarthe,
    #[strum(serialize = "73")]
    Savoie,
    #[strum(serialize = "77")]
    SeineEtMarne,
    #[strum(serialize = "76")]
    SeineMaritime,
    #[strum(serialize = "93")]
    SeineSaintDenis,
    #[strum(serialize = "80")]
    Somme,
    #[strum(serialize = "81")]
    Tarn,
    #[strum(serialize = "82")]
    TarnEtGaronne,
    #[strum(serialize = "90")]
    TerritoireDeBelfort,
    #[strum(serialize = "95")]
    ValDoise,
    #[strum(serialize = "94")]
    ValDeMarne,
    #[strum(serialize = "83")]
    Var,
    #[strum(serialize = "84")]
    Vaucluse,
    #[strum(serialize = "85")]
    Vendee,
    #[strum(serialize = "86")]
    Vienne,
    #[strum(serialize = "88")]
    Vosges,
    #[strum(serialize = "WF")]
    WallisAndFutuna,
    #[strum(serialize = "89")]
    Yonne,
    #[strum(serialize = "78")]
    Yvelines,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum GermanyStatesAbbreviation {
    BW,
    BY,
    BE,
    BB,
    HB,
    HH,
    HE,
    NI,
    MV,
    NW,
    RP,
    SL,
    SN,
    ST,
    SH,
    TH,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum GreeceStatesAbbreviation {
    #[strum(serialize = "13")]
    AchaeaRegionalUnit,
    #[strum(serialize = "01")]
    AetoliaAcarnaniaRegionalUnit,
    #[strum(serialize = "12")]
    ArcadiaPrefecture,
    #[strum(serialize = "11")]
    ArgolisRegionalUnit,
    #[strum(serialize = "I")]
    AtticaRegion,
    #[strum(serialize = "03")]
    BoeotiaRegionalUnit,
    #[strum(serialize = "H")]
    CentralGreeceRegion,
    #[strum(serialize = "B")]
    CentralMacedonia,
    #[strum(serialize = "94")]
    ChaniaRegionalUnit,
    #[strum(serialize = "22")]
    CorfuPrefecture,
    #[strum(serialize = "15")]
    CorinthiaRegionalUnit,
    #[strum(serialize = "M")]
    CreteRegion,
    #[strum(serialize = "52")]
    DramaRegionalUnit,
    #[strum(serialize = "A2")]
    EastAtticaRegionalUnit,
    #[strum(serialize = "A")]
    EastMacedoniaAndThrace,
    #[strum(serialize = "D")]
    EpirusRegion,
    #[strum(serialize = "04")]
    Euboea,
    #[strum(serialize = "51")]
    GrevenaPrefecture,
    #[strum(serialize = "53")]
    ImathiaRegionalUnit,
    #[strum(serialize = "33")]
    IoanninaRegionalUnit,
    #[strum(serialize = "F")]
    IonianIslandsRegion,
    #[strum(serialize = "41")]
    KarditsaRegionalUnit,
    #[strum(serialize = "56")]
    KastoriaRegionalUnit,
    #[strum(serialize = "23")]
    KefaloniaPrefecture,
    #[strum(serialize = "57")]
    KilkisRegionalUnit,
    #[strum(serialize = "58")]
    KozaniPrefecture,
    #[strum(serialize = "16")]
    Laconia,
    #[strum(serialize = "42")]
    LarissaPrefecture,
    #[strum(serialize = "24")]
    LefkadaRegionalUnit,
    #[strum(serialize = "59")]
    PellaRegionalUnit,
    #[strum(serialize = "J")]
    PeloponneseRegion,
    #[strum(serialize = "06")]
    PhthiotisPrefecture,
    #[strum(serialize = "34")]
    PrevezaPrefecture,
    #[strum(serialize = "62")]
    SerresPrefecture,
    #[strum(serialize = "L")]
    SouthAegean,
    #[strum(serialize = "54")]
    ThessalonikiRegionalUnit,
    #[strum(serialize = "G")]
    WestGreeceRegion,
    #[strum(serialize = "C")]
    WestMacedoniaRegion,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum HungaryStatesAbbreviation {
    #[strum(serialize = "BA")]
    BaranyaCounty,
    #[strum(serialize = "BZ")]
    BorsodAbaujZemplenCounty,
    #[strum(serialize = "BU")]
    Budapest,
    #[strum(serialize = "BK")]
    BacsKiskunCounty,
    #[strum(serialize = "BE")]
    BekesCounty,
    #[strum(serialize = "BC")]
    Bekescsaba,
    #[strum(serialize = "CS")]
    CsongradCounty,
    #[strum(serialize = "DE")]
    Debrecen,
    #[strum(serialize = "DU")]
    Dunaujvaros,
    #[strum(serialize = "EG")]
    Eger,
    #[strum(serialize = "FE")]
    FejerCounty,
    #[strum(serialize = "GY")]
    Gyor,
    #[strum(serialize = "GS")]
    GyorMosonSopronCounty,
    #[strum(serialize = "HB")]
    HajduBiharCounty,
    #[strum(serialize = "HE")]
    HevesCounty,
    #[strum(serialize = "HV")]
    Hodmezovasarhely,
    #[strum(serialize = "JN")]
    JaszNagykunSzolnokCounty,
    #[strum(serialize = "KV")]
    Kaposvar,
    #[strum(serialize = "KM")]
    Kecskemet,
    #[strum(serialize = "MI")]
    Miskolc,
    #[strum(serialize = "NK")]
    Nagykanizsa,
    #[strum(serialize = "NY")]
    Nyiregyhaza,
    #[strum(serialize = "NO")]
    NogradCounty,
    #[strum(serialize = "PE")]
    PestCounty,
    #[strum(serialize = "PS")]
    Pecs,
    #[strum(serialize = "ST")]
    Salgotarjan,
    #[strum(serialize = "SO")]
    SomogyCounty,
    #[strum(serialize = "SN")]
    Sopron,
    #[strum(serialize = "SZ")]
    SzabolcsSzatmarBeregCounty,
    #[strum(serialize = "SD")]
    Szeged,
    #[strum(serialize = "SS")]
    Szekszard,
    #[strum(serialize = "SK")]
    Szolnok,
    #[strum(serialize = "SH")]
    Szombathely,
    #[strum(serialize = "SF")]
    Szekesfehervar,
    #[strum(serialize = "TB")]
    Tatabanya,
    #[strum(serialize = "TO")]
    TolnaCounty,
    #[strum(serialize = "VA")]
    VasCounty,
    #[strum(serialize = "VM")]
    Veszprem,
    #[strum(serialize = "VE")]
    VeszpremCounty,
    #[strum(serialize = "ZA")]
    ZalaCounty,
    #[strum(serialize = "ZE")]
    Zalaegerszeg,
    #[strum(serialize = "ER")]
    Erd,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum IcelandStatesAbbreviation {
    #[strum(serialize = "1")]
    CapitalRegion,
    #[strum(serialize = "7")]
    EasternRegion,
    #[strum(serialize = "6")]
    NortheasternRegion,
    #[strum(serialize = "5")]
    NorthwesternRegion,
    #[strum(serialize = "2")]
    SouthernPeninsulaRegion,
    #[strum(serialize = "8")]
    SouthernRegion,
    #[strum(serialize = "3")]
    WesternRegion,
    #[strum(serialize = "4")]
    Westfjords,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum IrelandStatesAbbreviation {
    #[strum(serialize = "C")]
    Connacht,
    #[strum(serialize = "CW")]
    CountyCarlow,
    #[strum(serialize = "CN")]
    CountyCavan,
    #[strum(serialize = "CE")]
    CountyClare,
    #[strum(serialize = "CO")]
    CountyCork,
    #[strum(serialize = "DL")]
    CountyDonegal,
    #[strum(serialize = "D")]
    CountyDublin,
    #[strum(serialize = "G")]
    CountyGalway,
    #[strum(serialize = "KY")]
    CountyKerry,
    #[strum(serialize = "KE")]
    CountyKildare,
    #[strum(serialize = "KK")]
    CountyKilkenny,
    #[strum(serialize = "LS")]
    CountyLaois,
    #[strum(serialize = "LK")]
    CountyLimerick,
    #[strum(serialize = "LD")]
    CountyLongford,
    #[strum(serialize = "LH")]
    CountyLouth,
    #[strum(serialize = "MO")]
    CountyMayo,
    #[strum(serialize = "MH")]
    CountyMeath,
    #[strum(serialize = "MN")]
    CountyMonaghan,
    #[strum(serialize = "OY")]
    CountyOffaly,
    #[strum(serialize = "RN")]
    CountyRoscommon,
    #[strum(serialize = "SO")]
    CountySligo,
    #[strum(serialize = "TA")]
    CountyTipperary,
    #[strum(serialize = "WD")]
    CountyWaterford,
    #[strum(serialize = "WH")]
    CountyWestmeath,
    #[strum(serialize = "WX")]
    CountyWexford,
    #[strum(serialize = "WW")]
    CountyWicklow,
    #[strum(serialize = "L")]
    Leinster,
    #[strum(serialize = "M")]
    Munster,
    #[strum(serialize = "U")]
    Ulster,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum LatviaStatesAbbreviation {
    #[strum(serialize = "001")]
    AglonaMunicipality,
    #[strum(serialize = "002")]
    AizkraukleMunicipality,
    #[strum(serialize = "003")]
    AizputeMunicipality,
    #[strum(serialize = "004")]
    AknīsteMunicipality,
    #[strum(serialize = "005")]
    AlojaMunicipality,
    #[strum(serialize = "006")]
    AlsungaMunicipality,
    #[strum(serialize = "007")]
    AlūksneMunicipality,
    #[strum(serialize = "008")]
    AmataMunicipality,
    #[strum(serialize = "009")]
    ApeMunicipality,
    #[strum(serialize = "010")]
    AuceMunicipality,
    #[strum(serialize = "012")]
    BabīteMunicipality,
    #[strum(serialize = "013")]
    BaldoneMunicipality,
    #[strum(serialize = "014")]
    BaltinavaMunicipality,
    #[strum(serialize = "015")]
    BalviMunicipality,
    #[strum(serialize = "016")]
    BauskaMunicipality,
    #[strum(serialize = "017")]
    BeverīnaMunicipality,
    #[strum(serialize = "018")]
    BrocēniMunicipality,
    #[strum(serialize = "019")]
    BurtniekiMunicipality,
    #[strum(serialize = "020")]
    CarnikavaMunicipality,
    #[strum(serialize = "021")]
    CesvaineMunicipality,
    #[strum(serialize = "023")]
    CiblaMunicipality,
    #[strum(serialize = "022")]
    CēsisMunicipality,
    #[strum(serialize = "024")]
    DagdaMunicipality,
    #[strum(serialize = "DGV")]
    Daugavpils,
    #[strum(serialize = "025")]
    DaugavpilsMunicipality,
    #[strum(serialize = "026")]
    DobeleMunicipality,
    #[strum(serialize = "027")]
    DundagaMunicipality,
    #[strum(serialize = "028")]
    DurbeMunicipality,
    #[strum(serialize = "029")]
    EngureMunicipality,
    #[strum(serialize = "031")]
    GarkalneMunicipality,
    #[strum(serialize = "032")]
    GrobiņaMunicipality,
    #[strum(serialize = "033")]
    GulbeneMunicipality,
    #[strum(serialize = "034")]
    IecavaMunicipality,
    #[strum(serialize = "035")]
    IkšķileMunicipality,
    #[strum(serialize = "036")]
    IlūksteMunicipality,
    #[strum(serialize = "037")]
    InčukalnsMunicipality,
    #[strum(serialize = "038")]
    JaunjelgavaMunicipality,
    #[strum(serialize = "039")]
    JaunpiebalgaMunicipality,
    #[strum(serialize = "040")]
    JaunpilsMunicipality,
    #[strum(serialize = "JEL")]
    Jelgava,
    #[strum(serialize = "041")]
    JelgavaMunicipality,
    #[strum(serialize = "JKB")]
    Jēkabpils,
    #[strum(serialize = "042")]
    JēkabpilsMunicipality,
    #[strum(serialize = "JUR")]
    Jūrmala,
    #[strum(serialize = "043")]
    KandavaMunicipality,
    #[strum(serialize = "045")]
    KocēniMunicipality,
    #[strum(serialize = "046")]
    KokneseMunicipality,
    #[strum(serialize = "048")]
    KrimuldaMunicipality,
    #[strum(serialize = "049")]
    KrustpilsMunicipality,
    #[strum(serialize = "047")]
    KrāslavaMunicipality,
    #[strum(serialize = "050")]
    KuldīgaMunicipality,
    #[strum(serialize = "044")]
    KārsavaMunicipality,
    #[strum(serialize = "053")]
    LielvārdeMunicipality,
    #[strum(serialize = "LPX")]
    Liepāja,
    #[strum(serialize = "054")]
    LimbažiMunicipality,
    #[strum(serialize = "057")]
    LubānaMunicipality,
    #[strum(serialize = "058")]
    LudzaMunicipality,
    #[strum(serialize = "055")]
    LīgatneMunicipality,
    #[strum(serialize = "056")]
    LīvāniMunicipality,
    #[strum(serialize = "059")]
    MadonaMunicipality,
    #[strum(serialize = "060")]
    MazsalacaMunicipality,
    #[strum(serialize = "061")]
    MālpilsMunicipality,
    #[strum(serialize = "062")]
    MārupeMunicipality,
    #[strum(serialize = "063")]
    MērsragsMunicipality,
    #[strum(serialize = "064")]
    NaukšēniMunicipality,
    #[strum(serialize = "065")]
    NeretaMunicipality,
    #[strum(serialize = "066")]
    NīcaMunicipality,
    #[strum(serialize = "067")]
    OgreMunicipality,
    #[strum(serialize = "068")]
    OlaineMunicipality,
    #[strum(serialize = "069")]
    OzolniekiMunicipality,
    #[strum(serialize = "073")]
    PreiļiMunicipality,
    #[strum(serialize = "074")]
    PriekuleMunicipality,
    #[strum(serialize = "075")]
    PriekuļiMunicipality,
    #[strum(serialize = "070")]
    PārgaujaMunicipality,
    #[strum(serialize = "071")]
    PāvilostaMunicipality,
    #[strum(serialize = "072")]
    PļaviņasMunicipality,
    #[strum(serialize = "076")]
    RaunaMunicipality,
    #[strum(serialize = "078")]
    RiebiņiMunicipality,
    #[strum(serialize = "RIX")]
    Riga,
    #[strum(serialize = "079")]
    RojaMunicipality,
    #[strum(serialize = "080")]
    RopažiMunicipality,
    #[strum(serialize = "081")]
    RucavaMunicipality,
    #[strum(serialize = "082")]
    RugājiMunicipality,
    #[strum(serialize = "083")]
    RundāleMunicipality,
    #[strum(serialize = "REZ")]
    Rēzekne,
    #[strum(serialize = "077")]
    RēzekneMunicipality,
    #[strum(serialize = "084")]
    RūjienaMunicipality,
    #[strum(serialize = "085")]
    SalaMunicipality,
    #[strum(serialize = "086")]
    SalacgrīvaMunicipality,
    #[strum(serialize = "087")]
    SalaspilsMunicipality,
    #[strum(serialize = "088")]
    SaldusMunicipality,
    #[strum(serialize = "089")]
    SaulkrastiMunicipality,
    #[strum(serialize = "091")]
    SiguldaMunicipality,
    #[strum(serialize = "093")]
    SkrundaMunicipality,
    #[strum(serialize = "092")]
    SkrīveriMunicipality,
    #[strum(serialize = "094")]
    SmilteneMunicipality,
    #[strum(serialize = "095")]
    StopiņiMunicipality,
    #[strum(serialize = "096")]
    StrenčiMunicipality,
    #[strum(serialize = "090")]
    SējaMunicipality,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum ItalyStatesAbbreviation {
    #[strum(serialize = "65")]
    Abruzzo,
    #[strum(serialize = "23")]
    AostaValley,
    #[strum(serialize = "75")]
    Apulia,
    #[strum(serialize = "77")]
    Basilicata,
    #[strum(serialize = "BN")]
    BeneventoProvince,
    #[strum(serialize = "78")]
    Calabria,
    #[strum(serialize = "72")]
    Campania,
    #[strum(serialize = "45")]
    EmiliaRomagna,
    #[strum(serialize = "36")]
    FriuliVeneziaGiulia,
    #[strum(serialize = "62")]
    Lazio,
    #[strum(serialize = "42")]
    Liguria,
    #[strum(serialize = "25")]
    Lombardy,
    #[strum(serialize = "57")]
    Marche,
    #[strum(serialize = "67")]
    Molise,
    #[strum(serialize = "21")]
    Piedmont,
    #[strum(serialize = "88")]
    Sardinia,
    #[strum(serialize = "82")]
    Sicily,
    #[strum(serialize = "32")]
    TrentinoSouthTyrol,
    #[strum(serialize = "52")]
    Tuscany,
    #[strum(serialize = "55")]
    Umbria,
    #[strum(serialize = "34")]
    Veneto,
    #[strum(serialize = "AG")]
    Agrigento,
    #[strum(serialize = "CL")]
    Caltanissetta,
    #[strum(serialize = "EN")]
    Enna,
    #[strum(serialize = "RG")]
    Ragusa,
    #[strum(serialize = "SR")]
    Siracusa,
    #[strum(serialize = "TP")]
    Trapani,
    #[strum(serialize = "BA")]
    Bari,
    #[strum(serialize = "BO")]
    Bologna,
    #[strum(serialize = "CA")]
    Cagliari,
    #[strum(serialize = "CT")]
    Catania,
    #[strum(serialize = "FI")]
    Florence,
    #[strum(serialize = "GE")]
    Genoa,
    #[strum(serialize = "ME")]
    Messina,
    #[strum(serialize = "MI")]
    Milan,
    #[strum(serialize = "NA")]
    Naples,
    #[strum(serialize = "PA")]
    Palermo,
    #[strum(serialize = "RC")]
    ReggioCalabria,
    #[strum(serialize = "RM")]
    Rome,
    #[strum(serialize = "TO")]
    Turin,
    #[strum(serialize = "VE")]
    Venice,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum LiechtensteinStatesAbbreviation {
    #[strum(serialize = "01")]
    Balzers,
    #[strum(serialize = "02")]
    Eschen,
    #[strum(serialize = "03")]
    Gamprin,
    #[strum(serialize = "04")]
    Mauren,
    #[strum(serialize = "05")]
    Planken,
    #[strum(serialize = "06")]
    Ruggell,
    #[strum(serialize = "07")]
    Schaan,
    #[strum(serialize = "08")]
    Schellenberg,
    #[strum(serialize = "09")]
    Triesen,
    #[strum(serialize = "10")]
    Triesenberg,
    #[strum(serialize = "11")]
    Vaduz,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum LithuaniaStatesAbbreviation {
    #[strum(serialize = "01")]
    AkmeneDistrictMunicipality,
    #[strum(serialize = "02")]
    AlytusCityMunicipality,
    #[strum(serialize = "AL")]
    AlytusCounty,
    #[strum(serialize = "03")]
    AlytusDistrictMunicipality,
    #[strum(serialize = "05")]
    BirstonasMunicipality,
    #[strum(serialize = "06")]
    BirzaiDistrictMunicipality,
    #[strum(serialize = "07")]
    DruskininkaiMunicipality,
    #[strum(serialize = "08")]
    ElektrenaiMunicipality,
    #[strum(serialize = "09")]
    IgnalinaDistrictMunicipality,
    #[strum(serialize = "10")]
    JonavaDistrictMunicipality,
    #[strum(serialize = "11")]
    JoniskisDistrictMunicipality,
    #[strum(serialize = "12")]
    JurbarkasDistrictMunicipality,
    #[strum(serialize = "13")]
    KaisiadorysDistrictMunicipality,
    #[strum(serialize = "14")]
    KalvarijaMunicipality,
    #[strum(serialize = "15")]
    KaunasCityMunicipality,
    #[strum(serialize = "KU")]
    KaunasCounty,
    #[strum(serialize = "16")]
    KaunasDistrictMunicipality,
    #[strum(serialize = "17")]
    KazluRudaMunicipality,
    #[strum(serialize = "19")]
    KelmeDistrictMunicipality,
    #[strum(serialize = "20")]
    KlaipedaCityMunicipality,
    #[strum(serialize = "KL")]
    KlaipedaCounty,
    #[strum(serialize = "21")]
    KlaipedaDistrictMunicipality,
    #[strum(serialize = "22")]
    KretingaDistrictMunicipality,
    #[strum(serialize = "23")]
    KupiskisDistrictMunicipality,
    #[strum(serialize = "18")]
    KedainiaiDistrictMunicipality,
    #[strum(serialize = "24")]
    LazdijaiDistrictMunicipality,
    #[strum(serialize = "MR")]
    MarijampoleCounty,
    #[strum(serialize = "25")]
    MarijampoleMunicipality,
    #[strum(serialize = "26")]
    MazeikiaiDistrictMunicipality,
    #[strum(serialize = "27")]
    MoletaiDistrictMunicipality,
    #[strum(serialize = "28")]
    NeringaMunicipality,
    #[strum(serialize = "29")]
    PagegiaiMunicipality,
    #[strum(serialize = "30")]
    PakruojisDistrictMunicipality,
    #[strum(serialize = "31")]
    PalangaCityMunicipality,
    #[strum(serialize = "32")]
    PanevezysCityMunicipality,
    #[strum(serialize = "PN")]
    PanevezysCounty,
    #[strum(serialize = "33")]
    PanevezysDistrictMunicipality,
    #[strum(serialize = "34")]
    PasvalysDistrictMunicipality,
    #[strum(serialize = "35")]
    PlungeDistrictMunicipality,
    #[strum(serialize = "36")]
    PrienaiDistrictMunicipality,
    #[strum(serialize = "37")]
    RadviliskisDistrictMunicipality,
    #[strum(serialize = "38")]
    RaseiniaiDistrictMunicipality,
    #[strum(serialize = "39")]
    RietavasMunicipality,
    #[strum(serialize = "40")]
    RokiskisDistrictMunicipality,
    #[strum(serialize = "48")]
    SkuodasDistrictMunicipality,
    #[strum(serialize = "TA")]
    TaurageCounty,
    #[strum(serialize = "50")]
    TaurageDistrictMunicipality,
    #[strum(serialize = "TE")]
    TelsiaiCounty,
    #[strum(serialize = "51")]
    TelsiaiDistrictMunicipality,
    #[strum(serialize = "52")]
    TrakaiDistrictMunicipality,
    #[strum(serialize = "53")]
    UkmergeDistrictMunicipality,
    #[strum(serialize = "UT")]
    UtenaCounty,
    #[strum(serialize = "54")]
    UtenaDistrictMunicipality,
    #[strum(serialize = "55")]
    VarenaDistrictMunicipality,
    #[strum(serialize = "56")]
    VilkaviskisDistrictMunicipality,
    #[strum(serialize = "57")]
    VilniusCityMunicipality,
    #[strum(serialize = "VL")]
    VilniusCounty,
    #[strum(serialize = "58")]
    VilniusDistrictMunicipality,
    #[strum(serialize = "59")]
    VisaginasMunicipality,
    #[strum(serialize = "60")]
    ZarasaiDistrictMunicipality,
    #[strum(serialize = "41")]
    SakiaiDistrictMunicipality,
    #[strum(serialize = "42")]
    SalcininkaiDistrictMunicipality,
    #[strum(serialize = "43")]
    SiauliaiCityMunicipality,
    #[strum(serialize = "SA")]
    SiauliaiCounty,
    #[strum(serialize = "44")]
    SiauliaiDistrictMunicipality,
    #[strum(serialize = "45")]
    SilaleDistrictMunicipality,
    #[strum(serialize = "46")]
    SiluteDistrictMunicipality,
    #[strum(serialize = "47")]
    SirvintosDistrictMunicipality,
    #[strum(serialize = "49")]
    SvencionysDistrictMunicipality,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum MaltaStatesAbbreviation {
    #[strum(serialize = "01")]
    Attard,
    #[strum(serialize = "02")]
    Balzan,
    #[strum(serialize = "03")]
    Birgu,
    #[strum(serialize = "04")]
    Birkirkara,
    #[strum(serialize = "05")]
    Birżebbuġa,
    #[strum(serialize = "06")]
    Cospicua,
    #[strum(serialize = "07")]
    Dingli,
    #[strum(serialize = "08")]
    Fgura,
    #[strum(serialize = "09")]
    Floriana,
    #[strum(serialize = "10")]
    Fontana,
    #[strum(serialize = "11")]
    Gudja,
    #[strum(serialize = "12")]
    Gżira,
    #[strum(serialize = "13")]
    Għajnsielem,
    #[strum(serialize = "14")]
    Għarb,
    #[strum(serialize = "15")]
    Għargħur,
    #[strum(serialize = "16")]
    Għasri,
    #[strum(serialize = "17")]
    Għaxaq,
    #[strum(serialize = "18")]
    Ħamrun,
    #[strum(serialize = "19")]
    Iklin,
    #[strum(serialize = "20")]
    Senglea,
    #[strum(serialize = "21")]
    Kalkara,
    #[strum(serialize = "22")]
    Kerċem,
    #[strum(serialize = "23")]
    Kirkop,
    #[strum(serialize = "24")]
    Lija,
    #[strum(serialize = "25")]
    Luqa,
    #[strum(serialize = "26")]
    Marsa,
    #[strum(serialize = "27")]
    Marsaskala,
    #[strum(serialize = "28")]
    Marsaxlokk,
    #[strum(serialize = "29")]
    Mdina,
    #[strum(serialize = "30")]
    Mellieħa,
    #[strum(serialize = "31")]
    Mġarr,
    #[strum(serialize = "32")]
    Mosta,
    #[strum(serialize = "33")]
    Mqabba,
    #[strum(serialize = "34")]
    Msida,
    #[strum(serialize = "35")]
    Mtarfa,
    #[strum(serialize = "36")]
    Munxar,
    #[strum(serialize = "37")]
    Nadur,
    #[strum(serialize = "38")]
    Naxxar,
    #[strum(serialize = "39")]
    Paola,
    #[strum(serialize = "40")]
    Pembroke,
    #[strum(serialize = "41")]
    Pietà,
    #[strum(serialize = "42")]
    Qala,
    #[strum(serialize = "43")]
    Qormi,
    #[strum(serialize = "44")]
    Qrendi,
    #[strum(serialize = "45")]
    Victoria,
    #[strum(serialize = "46")]
    Rabat,
    #[strum(serialize = "48")]
    StJulians,
    #[strum(serialize = "49")]
    SanĠwann,
    #[strum(serialize = "50")]
    SaintLawrence,
    #[strum(serialize = "51")]
    StPaulsBay,
    #[strum(serialize = "52")]
    Sannat,
    #[strum(serialize = "53")]
    SantaLuċija,
    #[strum(serialize = "54")]
    SantaVenera,
    #[strum(serialize = "55")]
    Siġġiewi,
    #[strum(serialize = "56")]
    Sliema,
    #[strum(serialize = "57")]
    Swieqi,
    #[strum(serialize = "58")]
    TaXbiex,
    #[strum(serialize = "59")]
    Tarxien,
    #[strum(serialize = "60")]
    Valletta,
    #[strum(serialize = "61")]
    Xagħra,
    #[strum(serialize = "62")]
    Xewkija,
    #[strum(serialize = "63")]
    Xgħajra,
    #[strum(serialize = "64")]
    Żabbar,
    #[strum(serialize = "65")]
    ŻebbuġGozo,
    #[strum(serialize = "66")]
    ŻebbuġMalta,
    #[strum(serialize = "67")]
    Żejtun,
    #[strum(serialize = "68")]
    Żurrieq,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum MoldovaStatesAbbreviation {
    #[strum(serialize = "AN")]
    AneniiNoiDistrict,
    #[strum(serialize = "BS")]
    BasarabeascaDistrict,
    #[strum(serialize = "BD")]
    BenderMunicipality,
    #[strum(serialize = "BR")]
    BriceniDistrict,
    #[strum(serialize = "BA")]
    BălțiMunicipality,
    #[strum(serialize = "CA")]
    CahulDistrict,
    #[strum(serialize = "CT")]
    CantemirDistrict,
    #[strum(serialize = "CU")]
    ChișinăuMunicipality,
    #[strum(serialize = "CM")]
    CimișliaDistrict,
    #[strum(serialize = "CR")]
    CriuleniDistrict,
    #[strum(serialize = "CL")]
    CălărașiDistrict,
    #[strum(serialize = "CS")]
    CăușeniDistrict,
    #[strum(serialize = "DO")]
    DondușeniDistrict,
    #[strum(serialize = "DR")]
    DrochiaDistrict,
    #[strum(serialize = "DU")]
    DubăsariDistrict,
    #[strum(serialize = "ED")]
    EdinețDistrict,
    #[strum(serialize = "FL")]
    FloreștiDistrict,
    #[strum(serialize = "FA")]
    FăleștiDistrict,
    #[strum(serialize = "GA")]
    Găgăuzia,
    #[strum(serialize = "GL")]
    GlodeniDistrict,
    #[strum(serialize = "HI")]
    HînceștiDistrict,
    #[strum(serialize = "IA")]
    IaloveniDistrict,
    #[strum(serialize = "NI")]
    NisporeniDistrict,
    #[strum(serialize = "OC")]
    OcnițaDistrict,
    #[strum(serialize = "OR")]
    OrheiDistrict,
    #[strum(serialize = "RE")]
    RezinaDistrict,
    #[strum(serialize = "RI")]
    RîșcaniDistrict,
    #[strum(serialize = "SO")]
    SorocaDistrict,
    #[strum(serialize = "ST")]
    StrășeniDistrict,
    #[strum(serialize = "SI")]
    SîngereiDistrict,
    #[strum(serialize = "TA")]
    TaracliaDistrict,
    #[strum(serialize = "TE")]
    TeleneștiDistrict,
    #[strum(serialize = "SN")]
    TransnistriaAutonomousTerritorialUnit,
    #[strum(serialize = "UN")]
    UngheniDistrict,
    #[strum(serialize = "SD")]
    ȘoldăneștiDistrict,
    #[strum(serialize = "SV")]
    ȘtefanVodăDistrict,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum MonacoStatesAbbreviation {
    Monaco,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum MontenegroStatesAbbreviation {
    #[strum(serialize = "01")]
    AndrijevicaMunicipality,
    #[strum(serialize = "02")]
    BarMunicipality,
    #[strum(serialize = "03")]
    BeraneMunicipality,
    #[strum(serialize = "04")]
    BijeloPoljeMunicipality,
    #[strum(serialize = "05")]
    BudvaMunicipality,
    #[strum(serialize = "07")]
    DanilovgradMunicipality,
    #[strum(serialize = "22")]
    GusinjeMunicipality,
    #[strum(serialize = "09")]
    KolasinMunicipality,
    #[strum(serialize = "10")]
    KotorMunicipality,
    #[strum(serialize = "11")]
    MojkovacMunicipality,
    #[strum(serialize = "12")]
    NiksicMunicipality,
    #[strum(serialize = "06")]
    OldRoyalCapitalCetinje,
    #[strum(serialize = "23")]
    PetnjicaMunicipality,
    #[strum(serialize = "13")]
    PlavMunicipality,
    #[strum(serialize = "14")]
    PljevljaMunicipality,
    #[strum(serialize = "15")]
    PlužineMunicipality,
    #[strum(serialize = "16")]
    PodgoricaMunicipality,
    #[strum(serialize = "17")]
    RožajeMunicipality,
    #[strum(serialize = "19")]
    TivatMunicipality,
    #[strum(serialize = "20")]
    UlcinjMunicipality,
    #[strum(serialize = "18")]
    SavnikMunicipality,
    #[strum(serialize = "21")]
    ŽabljakMunicipality,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum NetherlandsStatesAbbreviation {
    #[strum(serialize = "BQ1")]
    Bonaire,
    #[strum(serialize = "DR")]
    Drenthe,
    #[strum(serialize = "FL")]
    Flevoland,
    #[strum(serialize = "FR")]
    Friesland,
    #[strum(serialize = "GE")]
    Gelderland,
    #[strum(serialize = "GR")]
    Groningen,
    #[strum(serialize = "LI")]
    Limburg,
    #[strum(serialize = "NB")]
    NorthBrabant,
    #[strum(serialize = "NH")]
    NorthHolland,
    #[strum(serialize = "OV")]
    Overijssel,
    #[strum(serialize = "BQ2")]
    Saba,
    #[strum(serialize = "BQ3")]
    SintEustatius,
    #[strum(serialize = "ZH")]
    SouthHolland,
    #[strum(serialize = "UT")]
    Utrecht,
    #[strum(serialize = "ZE")]
    Zeeland,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum NorthMacedoniaStatesAbbreviation {
    #[strum(serialize = "01")]
    AerodromMunicipality,
    #[strum(serialize = "02")]
    AracinovoMunicipality,
    #[strum(serialize = "03")]
    BerovoMunicipality,
    #[strum(serialize = "04")]
    BitolaMunicipality,
    #[strum(serialize = "05")]
    BogdanciMunicipality,
    #[strum(serialize = "06")]
    BogovinjeMunicipality,
    #[strum(serialize = "07")]
    BosilovoMunicipality,
    #[strum(serialize = "08")]
    BrvenicaMunicipality,
    #[strum(serialize = "09")]
    ButelMunicipality,
    #[strum(serialize = "77")]
    CentarMunicipality,
    #[strum(serialize = "78")]
    CentarZupaMunicipality,
    #[strum(serialize = "22")]
    DebarcaMunicipality,
    #[strum(serialize = "23")]
    DelcevoMunicipality,
    #[strum(serialize = "25")]
    DemirHisarMunicipality,
    #[strum(serialize = "24")]
    DemirKapijaMunicipality,
    #[strum(serialize = "26")]
    DojranMunicipality,
    #[strum(serialize = "27")]
    DolneniMunicipality,
    #[strum(serialize = "28")]
    DrugovoMunicipality,
    #[strum(serialize = "17")]
    GaziBabaMunicipality,
    #[strum(serialize = "18")]
    GevgelijaMunicipality,
    #[strum(serialize = "29")]
    GjorcePetrovMunicipality,
    #[strum(serialize = "19")]
    GostivarMunicipality,
    #[strum(serialize = "20")]
    GradskoMunicipality,
    #[strum(serialize = "85")]
    GreaterSkopje,
    #[strum(serialize = "34")]
    IlindenMunicipality,
    #[strum(serialize = "35")]
    JegunovceMunicipality,
    #[strum(serialize = "37")]
    Karbinci,
    #[strum(serialize = "38")]
    KarposMunicipality,
    #[strum(serialize = "36")]
    KavadarciMunicipality,
    #[strum(serialize = "39")]
    KiselaVodaMunicipality,
    #[strum(serialize = "40")]
    KicevoMunicipality,
    #[strum(serialize = "41")]
    KonceMunicipality,
    #[strum(serialize = "42")]
    KocaniMunicipality,
    #[strum(serialize = "43")]
    KratovoMunicipality,
    #[strum(serialize = "44")]
    KrivaPalankaMunicipality,
    #[strum(serialize = "45")]
    KrivogastaniMunicipality,
    #[strum(serialize = "46")]
    KrusevoMunicipality,
    #[strum(serialize = "47")]
    KumanovoMunicipality,
    #[strum(serialize = "48")]
    LipkovoMunicipality,
    #[strum(serialize = "49")]
    LozovoMunicipality,
    #[strum(serialize = "51")]
    MakedonskaKamenicaMunicipality,
    #[strum(serialize = "52")]
    MakedonskiBrodMunicipality,
    #[strum(serialize = "50")]
    MavrovoAndRostusaMunicipality,
    #[strum(serialize = "53")]
    MogilaMunicipality,
    #[strum(serialize = "54")]
    NegotinoMunicipality,
    #[strum(serialize = "55")]
    NovaciMunicipality,
    #[strum(serialize = "56")]
    NovoSeloMunicipality,
    #[strum(serialize = "58")]
    OhridMunicipality,
    #[strum(serialize = "57")]
    OslomejMunicipality,
    #[strum(serialize = "60")]
    PehcevoMunicipality,
    #[strum(serialize = "59")]
    PetrovecMunicipality,
    #[strum(serialize = "61")]
    PlasnicaMunicipality,
    #[strum(serialize = "62")]
    PrilepMunicipality,
    #[strum(serialize = "63")]
    ProbishtipMunicipality,
    #[strum(serialize = "64")]
    RadovisMunicipality,
    #[strum(serialize = "65")]
    RankovceMunicipality,
    #[strum(serialize = "66")]
    ResenMunicipality,
    #[strum(serialize = "67")]
    RosomanMunicipality,
    #[strum(serialize = "68")]
    SarajMunicipality,
    #[strum(serialize = "70")]
    SopisteMunicipality,
    #[strum(serialize = "71")]
    StaroNagoricaneMunicipality,
    #[strum(serialize = "72")]
    StrugaMunicipality,
    #[strum(serialize = "73")]
    StrumicaMunicipality,
    #[strum(serialize = "74")]
    StudenicaniMunicipality,
    #[strum(serialize = "69")]
    SvetiNikoleMunicipality,
    #[strum(serialize = "75")]
    TearceMunicipality,
    #[strum(serialize = "76")]
    TetovoMunicipality,
    #[strum(serialize = "10")]
    ValandovoMunicipality,
    #[strum(serialize = "11")]
    VasilevoMunicipality,
    #[strum(serialize = "13")]
    VelesMunicipality,
    #[strum(serialize = "12")]
    VevcaniMunicipality,
    #[strum(serialize = "14")]
    VinicaMunicipality,
    #[strum(serialize = "15")]
    VranesticaMunicipality,
    #[strum(serialize = "16")]
    VrapcisteMunicipality,
    #[strum(serialize = "31")]
    ZajasMunicipality,
    #[strum(serialize = "32")]
    ZelenikovoMunicipality,
    #[strum(serialize = "33")]
    ZrnovciMunicipality,
    #[strum(serialize = "79")]
    CairMunicipality,
    #[strum(serialize = "80")]
    CaskaMunicipality,
    #[strum(serialize = "81")]
    CesinovoOblesevoMunicipality,
    #[strum(serialize = "82")]
    CucerSandevoMunicipality,
    #[strum(serialize = "83")]
    StipMunicipality,
    #[strum(serialize = "84")]
    ShutoOrizariMunicipality,
    #[strum(serialize = "30")]
    ZelinoMunicipality,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum NorwayStatesAbbreviation {
    #[strum(serialize = "02")]
    Akershus,
    #[strum(serialize = "06")]
    Buskerud,
    #[strum(serialize = "20")]
    Finnmark,
    #[strum(serialize = "04")]
    Hedmark,
    #[strum(serialize = "12")]
    Hordaland,
    #[strum(serialize = "22")]
    JanMayen,
    #[strum(serialize = "15")]
    MoreOgRomsdal,
    #[strum(serialize = "17")]
    NordTrondelag,
    #[strum(serialize = "18")]
    Nordland,
    #[strum(serialize = "05")]
    Oppland,
    #[strum(serialize = "03")]
    Oslo,
    #[strum(serialize = "11")]
    Rogaland,
    #[strum(serialize = "14")]
    SognOgFjordane,
    #[strum(serialize = "21")]
    Svalbard,
    #[strum(serialize = "16")]
    SorTrondelag,
    #[strum(serialize = "08")]
    Telemark,
    #[strum(serialize = "19")]
    Troms,
    #[strum(serialize = "50")]
    Trondelag,
    #[strum(serialize = "10")]
    VestAgder,
    #[strum(serialize = "07")]
    Vestfold,
    #[strum(serialize = "01")]
    Ostfold,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum PolandStatesAbbreviation {
    #[strum(serialize = "30")]
    GreaterPoland,
    #[strum(serialize = "26")]
    HolyCross,
    #[strum(serialize = "04")]
    KuyaviaPomerania,
    #[strum(serialize = "12")]
    LesserPoland,
    #[strum(serialize = "02")]
    LowerSilesia,
    #[strum(serialize = "06")]
    Lublin,
    #[strum(serialize = "08")]
    Lubusz,
    #[strum(serialize = "10")]
    Łódź,
    #[strum(serialize = "14")]
    Mazovia,
    #[strum(serialize = "20")]
    Podlaskie,
    #[strum(serialize = "22")]
    Pomerania,
    #[strum(serialize = "24")]
    Silesia,
    #[strum(serialize = "18")]
    Subcarpathia,
    #[strum(serialize = "16")]
    UpperSilesia,
    #[strum(serialize = "28")]
    WarmiaMasuria,
    #[strum(serialize = "32")]
    WestPomerania,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum PortugalStatesAbbreviation {
    #[strum(serialize = "01")]
    AveiroDistrict,
    #[strum(serialize = "20")]
    Azores,
    #[strum(serialize = "02")]
    BejaDistrict,
    #[strum(serialize = "03")]
    BragaDistrict,
    #[strum(serialize = "04")]
    BragancaDistrict,
    #[strum(serialize = "05")]
    CasteloBrancoDistrict,
    #[strum(serialize = "06")]
    CoimbraDistrict,
    #[strum(serialize = "08")]
    FaroDistrict,
    #[strum(serialize = "09")]
    GuardaDistrict,
    #[strum(serialize = "10")]
    LeiriaDistrict,
    #[strum(serialize = "11")]
    LisbonDistrict,
    #[strum(serialize = "30")]
    Madeira,
    #[strum(serialize = "12")]
    PortalegreDistrict,
    #[strum(serialize = "13")]
    PortoDistrict,
    #[strum(serialize = "14")]
    SantaremDistrict,
    #[strum(serialize = "15")]
    SetubalDistrict,
    #[strum(serialize = "16")]
    VianaDoCasteloDistrict,
    #[strum(serialize = "17")]
    VilaRealDistrict,
    #[strum(serialize = "18")]
    ViseuDistrict,
    #[strum(serialize = "07")]
    EvoraDistrict,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum SpainStatesAbbreviation {
    #[strum(serialize = "C")]
    ACorunaProvince,
    #[strum(serialize = "AB")]
    AlbaceteProvince,
    #[strum(serialize = "A")]
    AlicanteProvince,
    #[strum(serialize = "AL")]
    AlmeriaProvince,
    #[strum(serialize = "AN")]
    Andalusia,
    #[strum(serialize = "VI")]
    ArabaAlava,
    #[strum(serialize = "AR")]
    Aragon,
    #[strum(serialize = "BA")]
    BadajozProvince,
    #[strum(serialize = "PM")]
    BalearicIslands,
    #[strum(serialize = "B")]
    BarcelonaProvince,
    #[strum(serialize = "PV")]
    BasqueCountry,
    #[strum(serialize = "BI")]
    Biscay,
    #[strum(serialize = "BU")]
    BurgosProvince,
    #[strum(serialize = "CN")]
    CanaryIslands,
    #[strum(serialize = "S")]
    Cantabria,
    #[strum(serialize = "CS")]
    CastellonProvince,
    #[strum(serialize = "CL")]
    CastileAndLeon,
    #[strum(serialize = "CM")]
    CastileLaMancha,
    #[strum(serialize = "CT")]
    Catalonia,
    #[strum(serialize = "CE")]
    Ceuta,
    #[strum(serialize = "CR")]
    CiudadRealProvince,
    #[strum(serialize = "MD")]
    CommunityOfMadrid,
    #[strum(serialize = "CU")]
    CuencaProvince,
    #[strum(serialize = "CC")]
    CaceresProvince,
    #[strum(serialize = "CA")]
    CadizProvince,
    #[strum(serialize = "CO")]
    CordobaProvince,
    #[strum(serialize = "EX")]
    Extremadura,
    #[strum(serialize = "GA")]
    Galicia,
    #[strum(serialize = "SS")]
    Gipuzkoa,
    #[strum(serialize = "GI")]
    GironaProvince,
    #[strum(serialize = "GR")]
    GranadaProvince,
    #[strum(serialize = "GU")]
    GuadalajaraProvince,
    #[strum(serialize = "H")]
    HuelvaProvince,
    #[strum(serialize = "HU")]
    HuescaProvince,
    #[strum(serialize = "J")]
    JaenProvince,
    #[strum(serialize = "RI")]
    LaRioja,
    #[strum(serialize = "GC")]
    LasPalmasProvince,
    #[strum(serialize = "LE")]
    LeonProvince,
    #[strum(serialize = "L")]
    LleidaProvince,
    #[strum(serialize = "LU")]
    LugoProvince,
    #[strum(serialize = "M")]
    MadridProvince,
    #[strum(serialize = "ML")]
    Melilla,
    #[strum(serialize = "MU")]
    MurciaProvince,
    #[strum(serialize = "MA")]
    MalagaProvince,
    #[strum(serialize = "NC")]
    Navarre,
    #[strum(serialize = "OR")]
    OurenseProvince,
    #[strum(serialize = "P")]
    PalenciaProvince,
    #[strum(serialize = "PO")]
    PontevedraProvince,
    #[strum(serialize = "O")]
    ProvinceOfAsturias,
    #[strum(serialize = "AV")]
    ProvinceOfAvila,
    #[strum(serialize = "MC")]
    RegionOfMurcia,
    #[strum(serialize = "SA")]
    SalamancaProvince,
    #[strum(serialize = "TF")]
    SantaCruzDeTenerifeProvince,
    #[strum(serialize = "SG")]
    SegoviaProvince,
    #[strum(serialize = "SE")]
    SevilleProvince,
    #[strum(serialize = "SO")]
    SoriaProvince,
    #[strum(serialize = "T")]
    TarragonaProvince,
    #[strum(serialize = "TE")]
    TeruelProvince,
    #[strum(serialize = "TO")]
    ToledoProvince,
    #[strum(serialize = "V")]
    ValenciaProvince,
    #[strum(serialize = "VC")]
    ValencianCommunity,
    #[strum(serialize = "VA")]
    ValladolidProvince,
    #[strum(serialize = "ZA")]
    ZamoraProvince,
    #[strum(serialize = "Z")]
    ZaragozaProvince,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum SwitzerlandStatesAbbreviation {
    #[strum(serialize = "AG")]
    Aargau,
    #[strum(serialize = "AR")]
    AppenzellAusserrhoden,
    #[strum(serialize = "AI")]
    AppenzellInnerrhoden,
    #[strum(serialize = "BL")]
    BaselLandschaft,
    #[strum(serialize = "FR")]
    CantonOfFribourg,
    #[strum(serialize = "GE")]
    CantonOfGeneva,
    #[strum(serialize = "JU")]
    CantonOfJura,
    #[strum(serialize = "LU")]
    CantonOfLucerne,
    #[strum(serialize = "NE")]
    CantonOfNeuchatel,
    #[strum(serialize = "SH")]
    CantonOfSchaffhausen,
    #[strum(serialize = "SO")]
    CantonOfSolothurn,
    #[strum(serialize = "SG")]
    CantonOfStGallen,
    #[strum(serialize = "VS")]
    CantonOfValais,
    #[strum(serialize = "VD")]
    CantonOfVaud,
    #[strum(serialize = "ZG")]
    CantonOfZug,
    #[strum(serialize = "GL")]
    Glarus,
    #[strum(serialize = "GR")]
    Graubunden,
    #[strum(serialize = "NW")]
    Nidwalden,
    #[strum(serialize = "OW")]
    Obwalden,
    #[strum(serialize = "SZ")]
    Schwyz,
    #[strum(serialize = "TG")]
    Thurgau,
    #[strum(serialize = "TI")]
    Ticino,
    #[strum(serialize = "UR")]
    Uri,
    #[strum(serialize = "BE")]
    CantonOfBern,
    #[strum(serialize = "ZH")]
    CantonOfZurich,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum UnitedKingdomStatesAbbreviation {
    #[strum(serialize = "ABE")]
    Aberdeen,
    #[strum(serialize = "ABD")]
    Aberdeenshire,
    #[strum(serialize = "ANS")]
    Angus,
    #[strum(serialize = "ANT")]
    Antrim,
    #[strum(serialize = "ANN")]
    AntrimAndNewtownabbey,
    #[strum(serialize = "ARD")]
    Ards,
    #[strum(serialize = "AND")]
    ArdsAndNorthDown,
    #[strum(serialize = "AGB")]
    ArgyllAndBute,
    #[strum(serialize = "ARM")]
    ArmaghCityAndDistrictCouncil,
    #[strum(serialize = "ABC")]
    ArmaghBanbridgeAndCraigavon,
    #[strum(serialize = "SH-AC")]
    AscensionIsland,
    #[strum(serialize = "BLA")]
    BallymenaBorough,
    #[strum(serialize = "BLY")]
    Ballymoney,
    #[strum(serialize = "BNB")]
    Banbridge,
    #[strum(serialize = "BNS")]
    Barnsley,
    #[strum(serialize = "BAS")]
    BathAndNorthEastSomerset,
    #[strum(serialize = "BDF")]
    Bedford,
    #[strum(serialize = "BFS")]
    BelfastDistrict,
    #[strum(serialize = "BIR")]
    Birmingham,
    #[strum(serialize = "BBD")]
    BlackburnWithDarwen,
    #[strum(serialize = "BPL")]
    Blackpool,
    #[strum(serialize = "BGW")]
    BlaenauGwentCountyBorough,
    #[strum(serialize = "BOL")]
    Bolton,
    #[strum(serialize = "BMH")]
    Bournemouth,
    #[strum(serialize = "BRC")]
    BracknellForest,
    #[strum(serialize = "BRD")]
    Bradford,
    #[strum(serialize = "BGE")]
    BridgendCountyBorough,
    #[strum(serialize = "BNH")]
    BrightonAndHove,
    #[strum(serialize = "BKM")]
    Buckinghamshire,
    #[strum(serialize = "BUR")]
    Bury,
    #[strum(serialize = "CAY")]
    CaerphillyCountyBorough,
    #[strum(serialize = "CLD")]
    Calderdale,
    #[strum(serialize = "CAM")]
    Cambridgeshire,
    #[strum(serialize = "CMN")]
    Carmarthenshire,
    #[strum(serialize = "CKF")]
    CarrickfergusBoroughCouncil,
    #[strum(serialize = "CSR")]
    Castlereagh,
    #[strum(serialize = "CCG")]
    CausewayCoastAndGlens,
    #[strum(serialize = "CBF")]
    CentralBedfordshire,
    #[strum(serialize = "CGN")]
    Ceredigion,
    #[strum(serialize = "CHE")]
    CheshireEast,
    #[strum(serialize = "CHW")]
    CheshireWestAndChester,
    #[strum(serialize = "CRF")]
    CityAndCountyOfCardiff,
    #[strum(serialize = "SWA")]
    CityAndCountyOfSwansea,
    #[strum(serialize = "BST")]
    CityOfBristol,
    #[strum(serialize = "DER")]
    CityOfDerby,
    #[strum(serialize = "KHL")]
    CityOfKingstonUponHull,
    #[strum(serialize = "LCE")]
    CityOfLeicester,
    #[strum(serialize = "LND")]
    CityOfLondon,
    #[strum(serialize = "NGM")]
    CityOfNottingham,
    #[strum(serialize = "PTE")]
    CityOfPeterborough,
    #[strum(serialize = "PLY")]
    CityOfPlymouth,
    #[strum(serialize = "POR")]
    CityOfPortsmouth,
    #[strum(serialize = "STH")]
    CityOfSouthampton,
    #[strum(serialize = "STE")]
    CityOfStokeOnTrent,
    #[strum(serialize = "SND")]
    CityOfSunderland,
    #[strum(serialize = "WSM")]
    CityOfWestminster,
    #[strum(serialize = "WLV")]
    CityOfWolverhampton,
    #[strum(serialize = "YOR")]
    CityOfYork,
    #[strum(serialize = "CLK")]
    Clackmannanshire,
    #[strum(serialize = "CLR")]
    ColeraineBoroughCouncil,
    #[strum(serialize = "CWY")]
    ConwyCountyBorough,
    #[strum(serialize = "CKT")]
    CookstownDistrictCouncil,
    #[strum(serialize = "CON")]
    Cornwall,
    #[strum(serialize = "DUR")]
    CountyDurham,
    #[strum(serialize = "COV")]
    Coventry,
    #[strum(serialize = "CGV")]
    CraigavonBoroughCouncil,
    #[strum(serialize = "CMA")]
    Cumbria,
    #[strum(serialize = "DAL")]
    Darlington,
    #[strum(serialize = "DEN")]
    Denbighshire,
    #[strum(serialize = "DBY")]
    Derbyshire,
    #[strum(serialize = "DRS")]
    DerryCityAndStrabane,
    #[strum(serialize = "DRY")]
    DerryCityCouncil,
    #[strum(serialize = "DEV")]
    Devon,
    #[strum(serialize = "DNC")]
    Doncaster,
    #[strum(serialize = "DOR")]
    Dorset,
    #[strum(serialize = "DOW")]
    DownDistrictCouncil,
    #[strum(serialize = "DUD")]
    Dudley,
    #[strum(serialize = "DGY")]
    DumfriesAndGalloway,
    #[strum(serialize = "DND")]
    Dundee,
    #[strum(serialize = "DGN")]
    DungannonAndSouthTyroneBoroughCouncil,
    #[strum(serialize = "EAY")]
    EastAyrshire,
    #[strum(serialize = "EDU")]
    EastDunbartonshire,
    #[strum(serialize = "ELN")]
    EastLothian,
    #[strum(serialize = "ERW")]
    EastRenfrewshire,
    #[strum(serialize = "ERY")]
    EastRidingOfYorkshire,
    #[strum(serialize = "ESX")]
    EastSussex,
    #[strum(serialize = "EDH")]
    Edinburgh,
    #[strum(serialize = "ENG")]
    England,
    #[strum(serialize = "ESS")]
    Essex,
    #[strum(serialize = "FAL")]
    Falkirk,
    #[strum(serialize = "FMO")]
    FermanaghAndOmagh,
    #[strum(serialize = "FER")]
    FermanaghDistrictCouncil,
    #[strum(serialize = "FIF")]
    Fife,
    #[strum(serialize = "FLN")]
    Flintshire,
    #[strum(serialize = "GAT")]
    Gateshead,
    #[strum(serialize = "GLG")]
    Glasgow,
    #[strum(serialize = "GLS")]
    Gloucestershire,
    #[strum(serialize = "GWN")]
    Gwynedd,
    #[strum(serialize = "HAL")]
    Halton,
    #[strum(serialize = "HAM")]
    Hampshire,
    #[strum(serialize = "HPL")]
    Hartlepool,
    #[strum(serialize = "HEF")]
    Herefordshire,
    #[strum(serialize = "HRT")]
    Hertfordshire,
    #[strum(serialize = "HLD")]
    Highland,
    #[strum(serialize = "IVC")]
    Inverclyde,
    #[strum(serialize = "IOW")]
    IsleOfWight,
    #[strum(serialize = "IOS")]
    IslesOfScilly,
    #[strum(serialize = "KEN")]
    Kent,
    #[strum(serialize = "KIR")]
    Kirklees,
    #[strum(serialize = "KWL")]
    Knowsley,
    #[strum(serialize = "LAN")]
    Lancashire,
    #[strum(serialize = "LRN")]
    LarneBoroughCouncil,
    #[strum(serialize = "LDS")]
    Leeds,
    #[strum(serialize = "LEC")]
    Leicestershire,
    #[strum(serialize = "LMV")]
    LimavadyBoroughCouncil,
    #[strum(serialize = "LIN")]
    Lincolnshire,
    #[strum(serialize = "LBC")]
    LisburnAndCastlereagh,
    #[strum(serialize = "LSB")]
    LisburnCityCouncil,
    #[strum(serialize = "LIV")]
    Liverpool,
    #[strum(serialize = "BDG")]
    LondonBoroughOfBarkingAndDagenham,
    #[strum(serialize = "BNE")]
    LondonBoroughOfBarnet,
    #[strum(serialize = "BEX")]
    LondonBoroughOfBexley,
    #[strum(serialize = "BEN")]
    LondonBoroughOfBrent,
    #[strum(serialize = "BRY")]
    LondonBoroughOfBromley,
    #[strum(serialize = "CMD")]
    LondonBoroughOfCamden,
    #[strum(serialize = "CRY")]
    LondonBoroughOfCroydon,
    #[strum(serialize = "EAL")]
    LondonBoroughOfEaling,
    #[strum(serialize = "ENF")]
    LondonBoroughOfEnfield,
    #[strum(serialize = "HCK")]
    LondonBoroughOfHackney,
    #[strum(serialize = "HMF")]
    LondonBoroughOfHammersmithAndFulham,
    #[strum(serialize = "HRY")]
    LondonBoroughOfHaringey,
    #[strum(serialize = "HRW")]
    LondonBoroughOfHarrow,
    #[strum(serialize = "HAV")]
    LondonBoroughOfHavering,
    #[strum(serialize = "HIL")]
    LondonBoroughOfHillingdon,
    #[strum(serialize = "HNS")]
    LondonBoroughOfHounslow,
    #[strum(serialize = "ISL")]
    LondonBoroughOfIslington,
    #[strum(serialize = "LBH")]
    LondonBoroughOfLambeth,
    #[strum(serialize = "LEW")]
    LondonBoroughOfLewisham,
    #[strum(serialize = "MRT")]
    LondonBoroughOfMerton,
    #[strum(serialize = "NWM")]
    LondonBoroughOfNewham,
    #[strum(serialize = "RDB")]
    LondonBoroughOfRedbridge,
    #[strum(serialize = "RIC")]
    LondonBoroughOfRichmondUponThames,
    #[strum(serialize = "SWK")]
    LondonBoroughOfSouthwark,
    #[strum(serialize = "STN")]
    LondonBoroughOfSutton,
    #[strum(serialize = "TWH")]
    LondonBoroughOfTowerHamlets,
    #[strum(serialize = "WFT")]
    LondonBoroughOfWalthamForest,
    #[strum(serialize = "WND")]
    LondonBoroughOfWandsworth,
    #[strum(serialize = "MFT")]
    MagherafeltDistrictCouncil,
    #[strum(serialize = "MAN")]
    Manchester,
    #[strum(serialize = "MDW")]
    Medway,
    #[strum(serialize = "MTY")]
    MerthyrTydfilCountyBorough,
    #[strum(serialize = "WGN")]
    MetropolitanBoroughOfWigan,
    #[strum(serialize = "MEA")]
    MidAndEastAntrim,
    #[strum(serialize = "MUL")]
    MidUlster,
    #[strum(serialize = "MDB")]
    Middlesbrough,
    #[strum(serialize = "MLN")]
    Midlothian,
    #[strum(serialize = "MIK")]
    MiltonKeynes,
    #[strum(serialize = "MON")]
    Monmouthshire,
    #[strum(serialize = "MRY")]
    Moray,
    #[strum(serialize = "MYL")]
    MoyleDistrictCouncil,
    #[strum(serialize = "NTL")]
    NeathPortTalbotCountyBorough,
    #[strum(serialize = "NET")]
    NewcastleUponTyne,
    #[strum(serialize = "NWP")]
    Newport,
    #[strum(serialize = "NYM")]
    NewryAndMourneDistrictCouncil,
    #[strum(serialize = "NMD")]
    NewryMourneAndDown,
    #[strum(serialize = "NTA")]
    NewtownabbeyBoroughCouncil,
    #[strum(serialize = "NFK")]
    Norfolk,
    #[strum(serialize = "NAY")]
    NorthAyrshire,
    #[strum(serialize = "NDN")]
    NorthDownBoroughCouncil,
    #[strum(serialize = "NEL")]
    NorthEastLincolnshire,
    #[strum(serialize = "NLK")]
    NorthLanarkshire,
    #[strum(serialize = "NLN")]
    NorthLincolnshire,
    #[strum(serialize = "NSM")]
    NorthSomerset,
    #[strum(serialize = "NTY")]
    NorthTyneside,
    #[strum(serialize = "NYK")]
    NorthYorkshire,
    #[strum(serialize = "NTH")]
    Northamptonshire,
    #[strum(serialize = "NIR")]
    NorthernIreland,
    #[strum(serialize = "NBL")]
    Northumberland,
    #[strum(serialize = "NTT")]
    Nottinghamshire,
    #[strum(serialize = "OLD")]
    Oldham,
    #[strum(serialize = "OMH")]
    OmaghDistrictCouncil,
    #[strum(serialize = "ORK")]
    OrkneyIslands,
    #[strum(serialize = "ELS")]
    OuterHebrides,
    #[strum(serialize = "OXF")]
    Oxfordshire,
    #[strum(serialize = "PEM")]
    Pembrokeshire,
    #[strum(serialize = "PKN")]
    PerthAndKinross,
    #[strum(serialize = "POL")]
    Poole,
    #[strum(serialize = "POW")]
    Powys,
    #[strum(serialize = "RDG")]
    Reading,
    #[strum(serialize = "RCC")]
    RedcarAndCleveland,
    #[strum(serialize = "RFW")]
    Renfrewshire,
    #[strum(serialize = "RCT")]
    RhonddaCynonTaf,
    #[strum(serialize = "RCH")]
    Rochdale,
    #[strum(serialize = "ROT")]
    Rotherham,
    #[strum(serialize = "GRE")]
    RoyalBoroughOfGreenwich,
    #[strum(serialize = "KEC")]
    RoyalBoroughOfKensingtonAndChelsea,
    #[strum(serialize = "KTT")]
    RoyalBoroughOfKingstonUponThames,
    #[strum(serialize = "RUT")]
    Rutland,
    #[strum(serialize = "SH-HL")]
    SaintHelena,
    #[strum(serialize = "SLF")]
    Salford,
    #[strum(serialize = "SAW")]
    Sandwell,
    #[strum(serialize = "SCT")]
    Scotland,
    #[strum(serialize = "SCB")]
    ScottishBorders,
    #[strum(serialize = "SFT")]
    Sefton,
    #[strum(serialize = "SHF")]
    Sheffield,
    #[strum(serialize = "ZET")]
    ShetlandIslands,
    #[strum(serialize = "SHR")]
    Shropshire,
    #[strum(serialize = "SLG")]
    Slough,
    #[strum(serialize = "SOL")]
    Solihull,
    #[strum(serialize = "SOM")]
    Somerset,
    #[strum(serialize = "SAY")]
    SouthAyrshire,
    #[strum(serialize = "SGC")]
    SouthGloucestershire,
    #[strum(serialize = "SLK")]
    SouthLanarkshire,
    #[strum(serialize = "STY")]
    SouthTyneside,
    #[strum(serialize = "SOS")]
    SouthendOnSea,
    #[strum(serialize = "SHN")]
    StHelens,
    #[strum(serialize = "STS")]
    Staffordshire,
    #[strum(serialize = "STG")]
    Stirling,
    #[strum(serialize = "SKP")]
    Stockport,
    #[strum(serialize = "STT")]
    StocktonOnTees,
    #[strum(serialize = "STB")]
    StrabaneDistrictCouncil,
    #[strum(serialize = "SFK")]
    Suffolk,
    #[strum(serialize = "SRY")]
    Surrey,
    #[strum(serialize = "SWD")]
    Swindon,
    #[strum(serialize = "TAM")]
    Tameside,
    #[strum(serialize = "TFW")]
    TelfordAndWrekin,
    #[strum(serialize = "THR")]
    Thurrock,
    #[strum(serialize = "TOB")]
    Torbay,
    #[strum(serialize = "TOF")]
    Torfaen,
    #[strum(serialize = "TRF")]
    Trafford,
    #[strum(serialize = "UKM")]
    UnitedKingdom,
    #[strum(serialize = "VGL")]
    ValeOfGlamorgan,
    #[strum(serialize = "WKF")]
    Wakefield,
    #[strum(serialize = "WLS")]
    Wales,
    #[strum(serialize = "WLL")]
    Walsall,
    #[strum(serialize = "WRT")]
    Warrington,
    #[strum(serialize = "WAR")]
    Warwickshire,
    #[strum(serialize = "WBK")]
    WestBerkshire,
    #[strum(serialize = "WDU")]
    WestDunbartonshire,
    #[strum(serialize = "WLN")]
    WestLothian,
    #[strum(serialize = "WSX")]
    WestSussex,
    #[strum(serialize = "WIL")]
    Wiltshire,
    #[strum(serialize = "WNM")]
    WindsorAndMaidenhead,
    #[strum(serialize = "WRL")]
    Wirral,
    #[strum(serialize = "WOK")]
    Wokingham,
    #[strum(serialize = "WOR")]
    Worcestershire,
    #[strum(serialize = "WRX")]
    WrexhamCountyBorough,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum BelgiumStatesAbbreviation {
    #[strum(serialize = "VAN")]
    Antwerp,
    #[strum(serialize = "BRU")]
    BrusselsCapitalRegion,
    #[strum(serialize = "VOV")]
    EastFlanders,
    #[strum(serialize = "VLG")]
    Flanders,
    #[strum(serialize = "VBR")]
    FlemishBrabant,
    #[strum(serialize = "WHT")]
    Hainaut,
    #[strum(serialize = "VLI")]
    Limburg,
    #[strum(serialize = "WLG")]
    Liege,
    #[strum(serialize = "WLX")]
    Luxembourg,
    #[strum(serialize = "WNA")]
    Namur,
    #[strum(serialize = "WAL")]
    Wallonia,
    #[strum(serialize = "WBR")]
    WalloonBrabant,
    #[strum(serialize = "VWV")]
    WestFlanders,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum LuxembourgStatesAbbreviation {
    #[strum(serialize = "CA")]
    CantonOfCapellen,
    #[strum(serialize = "CL")]
    CantonOfClervaux,
    #[strum(serialize = "DI")]
    CantonOfDiekirch,
    #[strum(serialize = "EC")]
    CantonOfEchternach,
    #[strum(serialize = "ES")]
    CantonOfEschSurAlzette,
    #[strum(serialize = "GR")]
    CantonOfGrevenmacher,
    #[strum(serialize = "LU")]
    CantonOfLuxembourg,
    #[strum(serialize = "ME")]
    CantonOfMersch,
    #[strum(serialize = "RD")]
    CantonOfRedange,
    #[strum(serialize = "RM")]
    CantonOfRemich,
    #[strum(serialize = "VD")]
    CantonOfVianden,
    #[strum(serialize = "WI")]
    CantonOfWiltz,
    #[strum(serialize = "D")]
    DiekirchDistrict,
    #[strum(serialize = "G")]
    GrevenmacherDistrict,
    #[strum(serialize = "L")]
    LuxembourgDistrict,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum RussiaStatesAbbreviation {
    #[strum(serialize = "ALT")]
    AltaiKrai,
    #[strum(serialize = "AL")]
    AltaiRepublic,
    #[strum(serialize = "AMU")]
    AmurOblast,
    #[strum(serialize = "ARK")]
    Arkhangelsk,
    #[strum(serialize = "AST")]
    AstrakhanOblast,
    #[strum(serialize = "BEL")]
    BelgorodOblast,
    #[strum(serialize = "BRY")]
    BryanskOblast,
    #[strum(serialize = "CE")]
    ChechenRepublic,
    #[strum(serialize = "CHE")]
    ChelyabinskOblast,
    #[strum(serialize = "CHU")]
    ChukotkaAutonomousOkrug,
    #[strum(serialize = "CU")]
    ChuvashRepublic,
    #[strum(serialize = "IRK")]
    Irkutsk,
    #[strum(serialize = "IVA")]
    IvanovoOblast,
    #[strum(serialize = "YEV")]
    JewishAutonomousOblast,
    #[strum(serialize = "KB")]
    KabardinoBalkarRepublic,
    #[strum(serialize = "KGD")]
    Kaliningrad,
    #[strum(serialize = "KLU")]
    KalugaOblast,
    #[strum(serialize = "KAM")]
    KamchatkaKrai,
    #[strum(serialize = "KC")]
    KarachayCherkessRepublic,
    #[strum(serialize = "KEM")]
    KemerovoOblast,
    #[strum(serialize = "KHA")]
    KhabarovskKrai,
    #[strum(serialize = "KHM")]
    KhantyMansiAutonomousOkrug,
    #[strum(serialize = "KIR")]
    KirovOblast,
    #[strum(serialize = "KO")]
    KomiRepublic,
    #[strum(serialize = "KOS")]
    KostromaOblast,
    #[strum(serialize = "KDA")]
    KrasnodarKrai,
    #[strum(serialize = "KYA")]
    KrasnoyarskKrai,
    #[strum(serialize = "KGN")]
    KurganOblast,
    #[strum(serialize = "KRS")]
    KurskOblast,
    #[strum(serialize = "LEN")]
    LeningradOblast,
    #[strum(serialize = "LIP")]
    LipetskOblast,
    #[strum(serialize = "MAG")]
    MagadanOblast,
    #[strum(serialize = "ME")]
    MariElRepublic,
    #[strum(serialize = "MOW")]
    Moscow,
    #[strum(serialize = "MOS")]
    MoscowOblast,
    #[strum(serialize = "MUR")]
    MurmanskOblast,
    #[strum(serialize = "NEN")]
    NenetsAutonomousOkrug,
    #[strum(serialize = "NIZ")]
    NizhnyNovgorodOblast,
    #[strum(serialize = "NGR")]
    NovgorodOblast,
    #[strum(serialize = "NVS")]
    Novosibirsk,
    #[strum(serialize = "OMS")]
    OmskOblast,
    #[strum(serialize = "ORE")]
    OrenburgOblast,
    #[strum(serialize = "ORL")]
    OryolOblast,
    #[strum(serialize = "PNZ")]
    PenzaOblast,
    #[strum(serialize = "PER")]
    PermKrai,
    #[strum(serialize = "PRI")]
    PrimorskyKrai,
    #[strum(serialize = "PSK")]
    PskovOblast,
    #[strum(serialize = "AD")]
    RepublicOfAdygea,
    #[strum(serialize = "BA")]
    RepublicOfBashkortostan,
    #[strum(serialize = "BU")]
    RepublicOfBuryatia,
    #[strum(serialize = "DA")]
    RepublicOfDagestan,
    #[strum(serialize = "IN")]
    RepublicOfIngushetia,
    #[strum(serialize = "KL")]
    RepublicOfKalmykia,
    #[strum(serialize = "KR")]
    RepublicOfKarelia,
    #[strum(serialize = "KK")]
    RepublicOfKhakassia,
    #[strum(serialize = "MO")]
    RepublicOfMordovia,
    #[strum(serialize = "SE")]
    RepublicOfNorthOssetiaAlania,
    #[strum(serialize = "TA")]
    RepublicOfTatarstan,
    #[strum(serialize = "ROS")]
    RostovOblast,
    #[strum(serialize = "RYA")]
    RyazanOblast,
    #[strum(serialize = "SPE")]
    SaintPetersburg,
    #[strum(serialize = "SA")]
    SakhaRepublic,
    #[strum(serialize = "SAK")]
    Sakhalin,
    #[strum(serialize = "SAM")]
    SamaraOblast,
    #[strum(serialize = "SAR")]
    SaratovOblast,
    #[strum(serialize = "UA-40")]
    Sevastopol,
    #[strum(serialize = "SMO")]
    SmolenskOblast,
    #[strum(serialize = "STA")]
    StavropolKrai,
    #[strum(serialize = "SVE")]
    Sverdlovsk,
    #[strum(serialize = "TAM")]
    TambovOblast,
    #[strum(serialize = "TOM")]
    TomskOblast,
    #[strum(serialize = "TUL")]
    TulaOblast,
    #[strum(serialize = "TY")]
    TuvaRepublic,
    #[strum(serialize = "TVE")]
    TverOblast,
    #[strum(serialize = "TYU")]
    TyumenOblast,
    #[strum(serialize = "UD")]
    UdmurtRepublic,
    #[strum(serialize = "ULY")]
    UlyanovskOblast,
    #[strum(serialize = "VLA")]
    VladimirOblast,
    #[strum(serialize = "VLG")]
    VologdaOblast,
    #[strum(serialize = "VOR")]
    VoronezhOblast,
    #[strum(serialize = "YAN")]
    YamaloNenetsAutonomousOkrug,
    #[strum(serialize = "YAR")]
    YaroslavlOblast,
    #[strum(serialize = "ZAB")]
    ZabaykalskyKrai,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum SanMarinoStatesAbbreviation {
    #[strum(serialize = "01")]
    Acquaviva,
    #[strum(serialize = "06")]
    BorgoMaggiore,
    #[strum(serialize = "02")]
    Chiesanuova,
    #[strum(serialize = "03")]
    Domagnano,
    #[strum(serialize = "04")]
    Faetano,
    #[strum(serialize = "05")]
    Fiorentino,
    #[strum(serialize = "08")]
    Montegiardino,
    #[strum(serialize = "07")]
    SanMarino,
    #[strum(serialize = "09")]
    Serravalle,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum SerbiaStatesAbbreviation {
    #[strum(serialize = "00")]
    Belgrade,

    #[strum(serialize = "01")]
    BorDistrict,

    #[strum(serialize = "02")]
    BraničevoDistrict,

    #[strum(serialize = "03")]
    CentralBanatDistrict,

    #[strum(serialize = "04")]
    JablanicaDistrict,

    #[strum(serialize = "05")]
    KolubaraDistrict,

    #[strum(serialize = "06")]
    MačvaDistrict,

    #[strum(serialize = "07")]
    MoravicaDistrict,

    #[strum(serialize = "08")]
    NišavaDistrict,

    #[strum(serialize = "09")]
    NorthBanatDistrict,

    #[strum(serialize = "10")]
    NorthBačkaDistrict,

    #[strum(serialize = "11")]
    PirotDistrict,

    #[strum(serialize = "12")]
    PodunavljeDistrict,

    #[strum(serialize = "13")]
    PomoravljeDistrict,

    #[strum(serialize = "14")]
    PčinjaDistrict,

    #[strum(serialize = "15")]
    RasinaDistrict,

    #[strum(serialize = "16")]
    RaškaDistrict,

    #[strum(serialize = "17")]
    SouthBanatDistrict,

    #[strum(serialize = "18")]
    SouthBačkaDistrict,

    #[strum(serialize = "19")]
    SremDistrict,

    #[strum(serialize = "20")]
    ToplicaDistrict,

    #[strum(serialize = "21")]
    Vojvodina,

    #[strum(serialize = "22")]
    WestBačkaDistrict,

    #[strum(serialize = "23")]
    ZaječarDistrict,

    #[strum(serialize = "24")]
    ZlatiborDistrict,

    #[strum(serialize = "25")]
    ŠumadijaDistrict,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum SlovakiaStatesAbbreviation {
    #[strum(serialize = "BC")]
    BanskaBystricaRegion,
    #[strum(serialize = "BL")]
    BratislavaRegion,
    #[strum(serialize = "KI")]
    KosiceRegion,
    #[strum(serialize = "NI")]
    NitraRegion,
    #[strum(serialize = "PV")]
    PresovRegion,
    #[strum(serialize = "TC")]
    TrencinRegion,
    #[strum(serialize = "TA")]
    TrnavaRegion,
    #[strum(serialize = "ZI")]
    ZilinaRegion,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum SloveniaStatesAbbreviation {
    #[strum(serialize = "001")]
    Ajdovščina,
    #[strum(serialize = "213")]
    Ankaran,
    #[strum(serialize = "002")]
    Beltinci,
    #[strum(serialize = "148")]
    Benedikt,
    #[strum(serialize = "149")]
    BistricaObSotli,
    #[strum(serialize = "003")]
    Bled,
    #[strum(serialize = "150")]
    Bloke,
    #[strum(serialize = "004")]
    Bohinj,
    #[strum(serialize = "005")]
    Borovnica,
    #[strum(serialize = "006")]
    Bovec,
    #[strum(serialize = "151")]
    Braslovče,
    #[strum(serialize = "007")]
    Brda,
    #[strum(serialize = "008")]
    Brezovica,
    #[strum(serialize = "009")]
    Brežice,
    #[strum(serialize = "152")]
    Cankova,
    #[strum(serialize = "012")]
    CerkljeNaGorenjskem,
    #[strum(serialize = "013")]
    Cerknica,
    #[strum(serialize = "014")]
    Cerkno,
    #[strum(serialize = "153")]
    Cerkvenjak,
    #[strum(serialize = "011")]
    CityMunicipalityOfCelje,
    #[strum(serialize = "085")]
    CityMunicipalityOfNovoMesto,
    #[strum(serialize = "018")]
    Destrnik,
    #[strum(serialize = "019")]
    Divača,
    #[strum(serialize = "154")]
    Dobje,
    #[strum(serialize = "020")]
    Dobrepolje,
    #[strum(serialize = "155")]
    Dobrna,
    #[strum(serialize = "021")]
    DobrovaPolhovGradec,
    #[strum(serialize = "156")]
    Dobrovnik,
    #[strum(serialize = "022")]
    DolPriLjubljani,
    #[strum(serialize = "157")]
    DolenjskeToplice,
    #[strum(serialize = "023")]
    Domžale,
    #[strum(serialize = "024")]
    Dornava,
    #[strum(serialize = "025")]
    Dravograd,
    #[strum(serialize = "026")]
    Duplek,
    #[strum(serialize = "027")]
    GorenjaVasPoljane,
    #[strum(serialize = "028")]
    Gorišnica,
    #[strum(serialize = "207")]
    Gorje,
    #[strum(serialize = "029")]
    GornjaRadgona,
    #[strum(serialize = "030")]
    GornjiGrad,
    #[strum(serialize = "031")]
    GornjiPetrovci,
    #[strum(serialize = "158")]
    Grad,
    #[strum(serialize = "032")]
    Grosuplje,
    #[strum(serialize = "159")]
    Hajdina,
    #[strum(serialize = "161")]
    Hodoš,
    #[strum(serialize = "162")]
    Horjul,
    #[strum(serialize = "160")]
    HočeSlivnica,
    #[strum(serialize = "034")]
    Hrastnik,
    #[strum(serialize = "035")]
    HrpeljeKozina,
    #[strum(serialize = "036")]
    Idrija,
    #[strum(serialize = "037")]
    Ig,
    #[strum(serialize = "039")]
    IvančnaGorica,
    #[strum(serialize = "040")]
    Izola,
    #[strum(serialize = "041")]
    Jesenice,
    #[strum(serialize = "163")]
    Jezersko,
    #[strum(serialize = "042")]
    Jursinci,
    #[strum(serialize = "043")]
    Kamnik,
    #[strum(serialize = "044")]
    KanalObSoci,
    #[strum(serialize = "045")]
    Kidricevo,
    #[strum(serialize = "046")]
    Kobarid,
    #[strum(serialize = "047")]
    Kobilje,
    #[strum(serialize = "049")]
    Komen,
    #[strum(serialize = "164")]
    Komenda,
    #[strum(serialize = "050")]
    Koper,
    #[strum(serialize = "197")]
    KostanjevicaNaKrki,
    #[strum(serialize = "165")]
    Kostel,
    #[strum(serialize = "051")]
    Kozje,
    #[strum(serialize = "048")]
    Kocevje,
    #[strum(serialize = "052")]
    Kranj,
    #[strum(serialize = "053")]
    KranjskaGora,
    #[strum(serialize = "166")]
    Krizevci,
    #[strum(serialize = "055")]
    Kungota,
    #[strum(serialize = "056")]
    Kuzma,
    #[strum(serialize = "057")]
    Lasko,
    #[strum(serialize = "058")]
    Lenart,
    #[strum(serialize = "059")]
    Lendava,
    #[strum(serialize = "060")]
    Litija,
    #[strum(serialize = "061")]
    Ljubljana,
    #[strum(serialize = "062")]
    Ljubno,
    #[strum(serialize = "063")]
    Ljutomer,
    #[strum(serialize = "064")]
    Logatec,
    #[strum(serialize = "208")]
    LogDragomer,
    #[strum(serialize = "167")]
    LovrencNaPohorju,
    #[strum(serialize = "065")]
    LoskaDolina,
    #[strum(serialize = "066")]
    LoskiPotok,
    #[strum(serialize = "068")]
    Lukovica,
    #[strum(serialize = "067")]
    Luče,
    #[strum(serialize = "069")]
    Majsperk,
    #[strum(serialize = "198")]
    Makole,
    #[strum(serialize = "070")]
    Maribor,
    #[strum(serialize = "168")]
    Markovci,
    #[strum(serialize = "071")]
    Medvode,
    #[strum(serialize = "072")]
    Menges,
    #[strum(serialize = "073")]
    Metlika,
    #[strum(serialize = "074")]
    Mezica,
    #[strum(serialize = "169")]
    MiklavzNaDravskemPolju,
    #[strum(serialize = "075")]
    MirenKostanjevica,
    #[strum(serialize = "212")]
    Mirna,
    #[strum(serialize = "170")]
    MirnaPec,
    #[strum(serialize = "076")]
    Mislinja,
    #[strum(serialize = "199")]
    MokronogTrebelno,
    #[strum(serialize = "078")]
    MoravskeToplice,
    #[strum(serialize = "077")]
    Moravce,
    #[strum(serialize = "079")]
    Mozirje,
    #[strum(serialize = "195")]
    Apače,
    #[strum(serialize = "196")]
    Cirkulane,
    #[strum(serialize = "038")]
    IlirskaBistrica,
    #[strum(serialize = "054")]
    Krsko,
    #[strum(serialize = "123")]
    Skofljica,
    #[strum(serialize = "080")]
    MurskaSobota,
    #[strum(serialize = "081")]
    Muta,
    #[strum(serialize = "082")]
    Naklo,
    #[strum(serialize = "083")]
    Nazarje,
    #[strum(serialize = "084")]
    NovaGorica,
    #[strum(serialize = "086")]
    Odranci,
    #[strum(serialize = "171")]
    Oplotnica,
    #[strum(serialize = "087")]
    Ormoz,
    #[strum(serialize = "088")]
    Osilnica,
    #[strum(serialize = "089")]
    Pesnica,
    #[strum(serialize = "090")]
    Piran,
    #[strum(serialize = "091")]
    Pivka,
    #[strum(serialize = "172")]
    Podlehnik,
    #[strum(serialize = "093")]
    Podvelka,
    #[strum(serialize = "092")]
    Podcetrtek,
    #[strum(serialize = "200")]
    Poljcane,
    #[strum(serialize = "173")]
    Polzela,
    #[strum(serialize = "094")]
    Postojna,
    #[strum(serialize = "174")]
    Prebold,
    #[strum(serialize = "095")]
    Preddvor,
    #[strum(serialize = "175")]
    Prevalje,
    #[strum(serialize = "096")]
    Ptuj,
    #[strum(serialize = "097")]
    Puconci,
    #[strum(serialize = "100")]
    Radenci,
    #[strum(serialize = "099")]
    Radece,
    #[strum(serialize = "101")]
    RadljeObDravi,
    #[strum(serialize = "102")]
    Radovljica,
    #[strum(serialize = "103")]
    RavneNaKoroskem,
    #[strum(serialize = "176")]
    Razkrizje,
    #[strum(serialize = "098")]
    RaceFram,
    #[strum(serialize = "201")]
    RenčeVogrsko,
    #[strum(serialize = "209")]
    RecicaObSavinji,
    #[strum(serialize = "104")]
    Ribnica,
    #[strum(serialize = "177")]
    RibnicaNaPohorju,
    #[strum(serialize = "107")]
    Rogatec,
    #[strum(serialize = "106")]
    RogaskaSlatina,
    #[strum(serialize = "105")]
    Rogasovci,
    #[strum(serialize = "108")]
    Ruse,
    #[strum(serialize = "178")]
    SelnicaObDravi,
    #[strum(serialize = "109")]
    Semic,
    #[strum(serialize = "110")]
    Sevnica,
    #[strum(serialize = "111")]
    Sezana,
    #[strum(serialize = "112")]
    SlovenjGradec,
    #[strum(serialize = "113")]
    SlovenskaBistrica,
    #[strum(serialize = "114")]
    SlovenskeKonjice,
    #[strum(serialize = "179")]
    Sodrazica,
    #[strum(serialize = "180")]
    Solcava,
    #[strum(serialize = "202")]
    SredisceObDravi,
    #[strum(serialize = "115")]
    Starse,
    #[strum(serialize = "203")]
    Straza,
    #[strum(serialize = "181")]
    SvetaAna,
    #[strum(serialize = "204")]
    SvetaTrojica,
    #[strum(serialize = "182")]
    SvetiAndraz,
    #[strum(serialize = "116")]
    SvetiJurijObScavnici,
    #[strum(serialize = "210")]
    SvetiJurijVSlovenskihGoricah,
    #[strum(serialize = "205")]
    SvetiTomaz,
    #[strum(serialize = "184")]
    Tabor,
    #[strum(serialize = "010")]
    Tišina,
    #[strum(serialize = "128")]
    Tolmin,
    #[strum(serialize = "129")]
    Trbovlje,
    #[strum(serialize = "130")]
    Trebnje,
    #[strum(serialize = "185")]
    TrnovskaVas,
    #[strum(serialize = "186")]
    Trzin,
    #[strum(serialize = "131")]
    Tržič,
    #[strum(serialize = "132")]
    Turnišče,
    #[strum(serialize = "187")]
    VelikaPolana,
    #[strum(serialize = "134")]
    VelikeLašče,
    #[strum(serialize = "188")]
    Veržej,
    #[strum(serialize = "135")]
    Videm,
    #[strum(serialize = "136")]
    Vipava,
    #[strum(serialize = "137")]
    Vitanje,
    #[strum(serialize = "138")]
    Vodice,
    #[strum(serialize = "139")]
    Vojnik,
    #[strum(serialize = "189")]
    Vransko,
    #[strum(serialize = "140")]
    Vrhnika,
    #[strum(serialize = "141")]
    Vuzenica,
    #[strum(serialize = "142")]
    ZagorjeObSavi,
    #[strum(serialize = "143")]
    Zavrč,
    #[strum(serialize = "144")]
    Zreče,
    #[strum(serialize = "015")]
    Črenšovci,
    #[strum(serialize = "016")]
    ČrnaNaKoroškem,
    #[strum(serialize = "017")]
    Črnomelj,
    #[strum(serialize = "033")]
    Šalovci,
    #[strum(serialize = "183")]
    ŠempeterVrtojba,
    #[strum(serialize = "118")]
    Šentilj,
    #[strum(serialize = "119")]
    Šentjernej,
    #[strum(serialize = "120")]
    Šentjur,
    #[strum(serialize = "211")]
    Šentrupert,
    #[strum(serialize = "117")]
    Šenčur,
    #[strum(serialize = "121")]
    Škocjan,
    #[strum(serialize = "122")]
    ŠkofjaLoka,
    #[strum(serialize = "124")]
    ŠmarjePriJelšah,
    #[strum(serialize = "206")]
    ŠmarješkeToplice,
    #[strum(serialize = "125")]
    ŠmartnoObPaki,
    #[strum(serialize = "194")]
    ŠmartnoPriLitiji,
    #[strum(serialize = "126")]
    Šoštanj,
    #[strum(serialize = "127")]
    Štore,
    #[strum(serialize = "190")]
    Žalec,
    #[strum(serialize = "146")]
    Železniki,
    #[strum(serialize = "191")]
    Žetale,
    #[strum(serialize = "147")]
    Žiri,
    #[strum(serialize = "192")]
    Žirovnica,
    #[strum(serialize = "193")]
    Žužemberk,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum SwedenStatesAbbreviation {
    #[strum(serialize = "K")]
    Blekinge,
    #[strum(serialize = "W")]
    DalarnaCounty,
    #[strum(serialize = "I")]
    GotlandCounty,
    #[strum(serialize = "X")]
    GävleborgCounty,
    #[strum(serialize = "N")]
    HallandCounty,
    #[strum(serialize = "F")]
    JönköpingCounty,
    #[strum(serialize = "H")]
    KalmarCounty,
    #[strum(serialize = "G")]
    KronobergCounty,
    #[strum(serialize = "BD")]
    NorrbottenCounty,
    #[strum(serialize = "M")]
    SkåneCounty,
    #[strum(serialize = "AB")]
    StockholmCounty,
    #[strum(serialize = "D")]
    SödermanlandCounty,
    #[strum(serialize = "C")]
    UppsalaCounty,
    #[strum(serialize = "S")]
    VärmlandCounty,
    #[strum(serialize = "AC")]
    VästerbottenCounty,
    #[strum(serialize = "Y")]
    VästernorrlandCounty,
    #[strum(serialize = "U")]
    VästmanlandCounty,
    #[strum(serialize = "O")]
    VästraGötalandCounty,
    #[strum(serialize = "T")]
    ÖrebroCounty,
    #[strum(serialize = "E")]
    ÖstergötlandCounty,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum UkraineStatesAbbreviation {
    #[strum(serialize = "43")]
    AutonomousRepublicOfCrimea,
    #[strum(serialize = "71")]
    CherkasyOblast,
    #[strum(serialize = "74")]
    ChernihivOblast,
    #[strum(serialize = "77")]
    ChernivtsiOblast,
    #[strum(serialize = "12")]
    DnipropetrovskOblast,
    #[strum(serialize = "14")]
    DonetskOblast,
    #[strum(serialize = "26")]
    IvanoFrankivskOblast,
    #[strum(serialize = "63")]
    KharkivOblast,
    #[strum(serialize = "65")]
    KhersonOblast,
    #[strum(serialize = "68")]
    KhmelnytskyOblast,
    #[strum(serialize = "30")]
    Kiev,
    #[strum(serialize = "35")]
    KirovohradOblast,
    #[strum(serialize = "32")]
    KyivOblast,
    #[strum(serialize = "09")]
    LuhanskOblast,
    #[strum(serialize = "46")]
    LvivOblast,
    #[strum(serialize = "48")]
    MykolaivOblast,
    #[strum(serialize = "51")]
    OdessaOblast,
    #[strum(serialize = "56")]
    RivneOblast,
    #[strum(serialize = "59")]
    SumyOblast,
    #[strum(serialize = "61")]
    TernopilOblast,
    #[strum(serialize = "05")]
    VinnytsiaOblast,
    #[strum(serialize = "07")]
    VolynOblast,
    #[strum(serialize = "21")]
    ZakarpattiaOblast,
    #[strum(serialize = "23")]
    ZaporizhzhyaOblast,
    #[strum(serialize = "18")]
    ZhytomyrOblast,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum RomaniaStatesAbbreviation {
    #[strum(serialize = "AB")]
    Alba,
    #[strum(serialize = "AR")]
    AradCounty,
    #[strum(serialize = "AG")]
    Arges,
    #[strum(serialize = "BC")]
    BacauCounty,
    #[strum(serialize = "BH")]
    BihorCounty,
    #[strum(serialize = "BN")]
    BistritaNasaudCounty,
    #[strum(serialize = "BT")]
    BotosaniCounty,
    #[strum(serialize = "BR")]
    Braila,
    #[strum(serialize = "BV")]
    BrasovCounty,
    #[strum(serialize = "B")]
    Bucharest,
    #[strum(serialize = "BZ")]
    BuzauCounty,
    #[strum(serialize = "CS")]
    CarasSeverinCounty,
    #[strum(serialize = "CJ")]
    ClujCounty,
    #[strum(serialize = "CT")]
    ConstantaCounty,
    #[strum(serialize = "CV")]
    CovasnaCounty,
    #[strum(serialize = "CL")]
    CalarasiCounty,
    #[strum(serialize = "DJ")]
    DoljCounty,
    #[strum(serialize = "DB")]
    DambovitaCounty,
    #[strum(serialize = "GL")]
    GalatiCounty,
    #[strum(serialize = "GR")]
    GiurgiuCounty,
    #[strum(serialize = "GJ")]
    GorjCounty,
    #[strum(serialize = "HR")]
    HarghitaCounty,
    #[strum(serialize = "HD")]
    HunedoaraCounty,
    #[strum(serialize = "IL")]
    IalomitaCounty,
    #[strum(serialize = "IS")]
    IasiCounty,
    #[strum(serialize = "IF")]
    IlfovCounty,
    #[strum(serialize = "MH")]
    MehedintiCounty,
    #[strum(serialize = "MM")]
    MuresCounty,
    #[strum(serialize = "NT")]
    NeamtCounty,
    #[strum(serialize = "OT")]
    OltCounty,
    #[strum(serialize = "PH")]
    PrahovaCounty,
    #[strum(serialize = "SM")]
    SatuMareCounty,
    #[strum(serialize = "SB")]
    SibiuCounty,
    #[strum(serialize = "SV")]
    SuceavaCounty,
    #[strum(serialize = "SJ")]
    SalajCounty,
    #[strum(serialize = "TR")]
    TeleormanCounty,
    #[strum(serialize = "TM")]
    TimisCounty,
    #[strum(serialize = "TL")]
    TulceaCounty,
    #[strum(serialize = "VS")]
    VasluiCounty,
    #[strum(serialize = "VN")]
    VranceaCounty,
    #[strum(serialize = "VL")]
    ValceaCounty,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PayoutStatus {
    Success,
    Failed,
    Cancelled,
    Initiated,
    Expired,
    Reversed,
    Pending,
    Ineligible,
    #[default]
    RequiresCreation,
    RequiresConfirmation,
    RequiresPayoutMethodData,
    RequiresFulfillment,
    RequiresVendorAccountCreation,
}

/// The payout_type of the payout request is a mandatory field for confirming the payouts. It should be specified in the Create request. If not provided, it must be updated in the Payout Update request before it can be confirmed.
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
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PayoutType {
    #[default]
    Card,
    Bank,
    Wallet,
}

/// Type of entity to whom the payout is being carried out to, select from the given list of options
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
#[serde(rename_all = "PascalCase")]
#[strum(serialize_all = "PascalCase")]
pub enum PayoutEntityType {
    /// Adyen
    #[default]
    Individual,
    Company,
    NonProfit,
    PublicSector,
    NaturalPerson,

    /// Wise
    #[strum(serialize = "lowercase")]
    #[serde(rename = "lowercase")]
    Business,
    Personal,
}

/// The send method which will be required for processing payouts, check options for better understanding.
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
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PayoutSendPriority {
    Instant,
    Fast,
    Regular,
    Wire,
    CrossBorder,
    Internal,
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
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PaymentSource {
    #[default]
    MerchantServer,
    Postman,
    Dashboard,
    Sdk,
    Webhook,
    ExternalAuthenticator,
}

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize, strum::EnumString)]
pub enum BrowserName {
    #[default]
    Safari,
    #[serde(other)]
    Unknown,
}

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum ClientPlatform {
    #[default]
    Web,
    Ios,
    Android,
    #[serde(other)]
    Unknown,
}

impl PaymentSource {
    pub fn is_for_internal_use_only(self) -> bool {
        match self {
            Self::Dashboard | Self::Sdk | Self::MerchantServer | Self::Postman => false,
            Self::Webhook | Self::ExternalAuthenticator => true,
        }
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
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
pub enum MerchantDecision {
    Approved,
    Rejected,
    AutoRefunded,
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
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FrmSuggestion {
    #[default]
    FrmCancelTransaction,
    FrmManualReview,
    FrmAuthorizeTransaction, // When manual capture payment which was marked fraud and held, when approved needs to be authorized.
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
    utoipa::ToSchema,
    Copy,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReconStatus {
    NotRequested,
    Requested,
    Active,
    Disabled,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthenticationConnectors {
    Threedsecureio,
    Netcetera,
    Gpayments,
    CtpMastercard,
    UnifiedAuthenticationService,
}

impl AuthenticationConnectors {
    pub fn is_separate_version_call_required(self) -> bool {
        match self {
            Self::Threedsecureio
            | Self::Netcetera
            | Self::CtpMastercard
            | Self::UnifiedAuthenticationService => false,
            Self::Gpayments => true,
        }
    }
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
    Copy,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthenticationStatus {
    #[default]
    Started,
    Pending,
    Success,
    Failed,
}

impl AuthenticationStatus {
    pub fn is_terminal_status(self) -> bool {
        match self {
            Self::Started | Self::Pending => false,
            Self::Success | Self::Failed => true,
        }
    }

    pub fn is_failed(self) -> bool {
        self == Self::Failed
    }
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
    Copy,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DecoupledAuthenticationType {
    #[default]
    Challenge,
    Frictionless,
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
    Copy,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthenticationLifecycleStatus {
    Used,
    #[default]
    Unused,
    Expired,
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
    Default,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ConnectorStatus {
    #[default]
    Inactive,
    Active,
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
    Default,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    #[default]
    Payment,
    #[cfg(feature = "payouts")]
    Payout,
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoleScope {
    Organization,
    Merchant,
    Profile,
}

impl From<RoleScope> for EntityType {
    fn from(role_scope: RoleScope) -> Self {
        match role_scope {
            RoleScope::Organization => Self::Organization,
            RoleScope::Merchant => Self::Merchant,
            RoleScope::Profile => Self::Profile,
        }
    }
}

/// Indicates the transaction status
#[derive(
    Clone,
    Default,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Eq,
    PartialEq,
    ToSchema,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "text")]
pub enum TransactionStatus {
    /// Authentication/ Account Verification Successful
    #[serde(rename = "Y")]
    Success,
    /// Not Authenticated /Account Not Verified; Transaction denied
    #[default]
    #[serde(rename = "N")]
    Failure,
    /// Authentication/ Account Verification Could Not Be Performed; Technical or other problem, as indicated in Authentication Response(ARes) or Result Request (RReq)
    #[serde(rename = "U")]
    VerificationNotPerformed,
    /// Attempts Processing Performed; Not Authenticated/Verified , but a proof of attempted authentication/verification is provided
    #[serde(rename = "A")]
    NotVerified,
    /// Authentication/ Account Verification Rejected; Issuer is rejecting authentication/verification and request that authorisation not be attempted.
    #[serde(rename = "R")]
    Rejected,
    /// Challenge Required; Additional authentication is required using the Challenge Request (CReq) / Challenge Response (CRes)
    #[serde(rename = "C")]
    ChallengeRequired,
    /// Challenge Required; Decoupled Authentication confirmed.
    #[serde(rename = "D")]
    ChallengeRequiredDecoupledAuthentication,
    /// Informational Only; 3DS Requestor challenge preference acknowledged.
    #[serde(rename = "I")]
    InformationOnly,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PermissionGroup {
    OperationsView,
    OperationsManage,
    ConnectorsView,
    ConnectorsManage,
    WorkflowsView,
    WorkflowsManage,
    AnalyticsView,
    UsersView,
    UsersManage,
    // TODO: To be deprecated, make sure DB is migrated before removing
    MerchantDetailsView,
    // TODO: To be deprecated, make sure DB is migrated before removing
    MerchantDetailsManage,
    // TODO: To be deprecated, make sure DB is migrated before removing
    OrganizationManage,
    AccountView,
    AccountManage,
    ReconReportsView,
    ReconReportsManage,
    ReconOpsView,
    ReconOpsManage,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq, Hash, strum::EnumIter)]
pub enum ParentGroup {
    Operations,
    Connectors,
    Workflows,
    Analytics,
    Users,
    ReconOps,
    ReconReports,
    Account,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    Payment,
    Refund,
    ApiKey,
    Account,
    Connector,
    Routing,
    Dispute,
    Mandate,
    Customer,
    Analytics,
    ThreeDsDecisionManager,
    SurchargeDecisionManager,
    User,
    WebhookEvent,
    Payout,
    Report,
    ReconToken,
    ReconFiles,
    ReconAndSettlementAnalytics,
    ReconUpload,
    ReconReports,
    RunRecon,
    ReconConfig,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, serde::Serialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    Read = 0,
    Write = 1,
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
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BankNames {
    AmericanExpress,
    AffinBank,
    AgroBank,
    AllianceBank,
    AmBank,
    BankOfAmerica,
    BankOfChina,
    BankIslam,
    BankMuamalat,
    BankRakyat,
    BankSimpananNasional,
    Barclays,
    BlikPSP,
    CapitalOne,
    Chase,
    Citi,
    CimbBank,
    Discover,
    NavyFederalCreditUnion,
    PentagonFederalCreditUnion,
    SynchronyBank,
    WellsFargo,
    AbnAmro,
    AsnBank,
    Bunq,
    Handelsbanken,
    HongLeongBank,
    HsbcBank,
    Ing,
    Knab,
    KuwaitFinanceHouse,
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
    BankMillennium,
    BankPEKAOSA,
    BawagPskAg,
    BksBankAg,
    BrullKallmusBankAg,
    BtvVierLanderBank,
    CapitalBankGraweGruppeAg,
    CeskaSporitelna,
    Dolomitenbank,
    EasybankAg,
    EPlatbyVUB,
    ErsteBankUndSparkassen,
    FrieslandBank,
    HypoAlpeadriabankInternationalAg,
    HypoNoeLbFurNiederosterreichUWien,
    HypoOberosterreichSalzburgSteiermark,
    HypoTirolBankAg,
    HypoVorarlbergBankAg,
    HypoBankBurgenlandAktiengesellschaft,
    KomercniBanka,
    MBank,
    MarchfelderBank,
    Maybank,
    OberbankAg,
    OsterreichischeArzteUndApothekerbank,
    OcbcBank,
    PayWithING,
    PlaceZIPKO,
    PlatnoscOnlineKartaPlatnicza,
    PosojilnicaBankEGen,
    PostovaBanka,
    PublicBank,
    RaiffeisenBankengruppeOsterreich,
    RhbBank,
    SchelhammerCapitalBankAg,
    StandardCharteredBank,
    SchoellerbankAg,
    SpardaBankWien,
    SporoPay,
    SantanderPrzelew24,
    TatraPay,
    Viamo,
    VolksbankGruppe,
    VolkskreditbankAg,
    VrBankBraunau,
    UobBank,
    PayWithAliorBank,
    BankiSpoldzielcze,
    PayWithInteligo,
    BNPParibasPoland,
    BankNowySA,
    CreditAgricole,
    PayWithBOS,
    PayWithCitiHandlowy,
    PayWithPlusBank,
    ToyotaBank,
    VeloBank,
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
    BangkokBank,
    KrungsriBank,
    KrungThaiBank,
    TheSiamCommercialBank,
    KasikornBank,
    OpenBankSuccess,
    OpenBankFailure,
    OpenBankCancelled,
    Aib,
    BankOfScotland,
    DanskeBank,
    FirstDirect,
    FirstTrust,
    Halifax,
    Lloyds,
    Monzo,
    NatWest,
    NationwideBank,
    RoyalBankOfScotland,
    Starling,
    TsbBank,
    TescoBank,
    UlsterBank,
    Yoursafe,
    N26,
    NationaleNederlanden,
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
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BankType {
    Checking,
    Savings,
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
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BankHolderType {
    Personal,
    Business,
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
    strum::Display,
    serde::Serialize,
    strum::EnumIter,
    strum::EnumString,
    strum::VariantNames,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum GenericLinkType {
    #[default]
    PaymentMethodCollect,
    PayoutLink,
}

#[derive(Debug, Clone, PartialEq, Eq, strum::Display, serde::Deserialize, serde::Serialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TokenPurpose {
    AuthSelect,
    #[serde(rename = "sso")]
    #[strum(serialize = "sso")]
    SSO,
    #[serde(rename = "totp")]
    #[strum(serialize = "totp")]
    TOTP,
    VerifyEmail,
    AcceptInvitationFromEmail,
    ForceSetPassword,
    ResetPassword,
    AcceptInvite,
    UserInfo,
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
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserAuthType {
    OpenIdConnect,
    MagicLink,
    #[default]
    Password,
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
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    Organization,
    Tenant,
    Internal,
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ApiVersion {
    V1,
    V2,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    ToSchema,
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Tenant = 3,
    Organization = 2,
    Merchant = 1,
    Profile = 0,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayoutRetryType {
    SingleConnector,
    MultiConnector,
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
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OrderFulfillmentTimeOrigin {
    Create,
    Confirm,
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
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum UIWidgetFormLayout {
    Tabs,
    Journey,
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
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DeleteStatus {
    Active,
    Redacted,
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
    Hash,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[router_derive::diesel_enum(storage_type = "db_enum")]
pub enum SuccessBasedRoutingConclusiveState {
    // pc: payment connector
    // sc: success based routing outcome/first connector
    // status: payment status
    //
    // status = success && pc == sc
    TruePositive,
    // status = failed && pc == sc
    FalsePositive,
    // status = failed && pc != sc
    TrueNegative,
    // status = success && pc != sc
    FalseNegative,
    // status = processing
    NonDeterministic,
}

/// Whether 3ds authentication is requested or not
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default, ToSchema)]
pub enum External3dsAuthenticationRequest {
    /// Request for 3ds authentication
    Enable,
    /// Skip 3ds authentication
    #[default]
    Skip,
}

/// Whether payment link is requested to be enabled or not for this transaction
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default, ToSchema)]
pub enum EnablePaymentLinkRequest {
    /// Request for enabling payment link
    Enable,
    /// Skip enabling payment link
    #[default]
    Skip,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default, ToSchema)]
pub enum MitExemptionRequest {
    /// Request for applying MIT exemption
    Apply,
    /// Skip applying MIT exemption
    #[default]
    Skip,
}

/// Set to `present` to indicate that the customer is in your checkout flow during this payment, and therefore is able to authenticate. This parameter should be `absent` when merchant's doing merchant initiated payments and customer is not present while doing the payment.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PresenceOfCustomerDuringPayment {
    /// Customer is present during the payment. This is the default value
    #[default]
    Present,
    /// Customer is absent during the payment
    Absent,
}

impl From<ConnectorType> for TransactionType {
    fn from(connector_type: ConnectorType) -> Self {
        match connector_type {
            #[cfg(feature = "payouts")]
            ConnectorType::PayoutProcessor => Self::Payout,
            _ => Self::Payment,
        }
    }
}

impl From<RefundStatus> for RelayStatus {
    fn from(refund_status: RefundStatus) -> Self {
        match refund_status {
            RefundStatus::Failure | RefundStatus::TransactionFailure => Self::Failure,
            RefundStatus::ManualReview | RefundStatus::Pending => Self::Pending,
            RefundStatus::Success => Self::Success,
        }
    }
}

impl From<RelayStatus> for RefundStatus {
    fn from(relay_status: RelayStatus) -> Self {
        match relay_status {
            RelayStatus::Failure => Self::Failure,
            RelayStatus::Pending | RelayStatus::Created => Self::Pending,
            RelayStatus::Success => Self::Success,
        }
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default, ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TaxCalculationOverride {
    /// Skip calling the external tax provider
    #[default]
    Skip,
    /// Calculate tax by calling the external tax provider
    Calculate,
}

impl From<Option<bool>> for TaxCalculationOverride {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => Self::Calculate,
            _ => Self::Skip,
        }
    }
}

impl TaxCalculationOverride {
    pub fn as_bool(self) -> bool {
        match self {
            Self::Skip => false,
            Self::Calculate => true,
        }
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default, ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SurchargeCalculationOverride {
    /// Skip calculating surcharge
    #[default]
    Skip,
    /// Calculate surcharge
    Calculate,
}

impl From<Option<bool>> for SurchargeCalculationOverride {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => Self::Calculate,
            _ => Self::Skip,
        }
    }
}

impl SurchargeCalculationOverride {
    pub fn as_bool(self) -> bool {
        match self {
            Self::Skip => false,
            Self::Calculate => true,
        }
    }
}

/// Connector Mandate Status
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, strum::Display,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ConnectorMandateStatus {
    /// Indicates that the connector mandate is active and can be used for payments.
    Active,
    /// Indicates that the connector mandate  is not active and hence cannot be used for payments.
    Inactive,
}

#[derive(
    Clone,
    Copy,
    Debug,
    strum::Display,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::EnumString,
    ToSchema,
    PartialOrd,
    Ord,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ErrorCategory {
    FrmDecline,
    ProcessorDowntime,
    ProcessorDeclineUnauthorized,
    IssueWithPaymentMethod,
    ProcessorDeclineIncorrectData,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
    Hash,
)]
pub enum PaymentChargeType {
    #[serde(untagged)]
    Stripe(StripeChargeType),
}

#[derive(
    Clone,
    Debug,
    Default,
    Hash,
    Eq,
    PartialEq,
    ToSchema,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum StripeChargeType {
    #[default]
    Direct,
    Destination,
}

/// Authentication Products
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthenticationProduct {
    ClickToPay,
}

/// Connector Access Method
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
    ToSchema,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentConnectorCategory {
    PaymentGateway,
    AlternativePaymentMethod,
    BankAcquirer,
}

/// The status of the feature
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    ToSchema,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    NotSupported,
    Supported,
}

/// The type of tokenization to use for the payment method
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    ToSchema,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TokenizationType {
    /// Create a single use token for the given payment method
    /// The user might have to go through additional factor authentication when using the single use token if required by the payment method
    SingleUse,
    /// Create a multi use token for the given payment method
    /// User will have to complete the additional factor authentication only once when creating the multi use token
    /// This will create a mandate at the connector which can be used for recurring payments
    MultiUse,
}

/// The network tokenization toggle, whether to enable or skip the network tokenization
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub enum NetworkTokenizationToggle {
    /// Enable network tokenization for the payment method
    Enable,
    /// Skip network tokenization for the payment method
    Skip,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GooglePayAuthMethod {
    /// Contain pan data only
    PanOnly,
    /// Contain cryptogram data along with pan data
    #[serde(rename = "CRYPTOGRAM_3DS")]
    Cryptogram,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[strum(serialize_all = "PascalCase")]
#[serde(rename_all = "PascalCase")]
pub enum AdyenSplitType {
    /// Books split amount to the specified account.
    BalanceAccount,
    /// The aggregated amount of the interchange and scheme fees.
    AcquiringFees,
    /// The aggregated amount of all transaction fees.
    PaymentFee,
    /// The aggregated amount of Adyen's commission and markup fees.
    AdyenFees,
    ///  The transaction fees due to Adyen under blended rates.
    AdyenCommission,
    /// The transaction fees due to Adyen under Interchange ++ pricing.
    AdyenMarkup,
    ///  The fees paid to the issuer for each payment made with the card network.
    Interchange,
    ///  The fees paid to the card scheme for using their network.
    SchemeFee,
    /// Your platform's commission on the payment (specified in amount), booked to your liable balance account.
    Commission,
    /// Allows you and your users to top up balance accounts using direct debit, card payments, or other payment methods.
    TopUp,
    /// The value-added tax charged on the payment, booked to your platforms liable balance account.
    Vat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename = "snake_case")]
pub enum PaymentConnectorTransmission {
    /// Failed to call the payment connector
    ConnectorCallFailed,
    /// Payment Connector call succeeded
    ConnectorCallSucceeded,
}
