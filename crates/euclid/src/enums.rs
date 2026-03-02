use common_enums::connector_enums::Connector;
pub use common_enums::{
    AuthenticationType, CaptureMethod, CardNetwork, Country, CountryAlpha2, Currency,
    FutureUsage as SetupFutureUsage, PaymentMethod, PaymentMethodType,
};
use strum::VariantNames;
use utoipa::ToSchema;

pub trait CollectVariants {
    fn variants<T: FromIterator<String>>() -> T;
}
macro_rules! collect_variants {
    ($the_enum:ident) => {
        impl $crate::enums::CollectVariants for $the_enum {
            fn variants<T>() -> T
            where
                T: FromIterator<String>,
            {
                Self::VARIANTS.iter().map(|s| String::from(*s)).collect()
            }
        }
    };
}

pub(crate) use collect_variants;

collect_variants!(PaymentMethod);
collect_variants!(RoutableConnectors);
collect_variants!(PaymentType);
collect_variants!(MandateType);
collect_variants!(MandateAcceptanceType);
collect_variants!(PaymentMethodType);
collect_variants!(CardNetwork);
collect_variants!(AuthenticationType);
collect_variants!(CaptureMethod);
collect_variants!(Currency);
collect_variants!(Country);
collect_variants!(SetupFutureUsage);
#[cfg(feature = "payouts")]
collect_variants!(PayoutType);
#[cfg(feature = "payouts")]
collect_variants!(PayoutBankTransferType);
#[cfg(feature = "payouts")]
collect_variants!(PayoutWalletType);

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
pub enum MandateAcceptanceType {
    Online,
    Offline,
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
pub enum PaymentType {
    SetupMandate,
    NonMandate,
    NewMandate,
    UpdateMandate,
    PptMandate,
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
pub enum MandateType {
    SingleUse,
    MultiUse,
}

#[cfg(feature = "payouts")]
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
pub enum PayoutBankTransferType {
    Ach,
    Bacs,
    SepaBankTransfer,
}

#[cfg(feature = "payouts")]
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
pub enum PayoutWalletType {
    Paypal,
}

#[cfg(feature = "payouts")]
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
pub enum PayoutType {
    Card,
    BankTransfer,
    Wallet,
    BankRedirect,
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
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
/// RoutableConnectors are the subset of Connectors that are eligible for payments routing
pub enum RoutableConnectors {
    Authipay,
    Adyenplatform,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "stripe_billing_test")]
    #[strum(serialize = "stripe_billing_test")]
    DummyBillingConnector,
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
    Affirm,
    Airwallex,
    Amazonpay,
    Archipel,
    Authorizedotnet,
    Bankofamerica,
    Barclaycard,
    Billwerk,
    Bitpay,
    Bambora,
    Blackhawknetwork,
    Bamboraapac,
    Bluesnap,
    #[serde(alias = "bluecode")]
    Calida,
    Boku,
    Braintree,
    Breadpay,
    Cashtocode,
    Celero,
    Chargebee,
    Custombilling,
    Checkbook,
    Checkout,
    Coinbase,
    Coingate,
    Cryptopay,
    Cybersource,
    Cybersourcedecisionmanager,
    Datatrans,
    Deutschebank,
    Digitalvirgo,
    Dlocal,
    Dwolla,
    Ebanx,
    Elavon,
    Facilitapay,
    Finix,
    Fiserv,
    Fiservemea,
    Fiuu,
    Flexiti,
    Forte,
    Getnet,
    Gigadat,
    Globalpay,
    Globepay,
    Gocardless,
    Hipay,
    Helcim,
    Hyperpg,
    Iatapay,
    Inespay,
    Itaubank,
    Jpmorgan,
    Klarna,
    Loonio,
    Mifinity,
    Mollie,
    Moneris,
    Multisafepay,
    Nexinets,
    Nexixpay,
    Nmi,
    Nomupay,
    Noon,
    Nordea,
    Novalnet,
    Nuvei,
    // Opayo, added as template code for future usage
    Opennode,
    // Payeezy, As psync and rsync are not supported by this connector, it is added as template code for future usage
    Paybox,
    Payme,
    Payload,
    Payone,
    Paypal,
    Paysafe,
    Paystack,
    Paytm,
    Payu,
    Peachpayments,
    Payjustnow,
    Payjustnowinstore,
    Phonepe,
    Placetopay,
    Powertranz,
    Prophetpay,
    Rapyd,
    Razorpay,
    Recurly,
    Redsys,
    Revolv3,
    Riskified,
    Santander,
    Shift4,
    Signifyd,
    Silverflow,
    Square,
    Stax,
    Stripe,
    Stripebilling,
    Tesouro,
    // Taxjar,
    // Truelayer,
    Trustpay,
    Trustpayments,
    // Thunes
    Tokenio,
    // Tsys,
    Tsys,
    // UnifiedAuthenticationService,
    // Vgs
    Volt,
    Wellsfargo,
    // Wellsfargopayout,
    Wise,
    Worldline,
    Worldpay,
    Worldpaymodular,
    Worldpayvantiv,
    Worldpayxml,
    Xendit,
    Zen,
    Zift,
    Plaid,
    Zsl,
    Juspaythreedsserver,
    CtpMastercard,
    CtpVisa,
    Netcetera,
    Cardinal,
    Threedsecureio,
}

impl TryFrom<Connector> for RoutableConnectors {
    type Error = &'static str;

