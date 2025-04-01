use std::collections::HashSet;

use utoipa::ToSchema;

pub use super::enums::{PaymentMethod, PayoutType};
pub use crate::PaymentMethodType;

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
    Adyenplatform,
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
    // Amazonpay,
    Authorizedotnet,
    Bankofamerica,
    Billwerk,
    Bitpay,
    Bambora,
    Bamboraapac,
    Bluesnap,
    Boku,
    Braintree,
    Cashtocode,
    Chargebee,
    Checkout,
    Coinbase,
    Coingate,
    Cryptopay,
    Cybersource,
    Datatrans,
    Deutschebank,
    Digitalvirgo,
    Dlocal,
    Ebanx,
    Elavon,
    // Facilitapay,
    Fiserv,
    Fiservemea,
    Fiuu,
    Forte,
    Getnet,
    Globalpay,
    Globepay,
    Gocardless,
    Hipay,
    Helcim,
    Iatapay,
    Inespay,
    Itaubank,
    Jpmorgan,
    Klarna,
    Mifinity,
    Mollie,
    Moneris,
    Multisafepay,
    Nexinets,
    Nexixpay,
    Nmi,
    Nomupay,
    Noon,
    Novalnet,
    Nuvei,
    // Opayo, added as template code for future usage
    Opennode,
    // Payeezy, As psync and rsync are not supported by this connector, it is added as template code for future usage
    Paybox,
    Payme,
    Payone,
    Paypal,
    Paystack,
    Payu,
    Placetopay,
    Powertranz,
    Prophetpay,
    Rapyd,
    Razorpay,
    Recurly,
    Redsys,
    Riskified,
    Shift4,
    Signifyd,
    Square,
    Stax,
    Stripe,
    //Stripebilling,
    // Taxjar,
    Trustpay,
    // Thunes
    // Tsys,
    Tsys,
    // UnifiedAuthenticationService,
    Volt,
    Wellsfargo,
    // Wellsfargopayout,
    Wise,
    Worldline,
    Worldpay,
    Xendit,
    Zen,
    Plaid,
    Zsl,
}

// A connector is an integration to fulfill payments
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
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Connector {
    Adyenplatform,
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
    // Amazonpay,
    Authorizedotnet,
    Bambora,
    Bamboraapac,
    Bankofamerica,
    Billwerk,
    Bitpay,
    Bluesnap,
    Boku,
    Braintree,
    Cashtocode,
    Chargebee,
    Checkout,
    Coinbase,
    Coingate,
    Cryptopay,
    CtpMastercard,
    CtpVisa,
    Cybersource,
    Datatrans,
    Deutschebank,
    Digitalvirgo,
    Dlocal,
    Ebanx,
    Elavon,
    // Facilitapay,
    Fiserv,
    Fiservemea,
    Fiuu,
    Forte,
    Getnet,
    Globalpay,
    Globepay,
    Gocardless,
    Gpayments,
    Hipay,
    Helcim,
    Inespay,
    Iatapay,
    Itaubank,
    Jpmorgan,
    Juspaythreedsserver,
    Klarna,
    Mifinity,
    Mollie,
    Moneris,
    Multisafepay,
    Netcetera,
    Nexinets,
    Nexixpay,
    Nmi,
    Nomupay,
    Noon,
    Novalnet,
    Nuvei,
    // Opayo, added as template code for future usage
    Opennode,
    Paybox,
    // Payeezy, As psync and rsync are not supported by this connector, it is added as template code for future usage
    Payme,
    Payone,
    Paypal,
    Paystack,
    Payu,
    Placetopay,
    Powertranz,
    Prophetpay,
    Rapyd,
    Razorpay,
    Recurly,
    Redsys,
    Shift4,
    Square,
    Stax,
    Stripe,
    // Stripebilling,
    Taxjar,
    Threedsecureio,
    //Thunes,
    Trustpay,
    Tsys,
    // UnifiedAuthenticationService,
    Volt,
    Wellsfargo,
    // Wellsfargopayout,
    Wise,
    Worldline,
    Worldpay,
    Signifyd,
    Plaid,
    Riskified,
    Xendit,
    Zen,
    Zsl,
}

