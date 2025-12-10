use std::collections::HashSet;

use smithy::SmithyModel;
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
    Phonepe,
    Placetopay,
    Powertranz,
    Prophetpay,
    Rapyd,
    Razorpay,
    Recurly,
    Redsys,
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
    SmithyModel,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum Connector {
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
    Bambora,
    Bamboraapac,
    Bankofamerica,
    Barclaycard,
    Billwerk,
    Bitpay,
    Bluesnap,
    Blackhawknetwork,
    #[serde(alias = "bluecode")]
    Calida,
    Boku,
    Braintree,
    Breadpay,
    Cardinal,
    Cashtocode,
    Celero,
    Chargebee,
    Checkbook,
    Checkout,
    Coinbase,
    Coingate,
    Custombilling,
    Cryptopay,
    CtpMastercard,
    CtpVisa,
    Cybersource,
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
    Gpayments,
    Hipay,
    Helcim,
    HyperswitchVault,
    // Hyperwallet, added as template code for future usage
    Inespay,
    Iatapay,
    Itaubank,
    Jpmorgan,
    Juspaythreedsserver,
    Klarna,
    Loonio,
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
    Nordea,
    Novalnet,
    Nuvei,
    // Opayo, added as template code for future usage
    Opennode,
    Paybox,
    // Payeezy, As psync and rsync are not supported by this connector, it is added as template code for future usage
    Payload,
    Payme,
    Payone,
    Paypal,
    Paysafe,
    Paystack,
    Paytm,
    Payu,
    Peachpayments,
    Payjustnow,
    Phonepe,
    Placetopay,
    Powertranz,
    Prophetpay,
    Rapyd,
    Razorpay,
    Recurly,
    Redsys,
    Santander,
    Shift4,
    Silverflow,
    Square,
    Stax,
    Stripe,
    Stripebilling,
    Taxjar,
    Threedsecureio,
    // Tokenio,
    //Thunes,
    Tesouro,
    Tokenex,
    Tokenio,
    Trustpay,
    Trustpayments,
    Tsys,
    // UnifiedAuthenticationService,
    Vgs,
    Volt,
    Wellsfargo,
    // Wellsfargopayout,
    Wise,
    Worldline,
    Worldpay,
    Worldpayvantiv,
    Worldpayxml,
    Signifyd,
    Plaid,
    Riskified,
    Xendit,
    Zen,
    Zift,
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
                | (Self::Loonio, _)
                | (Self::Worldpay, Some(PayoutType::Wallet))
                | (Self::Worldpayxml, Some(PayoutType::Wallet))
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
        matches!(self, Self::Wise | Self::Gigadat)
    }
    #[cfg(feature = "payouts")]
    pub fn supports_access_token_for_payout(self, payout_method: Option<PayoutType>) -> bool {
        matches!((self, payout_method), (Self::Paypal, _))
    }
    #[cfg(feature = "payouts")]
    pub fn supports_access_token_for_external_vault(self) -> bool {
        matches!(self, Self::Vgs)
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
                | (Self::Nordea, _)
                | (Self::Paypal, _)
                | (Self::Payu, _)
                | (
                    Self::Trustpay,
                    PaymentMethod::BankRedirect | PaymentMethod::BankTransfer
                )
                | (Self::Tesouro, _)
                | (Self::Iatapay, _)
                | (Self::Volt, _)
                | (Self::Itaubank, _)
                | (Self::Facilitapay, _)
                | (Self::Dwolla, _)
        )
    }
    pub fn requires_order_creation_before_payment(self, payment_method: PaymentMethod) -> bool {
        matches!(
            (self, payment_method),
            (Self::Razorpay, PaymentMethod::Upi) | (Self::Airwallex, PaymentMethod::Card)
        )
    }
    pub fn supports_file_storage_module(self) -> bool {
        matches!(self, Self::Stripe | Self::Checkout | Self::Worldpayvantiv)
    }
    pub fn requires_defend_dispute(self) -> bool {
        matches!(self, Self::Checkout)
    }
    pub fn is_separate_authentication_supported(self) -> bool {
        match self {
            #[cfg(feature = "dummy_connector")]
            Self::DummyBillingConnector => false,
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
			| Self::Authipay
            | Self::Affirm
            | Self::Adyenplatform
            | Self::Airwallex
            | Self::Amazonpay
            | Self::Authorizedotnet
            | Self::Bambora
            | Self::Bamboraapac
            | Self::Bankofamerica
            | Self::Barclaycard
            | Self::Billwerk
            | Self::Bitpay
            | Self::Bluesnap
            | Self::Blackhawknetwork
            | Self::Calida
            | Self::Boku
            | Self::Braintree
            | Self::Breadpay
            | Self::Cashtocode
            | Self::Celero
            | Self::Chargebee
            | Self::Checkbook
            | Self::Coinbase
            | Self::Coingate
            | Self::Cryptopay
            | Self::Custombilling
            | Self::Deutschebank
            | Self::Digitalvirgo
            | Self::Dlocal
            | Self::Dwolla
            | Self::Ebanx
            | Self::Elavon
            | Self::Facilitapay
            | Self::Finix
            | Self::Fiserv
            | Self::Fiservemea
            | Self::Fiuu
            | Self::Flexiti
            | Self::Forte
            | Self::Getnet
            | Self::Gigadat
            | Self::Globalpay
            | Self::Globepay
            | Self::Gocardless
            | Self::Gpayments
            | Self::Hipay
            | Self::Helcim
            | Self::HyperswitchVault
            | Self::Iatapay
			| Self::Inespay
            | Self::Itaubank
            | Self::Jpmorgan
            | Self::Juspaythreedsserver
            | Self::Klarna
            | Self::Loonio
            | Self::Mifinity
            | Self::Mollie
            | Self::Moneris
            | Self::Multisafepay
            | Self::Nexinets
            | Self::Nexixpay
            | Self::Nomupay
            | Self::Nordea
            | Self::Novalnet
            | Self::Opennode
            | Self::Paybox
            | Self::Payload
            | Self::Payme
            | Self::Payone
            | Self::Paypal
            | Self::Paysafe
            | Self::Paystack
            | Self::Payu
            | Self::Peachpayments
            | Self::Placetopay
            | Self::Powertranz
            | Self::Prophetpay
            | Self::Rapyd
            | Self::Recurly
            | Self::Redsys
            | Self::Santander
            | Self::Shift4
            | Self::Silverflow
            | Self::Square
            | Self::Stax
            | Self::Stripebilling
            | Self::Taxjar
            | Self::Tesouro
            // | Self::Thunes
            | Self::Trustpay
            | Self::Trustpayments
            // | Self::Tokenio
            | Self::Tsys
            // | Self::UnifiedAuthenticationService
            | Self::Vgs
            | Self::Volt
            | Self::Wellsfargo
            // | Self::Wellsfargopayout
            | Self::Wise
            | Self::Worldline
            | Self::Worldpay
            | Self::Worldpayvantiv
            | Self::Worldpayxml
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
            | Self::Cardinal
            | Self::CtpVisa
            | Self::Noon
            | Self::Tokenex
            | Self::Tokenio
            | Self::Stripe
            | Self::Datatrans
            | Self::Paytm
            | Self::Payjustnow
            | Self::Phonepe => false,
            Self::Checkout |Self::Zift| Self::Nmi |Self::Cybersource | Self::Archipel | Self::Nuvei | Self::Adyen => true,
        }
    }

    pub fn get_payment_methods_supporting_extended_authorization(self) -> HashSet<PaymentMethod> {
        HashSet::from([PaymentMethod::Card])
    }
    pub fn get_payment_method_types_supporting_extended_authorization(
        self,
    ) -> HashSet<PaymentMethodType> {
        HashSet::from([PaymentMethodType::Credit, PaymentMethodType::Debit])
    }

    pub fn is_overcapture_supported_by_connector(self) -> bool {
        matches!(self, Self::Stripe | Self::Adyen)
    }

    pub fn should_acknowledge_webhook_for_resource_not_found_errors(self) -> bool {
        matches!(self, Self::Adyenplatform | Self::Adyen)
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
            RoutableConnectors::Trustpay => Self::Trustpay,
            RoutableConnectors::Trustpayments => Self::Trustpayments,
            // RoutableConnectors::Tokenio => Self::Tokenio,
            RoutableConnectors::Tsys => Self::Tsys,
            RoutableConnectors::Volt => Self::Volt,
            RoutableConnectors::Wellsfargo => Self::Wellsfargo,
            RoutableConnectors::Wise => Self::Wise,
            RoutableConnectors::Worldline => Self::Worldline,
            RoutableConnectors::Worldpay => Self::Worldpay,
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
            RoutableConnectors::Juspaythreedsserver => Self::Juspaythreedsserver,
            RoutableConnectors::CtpMastercard => Self::CtpMastercard,
            RoutableConnectors::CtpVisa => Self::CtpVisa,
            RoutableConnectors::Netcetera => Self::Netcetera,
            RoutableConnectors::Cardinal => Self::Cardinal,
            RoutableConnectors::Threedsecureio => Self::Threedsecureio,
        }
    }
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
            Connector::Trustpay => Ok(Self::Trustpay),
            Connector::Trustpayments => Ok(Self::Trustpayments),
            Connector::Tsys => Ok(Self::Tsys),
            Connector::Volt => Ok(Self::Volt),
            Connector::Wellsfargo => Ok(Self::Wellsfargo),
            Connector::Wise => Ok(Self::Wise),
            Connector::Worldline => Ok(Self::Worldline),
            Connector::Worldpay => Ok(Self::Worldpay),
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
            Connector::Paytm => Ok(Self::Paytm),
            Connector::Phonepe => Ok(Self::Phonepe),
            Connector::Payjustnow => Ok(Self::Payjustnow),
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

// Enum representing different status an invoice can have.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum InvoiceStatus {
    InvoiceCreated,
    PaymentPending,
    PaymentPendingTimeout,
    PaymentSucceeded,
    PaymentFailed,
    PaymentCanceled,
    InvoicePaid,
    ManualReview,
    Voided,
}
