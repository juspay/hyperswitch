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
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum AlbaniaStatesAbbreviation {
    #[serde(rename = "01")]
    Berat,
    #[serde(rename = "09")]
    Diber,
    #[serde(rename = "02")]
    Durres,
    #[serde(rename = "03")]
    Elbasan,
    #[serde(rename = "04")]
    Fier,
    #[serde(rename = "05")]
    Gjirokaster,
    #[serde(rename = "06")]
    Korce,
    #[serde(rename = "07")]
    Kukes,
    #[serde(rename = "08")]
    Lezhe,
    #[serde(rename = "10")]
    Shkoder,
    #[serde(rename = "11")]
    Tirane,
    #[serde(rename = "12")]
    Vlore,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum AndorraStatesAbbreviation {
    #[serde(rename = "07")]
    AndorraLaVella,
    #[serde(rename = "02")]
    Canillo,
    #[serde(rename = "03")]
    Encamp,
    #[serde(rename = "08")]
    EscaldesEngordany,
    #[serde(rename = "04")]
    LaMassana,
    #[serde(rename = "05")]
    Ordino,
    #[serde(rename = "06")]
    SantJuliaDeLoria,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum AustriaStatesAbbreviation {
    #[serde(rename = "1")]
    Burgenland,
    #[serde(rename = "2")]
    Carinthia,
    #[serde(rename = "3")]
    LowerAustria,
    #[serde(rename = "5")]
    Salzburg,
    #[serde(rename = "6")]
    Styria,
    #[serde(rename = "7")]
    Tyrol,
    #[serde(rename = "4")]
    UpperAustria,
    #[serde(rename = "9")]
    Vienna,
    #[serde(rename = "8")]
    Vorarlberg,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum BelarusStatesAbbreviation {
    #[serde(rename = "BR")]
    BrestRegion,
    #[serde(rename = "HO")]
    GomelRegion,
    #[serde(rename = "HR")]
    GrodnoRegion,
    #[serde(rename = "HM")]
    Minsk,
    #[serde(rename = "MI")]
    MinskRegion,
    #[serde(rename = "MA")]
    MogilevRegion,
    #[serde(rename = "VI")]
    VitebskRegion,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum BosniaAndHerzegovinaStatesAbbreviation {
    #[serde(rename = "05")]
    BosnianPodrinjeCanton,
    #[serde(rename = "BRC")]
    BrckoDistrict,
    #[serde(rename = "10")]
    Canton10,
    #[serde(rename = "06")]
    CentralBosniaCanton,
    #[serde(rename = "BIH")]
    FederationOfBosniaAndHerzegovina,
    #[serde(rename = "07")]
    HerzegovinaNeretvaCanton,
    #[serde(rename = "02")]
    PosavinaCanton,
    #[serde(rename = "SRP")]
    RepublikaSrpska,
    #[serde(rename = "09")]
    SarajevoCanton,
    #[serde(rename = "03")]
    TuzlaCanton,
    #[serde(rename = "01")]
    UnaSanaCanton,
    #[serde(rename = "08")]
    WestHerzegovinaCanton,
    #[serde(rename = "04")]
    ZenicaDobojCanton,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum BulgariaStatesAbbreviation {
    #[serde(rename = "01")]
    BlagoevgradProvince,
    #[serde(rename = "02")]
    BurgasProvince,
    #[serde(rename = "08")]
    DobrichProvince,
    #[serde(rename = "07")]
    GabrovoProvince,
    #[serde(rename = "26")]
    HaskovoProvince,
    #[serde(rename = "09")]
    KardzhaliProvince,
    #[serde(rename = "10")]
    KyustendilProvince,
    #[serde(rename = "11")]
    LovechProvince,
    #[serde(rename = "12")]
    MontanaProvince,
    #[serde(rename = "13")]
    PazardzhikProvince,
    #[serde(rename = "14")]
    PernikProvince,
    #[serde(rename = "15")]
    PlevenProvince,
    #[serde(rename = "16")]
    PlovdivProvince,
    #[serde(rename = "17")]
    RazgradProvince,
    #[serde(rename = "18")]
    RuseProvince,
    #[serde(rename = "27")]
    Shumen,
    #[serde(rename = "19")]
    SilistraProvince,
    #[serde(rename = "20")]
    SlivenProvince,
    #[serde(rename = "21")]
    SmolyanProvince,
    #[serde(rename = "22")]
    SofiaCityProvince,
    #[serde(rename = "23")]
    SofiaProvince,
    #[serde(rename = "24")]
    StaraZagoraProvince,
    #[serde(rename = "25")]
    TargovishteProvince,
    #[serde(rename = "03")]
    VarnaProvince,
    #[serde(rename = "04")]
    VelikoTarnovoProvince,
    #[serde(rename = "05")]
    VidinProvince,
    #[serde(rename = "06")]
    VratsaProvince,
    #[serde(rename = "28")]
    YambolProvince,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum CroatiaStatesAbbreviation {
    #[serde(rename = "07")]
    BjelovarBilogoraCounty,
    #[serde(rename = "12")]
    BrodPosavinaCounty,
    #[serde(rename = "19")]
    DubrovnikNeretvaCounty,
    #[serde(rename = "18")]
    IstriaCounty,
    #[serde(rename = "06")]
    KoprivnicaKrizevciCounty,
    #[serde(rename = "02")]
    KrapinaZagorjeCounty,
    #[serde(rename = "09")]
    LikaSenjCounty,
    #[serde(rename = "20")]
    MedimurjeCounty,
    #[serde(rename = "14")]
    OsijekBaranjaCounty,
    #[serde(rename = "11")]
    PozegaSlavoniaCounty,
    #[serde(rename = "08")]
    PrimorjeGorskiKotarCounty,
    #[serde(rename = "03")]
    SisakMoslavinaCounty,
    #[serde(rename = "17")]
    SplitDalmatiaCounty,
    #[serde(rename = "05")]
    VarazdinCounty,
    #[serde(rename = "10")]
    ViroviticaPodravinaCounty,
    #[serde(rename = "16")]
    VukovarSyrmiaCounty,
    #[serde(rename = "13")]
    ZadarCounty,
    #[serde(rename = "21")]
    Zagreb,
    #[serde(rename = "01")]
    ZagrebCounty,
    #[serde(rename = "15")]
    SibenikKninCounty,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum CzechRepublicStatesAbbreviation {
    #[serde(rename = "201")]
    BenesovDistrict,
    #[serde(rename = "202")]
    BerounDistrict,
    #[serde(rename = "641")]
    BlanskoDistrict,
    #[serde(rename = "642")]
    BrnoCityDistrict,
    #[serde(rename = "643")]
    BrnoCountryDistrict,
    #[serde(rename = "801")]
    BruntalDistrict,
    #[serde(rename = "644")]
    BreclavDistrict,
    #[serde(rename = "20")]
    CentralBohemianRegion,
    #[serde(rename = "411")]
    ChebDistrict,
    #[serde(rename = "422")]
    ChomutovDistrict,
    #[serde(rename = "531")]
    ChrudimDistrict,
    #[serde(rename = "321")]
    DomazliceDistrict,
    #[serde(rename = "421")]
    DecinDistrict,
    #[serde(rename = "802")]
    FrydekMistekDistrict,
    #[serde(rename = "631")]
    HavlickuvBrodDistrict,
    #[serde(rename = "645")]
    HodoninDistrict,
    #[serde(rename = "120")]
    HorniPocernice,
    #[serde(rename = "521")]
    HradecKraloveDistrict,
    #[serde(rename = "52")]
    HradecKraloveRegion,
    #[serde(rename = "512")]
    JablonecNadNisouDistrict,
    #[serde(rename = "711")]
    JesenikDistrict,
    #[serde(rename = "632")]
    JihlavaDistrict,
    #[serde(rename = "313")]
    JindrichuvHradecDistrict,
    #[serde(rename = "522")]
    JicinDistrict,
    #[serde(rename = "412")]
    KarlovyVaryDistrict,
    #[serde(rename = "41")]
    KarlovyVaryRegion,
    #[serde(rename = "803")]
    KarvinaDistrict,
    #[serde(rename = "203")]
    KladnoDistrict,
    #[serde(rename = "322")]
    KlatovyDistrict,
    #[serde(rename = "204")]
    KolinDistrict,
    #[serde(rename = "721")]
    KromerizDistrict,
    #[serde(rename = "513")]
    LiberecDistrict,
    #[serde(rename = "51")]
    LiberecRegion,
    #[serde(rename = "423")]
    LitomericeDistrict,
    #[serde(rename = "424")]
    LounyDistrict,
    #[serde(rename = "207")]
    MladaBoleslavDistrict,
    #[serde(rename = "80")]
    MoravianSilesianRegion,
    #[serde(rename = "425")]
    MostDistrict,
    #[serde(rename = "206")]
    MelnikDistrict,
    #[serde(rename = "804")]
    NovyJicinDistrict,
    #[serde(rename = "208")]
    NymburkDistrict,
    #[serde(rename = "523")]
    NachodDistrict,
    #[serde(rename = "712")]
    OlomoucDistrict,
    #[serde(rename = "71")]
    OlomoucRegion,
    #[serde(rename = "805")]
    OpavaDistrict,
    #[serde(rename = "806")]
    OstravaCityDistrict,
    #[serde(rename = "532")]
    PardubiceDistrict,
    #[serde(rename = "53")]
    PardubiceRegion,
    #[serde(rename = "633")]
    PelhrimovDistrict,
    #[serde(rename = "32")]
    PlzenRegion,
    #[serde(rename = "323")]
    PlzenCityDistrict,
    #[serde(rename = "325")]
    PlzenNorthDistrict,
    #[serde(rename = "324")]
    PlzenSouthDistrict,
    #[serde(rename = "315")]
    PrachaticeDistrict,
    #[serde(rename = "10")]
    Prague,
    #[serde(rename = "101")]
    Prague1,
    #[serde(rename = "110")]
    Prague10,
    #[serde(rename = "111")]
    Prague11,
    #[serde(rename = "112")]
    Prague12,
    #[serde(rename = "113")]
    Prague13,
    #[serde(rename = "114")]
    Prague14,
    #[serde(rename = "115")]
    Prague15,
    #[serde(rename = "116")]
    Prague16,
    #[serde(rename = "102")]
    Prague2,
    #[serde(rename = "121")]
    Prague21,
    #[serde(rename = "103")]
    Prague3,
    #[serde(rename = "104")]
    Prague4,
    #[serde(rename = "105")]
    Prague5,
    #[serde(rename = "106")]
    Prague6,
    #[serde(rename = "107")]
    Prague7,
    #[serde(rename = "108")]
    Prague8,
    #[serde(rename = "109")]
    Prague9,
    #[serde(rename = "209")]
    PragueEastDistrict,
    #[serde(rename = "20A")]
    PragueWestDistrict,
    #[serde(rename = "713")]
    ProstejovDistrict,
    #[serde(rename = "314")]
    PisekDistrict,
    #[serde(rename = "714")]
    PrerovDistrict,
    #[serde(rename = "20B")]
    PribramDistrict,
    #[serde(rename = "20C")]
    RakovnikDistrict,
    #[serde(rename = "326")]
    RokycanyDistrict,
    #[serde(rename = "524")]
    RychnovNadKneznouDistrict,
    #[serde(rename = "514")]
    SemilyDistrict,
    #[serde(rename = "413")]
    SokolovDistrict,
    #[serde(rename = "31")]
    SouthBohemianRegion,
    #[serde(rename = "64")]
    SouthMoravianRegion,
    #[serde(rename = "316")]
    StrakoniceDistrict,
    #[serde(rename = "533")]
    SvitavyDistrict,
    #[serde(rename = "327")]
    TachovDistrict,
    #[serde(rename = "426")]
    TepliceDistrict,
    #[serde(rename = "525")]
    TrutnovDistrict,
    #[serde(rename = "317")]
    TaborDistrict,
    #[serde(rename = "634")]
    TrebicDistrict,
    #[serde(rename = "722")]
    UherskeHradisteDistrict,
    #[serde(rename = "723")]
    VsetinDistrict,
    #[serde(rename = "63")]
    VysocinaRegion,
    #[serde(rename = "646")]
    VyskovDistrict,
    #[serde(rename = "724")]
    ZlinDistrict,
    #[serde(rename = "72")]
    ZlinRegion,
    #[serde(rename = "647")]
    ZnojmoDistrict,
    #[serde(rename = "427")]
    UstiNadLabemDistrict,
    #[serde(rename = "42")]
    UstiNadLabemRegion,
    #[serde(rename = "534")]
    UstiNadOrliciDistrict,
    #[serde(rename = "511")]
    CeskaLipaDistrict,
    #[serde(rename = "311")]
    CeskeBudejoviceDistrict,
    #[serde(rename = "312")]
    CeskyKrumlovDistrict,
    #[serde(rename = "715")]
    SumperkDistrict,
    #[serde(rename = "635")]
    ZdarNadSazavouDistrict,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum DenmarkStatesAbbreviation {
    #[serde(rename = "84")]
    CapitalRegionOfDenmark,
    #[serde(rename = "82")]
    CentralDenmarkRegion,
    #[serde(rename = "81")]
    NorthDenmarkRegion,
    #[serde(rename = "85")]
    RegionZealand,
    #[serde(rename = "83")]
    RegionOfSouthernDenmark,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum FinlandStatesAbbreviation {
    #[serde(rename = "08")]
    CentralFinland,
    #[serde(rename = "07")]
    CentralOstrobothnia,
    #[serde(rename = "IS")]
    EasternFinlandProvince,
    #[serde(rename = "19")]
    FinlandProper,
    #[serde(rename = "05")]
    Kainuu,
    #[serde(rename = "09")]
    Kymenlaakso,
    #[serde(rename = "LL")]
    Lapland,
    #[serde(rename = "13")]
    NorthKarelia,
    #[serde(rename = "14")]
    NorthernOstrobothnia,
    #[serde(rename = "15")]
    NorthernSavonia,
    #[serde(rename = "12")]
    Ostrobothnia,
    #[serde(rename = "OL")]
    OuluProvince,
    #[serde(rename = "11")]
    Pirkanmaa,
    #[serde(rename = "16")]
    PaijanneTavastia,
    #[serde(rename = "17")]
    Satakunta,
    #[serde(rename = "02")]
    SouthKarelia,
    #[serde(rename = "03")]
    SouthernOstrobothnia,
    #[serde(rename = "04")]
    SouthernSavonia,
    #[serde(rename = "06")]
    TavastiaProper,
    #[serde(rename = "18")]
    Uusimaa,
    #[serde(rename = "01")]
    AlandIslands,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum FranceStatesAbbreviation {
    #[serde(rename = "WF-AL")]
    Alo,
    #[serde(rename = "A")]
    Alsace,
    #[serde(rename = "B")]
    Aquitaine,
    #[serde(rename = "C")]
    Auvergne,
    #[serde(rename = "ARA")]
    AuvergneRhoneAlpes,
    #[serde(rename = "BFC")]
    BourgogneFrancheComte,
    #[serde(rename = "BRE")]
    Brittany,
    #[serde(rename = "D")]
    Burgundy,
    #[serde(rename = "CVL")]
    CentreValDeLoire,
    #[serde(rename = "G")]
    ChampagneArdenne,
    #[serde(rename = "COR")]
    Corsica,
    #[serde(rename = "I")]
    FrancheComte,
    #[serde(rename = "GF")]
    FrenchGuiana,
    #[serde(rename = "PF")]
    FrenchPolynesia,
    #[serde(rename = "GES")]
    GrandEst,
    #[serde(rename = "GP")]
    Guadeloupe,
    #[serde(rename = "HDF")]
    HautsDeFrance,
    #[serde(rename = "K")]
    LanguedocRoussillon,
    #[serde(rename = "L")]
    Limousin,
    #[serde(rename = "M")]
    Lorraine,
    #[serde(rename = "P")]
    LowerNormandy,
    #[serde(rename = "MQ")]
    Martinique,
    #[serde(rename = "YT")]
    Mayotte,
    #[serde(rename = "O")]
    NordPasDeCalais,
    #[serde(rename = "NOR")]
    Normandy,
    #[serde(rename = "NAQ")]
    NouvelleAquitaine,
    #[serde(rename = "OCC")]
    Occitania,
    #[serde(rename = "75")]
    Paris,
    #[serde(rename = "PDL")]
    PaysDeLaLoire,
    #[serde(rename = "S")]
    Picardy,
    #[serde(rename = "T")]
    PoitouCharentes,
    #[serde(rename = "PAC")]
    ProvenceAlpesCoteDAzur,
    #[serde(rename = "V")]
    RhoneAlpes,
    #[serde(rename = "RE")]
    Reunion,
    #[serde(rename = "BL")]
    SaintBarthelemy,
    #[serde(rename = "MF")]
    SaintMartin,
    #[serde(rename = "PM")]
    SaintPierreAndMiquelon,
    #[serde(rename = "WF-SG")]
    Sigave,
    #[serde(rename = "Q")]
    UpperNormandy,
    #[serde(rename = "WF-UV")]
    Uvea,
    #[serde(rename = "WF")]
    WallisAndFutuna,
    #[serde(rename = "IDF")]
    IleDeFrance,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum GermanyStatesAbbreviation {
    #[serde(rename = "BW")]
    BadenWurttemberg,
    #[serde(rename = "BY")]
    Bavaria,
    #[serde(rename = "BE")]
    Berlin,
    #[serde(rename = "BB")]
    Brandenburg,
    #[serde(rename = "HB")]
    Bremen,
    #[serde(rename = "HH")]
    Hamburg,
    #[serde(rename = "HE")]
    Hesse,
    #[serde(rename = "NI")]
    LowerSaxony,
    #[serde(rename = "MV")]
    MecklenburgVorpommern,
    #[serde(rename = "NW")]
    NorthRhineWestphalia,
    #[serde(rename = "RP")]
    RhinelandPalatinate,
    #[serde(rename = "SL")]
    Saarland,
    #[serde(rename = "SN")]
    Saxony,
    #[serde(rename = "ST")]
    SaxonyAnhalt,
    #[serde(rename = "SH")]
    SchleswigHolstein,
    #[serde(rename = "TH")]
    Thuringia,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum GreeceStatesAbbreviation {
    #[serde(rename = "13")]
    AchaeaRegionalUnit,
    #[serde(rename = "01")]
    AetoliaAcarnaniaRegionalUnit,
    #[serde(rename = "12")]
    ArcadiaPrefecture,
    #[serde(rename = "11")]
    ArgolisRegionalUnit,
    #[serde(rename = "I")]
    AtticaRegion,
    #[serde(rename = "03")]
    BoeotiaRegionalUnit,
    #[serde(rename = "H")]
    CentralGreeceRegion,
    #[serde(rename = "B")]
    CentralMacedonia,
    #[serde(rename = "94")]
    ChaniaRegionalUnit,
    #[serde(rename = "22")]
    CorfuPrefecture,
    #[serde(rename = "15")]
    CorinthiaRegionalUnit,
    #[serde(rename = "M")]
    CreteRegion,
    #[serde(rename = "52")]
    DramaRegionalUnit,
    #[serde(rename = "A2")]
    EastAtticaRegionalUnit,
    #[serde(rename = "A")]
    EastMacedoniaAndThrace,
    #[serde(rename = "D")]
    EpirusRegion,
    #[serde(rename = "04")]
    Euboea,
    #[serde(rename = "51")]
    GrevenaPrefecture,
    #[serde(rename = "53")]
    ImathiaRegionalUnit,
    #[serde(rename = "33")]
    IoanninaRegionalUnit,
    #[serde(rename = "F")]
    IonianIslandsRegion,
    #[serde(rename = "41")]
    KarditsaRegionalUnit,
    #[serde(rename = "56")]
    KastoriaRegionalUnit,
    #[serde(rename = "23")]
    KefaloniaPrefecture,
    #[serde(rename = "57")]
    KilkisRegionalUnit,
    #[serde(rename = "58")]
    KozaniPrefecture,
    #[serde(rename = "16")]
    Laconia,
    #[serde(rename = "42")]
    LarissaPrefecture,
    #[serde(rename = "24")]
    LefkadaRegionalUnit,
    #[serde(rename = "59")]
    PellaRegionalUnit,
    #[serde(rename = "J")]
    PeloponneseRegion,
    #[serde(rename = "06")]
    PhthiotisPrefecture,
    #[serde(rename = "34")]
    PrevezaPrefecture,
    #[serde(rename = "62")]
    SerresPrefecture,
    #[serde(rename = "L")]
    SouthAegean,
    #[serde(rename = "54")]
    ThessalonikiRegionalUnit,
    #[serde(rename = "G")]
    WestGreeceRegion,
    #[serde(rename = "C")]
    WestMacedoniaRegion,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum HungaryStatesAbbreviation {
    #[serde(rename = "BA")]
    BaranyaCounty,
    #[serde(rename = "BZ")]
    BorsodAbaujZemplenCounty,
    #[serde(rename = "BU")]
    Budapest,
    #[serde(rename = "BK")]
    BacsKiskunCounty,
    #[serde(rename = "BE")]
    BekesCounty,
    #[serde(rename = "BC")]
    Bekescsaba,
    #[serde(rename = "CS")]
    CsongradCounty,
    #[serde(rename = "DE")]
    Debrecen,
    #[serde(rename = "DU")]
    Dunaujvaros,
    #[serde(rename = "EG")]
    Eger,
    #[serde(rename = "FE")]
    FejerCounty,
    #[serde(rename = "GY")]
    Gyor,
    #[serde(rename = "GS")]
    GyorMosonSopronCounty,
    #[serde(rename = "HB")]
    HajduBiharCounty,
    #[serde(rename = "HE")]
    HevesCounty,
    #[serde(rename = "HV")]
    Hodmezovasarhely,
    #[serde(rename = "JN")]
    JaszNagykunSzolnokCounty,
    #[serde(rename = "KV")]
    Kaposvar,
    #[serde(rename = "KM")]
    Kecskemet,
    #[serde(rename = "MI")]
    Miskolc,
    #[serde(rename = "NK")]
    Nagykanizsa,
    #[serde(rename = "NY")]
    Nyiregyhaza,
    #[serde(rename = "NO")]
    NogradCounty,
    #[serde(rename = "PE")]
    PestCounty,
    #[serde(rename = "PS")]
    Pecs,
    #[serde(rename = "ST")]
    Salgotarjan,
    #[serde(rename = "SO")]
    SomogyCounty,
    #[serde(rename = "SN")]
    Sopron,
    #[serde(rename = "SZ")]
    SzabolcsSzatmarBeregCounty,
    #[serde(rename = "SD")]
    Szeged,
    #[serde(rename = "SS")]
    Szekszard,
    #[serde(rename = "SK")]
    Szolnok,
    #[serde(rename = "SH")]
    Szombathely,
    #[serde(rename = "SF")]
    Szekesfehervar,
    #[serde(rename = "TB")]
    Tatabanya,
    #[serde(rename = "TO")]
    TolnaCounty,
    #[serde(rename = "VA")]
    VasCounty,
    #[serde(rename = "VM")]
    Veszprem,
    #[serde(rename = "VE")]
    VeszpremCounty,
    #[serde(rename = "ZA")]
    ZalaCounty,
    #[serde(rename = "ZE")]
    Zalaegerszeg,
    #[serde(rename = "ER")]
    Erd,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum IcelandStatesAbbreviation {
    #[serde(rename = "1")]
    CapitalRegion,
    #[serde(rename = "7")]
    EasternRegion,
    #[serde(rename = "6")]
    NortheasternRegion,
    #[serde(rename = "5")]
    NorthwesternRegion,
    #[serde(rename = "2")]
    SouthernPeninsulaRegion,
    #[serde(rename = "8")]
    SouthernRegion,
    #[serde(rename = "3")]
    WesternRegion,
    #[serde(rename = "4")]
    Westfjords,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum IrelandStatesAbbreviation {
    #[serde(rename = "C")]
    Connacht,
    #[serde(rename = "CW")]
    CountyCarlow,
    #[serde(rename = "CN")]
    CountyCavan,
    #[serde(rename = "CE")]
    CountyClare,
    #[serde(rename = "CO")]
    CountyCork,
    #[serde(rename = "DL")]
    CountyDonegal,
    #[serde(rename = "D")]
    CountyDublin,
    #[serde(rename = "G")]
    CountyGalway,
    #[serde(rename = "KY")]
    CountyKerry,
    #[serde(rename = "KE")]
    CountyKildare,
    #[serde(rename = "KK")]
    CountyKilkenny,
    #[serde(rename = "LS")]
    CountyLaois,
    #[serde(rename = "LK")]
    CountyLimerick,
    #[serde(rename = "LD")]
    CountyLongford,
    #[serde(rename = "LH")]
    CountyLouth,
    #[serde(rename = "MO")]
    CountyMayo,
    #[serde(rename = "MH")]
    CountyMeath,
    #[serde(rename = "MN")]
    CountyMonaghan,
    #[serde(rename = "OY")]
    CountyOffaly,
    #[serde(rename = "RN")]
    CountyRoscommon,
    #[serde(rename = "SO")]
    CountySligo,
    #[serde(rename = "TA")]
    CountyTipperary,
    #[serde(rename = "WD")]
    CountyWaterford,
    #[serde(rename = "WH")]
    CountyWestmeath,
    #[serde(rename = "WX")]
    CountyWexford,
    #[serde(rename = "WW")]
    CountyWicklow,
    #[serde(rename = "L")]
    Leinster,
    #[serde(rename = "M")]
    Munster,
    #[serde(rename = "U")]
    Ulster,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum LatviaStatesAbbreviation {
    #[serde(rename = "001")]
    AglonaMunicipality,
    #[serde(rename = "002")]
    AizkraukleMunicipality,
    #[serde(rename = "003")]
    AizputeMunicipality,
    #[serde(rename = "004")]
    AknīsteMunicipality,
    #[serde(rename = "005")]
    AlojaMunicipality,
    #[serde(rename = "006")]
    AlsungaMunicipality,
    #[serde(rename = "007")]
    AlūksneMunicipality,
    #[serde(rename = "008")]
    AmataMunicipality,
    #[serde(rename = "009")]
    ApeMunicipality,
    #[serde(rename = "010")]
    AuceMunicipality,
    #[serde(rename = "012")]
    BabīteMunicipality,
    #[serde(rename = "013")]
    BaldoneMunicipality,
    #[serde(rename = "014")]
    BaltinavaMunicipality,
    #[serde(rename = "015")]
    BalviMunicipality,
    #[serde(rename = "016")]
    BauskaMunicipality,
    #[serde(rename = "017")]
    BeverīnaMunicipality,
    #[serde(rename = "018")]
    BrocēniMunicipality,
    #[serde(rename = "019")]
    BurtniekiMunicipality,
    #[serde(rename = "020")]
    CarnikavaMunicipality,
    #[serde(rename = "021")]
    CesvaineMunicipality,
    #[serde(rename = "023")]
    CiblaMunicipality,
    #[serde(rename = "022")]
    CēsisMunicipality,
    #[serde(rename = "024")]
    DagdaMunicipality,
    #[serde(rename = "DGV")]
    Daugavpils,
    #[serde(rename = "025")]
    DaugavpilsMunicipality,
    #[serde(rename = "026")]
    DobeleMunicipality,
    #[serde(rename = "027")]
    DundagaMunicipality,
    #[serde(rename = "028")]
    DurbeMunicipality,
    #[serde(rename = "029")]
    EngureMunicipality,
    #[serde(rename = "031")]
    GarkalneMunicipality,
    #[serde(rename = "032")]
    GrobiņaMunicipality,
    #[serde(rename = "033")]
    GulbeneMunicipality,
    #[serde(rename = "034")]
    IecavaMunicipality,
    #[serde(rename = "035")]
    IkšķileMunicipality,
    #[serde(rename = "036")]
    IlūksteMunicipality,
    #[serde(rename = "037")]
    InčukalnsMunicipality,
    #[serde(rename = "038")]
    JaunjelgavaMunicipality,
    #[serde(rename = "039")]
    JaunpiebalgaMunicipality,
    #[serde(rename = "040")]
    JaunpilsMunicipality,
    #[serde(rename = "JEL")]
    Jelgava,
    #[serde(rename = "041")]
    JelgavaMunicipality,
    #[serde(rename = "JKB")]
    Jēkabpils,
    #[serde(rename = "042")]
    JēkabpilsMunicipality,
    #[serde(rename = "JUR")]
    Jūrmala,
    #[serde(rename = "043")]
    KandavaMunicipality,
    #[serde(rename = "045")]
    KocēniMunicipality,
    #[serde(rename = "046")]
    KokneseMunicipality,
    #[serde(rename = "048")]
    KrimuldaMunicipality,
    #[serde(rename = "049")]
    KrustpilsMunicipality,
    #[serde(rename = "047")]
    KrāslavaMunicipality,
    #[serde(rename = "050")]
    KuldīgaMunicipality,
    #[serde(rename = "044")]
    KārsavaMunicipality,
    #[serde(rename = "053")]
    LielvārdeMunicipality,
    #[serde(rename = "LPX")]
    Liepāja,
    #[serde(rename = "054")]
    LimbažiMunicipality,
    #[serde(rename = "057")]
    LubānaMunicipality,
    #[serde(rename = "058")]
    LudzaMunicipality,
    #[serde(rename = "055")]
    LīgatneMunicipality,
    #[serde(rename = "056")]
    LīvāniMunicipality,
    #[serde(rename = "059")]
    MadonaMunicipality,
    #[serde(rename = "060")]
    MazsalacaMunicipality,
    #[serde(rename = "061")]
    MālpilsMunicipality,
    #[serde(rename = "062")]
    MārupeMunicipality,
    #[serde(rename = "063")]
    MērsragsMunicipality,
    #[serde(rename = "064")]
    NaukšēniMunicipality,
    #[serde(rename = "065")]
    NeretaMunicipality,
    #[serde(rename = "066")]
    NīcaMunicipality,
    #[serde(rename = "067")]
    OgreMunicipality,
    #[serde(rename = "068")]
    OlaineMunicipality,
    #[serde(rename = "069")]
    OzolniekiMunicipality,
    #[serde(rename = "073")]
    PreiļiMunicipality,
    #[serde(rename = "074")]
    PriekuleMunicipality,
    #[serde(rename = "075")]
    PriekuļiMunicipality,
    #[serde(rename = "070")]
    PārgaujaMunicipality,
    #[serde(rename = "071")]
    PāvilostaMunicipality,
    #[serde(rename = "072")]
    PļaviņasMunicipality,
    #[serde(rename = "076")]
    RaunaMunicipality,
    #[serde(rename = "078")]
    RiebiņiMunicipality,
    #[serde(rename = "RIX")]
    Riga,
    #[serde(rename = "079")]
    RojaMunicipality,
    #[serde(rename = "080")]
    RopažiMunicipality,
    #[serde(rename = "081")]
    RucavaMunicipality,
    #[serde(rename = "082")]
    RugājiMunicipality,
    #[serde(rename = "083")]
    RundāleMunicipality,
    #[serde(rename = "REZ")]
    Rēzekne,
    #[serde(rename = "077")]
    RēzekneMunicipality,
    #[serde(rename = "084")]
    RūjienaMunicipality,
    #[serde(rename = "085")]
    SalaMunicipality,
    #[serde(rename = "086")]
    SalacgrīvaMunicipality,
    #[serde(rename = "087")]
    SalaspilsMunicipality,
    #[serde(rename = "088")]
    SaldusMunicipality,
    #[serde(rename = "089")]
    SaulkrastiMunicipality,
    #[serde(rename = "091")]
    SiguldaMunicipality,
    #[serde(rename = "093")]
    SkrundaMunicipality,
    #[serde(rename = "092")]
    SkrīveriMunicipality,
    #[serde(rename = "094")]
    SmilteneMunicipality,
    #[serde(rename = "095")]
    StopiņiMunicipality,
    #[serde(rename = "096")]
    StrenčiMunicipality,
    #[serde(rename = "090")]
    SējaMunicipality,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum ItalyStatesAbbreviation {
    #[serde(rename = "65")]
    Abruzzo,
    #[serde(rename = "23")]
    AostaValley,
    #[serde(rename = "75")]
    Apulia,
    #[serde(rename = "77")]
    Basilicata,
    #[serde(rename = "BN")]
    BeneventoProvince,
    #[serde(rename = "78")]
    Calabria,
    #[serde(rename = "72")]
    Campania,
    #[serde(rename = "45")]
    EmiliaRomagna,
    #[serde(rename = "36")]
    FriuliVeneziaGiulia,
    #[serde(rename = "62")]
    Lazio,
    #[serde(rename = "42")]
    Liguria,
    #[serde(rename = "25")]
    Lombardy,
    #[serde(rename = "57")]
    Marche,
    #[serde(rename = "67")]
    Molise,
    #[serde(rename = "21")]
    Piedmont,
    #[serde(rename = "88")]
    Sardinia,
    #[serde(rename = "82")]
    Sicily,
    #[serde(rename = "32")]
    TrentinoSouthTyrol,
    #[serde(rename = "52")]
    Tuscany,
    #[serde(rename = "55")]
    Umbria,
    #[serde(rename = "34")]
    Veneto,
    #[serde(rename = "AG")]
    Agrigento,
    #[serde(rename = "CL")]
    Caltanissetta,
    #[serde(rename = "EN")]
    Enna,
    #[serde(rename = "RG")]
    Ragusa,
    #[serde(rename = "SR")]
    Siracusa,
    #[serde(rename = "TP")]
    Trapani,
    #[serde(rename = "BA")]
    Bari,
    #[serde(rename = "BO")]
    Bologna,
    #[serde(rename = "CA")]
    Cagliari,
    #[serde(rename = "CT")]
    Catania,
    #[serde(rename = "FI")]
    Florence,
    #[serde(rename = "GE")]
    Genoa,
    #[serde(rename = "ME")]
    Messina,
    #[serde(rename = "MI")]
    Milan,
    #[serde(rename = "NA")]
    Naples,
    #[serde(rename = "PA")]
    Palermo,
    #[serde(rename = "RC")]
    ReggioCalabria,
    #[serde(rename = "RM")]
    Rome,
    #[serde(rename = "TO")]
    Turin,
    #[serde(rename = "VE")]
    Venice,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum LiechtensteinStatesAbbreviation {
    #[serde(rename = "01")]
    Balzers,
    #[serde(rename = "02")]
    Eschen,
    #[serde(rename = "03")]
    Gamprin,
    #[serde(rename = "04")]
    Mauren,
    #[serde(rename = "05")]
    Planken,
    #[serde(rename = "06")]
    Ruggell,
    #[serde(rename = "07")]
    Schaan,
    #[serde(rename = "08")]
    Schellenberg,
    #[serde(rename = "09")]
    Triesen,
    #[serde(rename = "10")]
    Triesenberg,
    #[serde(rename = "11")]
    Vaduz,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum LithuaniaStatesAbbreviation {
    #[serde(rename = "01")]
    AkmeneDistrictMunicipality,
    #[serde(rename = "02")]
    AlytusCityMunicipality,
    #[serde(rename = "AL")]
    AlytusCounty,
    #[serde(rename = "03")]
    AlytusDistrictMunicipality,
    #[serde(rename = "05")]
    BirstonasMunicipality,
    #[serde(rename = "06")]
    BirzaiDistrictMunicipality,
    #[serde(rename = "07")]
    DruskininkaiMunicipality,
    #[serde(rename = "08")]
    ElektrenaiMunicipality,
    #[serde(rename = "09")]
    IgnalinaDistrictMunicipality,
    #[serde(rename = "10")]
    JonavaDistrictMunicipality,
    #[serde(rename = "11")]
    JoniskisDistrictMunicipality,
    #[serde(rename = "12")]
    JurbarkasDistrictMunicipality,
    #[serde(rename = "13")]
    KaisiadorysDistrictMunicipality,
    #[serde(rename = "14")]
    KalvarijaMunicipality,
    #[serde(rename = "15")]
    KaunasCityMunicipality,
    #[serde(rename = "KU")]
    KaunasCounty,
    #[serde(rename = "16")]
    KaunasDistrictMunicipality,
    #[serde(rename = "17")]
    KazluRudaMunicipality,
    #[serde(rename = "19")]
    KelmeDistrictMunicipality,
    #[serde(rename = "20")]
    KlaipedaCityMunicipality,
    #[serde(rename = "KL")]
    KlaipedaCounty,
    #[serde(rename = "21")]
    KlaipedaDistrictMunicipality,
    #[serde(rename = "22")]
    KretingaDistrictMunicipality,
    #[serde(rename = "23")]
    KupiskisDistrictMunicipality,
    #[serde(rename = "18")]
    KedainiaiDistrictMunicipality,
    #[serde(rename = "24")]
    LazdijaiDistrictMunicipality,
    #[serde(rename = "MR")]
    MarijampoleCounty,
    #[serde(rename = "25")]
    MarijampoleMunicipality,
    #[serde(rename = "26")]
    MazeikiaiDistrictMunicipality,
    #[serde(rename = "27")]
    MoletaiDistrictMunicipality,
    #[serde(rename = "28")]
    NeringaMunicipality,
    #[serde(rename = "29")]
    PagegiaiMunicipality,
    #[serde(rename = "30")]
    PakruojisDistrictMunicipality,
    #[serde(rename = "31")]
    PalangaCityMunicipality,
    #[serde(rename = "32")]
    PanevezysCityMunicipality,
    #[serde(rename = "PN")]
    PanevezysCounty,
    #[serde(rename = "33")]
    PanevezysDistrictMunicipality,
    #[serde(rename = "34")]
    PasvalysDistrictMunicipality,
    #[serde(rename = "35")]
    PlungeDistrictMunicipality,
    #[serde(rename = "36")]
    PrienaiDistrictMunicipality,
    #[serde(rename = "37")]
    RadviliskisDistrictMunicipality,
    #[serde(rename = "38")]
    RaseiniaiDistrictMunicipality,
    #[serde(rename = "39")]
    RietavasMunicipality,
    #[serde(rename = "40")]
    RokiskisDistrictMunicipality,
    #[serde(rename = "48")]
    SkuodasDistrictMunicipality,
    #[serde(rename = "TA")]
    TaurageCounty,
    #[serde(rename = "50")]
    TaurageDistrictMunicipality,
    #[serde(rename = "TE")]
    TelsiaiCounty,
    #[serde(rename = "51")]
    TelsiaiDistrictMunicipality,
    #[serde(rename = "52")]
    TrakaiDistrictMunicipality,
    #[serde(rename = "53")]
    UkmergeDistrictMunicipality,
    #[serde(rename = "UT")]
    UtenaCounty,
    #[serde(rename = "54")]
    UtenaDistrictMunicipality,
    #[serde(rename = "55")]
    VarenaDistrictMunicipality,
    #[serde(rename = "56")]
    VilkaviskisDistrictMunicipality,
    #[serde(rename = "57")]
    VilniusCityMunicipality,
    #[serde(rename = "VL")]
    VilniusCounty,
    #[serde(rename = "58")]
    VilniusDistrictMunicipality,
    #[serde(rename = "59")]
    VisaginasMunicipality,
    #[serde(rename = "60")]
    ZarasaiDistrictMunicipality,
    #[serde(rename = "41")]
    SakiaiDistrictMunicipality,
    #[serde(rename = "42")]
    SalcininkaiDistrictMunicipality,
    #[serde(rename = "43")]
    SiauliaiCityMunicipality,
    #[serde(rename = "SA")]
    SiauliaiCounty,
    #[serde(rename = "44")]
    SiauliaiDistrictMunicipality,
    #[serde(rename = "45")]
    SilaleDistrictMunicipality,
    #[serde(rename = "46")]
    SiluteDistrictMunicipality,
    #[serde(rename = "47")]
    SirvintosDistrictMunicipality,
    #[serde(rename = "49")]
    SvencionysDistrictMunicipality,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum MaltaStatesAbbreviation {
    #[serde(rename = "01")]
    Attard,
    #[serde(rename = "02")]
    Balzan,
    #[serde(rename = "03")]
    Birgu,
    #[serde(rename = "04")]
    Birkirkara,
    #[serde(rename = "05")]
    Birzebbuga,
    #[serde(rename = "06")]
    Cospicua,
    #[serde(rename = "07")]
    Dingli,
    #[serde(rename = "08")]
    Fgura,
    #[serde(rename = "09")]
    Floriana,
    #[serde(rename = "10")]
    Fontana,
    #[serde(rename = "11")]
    Gudja,
    #[serde(rename = "12")]
    Gzira,
    #[serde(rename = "13")]
    Ghajnsielem,
    #[serde(rename = "14")]
    Gharb,
    #[serde(rename = "15")]
    Gharghur,
    #[serde(rename = "16")]
    Ghasri,
    #[serde(rename = "17")]
    Ghaxaq,
    #[serde(rename = "18")]
    Hamrun,
    #[serde(rename = "19")]
    Iklin,
    #[serde(rename = "20")]
    Senglea,
    #[serde(rename = "21")]
    Kalkara,
    #[serde(rename = "22")]
    Kercem,
    #[serde(rename = "23")]
    Kirkop,
    #[serde(rename = "24")]
    Lija,
    #[serde(rename = "25")]
    Luqa,
    #[serde(rename = "26")]
    Marsa,
    #[serde(rename = "27")]
    Marsaskala,
    #[serde(rename = "28")]
    Marsaxlokk,
    #[serde(rename = "29")]
    Mdina,
    #[serde(rename = "30")]
    Mellieha,
    #[serde(rename = "31")]
    Mgarr,
    #[serde(rename = "32")]
    Mosta,
    #[serde(rename = "33")]
    Mqabba,
    #[serde(rename = "34")]
    Msida,
    #[serde(rename = "35")]
    Mtarfa,
    #[serde(rename = "36")]
    Munxar,
    #[serde(rename = "37")]
    Nadur,
    #[serde(rename = "38")]
    Naxxar,
    #[serde(rename = "39")]
    Paola,
    #[serde(rename = "40")]
    Pembroke,
    #[serde(rename = "41")]
    Pieta,
    #[serde(rename = "42")]
    Qala,
    #[serde(rename = "43")]
    Qormi,
    #[serde(rename = "44")]
    Qrendi,
    #[serde(rename = "45")]
    Victoria,
    #[serde(rename = "46")]
    Rabat,
    #[serde(rename = "48")]
    StJulians,
    #[serde(rename = "49")]
    SanGwann,
    #[serde(rename = "50")]
    SaintLawrence,
    #[serde(rename = "51")]
    StPaulsBay,
    #[serde(rename = "52")]
    Sannat,
    #[serde(rename = "53")]
    SantaLucija,
    #[serde(rename = "54")]
    SantaVenera,
    #[serde(rename = "55")]
    Siggiewi,
    #[serde(rename = "56")]
    Sliema,
    #[serde(rename = "57")]
    Swieqi,
    #[serde(rename = "58")]
    TaXbiex,
    #[serde(rename = "59")]
    Tarxien,
    #[serde(rename = "60")]
    Valletta,
    #[serde(rename = "61")]
    Xaghra,
    #[serde(rename = "62")]
    Xewkija,
    #[serde(rename = "63")]
    Xghajra,
    #[serde(rename = "64")]
    Zabbar,
    #[serde(rename = "65")]
    ZebbugGozo,
    #[serde(rename = "66")]
    ZebbugMalta,
    #[serde(rename = "67")]
    Zejtun,
    #[serde(rename = "68")]
    Zurrieq,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum MoldovaStatesAbbreviation {
    #[serde(rename = "AN")]
    AneniiNoiDistrict,
    #[serde(rename = "BS")]
    BasarabeascaDistrict,
    #[serde(rename = "BD")]
    BenderMunicipality,
    #[serde(rename = "BR")]
    BriceniDistrict,
    #[serde(rename = "BA")]
    BaltiMunicipality,
    #[serde(rename = "CA")]
    CahulDistrict,
    #[serde(rename = "CT")]
    CantemirDistrict,
    #[serde(rename = "CU")]
    ChisinauMunicipality,
    #[serde(rename = "CM")]
    CimisliaDistrict,
    #[serde(rename = "CR")]
    CriuleniDistrict,
    #[serde(rename = "CL")]
    CalarasiDistrict,
    #[serde(rename = "CS")]
    CauseniDistrict,
    #[serde(rename = "DO")]
    DonduseniDistrict,
    #[serde(rename = "DR")]
    DrochiaDistrict,
    #[serde(rename = "DU")]
    DubasariDistrict,
    #[serde(rename = "ED")]
    EdinetDistrict,
    #[serde(rename = "FL")]
    FlorestiDistrict,
    #[serde(rename = "FA")]
    FalestiDistrict,
    #[serde(rename = "GA")]
    Gagauzia,
    #[serde(rename = "GL")]
    GlodeniDistrict,
    #[serde(rename = "HI")]
    HincestiDistrict,
    #[serde(rename = "IA")]
    IaloveniDistrict,
    #[serde(rename = "NI")]
    NisporeniDistrict,
    #[serde(rename = "OC")]
    OcnitaDistrict,
    #[serde(rename = "OR")]
    OrheiDistrict,
    #[serde(rename = "RE")]
    RezinaDistrict,
    #[serde(rename = "RI")]
    RiscaniDistrict,
    #[serde(rename = "SO")]
    SorocaDistrict,
    #[serde(rename = "ST")]
    StraseniDistrict,
    #[serde(rename = "SI")]
    SingereiDistrict,
    #[serde(rename = "TA")]
    TaracliaDistrict,
    #[serde(rename = "TE")]
    TelenestiDistrict,
    #[serde(rename = "SN")]
    TransnistriaAutonomousTerritorialUnit,
    #[serde(rename = "UN")]
    UngheniDistrict,
    #[serde(rename = "SD")]
    SoldanestiDistrict,
    #[serde(rename = "SV")]
    StefanVodaDistrict,
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
    #[serde(rename = "01")]
    AndrijevicaMunicipality,
    #[serde(rename = "02")]
    BarMunicipality,
    #[serde(rename = "03")]
    BeraneMunicipality,
    #[serde(rename = "04")]
    BijeloPoljeMunicipality,
    #[serde(rename = "05")]
    BudvaMunicipality,
    #[serde(rename = "07")]
    DanilovgradMunicipality,
    #[serde(rename = "22")]
    GusinjeMunicipality,
    #[serde(rename = "09")]
    KolasinMunicipality,
    #[serde(rename = "10")]
    KotorMunicipality,
    #[serde(rename = "11")]
    MojkovacMunicipality,
    #[serde(rename = "12")]
    NiksicMunicipality,
    #[serde(rename = "06")]
    OldRoyalCapitalCetinje,
    #[serde(rename = "23")]
    PetnjicaMunicipality,
    #[serde(rename = "13")]
    PlavMunicipality,
    #[serde(rename = "14")]
    PljevljaMunicipality,
    #[serde(rename = "15")]
    PluzineMunicipality,
    #[serde(rename = "16")]
    PodgoricaMunicipality,
    #[serde(rename = "17")]
    RozajeMunicipality,
    #[serde(rename = "19")]
    TivatMunicipality,
    #[serde(rename = "20")]
    UlcinjMunicipality,
    #[serde(rename = "18")]
    SavnikMunicipality,
    #[serde(rename = "21")]
    ZabljakMunicipality,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum NetherlandsStatesAbbreviation {
    #[serde(rename = "BQ1")]
    Bonaire,
    #[serde(rename = "DR")]
    Drenthe,
    #[serde(rename = "FL")]
    Flevoland,
    #[serde(rename = "FR")]
    Friesland,
    #[serde(rename = "GE")]
    Gelderland,
    #[serde(rename = "GR")]
    Groningen,
    #[serde(rename = "LI")]
    Limburg,
    #[serde(rename = "NB")]
    NorthBrabant,
    #[serde(rename = "NH")]
    NorthHolland,
    #[serde(rename = "OV")]
    Overijssel,
    #[serde(rename = "BQ2")]
    Saba,
    #[serde(rename = "BQ3")]
    SintEustatius,
    #[serde(rename = "ZH")]
    SouthHolland,
    #[serde(rename = "UT")]
    Utrecht,
    #[serde(rename = "ZE")]
    Zeeland,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum NorthMacedoniaStatesAbbreviation {
    #[serde(rename = "01")]
    AerodromMunicipality,
    #[serde(rename = "02")]
    AracinovoMunicipality,
    #[serde(rename = "03")]
    BerovoMunicipality,
    #[serde(rename = "04")]
    BitolaMunicipality,
    #[serde(rename = "05")]
    BogdanciMunicipality,
    #[serde(rename = "06")]
    BogovinjeMunicipality,
    #[serde(rename = "07")]
    BosilovoMunicipality,
    #[serde(rename = "08")]
    BrvenicaMunicipality,
    #[serde(rename = "09")]
    ButelMunicipality,
    #[serde(rename = "77")]
    CentarMunicipality,
    #[serde(rename = "78")]
    CentarZupaMunicipality,
    #[serde(rename = "22")]
    DebarcaMunicipality,
    #[serde(rename = "23")]
    DelcevoMunicipality,
    #[serde(rename = "25")]
    DemirHisarMunicipality,
    #[serde(rename = "24")]
    DemirKapijaMunicipality,
    #[serde(rename = "26")]
    DojranMunicipality,
    #[serde(rename = "27")]
    DolneniMunicipality,
    #[serde(rename = "28")]
    DrugovoMunicipality,
    #[serde(rename = "17")]
    GaziBabaMunicipality,
    #[serde(rename = "18")]
    GevgelijaMunicipality,
    #[serde(rename = "29")]
    GjorcePetrovMunicipality,
    #[serde(rename = "19")]
    GostivarMunicipality,
    #[serde(rename = "20")]
    GradskoMunicipality,
    #[serde(rename = "85")]
    GreaterSkopje,
    #[serde(rename = "34")]
    IlindenMunicipality,
    #[serde(rename = "35")]
    JegunovceMunicipality,
    #[serde(rename = "37")]
    Karbinci,
    #[serde(rename = "38")]
    KarposMunicipality,
    #[serde(rename = "36")]
    KavadarciMunicipality,
    #[serde(rename = "39")]
    KiselaVodaMunicipality,
    #[serde(rename = "40")]
    KicevoMunicipality,
    #[serde(rename = "41")]
    KonceMunicipality,
    #[serde(rename = "42")]
    KocaniMunicipality,
    #[serde(rename = "43")]
    KratovoMunicipality,
    #[serde(rename = "44")]
    KrivaPalankaMunicipality,
    #[serde(rename = "45")]
    KrivogastaniMunicipality,
    #[serde(rename = "46")]
    KrusevoMunicipality,
    #[serde(rename = "47")]
    KumanovoMunicipality,
    #[serde(rename = "48")]
    LipkovoMunicipality,
    #[serde(rename = "49")]
    LozovoMunicipality,
    #[serde(rename = "51")]
    MakedonskaKamenicaMunicipality,
    #[serde(rename = "52")]
    MakedonskiBrodMunicipality,
    #[serde(rename = "50")]
    MavrovoAndRostusaMunicipality,
    #[serde(rename = "53")]
    MogilaMunicipality,
    #[serde(rename = "54")]
    NegotinoMunicipality,
    #[serde(rename = "55")]
    NovaciMunicipality,
    #[serde(rename = "56")]
    NovoSeloMunicipality,
    #[serde(rename = "58")]
    OhridMunicipality,
    #[serde(rename = "57")]
    OslomejMunicipality,
    #[serde(rename = "60")]
    PehcevoMunicipality,
    #[serde(rename = "59")]
    PetrovecMunicipality,
    #[serde(rename = "61")]
    PlasnicaMunicipality,
    #[serde(rename = "62")]
    PrilepMunicipality,
    #[serde(rename = "63")]
    ProbistipMunicipality,
    #[serde(rename = "64")]
    RadovisMunicipality,
    #[serde(rename = "65")]
    RankovceMunicipality,
    #[serde(rename = "66")]
    ResenMunicipality,
    #[serde(rename = "67")]
    RosomanMunicipality,
    #[serde(rename = "68")]
    SarajMunicipality,
    #[serde(rename = "70")]
    SopisteMunicipality,
    #[serde(rename = "71")]
    StaroNagoricaneMunicipality,
    #[serde(rename = "72")]
    StrugaMunicipality,
    #[serde(rename = "73")]
    StrumicaMunicipality,
    #[serde(rename = "74")]
    StudenicaniMunicipality,
    #[serde(rename = "69")]
    SvetiNikoleMunicipality,
    #[serde(rename = "75")]
    TearceMunicipality,
    #[serde(rename = "76")]
    TetovoMunicipality,
    #[serde(rename = "10")]
    ValandovoMunicipality,
    #[serde(rename = "11")]
    VasilevoMunicipality,
    #[serde(rename = "13")]
    VelesMunicipality,
    #[serde(rename = "12")]
    VevcaniMunicipality,
    #[serde(rename = "14")]
    VinicaMunicipality,
    #[serde(rename = "15")]
    VranesticaMunicipality,
    #[serde(rename = "16")]
    VrapcisteMunicipality,
    #[serde(rename = "31")]
    ZajasMunicipality,
    #[serde(rename = "32")]
    ZelenikovoMunicipality,
    #[serde(rename = "33")]
    ZrnovciMunicipality,
    #[serde(rename = "79")]
    CairMunicipality,
    #[serde(rename = "80")]
    CaskaMunicipality,
    #[serde(rename = "81")]
    CesinovoOblesevoMunicipality,
    #[serde(rename = "82")]
    CucerSandevoMunicipality,
    #[serde(rename = "83")]
    StipMunicipality,
    #[serde(rename = "84")]
    SutoOrizariMunicipality,
    #[serde(rename = "30")]
    ZelinoMunicipality,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum NorwayStatesAbbreviation {
    #[serde(rename = "02")]
    Akershus,
    #[serde(rename = "06")]
    Buskerud,
    #[serde(rename = "20")]
    Finnmark,
    #[serde(rename = "04")]
    Hedmark,
    #[serde(rename = "12")]
    Hordaland,
    #[serde(rename = "22")]
    JanMayen,
    #[serde(rename = "15")]
    MoreOgRomsdal,
    #[serde(rename = "17")]
    NordTrondelag,
    #[serde(rename = "18")]
    Nordland,
    #[serde(rename = "05")]
    Oppland,
    #[serde(rename = "03")]
    Oslo,
    #[serde(rename = "11")]
    Rogaland,
    #[serde(rename = "14")]
    SognOgFjordane,
    #[serde(rename = "21")]
    Svalbard,
    #[serde(rename = "16")]
    SorTrondelag,
    #[serde(rename = "08")]
    Telemark,
    #[serde(rename = "19")]
    Troms,
    #[serde(rename = "50")]
    Trondelag,
    #[serde(rename = "10")]
    VestAgder,
    #[serde(rename = "07")]
    Vestfold,
    #[serde(rename = "01")]
    Ostfold,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum PolandStatesAbbreviation {
    #[serde(rename = "WP")]
    GreaterPolandVoivodeship,
    #[serde(rename = "KI")]
    Kielce,
    #[serde(rename = "KP")]
    KuyavianPomeranianVoivodeship,
    #[serde(rename = "MA")]
    LesserPolandVoivodeship,
    #[serde(rename = "DS")]
    LowerSilesianVoivodeship,
    #[serde(rename = "LU")]
    LublinVoivodeship,
    #[serde(rename = "LB")]
    LubuszVoivodeship,
    #[serde(rename = "MZ")]
    MasovianVoivodeship,
    #[serde(rename = "OP")]
    OpoleVoivodeship,
    #[serde(rename = "PK")]
    PodkarpackieVoivodeship,
    #[serde(rename = "PD")]
    PodlaskieVoivodeship,
    #[serde(rename = "PM")]
    PomeranianVoivodeship,
    #[serde(rename = "SL")]
    SilesianVoivodeship,
    #[serde(rename = "WN")]
    WarmianMasurianVoivodeship,
    #[serde(rename = "ZP")]
    WestPomeranianVoivodeship,
    #[serde(rename = "LD")]
    LodzVoivodeship,
    #[serde(rename = "SK")]
    SwietokrzyskieVoivodeship,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum PortugalStatesAbbreviation {
    #[serde(rename = "01")]
    AveiroDistrict,
    #[serde(rename = "20")]
    Azores,
    #[serde(rename = "02")]
    BejaDistrict,
    #[serde(rename = "03")]
    BragaDistrict,
    #[serde(rename = "04")]
    BragancaDistrict,
    #[serde(rename = "05")]
    CasteloBrancoDistrict,
    #[serde(rename = "06")]
    CoimbraDistrict,
    #[serde(rename = "08")]
    FaroDistrict,
    #[serde(rename = "09")]
    GuardaDistrict,
    #[serde(rename = "10")]
    LeiriaDistrict,
    #[serde(rename = "11")]
    LisbonDistrict,
    #[serde(rename = "30")]
    Madeira,
    #[serde(rename = "12")]
    PortalegreDistrict,
    #[serde(rename = "13")]
    PortoDistrict,
    #[serde(rename = "14")]
    SantaremDistrict,
    #[serde(rename = "15")]
    SetubalDistrict,
    #[serde(rename = "16")]
    VianaDoCasteloDistrict,
    #[serde(rename = "17")]
    VilaRealDistrict,
    #[serde(rename = "18")]
    ViseuDistrict,
    #[serde(rename = "07")]
    EvoraDistrict,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum SpainStatesAbbreviation {
    #[serde(rename = "C")]
    ACorunaProvince,
    #[serde(rename = "AB")]
    AlbaceteProvince,
    #[serde(rename = "A")]
    AlicanteProvince,
    #[serde(rename = "AL")]
    AlmeriaProvince,
    #[serde(rename = "AN")]
    Andalusia,
    #[serde(rename = "VI")]
    ArabaAlava,
    #[serde(rename = "AR")]
    Aragon,
    #[serde(rename = "BA")]
    BadajozProvince,
    #[serde(rename = "PM")]
    BalearicIslands,
    #[serde(rename = "B")]
    BarcelonaProvince,
    #[serde(rename = "PV")]
    BasqueCountry,
    #[serde(rename = "BI")]
    Biscay,
    #[serde(rename = "BU")]
    BurgosProvince,
    #[serde(rename = "CN")]
    CanaryIslands,
    #[serde(rename = "S")]
    Cantabria,
    #[serde(rename = "CS")]
    CastellonProvince,
    #[serde(rename = "CL")]
    CastileAndLeon,
    #[serde(rename = "CM")]
    CastileLaMancha,
    #[serde(rename = "CT")]
    Catalonia,
    #[serde(rename = "CE")]
    Ceuta,
    #[serde(rename = "CR")]
    CiudadRealProvince,
    #[serde(rename = "MD")]
    CommunityOfMadrid,
    #[serde(rename = "CU")]
    CuencaProvince,
    #[serde(rename = "CC")]
    CaceresProvince,
    #[serde(rename = "CA")]
    CadizProvince,
    #[serde(rename = "CO")]
    CordobaProvince,
    #[serde(rename = "EX")]
    Extremadura,
    #[serde(rename = "GA")]
    Galicia,
    #[serde(rename = "SS")]
    Gipuzkoa,
    #[serde(rename = "GI")]
    GironaProvince,
    #[serde(rename = "GR")]
    GranadaProvince,
    #[serde(rename = "GU")]
    GuadalajaraProvince,
    #[serde(rename = "H")]
    HuelvaProvince,
    #[serde(rename = "HU")]
    HuescaProvince,
    #[serde(rename = "J")]
    JaenProvince,
    #[serde(rename = "RI")]
    LaRioja,
    #[serde(rename = "GC")]
    LasPalmasProvince,
    #[serde(rename = "LE")]
    LeonProvince,
    #[serde(rename = "L")]
    LleidaProvince,
    #[serde(rename = "LU")]
    LugoProvince,
    #[serde(rename = "M")]
    MadridProvince,
    #[serde(rename = "ML")]
    Melilla,
    #[serde(rename = "MU")]
    MurciaProvince,
    #[serde(rename = "MA")]
    MalagaProvince,
    #[serde(rename = "NC")]
    Navarre,
    #[serde(rename = "OR")]
    OurenseProvince,
    #[serde(rename = "P")]
    PalenciaProvince,
    #[serde(rename = "PO")]
    PontevedraProvince,
    #[serde(rename = "O")]
    ProvinceOfAsturias,
    #[serde(rename = "AV")]
    ProvinceOfAvila,
    #[serde(rename = "MC")]
    RegionOfMurcia,
    #[serde(rename = "SA")]
    SalamancaProvince,
    #[serde(rename = "TF")]
    SantaCruzDeTenerifeProvince,
    #[serde(rename = "SG")]
    SegoviaProvince,
    #[serde(rename = "SE")]
    SevilleProvince,
    #[serde(rename = "SO")]
    SoriaProvince,
    #[serde(rename = "T")]
    TarragonaProvince,
    #[serde(rename = "TE")]
    TeruelProvince,
    #[serde(rename = "TO")]
    ToledoProvince,
    #[serde(rename = "V")]
    ValenciaProvince,
    #[serde(rename = "VC")]
    ValencianCommunity,
    #[serde(rename = "VA")]
    ValladolidProvince,
    #[serde(rename = "ZA")]
    ZamoraProvince,
    #[serde(rename = "Z")]
    ZaragozaProvince,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum SwitzerlandStatesAbbreviation {
    #[serde(rename = "AG")]
    Aargau,
    #[serde(rename = "AR")]
    AppenzellAusserrhoden,
    #[serde(rename = "AI")]
    AppenzellInnerrhoden,
    #[serde(rename = "BL")]
    BaselLandschaft,
    #[serde(rename = "FR")]
    CantonOfFribourg,
    #[serde(rename = "GE")]
    CantonOfGeneva,
    #[serde(rename = "JU")]
    CantonOfJura,
    #[serde(rename = "LU")]
    CantonOfLucerne,
    #[serde(rename = "NE")]
    CantonOfNeuchatel,
    #[serde(rename = "SH")]
    CantonOfSchaffhausen,
    #[serde(rename = "SO")]
    CantonOfSolothurn,
    #[serde(rename = "SG")]
    CantonOfStGallen,
    #[serde(rename = "VS")]
    CantonOfValais,
    #[serde(rename = "VD")]
    CantonOfVaud,
    #[serde(rename = "ZG")]
    CantonOfZug,
    #[serde(rename = "GL")]
    Glarus,
    #[serde(rename = "GR")]
    Graubunden,
    #[serde(rename = "NW")]
    Nidwalden,
    #[serde(rename = "OW")]
    Obwalden,
    #[serde(rename = "SZ")]
    Schwyz,
    #[serde(rename = "TG")]
    Thurgau,
    #[serde(rename = "TI")]
    Ticino,
    #[serde(rename = "UR")]
    Uri,
    #[serde(rename = "BE")]
    CantonOfBern,
    #[serde(rename = "ZH")]
    CantonOfZurich,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum UnitedKingdomStatesAbbreviation {
    #[serde(rename = "ABE")]
    AberdeenCity,
    #[serde(rename = "ABD")]
    Aberdeenshire,
    #[serde(rename = "ANS")]
    Angus,
    #[serde(rename = "ANN")]
    AntrimAndNewtownabbey,
    #[serde(rename = "AND")]
    ArdsAndNorthDown,
    #[serde(rename = "AGB")]
    ArgyllAndBute,
    #[serde(rename = "ABC")]
    ArmaghCityBanbridgeAndCraigavon,
    #[serde(rename = "BDG")]
    BarkingAndDagenham,
    #[serde(rename = "BNE")]
    Barnet,
    #[serde(rename = "BNS")]
    Barnsley,
    #[serde(rename = "BAS")]
    BathAndNorthEastSomerset,
    #[serde(rename = "BDF")]
    Bedford,
    #[serde(rename = "BFS")]
    BelfastCity,
    #[serde(rename = "BEX")]
    Bexley,
    #[serde(rename = "BIR")]
    Birmingham,
    #[serde(rename = "BBD")]
    BlackburnWithDarwen,
    #[serde(rename = "BPL")]
    Blackpool,
    #[serde(rename = "BGW")]
    BlaenauGwent,
    #[serde(rename = "BOL")]
    Bolton,
    #[serde(rename = "BCP")]
    BournemouthChristchurchAndPoole,
    #[serde(rename = "BRC")]
    BracknellForest,
    #[serde(rename = "BRD")]
    Bradford,
    #[serde(rename = "BEN")]
    Brent,
    #[serde(rename = "BGE")]
    Bridgend,
    #[serde(rename = "BNH")]
    BrightonAndHove,
    #[serde(rename = "BST")]
    BristolCityOf,
    #[serde(rename = "BRY")]
    Bromley,
    #[serde(rename = "BKM")]
    Buckinghamshire,
    #[serde(rename = "BUR")]
    Bury,
    #[serde(rename = "CAY")]
    Caerphilly,
    #[serde(rename = "CLD")]
    Calderdale,
    #[serde(rename = "CAM")]
    Cambridgeshire,
    #[serde(rename = "CMD")]
    Camden,
    #[serde(rename = "CRF")]
    Cardiff,
    #[serde(rename = "CMN")]
    Carmarthenshire,
    #[serde(rename = "CCG")]
    CausewayCoastAndGlens,
    #[serde(rename = "CBF")]
    CentralBedfordshire,
    #[serde(rename = "CGN")]
    Ceredigion,
    #[serde(rename = "CHE")]
    CheshireEast,
    #[serde(rename = "CHW")]
    CheshireWestAndChester,
    #[serde(rename = "CLK")]
    Clackmannanshire,
    #[serde(rename = "CWY")]
    Conwy,
    #[serde(rename = "CON")]
    Cornwall,
    #[serde(rename = "COV")]
    Coventry,
    #[serde(rename = "CRY")]
    Croydon,
    #[serde(rename = "CMA")]
    Cumbria,
    #[serde(rename = "DAL")]
    Darlington,
    #[serde(rename = "DEN")]
    Denbighshire,
    #[serde(rename = "DER")]
    Derby,
    #[serde(rename = "DBY")]
    Derbyshire,
    #[serde(rename = "DRS")]
    DerryAndStrabane,
    #[serde(rename = "DEV")]
    Devon,
    #[serde(rename = "DNC")]
    Doncaster,
    #[serde(rename = "DOR")]
    Dorset,
    #[serde(rename = "DUD")]
    Dudley,
    #[serde(rename = "DGY")]
    DumfriesAndGalloway,
    #[serde(rename = "DND")]
    DundeeCity,
    #[serde(rename = "DUR")]
    DurhamCounty,
    #[serde(rename = "EAL")]
    Ealing,
    #[serde(rename = "EAY")]
    EastAyrshire,
    #[serde(rename = "EDU")]
    EastDunbartonshire,
    #[serde(rename = "ELN")]
    EastLothian,
    #[serde(rename = "ERW")]
    EastRenfrewshire,
    #[serde(rename = "ERY")]
    EastRidingOfYorkshire,
    #[serde(rename = "ESX")]
    EastSussex,
    #[serde(rename = "EDH")]
    EdinburghCityOf,
    #[serde(rename = "ELS")]
    EileanSiar,
    #[serde(rename = "ENF")]
    Enfield,
    #[serde(rename = "ESS")]
    Essex,
    #[serde(rename = "FAL")]
    Falkirk,
    #[serde(rename = "FMO")]
    FermanaghAndOmagh,
    #[serde(rename = "FIF")]
    Fife,
    #[serde(rename = "FLN")]
    Flintshire,
    #[serde(rename = "GAT")]
    Gateshead,
    #[serde(rename = "GLG")]
    GlasgowCity,
    #[serde(rename = "GLS")]
    Gloucestershire,
    #[serde(rename = "GRE")]
    Greenwich,
    #[serde(rename = "GWN")]
    Gwynedd,
    #[serde(rename = "HCK")]
    Hackney,
    #[serde(rename = "HAL")]
    Halton,
    #[serde(rename = "HMF")]
    HammersmithAndFulham,
    #[serde(rename = "HAM")]
    Hampshire,
    #[serde(rename = "HRY")]
    Haringey,
    #[serde(rename = "HRW")]
    Harrow,
    #[serde(rename = "HPL")]
    Hartlepool,
    #[serde(rename = "HAV")]
    Havering,
    #[serde(rename = "HEF")]
    Herefordshire,
    #[serde(rename = "HRT")]
    Hertfordshire,
    #[serde(rename = "HLD")]
    Highland,
    #[serde(rename = "HIL")]
    Hillingdon,
    #[serde(rename = "HNS")]
    Hounslow,
    #[serde(rename = "IVC")]
    Inverclyde,
    #[serde(rename = "AGY")]
    IsleOfAnglesey,
    #[serde(rename = "IOW")]
    IsleOfWight,
    #[serde(rename = "IOS")]
    IslesOfScilly,
    #[serde(rename = "ISL")]
    Islington,
    #[serde(rename = "KEC")]
    KensingtonAndChelsea,
    #[serde(rename = "KEN")]
    Kent,
    #[serde(rename = "KHL")]
    KingstonUponHull,
    #[serde(rename = "KTT")]
    KingstonUponThames,
    #[serde(rename = "KIR")]
    Kirklees,
    #[serde(rename = "KWL")]
    Knowsley,
    #[serde(rename = "LBH")]
    Lambeth,
    #[serde(rename = "LAN")]
    Lancashire,
    #[serde(rename = "LDS")]
    Leeds,
    #[serde(rename = "LCE")]
    Leicester,
    #[serde(rename = "LEC")]
    Leicestershire,
    #[serde(rename = "LEW")]
    Lewisham,
    #[serde(rename = "LIN")]
    Lincolnshire,
    #[serde(rename = "LBC")]
    LisburnAndCastlereagh,
    #[serde(rename = "LIV")]
    Liverpool,
    #[serde(rename = "LND")]
    LondonCityOf,
    #[serde(rename = "LUT")]
    Luton,
    #[serde(rename = "MAN")]
    Manchester,
    #[serde(rename = "MDW")]
    Medway,
    #[serde(rename = "MTY")]
    MerthyrTydfil,
    #[serde(rename = "MRT")]
    Merton,
    #[serde(rename = "MEA")]
    MidAndEastAntrim,
    #[serde(rename = "MUL")]
    MidUlster,
    #[serde(rename = "MDB")]
    Middlesbrough,
    #[serde(rename = "MLN")]
    Midlothian,
    #[serde(rename = "MIK")]
    MiltonKeynes,
    #[serde(rename = "MON")]
    Monmouthshire,
    #[serde(rename = "MRY")]
    Moray,
    #[serde(rename = "NTL")]
    NeathPortTalbot,
    #[serde(rename = "NET")]
    NewcastleUponTyne,
    #[serde(rename = "NWM")]
    Newham,
    #[serde(rename = "NWP")]
    Newport,
    #[serde(rename = "NMD")]
    NewryMourneAndDown,
    #[serde(rename = "NFK")]
    Norfolk,
    #[serde(rename = "NAY")]
    NorthAyrshire,
    #[serde(rename = "NEL")]
    NorthEastLincolnshire,
    #[serde(rename = "NLK")]
    NorthLanarkshire,
    #[serde(rename = "NLN")]
    NorthLincolnshire,
    #[serde(rename = "NSM")]
    NorthSomerset,
    #[serde(rename = "NTY")]
    NorthTyneside,
    #[serde(rename = "NYK")]
    NorthYorkshire,
    #[serde(rename = "NTH")]
    Northamptonshire,
    #[serde(rename = "NBL")]
    Northumberland,
    #[serde(rename = "NGM")]
    Nottingham,
    #[serde(rename = "NTT")]
    Nottinghamshire,
    #[serde(rename = "OLD")]
    Oldham,
    #[serde(rename = "ORK")]
    OrkneyIslands,
    #[serde(rename = "OXF")]
    Oxfordshire,
    #[serde(rename = "PEM")]
    Pembrokeshire,
    #[serde(rename = "PKN")]
    PerthAndKinross,
    #[serde(rename = "PTE")]
    Peterborough,
    #[serde(rename = "PLY")]
    Plymouth,
    #[serde(rename = "POR")]
    Portsmouth,
    #[serde(rename = "POW")]
    Powys,
    #[serde(rename = "RDG")]
    Reading,
    #[serde(rename = "RDB")]
    Redbridge,
    #[serde(rename = "RCC")]
    RedcarAndCleveland,
    #[serde(rename = "RFW")]
    Renfrewshire,
    #[serde(rename = "RCT")]
    RhonddaCynonTaff,
    #[serde(rename = "RIC")]
    RichmondUponThames,
    #[serde(rename = "RCH")]
    Rochdale,
    #[serde(rename = "ROT")]
    Rotherham,
    #[serde(rename = "RUT")]
    Rutland,
    #[serde(rename = "SLF")]
    Salford,
    #[serde(rename = "SAW")]
    Sandwell,
    #[serde(rename = "SCB")]
    ScottishBorders,
    #[serde(rename = "SFT")]
    Sefton,
    #[serde(rename = "SHF")]
    Sheffield,
    #[serde(rename = "ZET")]
    ShetlandIslands,
    #[serde(rename = "SHR")]
    Shropshire,
    #[serde(rename = "SLG")]
    Slough,
    #[serde(rename = "SOL")]
    Solihull,
    #[serde(rename = "SOM")]
    Somerset,
    #[serde(rename = "SAY")]
    SouthAyrshire,
    #[serde(rename = "SGC")]
    SouthGloucestershire,
    #[serde(rename = "SLK")]
    SouthLanarkshire,
    #[serde(rename = "STY")]
    SouthTyneside,
    #[serde(rename = "STH")]
    Southampton,
    #[serde(rename = "SOS")]
    SouthendOnSea,
    #[serde(rename = "SWK")]
    Southwark,
    #[serde(rename = "SHN")]
    StHelens,
    #[serde(rename = "STS")]
    Staffordshire,
    #[serde(rename = "STG")]
    Stirling,
    #[serde(rename = "SKP")]
    Stockport,
    #[serde(rename = "STT")]
    StocktonOnTees,
    #[serde(rename = "STE")]
    StokeOnTrent,
    #[serde(rename = "SFK")]
    Suffolk,
    #[serde(rename = "SND")]
    Sunderland,
    #[serde(rename = "SRY")]
    Surrey,
    #[serde(rename = "STN")]
    Sutton,
    #[serde(rename = "SWA")]
    Swansea,
    #[serde(rename = "SWD")]
    Swindon,
    #[serde(rename = "TAM")]
    Tameside,
    #[serde(rename = "TFW")]
    TelfordAndWrekin,
    #[serde(rename = "THR")]
    Thurrock,
    #[serde(rename = "TOB")]
    Torbay,
    #[serde(rename = "TOF")]
    Torfaen,
    #[serde(rename = "TWH")]
    TowerHamlets,
    #[serde(rename = "TRF")]
    Trafford,
    #[serde(rename = "VGL")]
    ValeOfGlamorgan,
    #[serde(rename = "WKF")]
    Wakefield,
    #[serde(rename = "WLL")]
    Walsall,
    #[serde(rename = "WFT")]
    WalthamForest,
    #[serde(rename = "WND")]
    Wandsworth,
    #[serde(rename = "WRT")]
    Warrington,
    #[serde(rename = "WAR")]
    Warwickshire,
    #[serde(rename = "WBK")]
    WestBerkshire,
    #[serde(rename = "WDU")]
    WestDunbartonshire,
    #[serde(rename = "WLN")]
    WestLothian,
    #[serde(rename = "WSX")]
    WestSussex,
    #[serde(rename = "WSM")]
    Westminster,
    #[serde(rename = "WGN")]
    Wigan,
    #[serde(rename = "WIL")]
    Wiltshire,
    #[serde(rename = "WNM")]
    WindsorAndMaidenhead,
    #[serde(rename = "WRL")]
    Wirral,
    #[serde(rename = "WOK")]
    Wokingham,
    #[serde(rename = "WLV")]
    Wolverhampton,
    #[serde(rename = "WOR")]
    Worcestershire,
    #[serde(rename = "WRX")]
    Wrexham,
    #[serde(rename = "YOR")]
    York,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
pub enum RomaniaStatesAbbreviation {
    #[serde(rename = "AB")]
    Alba,
    #[serde(rename = "AR")]
    AradCounty,
    #[serde(rename = "AG")]
    Arges,
    #[serde(rename = "BC")]
    BacauCounty,
    #[serde(rename = "BH")]
    BihorCounty,
    #[serde(rename = "BN")]
    BistritaNasaudCounty,
    #[serde(rename = "BT")]
    BotosaniCounty,
    #[serde(rename = "BR")]
    Braila,
    #[serde(rename = "BV")]
    BrasovCounty,
    #[serde(rename = "B")]
    Bucharest,
    #[serde(rename = "BZ")]
    BuzauCounty,
    #[serde(rename = "CS")]
    CarasSeverinCounty,
    #[serde(rename = "CJ")]
    ClujCounty,
    #[serde(rename = "CT")]
    ConstantaCounty,
    #[serde(rename = "CV")]
    CovasnaCounty,
    #[serde(rename = "CL")]
    CalarasiCounty,
    #[serde(rename = "DJ")]
    DoljCounty,
    #[serde(rename = "DB")]
    DambovitaCounty,
    #[serde(rename = "GL")]
    GalatiCounty,
    #[serde(rename = "GR")]
    GiurgiuCounty,
    #[serde(rename = "GJ")]
    GorjCounty,
    #[serde(rename = "HR")]
    HarghitaCounty,
    #[serde(rename = "HD")]
    HunedoaraCounty,
    #[serde(rename = "IL")]
    IalomitaCounty,
    #[serde(rename = "IS")]
    IasiCounty,
    #[serde(rename = "IF")]
    IlfovCounty,
    #[serde(rename = "MH")]
    MehedintiCounty,
    #[serde(rename = "MM")]
    MuresCounty,
    #[serde(rename = "NT")]
    NeamtCounty,
    #[serde(rename = "OT")]
    OltCounty,
    #[serde(rename = "PH")]
    PrahovaCounty,
    #[serde(rename = "SM")]
    SatuMareCounty,
    #[serde(rename = "SB")]
    SibiuCounty,
    #[serde(rename = "SV")]
    SuceavaCounty,
    #[serde(rename = "SJ")]
    SalajCounty,
    #[serde(rename = "TR")]
    TeleormanCounty,
    #[serde(rename = "TM")]
    TimisCounty,
    #[serde(rename = "TL")]
    TulceaCounty,
    #[serde(rename = "VS")]
    VasluiCounty,
    #[serde(rename = "VN")]
    VranceaCounty,
    #[serde(rename = "VL")]
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