    fn try_from(connector: Connector) -> Result<Self, Self::Error> {
        match connector {
            Connector::Authipay => Ok(Self::Authipay),
            Connector::Adyenplatform => Ok(Self::Adyenplatform),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyBillingConnector => Ok(Self::DummyBillingConnector),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector1 => Ok(Self::DummyConnector1),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector2 => Ok(Self::DummyConnector2),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector3 => Ok(Self::DummyConnector3),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector4 => Ok(Self::DummyConnector4),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector5 => Ok(Self::DummyConnector5),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector6 => Ok(Self::DummyConnector6),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector7 => Ok(Self::DummyConnector7),
            Connector::Aci => Ok(Self::Aci),
            Connector::Adyen => Ok(Self::Adyen),
            Connector::Affirm => Ok(Self::Affirm),
            Connector::Airwallex => Ok(Self::Airwallex),
            Connector::Amazonpay => Ok(Self::Amazonpay),
            Connector::Archipel => Ok(Self::Archipel),
            Connector::Authorizedotnet => Ok(Self::Authorizedotnet),
            Connector::Bankofamerica => Ok(Self::Bankofamerica),
            Connector::Barclaycard => Ok(Self::Barclaycard),
            Connector::Billwerk => Ok(Self::Billwerk),
            Connector::Bitpay => Ok(Self::Bitpay),
            Connector::Bambora => Ok(Self::Bambora),
            Connector::Bamboraapac => Ok(Self::Bamboraapac),
            Connector::Bluesnap => Ok(Self::Bluesnap),
            Connector::Blackhawknetwork => Ok(Self::Blackhawknetwork),
            Connector::Calida => Ok(Self::Calida),
            Connector::Boku => Ok(Self::Boku),
            Connector::Braintree => Ok(Self::Braintree),
            Connector::Breadpay => Ok(Self::Breadpay),
            Connector::Cashtocode => Ok(Self::Cashtocode),
            Connector::Celero => Ok(Self::Celero),
            Connector::Chargebee => Ok(Self::Chargebee),
            Connector::Checkbook => Ok(Self::Checkbook),
            Connector::Checkout => Ok(Self::Checkout),
            Connector::Coinbase => Ok(Self::Coinbase),
            Connector::Coingate => Ok(Self::Coingate),
            Connector::Cryptopay => Ok(Self::Cryptopay),
            Connector::Custombilling => Ok(Self::Custombilling),
            Connector::Cybersource => Ok(Self::Cybersource),
            Connector::Cybersourcedecisionmanager => Ok(Self::Cybersourcedecisionmanager),
            Connector::Datatrans => Ok(Self::Datatrans),
            Connector::Deutschebank => Ok(Self::Deutschebank),
            Connector::Digitalvirgo => Ok(Self::Digitalvirgo),
            Connector::Dlocal => Ok(Self::Dlocal),
            Connector::Dwolla => Ok(Self::Dwolla),
            Connector::Ebanx => Ok(Self::Ebanx),
            Connector::Elavon => Ok(Self::Elavon),
            Connector::Facilitapay => Ok(Self::Facilitapay),
            Connector::Finix => Ok(Self::Finix),
            Connector::Fiserv => Ok(Self::Fiserv),
            Connector::Fiservemea => Ok(Self::Fiservemea),
            Connector::Fiuu => Ok(Self::Fiuu),
            Connector::Flexiti => Ok(Self::Flexiti),
            Connector::Forte => Ok(Self::Forte),
            Connector::Globalpay => Ok(Self::Globalpay),
            Connector::Globepay => Ok(Self::Globepay),
            Connector::Gocardless => Ok(Self::Gocardless),
            Connector::Helcim => Ok(Self::Helcim),
            Connector::Hyperpg => Ok(Self::Hyperpg),
            Connector::Iatapay => Ok(Self::Iatapay),
            Connector::Itaubank => Ok(Self::Itaubank),
            Connector::Jpmorgan => Ok(Self::Jpmorgan),
            Connector::Klarna => Ok(Self::Klarna),
            Connector::Loonio => Ok(Self::Loonio),
            Connector::Mifinity => Ok(Self::Mifinity),
            Connector::Mollie => Ok(Self::Mollie),
            Connector::Moneris => Ok(Self::Moneris),
            Connector::Multisafepay => Ok(Self::Multisafepay),
            Connector::Nexinets => Ok(Self::Nexinets),
            Connector::Nexixpay => Ok(Self::Nexixpay),
            Connector::Nmi => Ok(Self::Nmi),
            Connector::Nomupay => Ok(Self::Nomupay),
            Connector::Noon => Ok(Self::Noon),
            Connector::Nordea => Ok(Self::Nordea),
            Connector::Novalnet => Ok(Self::Novalnet),
            Connector::Nuvei => Ok(Self::Nuvei),
            Connector::Opennode => Ok(Self::Opennode),
            Connector::Paybox => Ok(Self::Paybox),
            Connector::Payload => Ok(Self::Payload),
            Connector::Payme => Ok(Self::Payme),
            Connector::Payone => Ok(Self::Payone),
            Connector::Paypal => Ok(Self::Paypal),
            Connector::Paysafe => Ok(Self::Paysafe),
            Connector::Paystack => Ok(Self::Paystack),
            Connector::Payu => Ok(Self::Payu),
            Connector::Peachpayments => Ok(Self::Peachpayments),
            Connector::Placetopay => Ok(Self::Placetopay),
            Connector::Powertranz => Ok(Self::Powertranz),
            Connector::Prophetpay => Ok(Self::Prophetpay),
            Connector::Rapyd => Ok(Self::Rapyd),
            Connector::Razorpay => Ok(Self::Razorpay),
            Connector::Riskified => Ok(Self::Riskified),
            Connector::Santander => Ok(Self::Santander),
            Connector::Shift4 => Ok(Self::Shift4),
            Connector::Signifyd => Ok(Self::Signifyd),
            Connector::Silverflow => Ok(Self::Silverflow),
            Connector::Square => Ok(Self::Square),
            Connector::Stax => Ok(Self::Stax),
            Connector::Stripe => Ok(Self::Stripe),
            Connector::Stripebilling => Ok(Self::Stripebilling),
            Connector::Tokenio => Ok(Self::Tokenio),
            Connector::Tesouro => Ok(Self::Tesouro),
            // Connector::Truelayer => Ok(Self::Truelayer),
            Connector::Trustpay => Ok(Self::Trustpay),
            Connector::Trustpayments => Ok(Self::Trustpayments),
            Connector::Tsys => Ok(Self::Tsys),
            Connector::Volt => Ok(Self::Volt),
            Connector::Wellsfargo => Ok(Self::Wellsfargo),
            Connector::Wise => Ok(Self::Wise),
            Connector::Worldline => Ok(Self::Worldline),
            Connector::Worldpay => Ok(Self::Worldpay),
            Connector::Worldpaymodular => Ok(Self::Worldpaymodular),
            Connector::Worldpayvantiv => Ok(Self::Worldpayvantiv),
            Connector::Worldpayxml => Ok(Self::Worldpayxml),
            Connector::Xendit => Ok(Self::Xendit),
            Connector::Zen => Ok(Self::Zen),
            Connector::Plaid => Ok(Self::Plaid),
            Connector::Zift => Ok(Self::Zift),
            Connector::Zsl => Ok(Self::Zsl),
            Connector::Recurly => Ok(Self::Recurly),
            Connector::Getnet => Ok(Self::Getnet),
            Connector::Gigadat => Ok(Self::Gigadat),
            Connector::Hipay => Ok(Self::Hipay),
            Connector::Inespay => Ok(Self::Inespay),
            Connector::Redsys => Ok(Self::Redsys),
            Connector::Revolv3 => Ok(Self::Revolv3),
            Connector::Paytm => Ok(Self::Paytm),
            Connector::Phonepe => Ok(Self::Phonepe),
            Connector::Payjustnow => Ok(Self::Payjustnow),
            Connector::Payjustnowinstore => Ok(Self::Payjustnowinstore),
            Connector::CtpMastercard
            | Connector::Gpayments
            | Connector::HyperswitchVault
            | Connector::Juspaythreedsserver
            | Connector::Netcetera
            | Connector::Taxjar
            | Connector::Threedsecureio
            | Connector::Vgs
            | Connector::CtpVisa
            | Connector::Cardinal
            | Connector::Tokenex => Err("Invalid conversion. Not a routable connector"),
        }
    }
}

/// Convert the RoutableConnectors to Connector
impl From<RoutableConnectors> for Connector {
    fn from(routable_connector: RoutableConnectors) -> Self {
        match routable_connector {
            RoutableConnectors::Authipay => Self::Authipay,
            RoutableConnectors::Adyenplatform => Self::Adyenplatform,
            #[cfg(feature = "dummy_connector")]
            RoutableConnectors::DummyBillingConnector => Self::DummyBillingConnector,
            #[cfg(feature = "dummy_connector")]
            RoutableConnectors::DummyConnector1 => Self::DummyConnector1,
            #[cfg(feature = "dummy_connector")]
            RoutableConnectors::DummyConnector2 => Self::DummyConnector2,
            #[cfg(feature = "dummy_connector")]
            RoutableConnectors::DummyConnector3 => Self::DummyConnector3,
            #[cfg(feature = "dummy_connector")]
            RoutableConnectors::DummyConnector4 => Self::DummyConnector4,
            #[cfg(feature = "dummy_connector")]
            RoutableConnectors::DummyConnector5 => Self::DummyConnector5,
            #[cfg(feature = "dummy_connector")]
            RoutableConnectors::DummyConnector6 => Self::DummyConnector6,
            #[cfg(feature = "dummy_connector")]
            RoutableConnectors::DummyConnector7 => Self::DummyConnector7,
            RoutableConnectors::Aci => Self::Aci,
            RoutableConnectors::Adyen => Self::Adyen,
            RoutableConnectors::Affirm => Self::Affirm,
            RoutableConnectors::Airwallex => Self::Airwallex,
            RoutableConnectors::Amazonpay => Self::Amazonpay,
            RoutableConnectors::Archipel => Self::Archipel,
            RoutableConnectors::Authorizedotnet => Self::Authorizedotnet,
            RoutableConnectors::Bankofamerica => Self::Bankofamerica,
            RoutableConnectors::Barclaycard => Self::Barclaycard,
            RoutableConnectors::Billwerk => Self::Billwerk,
            RoutableConnectors::Bitpay => Self::Bitpay,
            RoutableConnectors::Bambora => Self::Bambora,
            RoutableConnectors::Bamboraapac => Self::Bamboraapac,
            RoutableConnectors::Bluesnap => Self::Bluesnap,
            RoutableConnectors::Blackhawknetwork => Self::Blackhawknetwork,
            RoutableConnectors::Calida => Self::Calida,
            RoutableConnectors::Boku => Self::Boku,
            RoutableConnectors::Braintree => Self::Braintree,
            RoutableConnectors::Breadpay => Self::Breadpay,
            RoutableConnectors::Cashtocode => Self::Cashtocode,
            RoutableConnectors::Celero => Self::Celero,
            RoutableConnectors::Chargebee => Self::Chargebee,
            RoutableConnectors::Custombilling => Self::Custombilling,
            RoutableConnectors::Checkbook => Self::Checkbook,
            RoutableConnectors::Checkout => Self::Checkout,
            RoutableConnectors::Coinbase => Self::Coinbase,
            RoutableConnectors::Cryptopay => Self::Cryptopay,
            RoutableConnectors::Cybersource => Self::Cybersource,
            RoutableConnectors::Cybersourcedecisionmanager => Self::Cybersourcedecisionmanager,
            RoutableConnectors::Datatrans => Self::Datatrans,
            RoutableConnectors::Deutschebank => Self::Deutschebank,
            RoutableConnectors::Digitalvirgo => Self::Digitalvirgo,
            RoutableConnectors::Dlocal => Self::Dlocal,
            RoutableConnectors::Dwolla => Self::Dwolla,
            RoutableConnectors::Ebanx => Self::Ebanx,
            RoutableConnectors::Elavon => Self::Elavon,
            RoutableConnectors::Facilitapay => Self::Facilitapay,
            RoutableConnectors::Finix => Self::Finix,
            RoutableConnectors::Fiserv => Self::Fiserv,
            RoutableConnectors::Fiservemea => Self::Fiservemea,
            RoutableConnectors::Fiuu => Self::Fiuu,
            RoutableConnectors::Flexiti => Self::Flexiti,
            RoutableConnectors::Forte => Self::Forte,
            RoutableConnectors::Getnet => Self::Getnet,
            RoutableConnectors::Gigadat => Self::Gigadat,
            RoutableConnectors::Globalpay => Self::Globalpay,
            RoutableConnectors::Globepay => Self::Globepay,
            RoutableConnectors::Gocardless => Self::Gocardless,
            RoutableConnectors::Helcim => Self::Helcim,
            RoutableConnectors::Hyperpg => Self::Hyperpg,
            RoutableConnectors::Iatapay => Self::Iatapay,
            RoutableConnectors::Itaubank => Self::Itaubank,
            RoutableConnectors::Jpmorgan => Self::Jpmorgan,
            RoutableConnectors::Klarna => Self::Klarna,
            RoutableConnectors::Loonio => Self::Loonio,
            RoutableConnectors::Zift => Self::Zift,
            RoutableConnectors::Mifinity => Self::Mifinity,
            RoutableConnectors::Mollie => Self::Mollie,
            RoutableConnectors::Moneris => Self::Moneris,
            RoutableConnectors::Multisafepay => Self::Multisafepay,
            RoutableConnectors::Nexinets => Self::Nexinets,
            RoutableConnectors::Nexixpay => Self::Nexixpay,
            RoutableConnectors::Nmi => Self::Nmi,
            RoutableConnectors::Nomupay => Self::Nomupay,
            RoutableConnectors::Noon => Self::Noon,
            RoutableConnectors::Nordea => Self::Nordea,
            RoutableConnectors::Novalnet => Self::Novalnet,
            RoutableConnectors::Nuvei => Self::Nuvei,
            RoutableConnectors::Opennode => Self::Opennode,
            RoutableConnectors::Paybox => Self::Paybox,
            RoutableConnectors::Payload => Self::Payload,
            RoutableConnectors::Payme => Self::Payme,
            RoutableConnectors::Payone => Self::Payone,
            RoutableConnectors::Paypal => Self::Paypal,
            RoutableConnectors::Paysafe => Self::Paysafe,
            RoutableConnectors::Paystack => Self::Paystack,
            RoutableConnectors::Payu => Self::Payu,
            RoutableConnectors::Peachpayments => Self::Peachpayments,
            RoutableConnectors::Placetopay => Self::Placetopay,
            RoutableConnectors::Powertranz => Self::Powertranz,
            RoutableConnectors::Prophetpay => Self::Prophetpay,
            RoutableConnectors::Rapyd => Self::Rapyd,
            RoutableConnectors::Razorpay => Self::Razorpay,
            RoutableConnectors::Recurly => Self::Recurly,
            RoutableConnectors::Redsys => Self::Redsys,
            RoutableConnectors::Revolv3 => Self::Revolv3,
            RoutableConnectors::Riskified => Self::Riskified,
            RoutableConnectors::Santander => Self::Santander,
            RoutableConnectors::Shift4 => Self::Shift4,
            RoutableConnectors::Signifyd => Self::Signifyd,
            RoutableConnectors::Silverflow => Self::Silverflow,
            RoutableConnectors::Square => Self::Square,
            RoutableConnectors::Stax => Self::Stax,
            RoutableConnectors::Stripe => Self::Stripe,
            RoutableConnectors::Stripebilling => Self::Stripebilling,
            RoutableConnectors::Tesouro => Self::Tesouro,
            RoutableConnectors::Tokenio => Self::Tokenio,
            // RoutableConnectors::Truelayer => Self::Truelayer,
            RoutableConnectors::Trustpay => Self::Trustpay,
            RoutableConnectors::Trustpayments => Self::Trustpayments,
            // RoutableConnectors::Tokenio => Self::Tokenio,
            RoutableConnectors::Tsys => Self::Tsys,
            RoutableConnectors::Volt => Self::Volt,
            RoutableConnectors::Wellsfargo => Self::Wellsfargo,
            RoutableConnectors::Wise => Self::Wise,
            RoutableConnectors::Worldline => Self::Worldline,
            RoutableConnectors::Worldpay => Self::Worldpay,
            RoutableConnectors::Worldpaymodular => Self::Worldpaymodular,
            RoutableConnectors::Worldpayvantiv => Self::Worldpayvantiv,
            RoutableConnectors::Worldpayxml => Self::Worldpayxml,
            RoutableConnectors::Zen => Self::Zen,
            RoutableConnectors::Plaid => Self::Plaid,
            RoutableConnectors::Zsl => Self::Zsl,
            RoutableConnectors::Xendit => Self::Xendit,
            RoutableConnectors::Inespay => Self::Inespay,
            RoutableConnectors::Coingate => Self::Coingate,
            RoutableConnectors::Hipay => Self::Hipay,
            RoutableConnectors::Paytm => Self::Paytm,
            RoutableConnectors::Phonepe => Self::Phonepe,
            RoutableConnectors::Payjustnow => Self::Payjustnow,
            RoutableConnectors::Payjustnowinstore => Self::Payjustnowinstore,
            RoutableConnectors::Juspaythreedsserver => Self::Juspaythreedsserver,
            RoutableConnectors::CtpMastercard => Self::CtpMastercard,
            RoutableConnectors::CtpVisa => Self::CtpVisa,
            RoutableConnectors::Netcetera => Self::Netcetera,
            RoutableConnectors::Cardinal => Self::Cardinal,
            RoutableConnectors::Threedsecureio => Self::Threedsecureio,
        }
    }
}
