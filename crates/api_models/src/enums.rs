use std::str::FromStr;

pub use common_enums::*;
use utoipa::ToSchema;

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

/// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoutingAlgorithm {
    RoundRobin,
    MaxConversion,
    MinCost,
    Custom,
}

/// A connector is an integration to fulfill payments
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    ToSchema,
    serde::Deserialize,
    serde::Serialize,
    strum::VariantNames,
    strum::EnumIter,
    strum::Display,
    strum::EnumString,
    Hash,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Connector {
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "phonypay")]
    #[strum(serialize = "phonypay")]
    DummyConnector1,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "fauxpay")]
    #[strum(serialize = "fauxpay")]
    DummyConnector2,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "pretendpay")]
    #[strum(serialize = "pretendpay")]
    DummyConnector3,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "stripe_test")]
    #[strum(serialize = "stripe_test")]
    DummyConnector4,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "adyen_test")]
    #[strum(serialize = "adyen_test")]
    DummyConnector5,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "checkout_test")]
    #[strum(serialize = "checkout_test")]
    DummyConnector6,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "paypal_test")]
    #[strum(serialize = "paypal_test")]
    DummyConnector7,
    Aci,
    Adyen,
    Airwallex,
    Authorizedotnet,
    Bambora,
    Bankofamerica,
    Billwerk,
    Bitpay,
    Bluesnap,
    Boku,
    Braintree,
    Cashtocode,
    Checkout,
    Coinbase,
    Cryptopay,
    Cybersource,
    Dlocal,
    // Ebanx,
    Fiserv,
    Forte,
    Globalpay,
    Globepay,
    Gocardless,
    Helcim,
    Iatapay,
    Klarna,
    Mollie,
    Multisafepay,
    Nexinets,
    Nmi,
    Noon,
    Nuvei,
    // Opayo, added as template code for future usage
    Opennode,
    // Payeezy, As psync and rsync are not supported by this connector, it is added as template code for future usage
    Payme,
    Paypal,
    Payu,
    Placetopay,
    Powertranz,
    Prophetpay,
    Rapyd,
    Shift4,
    Square,
    Stax,
    Stripe,
    Threedsecureio,
    Trustpay,
    // Tsys,
    Tsys,
    Volt,
    Wise,
    Worldline,
    Worldpay,
    Zen,
    Signifyd,
    Plaid,
    Riskified,
    Zsl,
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
                | (Self::Volt, _)
        )
    }
    pub fn supports_file_storage_module(&self) -> bool {
        matches!(self, Self::Stripe | Self::Checkout)
    }
    pub fn requires_defend_dispute(&self) -> bool {
        matches!(self, Self::Checkout)
    }
    pub fn is_separate_authentication_supported(&self) -> bool {
        #[cfg(feature = "dummy_connector")]
        match self {
            Self::DummyConnector1
            | Self::DummyConnector2
            | Self::DummyConnector3
            | Self::DummyConnector4
            | Self::DummyConnector5
            | Self::DummyConnector6
            | Self::DummyConnector7 => false,
            Self::Aci
            | Self::Adyen
            | Self::Airwallex
            | Self::Authorizedotnet
            | Self::Bambora
            | Self::Bankofamerica
            | Self::Billwerk
            | Self::Bitpay
            | Self::Bluesnap
            | Self::Boku
            | Self::Braintree
            | Self::Cashtocode
            | Self::Coinbase
            | Self::Cryptopay
            | Self::Dlocal
            | Self::Fiserv
            | Self::Forte
            | Self::Globalpay
            | Self::Globepay
            | Self::Gocardless
            | Self::Helcim
            | Self::Iatapay
            | Self::Klarna
            | Self::Mollie
            | Self::Multisafepay
            | Self::Nexinets
            | Self::Nmi
            | Self::Nuvei
            | Self::Opennode
            | Self::Payme
            | Self::Paypal
            | Self::Payu
            | Self::Placetopay
            | Self::Powertranz
            | Self::Prophetpay
            | Self::Rapyd
            | Self::Shift4
            | Self::Square
            | Self::Stax
            | Self::Trustpay
            | Self::Tsys
            | Self::Volt
            | Self::Wise
            | Self::Worldline
            | Self::Worldpay
            | Self::Zen
            | Self::Zsl
            | Self::Signifyd
            | Self::Plaid
            | Self::Riskified
            | Self::Threedsecureio
            | Self::Cybersource
            | Self::Noon
            | Self::Stripe => false,
            Self::Checkout => true,
        }
        #[cfg(not(feature = "dummy_connector"))]
        match self {
            Self::Aci
            | Self::Adyen
            | Self::Airwallex
            | Self::Authorizedotnet
            | Self::Bambora
            | Self::Bankofamerica
            | Self::Billwerk
            | Self::Bitpay
            | Self::Bluesnap
            | Self::Boku
            | Self::Braintree
            | Self::Cashtocode
            | Self::Coinbase
            | Self::Cryptopay
            | Self::Dlocal
            | Self::Fiserv
            | Self::Forte
            | Self::Globalpay
            | Self::Globepay
            | Self::Gocardless
            | Self::Helcim
            | Self::Iatapay
            | Self::Klarna
            | Self::Mollie
            | Self::Multisafepay
            | Self::Nexinets
            | Self::Nmi
            | Self::Nuvei
            | Self::Opennode
            | Self::Payme
            | Self::Paypal
            | Self::Payu
            | Self::Placetopay
            | Self::Powertranz
            | Self::Prophetpay
            | Self::Rapyd
            | Self::Shift4
            | Self::Square
            | Self::Stax
            | Self::Trustpay
            | Self::Tsys
            | Self::Volt
            | Self::Wise
            | Self::Worldline
            | Self::Worldpay
            | Self::Zen
            | Self::Signifyd
            | Self::Plaid
            | Self::Riskified
            | Self::Threedsecureio
            | Self::Cybersource
            | Self::Noon
            | Self::Stripe => false,
            Self::Checkout => true,
        }
    }
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
}