impl Connector {
    #[cfg(feature = "payouts")]
    pub fn supports_instant_payout(self, payout_method: Option<PayoutType>) -> bool {
        matches!(
            (self, payout_method),
            (Self::Paypal, Some(PayoutType::Wallet))
                | (_, Some(PayoutType::Card))
                | (Self::Adyenplatform, _)
                | (Self::Nomupay, _)
        )
    }
    #[cfg(feature = "payouts")]
    pub fn supports_create_recipient(self, payout_method: Option<PayoutType>) -> bool {
        matches!((self, payout_method), (_, Some(PayoutType::Bank)))
    }
    #[cfg(feature = "payouts")]
    pub fn supports_payout_eligibility(self, payout_method: Option<PayoutType>) -> bool {
        matches!((self, payout_method), (_, Some(PayoutType::Card)))
    }
    #[cfg(feature = "payouts")]
    pub fn is_payout_quote_call_required(self) -> bool {
        matches!(self, Self::Wise)
    }
    #[cfg(feature = "payouts")]
    pub fn supports_access_token_for_payout(self, payout_method: Option<PayoutType>) -> bool {
        matches!((self, payout_method), (Self::Paypal, _))
    }
    #[cfg(feature = "payouts")]
    pub fn supports_vendor_disburse_account_create_for_payout(self) -> bool {
        matches!(self, Self::Stripe | Self::Nomupay)
    }
    pub fn supports_access_token(self, payment_method: PaymentMethod) -> bool {
        matches!(
            (self, payment_method),
            (Self::Airwallex, _)
                | (Self::Deutschebank, _)
                | (Self::Globalpay, _)
                | (Self::Jpmorgan, _)
                | (Self::Moneris, _)
                | (Self::Paypal, _)
                | (Self::Payu, _)
                | (
                    Self::Trustpay,
                    PaymentMethod::BankRedirect | PaymentMethod::BankTransfer
                )
                | (Self::Iatapay, _)
                | (Self::Volt, _)
                | (Self::Itaubank, _)
        )
    }
    pub fn supports_file_storage_module(self) -> bool {
        matches!(self, Self::Stripe | Self::Checkout)
    }
    pub fn requires_defend_dispute(self) -> bool {
        matches!(self, Self::Checkout)
    }
    pub fn is_separate_authentication_supported(self) -> bool {
        match self {
            #[cfg(feature = "dummy_connector")]
            Self::DummyConnector1
            | Self::DummyConnector2
            | Self::DummyConnector3
            | Self::DummyConnector4
            | Self::DummyConnector5
            | Self::DummyConnector6
            | Self::DummyConnector7 => false,
            Self::Aci
            // Add Separate authentication support for connectors
            | Self::Adyen
            | Self::Adyenplatform
            | Self::Airwallex
            // | Self::Amazonpay
            | Self::Authorizedotnet
            | Self::Bambora
            | Self::Bamboraapac
            | Self::Bankofamerica
            | Self::Billwerk
            | Self::Bitpay
            | Self::Bluesnap
            | Self::Boku
            | Self::Braintree
            | Self::Cashtocode
            | Self::Chargebee
            | Self::Coinbase
            | Self::Coingate
            | Self::Cryptopay
            | Self::Deutschebank
            | Self::Digitalvirgo
            | Self::Dlocal
            | Self::Ebanx
            | Self::Elavon
            // | Self::Facilitapay
            | Self::Fiserv
            | Self::Fiservemea
            | Self::Fiuu
            | Self::Forte
            | Self::Getnet
            | Self::Globalpay
            | Self::Globepay
            | Self::Gocardless
            | Self::Gpayments
            | Self::Hipay
            | Self::Helcim
            | Self::Iatapay
			| Self::Inespay
            | Self::Itaubank
            | Self::Jpmorgan
            | Self::Juspaythreedsserver
            | Self::Klarna
            | Self::Mifinity
            | Self::Mollie
            | Self::Moneris
            | Self::Multisafepay
            | Self::Nexinets
            | Self::Nexixpay
            | Self::Nomupay
            | Self::Novalnet
            | Self::Nuvei
            | Self::Opennode
            | Self::Paybox
            | Self::Payme
            | Self::Payone
            | Self::Paypal
            | Self::Paystack
            | Self::Payu
            | Self::Placetopay
            | Self::Powertranz
            | Self::Prophetpay
            | Self::Rapyd
            | Self::Recurly
            | Self::Redsys
            | Self::Shift4
            | Self::Square
            | Self::Stax
            // | Self::Stripebilling
            | Self::Taxjar
            // | Self::Thunes
            | Self::Trustpay
            | Self::Tsys
            // | Self::UnifiedAuthenticationService
            | Self::Volt
            | Self::Wellsfargo
            // | Self::Wellsfargopayout
            | Self::Wise
            | Self::Worldline
            | Self::Worldpay
            | Self::Xendit
            | Self::Zen
            | Self::Zsl
            | Self::Signifyd
            | Self::Plaid
            | Self::Razorpay
            | Self::Riskified
            | Self::Threedsecureio
            | Self::Netcetera
            | Self::CtpMastercard
            | Self::CtpVisa
            | Self::Noon
            | Self::Stripe
            | Self::Datatrans => false,
            Self::Checkout | Self::Nmi |Self::Cybersource => true,
        }
    }

