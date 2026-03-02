use std::collections::HashSet;

use smithy::SmithyModel;
use utoipa::ToSchema;

pub use super::enums::{PaymentMethod, PayoutType};
pub use crate::PaymentMethodType;

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
    Gpayments,
    Hipay,
    Helcim,
    Hyperpg,
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
    // Truelayer,
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
    Worldpaymodular,
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
                | (Self::Santander, _)
        )
    }
    pub fn requires_order_creation_before_payment(self, payment_method: PaymentMethod) -> bool {
        matches!(
            (self, payment_method),
            (Self::Razorpay, PaymentMethod::Upi) | (Self::Airwallex, _) //ordercreation required for all flows in airwallex
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
            | Self::Breadpay
            | Self::Cashtocode
            | Self::Celero
            | Self::Chargebee
            | Self::Checkbook
            | Self::Coinbase
            | Self::Coingate
            | Self::Cryptopay
            | Self::Custombilling
            | Self::Cybersourcedecisionmanager
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
            | Self::Hyperpg
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
            | Self::Revolv3
            | Self::Santander
            | Self::Shift4
            | Self::Silverflow
            | Self::Square
            | Self::Stax
            | Self::Stripebilling
            | Self::Taxjar
            | Self::Tesouro
            // | Self::Thunes
            // | Self::Truelayer
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
            | Self::Worldpaymodular
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
            | Self::Payjustnowinstore
            | Self::Phonepe => false,
            Self::Checkout |Self::Zift| Self::Nmi |Self::Braintree|
            Self::Cybersource | Self::Archipel | Self::Nuvei | Self::Adyen => true,
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