#[cfg(feature = "payouts")]
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
pub enum PayoutConnectors {
    Adyen,
    Wise,
}

#[cfg(feature = "payouts")]
impl From<PayoutConnectors> for RoutableConnectors {
    fn from(value: PayoutConnectors) -> Self {
        match value {
            PayoutConnectors::Adyen => Self::Adyen,
            PayoutConnectors::Wise => Self::Wise,
        }
    }
}

#[cfg(feature = "payouts")]
impl From<PayoutConnectors> for Connector {
    fn from(value: PayoutConnectors) -> Self {
        match value {
            PayoutConnectors::Adyen => Self::Adyen,
            PayoutConnectors::Wise => Self::Wise,
        }
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<Connector> for PayoutConnectors {
    type Error = String;
    fn try_from(value: Connector) -> Result<Self, Self::Error> {
        match value {
            Connector::Adyen => Ok(Self::Adyen),
            Connector::Wise => Ok(Self::Wise),
            _ => Err(format!("Invalid payout connector {}", value)),
        }
    }
}

#[cfg(feature = "frm")]
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
pub enum FrmConnectors {
    /// Signifyd Risk Manager. Official docs: https://docs.signifyd.com/
    Signifyd,
    Riskified,
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

/// Possible field type of required fields in payment_method_data
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
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FieldType {
    UserCardNumber,
    UserCardExpiryMonth,
    UserCardExpiryYear,
    UserCardCvc,
    UserFullName,
    UserEmailAddress,
    UserPhoneNumber,
    UserCountryCode,                      //phone number's country code
    UserCountry { options: Vec<String> }, //for country inside payment method data ex- bank redirect
    UserCurrency { options: Vec<String> },
    UserBillingName,
    UserAddressLine1,
    UserAddressLine2,
    UserAddressCity,
    UserAddressPincode,
    UserAddressState,
    UserAddressCountry { options: Vec<String> },
    UserBlikCode,
    UserBank,
    Text,
    DropDown { options: Vec<String> },
}

#[derive(
    Debug,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    Clone,
    PartialEq,
    Eq,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RetryAction {
    /// Payment can be retried from the client side until the payment is successful or payment expires or the attempts(configured by the merchant) for payment are exhausted
    ManualRetry,
    /// Denotes that the payment is requeued
    Requeue,
}

#[derive(Clone, Copy)]
pub enum LockerChoice {
    HyperswitchCardVault,
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
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PmAuthConnectors {
    Plaid,
}

pub fn convert_pm_auth_connector(connector_name: &str) -> Option<PmAuthConnectors> {
    PmAuthConnectors::from_str(connector_name).ok()
}

pub fn convert_authentication_connector(connector_name: &str) -> Option<AuthenticationConnectors> {
    AuthenticationConnectors::from_str(connector_name).ok()
}