    pub fn is_pre_processing_required_before_authorize(self) -> bool {
        matches!(self, Self::Airwallex)
    }

    pub fn get_payment_methods_supporting_extended_authorization(self) -> HashSet<PaymentMethod> {
        HashSet::new()
    }
    pub fn get_payment_method_types_supporting_extended_authorization(
        self,
    ) -> HashSet<PaymentMethodType> {
        HashSet::new()
    }

    pub fn should_acknowledge_webhook_for_resource_not_found_errors(self) -> bool {
        matches!(self, Self::Adyenplatform)
    }

    /// Validates if dummy connector can be created
    /// Dummy connectors can be created only if dummy_connector feature is enabled in the configs
    #[cfg(feature = "dummy_connector")]
    pub fn validate_dummy_connector_create(self, is_dummy_connector_enabled: bool) -> bool {
        matches!(
            self,
            Self::DummyConnector1
                | Self::DummyConnector2
                | Self::DummyConnector3
                | Self::DummyConnector4
                | Self::DummyConnector5
                | Self::DummyConnector6
                | Self::DummyConnector7
        ) && !is_dummy_connector_enabled
    }
}

/// Convert the RoutableConnectors to Connector
impl From<RoutableConnectors> for Connector {
    fn from(routable_connector: RoutableConnectors) -> Self {
        match routable_connector {
            RoutableConnectors::Adyenplatform => Self::Adyenplatform,
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
            RoutableConnectors::Airwallex => Self::Airwallex,
            RoutableConnectors::Authorizedotnet => Self::Authorizedotnet,
            RoutableConnectors::Bankofamerica => Self::Bankofamerica,
            RoutableConnectors::Billwerk => Self::Billwerk,
            RoutableConnectors::Bitpay => Self::Bitpay,
            RoutableConnectors::Bambora => Self::Bambora,
            RoutableConnectors::Bamboraapac => Self::Bamboraapac,
            RoutableConnectors::Bluesnap => Self::Bluesnap,
            RoutableConnectors::Boku => Self::Boku,
            RoutableConnectors::Braintree => Self::Braintree,
            RoutableConnectors::Cashtocode => Self::Cashtocode,
            RoutableConnectors::Chargebee => Self::Chargebee,
            RoutableConnectors::Checkout => Self::Checkout,
            RoutableConnectors::Coinbase => Self::Coinbase,
            RoutableConnectors::Cryptopay => Self::Cryptopay,
            RoutableConnectors::Cybersource => Self::Cybersource,
            RoutableConnectors::Datatrans => Self::Datatrans,
            RoutableConnectors::Deutschebank => Self::Deutschebank,
            RoutableConnectors::Digitalvirgo => Self::Digitalvirgo,
            RoutableConnectors::Dlocal => Self::Dlocal,
            RoutableConnectors::Ebanx => Self::Ebanx,
            RoutableConnectors::Elavon => Self::Elavon,
            // RoutableConnectors::Facilitapay => Self::Facilitapay,
            RoutableConnectors::Fiserv => Self::Fiserv,
            RoutableConnectors::Fiservemea => Self::Fiservemea,
            RoutableConnectors::Fiuu => Self::Fiuu,
            RoutableConnectors::Forte => Self::Forte,
            RoutableConnectors::Getnet => Self::Getnet,
            RoutableConnectors::Globalpay => Self::Globalpay,
            RoutableConnectors::Globepay => Self::Globepay,
            RoutableConnectors::Gocardless => Self::Gocardless,
            RoutableConnectors::Helcim => Self::Helcim,
            RoutableConnectors::Iatapay => Self::Iatapay,
            RoutableConnectors::Itaubank => Self::Itaubank,
            RoutableConnectors::Jpmorgan => Self::Jpmorgan,
            RoutableConnectors::Klarna => Self::Klarna,
            RoutableConnectors::Mifinity => Self::Mifinity,
            RoutableConnectors::Mollie => Self::Mollie,
            RoutableConnectors::Moneris => Self::Moneris,
            RoutableConnectors::Multisafepay => Self::Multisafepay,
            RoutableConnectors::Nexinets => Self::Nexinets,
            RoutableConnectors::Nexixpay => Self::Nexixpay,
            RoutableConnectors::Nmi => Self::Nmi,
            RoutableConnectors::Nomupay => Self::Nomupay,
            RoutableConnectors::Noon => Self::Noon,
            RoutableConnectors::Novalnet => Self::Novalnet,
            RoutableConnectors::Nuvei => Self::Nuvei,
            RoutableConnectors::Opennode => Self::Opennode,
            RoutableConnectors::Paybox => Self::Paybox,
            RoutableConnectors::Payme => Self::Payme,
            RoutableConnectors::Payone => Self::Payone,
            RoutableConnectors::Paypal => Self::Paypal,
            RoutableConnectors::Paystack => Self::Paystack,
            RoutableConnectors::Payu => Self::Payu,
            RoutableConnectors::Placetopay => Self::Placetopay,
            RoutableConnectors::Powertranz => Self::Powertranz,
            RoutableConnectors::Prophetpay => Self::Prophetpay,
            RoutableConnectors::Rapyd => Self::Rapyd,
            RoutableConnectors::Razorpay => Self::Razorpay,
            RoutableConnectors::Recurly => Self::Recurly,
            RoutableConnectors::Redsys => Self::Redsys,
            RoutableConnectors::Riskified => Self::Riskified,
            RoutableConnectors::Shift4 => Self::Shift4,
            RoutableConnectors::Signifyd => Self::Signifyd,
            RoutableConnectors::Square => Self::Square,
            RoutableConnectors::Stax => Self::Stax,
            RoutableConnectors::Stripe => Self::Stripe,
            // RoutableConnectors::Stripebilling => Self::Stripebilling,
            RoutableConnectors::Trustpay => Self::Trustpay,
            RoutableConnectors::Tsys => Self::Tsys,
            RoutableConnectors::Volt => Self::Volt,
            RoutableConnectors::Wellsfargo => Self::Wellsfargo,
            RoutableConnectors::Wise => Self::Wise,
            RoutableConnectors::Worldline => Self::Worldline,
            RoutableConnectors::Worldpay => Self::Worldpay,
            RoutableConnectors::Zen => Self::Zen,
            RoutableConnectors::Plaid => Self::Plaid,
            RoutableConnectors::Zsl => Self::Zsl,
            RoutableConnectors::Xendit => Self::Xendit,
            RoutableConnectors::Inespay => Self::Inespay,
            RoutableConnectors::Coingate => Self::Coingate,
            RoutableConnectors::Hipay => Self::Hipay,
        }
    }
}

