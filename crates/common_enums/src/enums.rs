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