impl TryFrom<Connector> for RoutableConnectors {
    type Error = &'static str;

    fn try_from(connector: Connector) -> Result<Self, Self::Error> {
        match connector {
            Connector::Adyenplatform => Ok(Self::Adyenplatform),
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
            Connector::Airwallex => Ok(Self::Airwallex),
            Connector::Authorizedotnet => Ok(Self::Authorizedotnet),
            Connector::Bankofamerica => Ok(Self::Bankofamerica),
            Connector::Billwerk => Ok(Self::Billwerk),
            Connector::Bitpay => Ok(Self::Bitpay),
            Connector::Bambora => Ok(Self::Bambora),
            Connector::Bamboraapac => Ok(Self::Bamboraapac),
            Connector::Bluesnap => Ok(Self::Bluesnap),
            Connector::Boku => Ok(Self::Boku),
            Connector::Braintree => Ok(Self::Braintree),
            Connector::Cashtocode => Ok(Self::Cashtocode),
            Connector::Chargebee => Ok(Self::Chargebee),
            Connector::Checkout => Ok(Self::Checkout),
            Connector::Coinbase => Ok(Self::Coinbase),
            Connector::Coingate => Ok(Self::Coingate),
            Connector::Cryptopay => Ok(Self::Cryptopay),
            Connector::Cybersource => Ok(Self::Cybersource),
            Connector::Datatrans => Ok(Self::Datatrans),
            Connector::Deutschebank => Ok(Self::Deutschebank),
            Connector::Digitalvirgo => Ok(Self::Digitalvirgo),
            Connector::Dlocal => Ok(Self::Dlocal),
            Connector::Ebanx => Ok(Self::Ebanx),
            Connector::Elavon => Ok(Self::Elavon),
            // Connector::Facilitapay => Ok(Self::Facilitapay),
            Connector::Fiserv => Ok(Self::Fiserv),
            Connector::Fiservemea => Ok(Self::Fiservemea),
            Connector::Fiuu => Ok(Self::Fiuu),
            Connector::Forte => Ok(Self::Forte),
            Connector::Globalpay => Ok(Self::Globalpay),
            Connector::Globepay => Ok(Self::Globepay),
            Connector::Gocardless => Ok(Self::Gocardless),
            Connector::Helcim => Ok(Self::Helcim),
            Connector::Iatapay => Ok(Self::Iatapay),
            Connector::Itaubank => Ok(Self::Itaubank),
            Connector::Jpmorgan => Ok(Self::Jpmorgan),
            Connector::Klarna => Ok(Self::Klarna),
            Connector::Mifinity => Ok(Self::Mifinity),
            Connector::Mollie => Ok(Self::Mollie),
            Connector::Moneris => Ok(Self::Moneris),
            Connector::Multisafepay => Ok(Self::Multisafepay),
            Connector::Nexinets => Ok(Self::Nexinets),
            Connector::Nexixpay => Ok(Self::Nexixpay),
            Connector::Nmi => Ok(Self::Nmi),
            Connector::Nomupay => Ok(Self::Nomupay),
            Connector::Noon => Ok(Self::Noon),
            Connector::Novalnet => Ok(Self::Novalnet),
            Connector::Nuvei => Ok(Self::Nuvei),
            Connector::Opennode => Ok(Self::Opennode),
            Connector::Paybox => Ok(Self::Paybox),
            Connector::Payme => Ok(Self::Payme),
            Connector::Payone => Ok(Self::Payone),
            Connector::Paypal => Ok(Self::Paypal),
            Connector::Paystack => Ok(Self::Paystack),
            Connector::Payu => Ok(Self::Payu),
            Connector::Placetopay => Ok(Self::Placetopay),
            Connector::Powertranz => Ok(Self::Powertranz),
            Connector::Prophetpay => Ok(Self::Prophetpay),
            Connector::Rapyd => Ok(Self::Rapyd),
            Connector::Razorpay => Ok(Self::Razorpay),
            Connector::Riskified => Ok(Self::Riskified),
            Connector::Shift4 => Ok(Self::Shift4),
            Connector::Signifyd => Ok(Self::Signifyd),
            Connector::Square => Ok(Self::Square),
            Connector::Stax => Ok(Self::Stax),
            Connector::Stripe => Ok(Self::Stripe),
            Connector::Trustpay => Ok(Self::Trustpay),
            Connector::Tsys => Ok(Self::Tsys),
            Connector::Volt => Ok(Self::Volt),
            Connector::Wellsfargo => Ok(Self::Wellsfargo),
            Connector::Wise => Ok(Self::Wise),
            Connector::Worldline => Ok(Self::Worldline),
            Connector::Worldpay => Ok(Self::Worldpay),
            Connector::Xendit => Ok(Self::Xendit),
            Connector::Zen => Ok(Self::Zen),
            Connector::Plaid => Ok(Self::Plaid),
            Connector::Zsl => Ok(Self::Zsl),
            Connector::Recurly => Ok(Self::Recurly),
            Connector::Getnet => Ok(Self::Getnet),
            Connector::Hipay => Ok(Self::Hipay),
            Connector::Inespay => Ok(Self::Inespay),
            Connector::Redsys => Ok(Self::Redsys),
            Connector::CtpMastercard
            | Connector::Gpayments
            | Connector::Juspaythreedsserver
            | Connector::Netcetera
            | Connector::Taxjar
            | Connector::Threedsecureio
            | Connector::CtpVisa => Err("Invalid conversion. Not a routable connector"),
        }
    }
}
