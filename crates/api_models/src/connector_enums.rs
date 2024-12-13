pub use common_enums::enums::{PaymentMethod, PayoutType};
#[cfg(feature = "dummy_connector")]
use common_utils::errors;
use common_utils::types;
use utoipa::ToSchema;

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
    Checkout,
    Coinbase,
    Cryptopay,
    Cybersource,
    Datatrans,
    Deutschebank,
    Digitalvirgo,
    Dlocal,
    Ebanx,
    Elavon,
    Fiserv,
    Fiservemea,
    Fiuu,
    Forte,
    Globalpay,
    Globepay,
    Gocardless,
    Gpayments,
    Helcim,
    // Inespay,
    Iatapay,
    Itaubank,
    //Jpmorgan,
    Klarna,
    Mifinity,
    Mollie,
    Multisafepay,
    Netcetera,
    Nexinets,
    Nexixpay,
    Nmi,
    // Nomupay,
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
    Payu,
    Placetopay,
    Powertranz,
    Prophetpay,
    Rapyd,
    Razorpay,
    // Redsys,
    Shift4,
    Square,
    Stax,
    Stripe,
    Taxjar,
    Threedsecureio,
    //Thunes,
    Trustpay,
    Tsys,
    Volt,
    Wellsfargo,
    // Wellsfargopayout,
    Wise,
    Worldline,
    Worldpay,
    Signifyd,
    Plaid,
    Riskified,
    // Xendit,
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
        matches!(self, Self::Stripe)
    }
    pub fn supports_access_token(self, payment_method: PaymentMethod) -> bool {
        matches!(
            (self, payment_method),
            (Self::Airwallex, _)
                | (Self::Deutschebank, _)
                | (Self::Globalpay, _)
                | (Self::Paypal, _)
                | (Self::Payu, _)
                | (Self::Trustpay, PaymentMethod::BankRedirect)
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
            | Self::Coinbase
            | Self::Cryptopay
            | Self::Deutschebank
            | Self::Digitalvirgo
            | Self::Dlocal
            | Self::Ebanx
            | Self::Elavon
            | Self::Fiserv
            | Self::Fiservemea
            | Self::Fiuu
            | Self::Forte
            | Self::Globalpay
            | Self::Globepay
            | Self::Gocardless
            | Self::Gpayments
            | Self::Helcim
            | Self::Iatapay
			// | Self::Inespay
            | Self::Itaubank
            //| Self::Jpmorgan
            | Self::Klarna
            | Self::Mifinity
            | Self::Mollie
            | Self::Multisafepay
            | Self::Nexinets
            | Self::Nexixpay
            // | Self::Nomupay
            | Self::Novalnet
            | Self::Nuvei
            | Self::Opennode
            | Self::Paybox
            | Self::Payme
            | Self::Payone
            | Self::Paypal
            | Self::Payu
            | Self::Placetopay
            | Self::Powertranz
            | Self::Prophetpay
            | Self::Rapyd
			// | Self::Redsys
            | Self::Shift4
            | Self::Square
            | Self::Stax
            | Self::Taxjar
            // | Self::Thunes
            | Self::Trustpay
            | Self::Tsys
            | Self::Volt
            | Self::Wellsfargo
            // | Self::Wellsfargopayout
            | Self::Wise
            | Self::Worldline
            | Self::Worldpay
            // | Self::Xendit
            | Self::Zen
            | Self::Zsl
            | Self::Signifyd
            | Self::Plaid
            | Self::Razorpay
            | Self::Riskified
            | Self::Threedsecureio
            | Self::Datatrans
            | Self::Netcetera
            | Self::Noon
            | Self::Stripe => false,
            Self::Checkout | Self::Nmi | Self::Cybersource => true,
        }
    }
    pub fn is_pre_processing_required_before_authorize(self) -> bool {
        matches!(self, Self::Airwallex)
    }
    pub fn should_acknowledge_webhook_for_resource_not_found_errors(self) -> bool {
        matches!(self, Self::Adyenplatform)
    }
    #[cfg(feature = "dummy_connector")]
    pub fn validate_dummy_connector_enabled(
        self,
        is_dummy_connector_enabled: bool,
    ) -> errors::CustomResult<(), errors::ValidationError> {
        if !is_dummy_connector_enabled
            && matches!(
                self,
                Self::DummyConnector1
                    | Self::DummyConnector2
                    | Self::DummyConnector3
                    | Self::DummyConnector4
                    | Self::DummyConnector5
                    | Self::DummyConnector6
                    | Self::DummyConnector7
            )
        {
            Err(errors::ValidationError::InvalidValue {
                message: "Invalid connector name".to_string(),
            }
            .into())
        } else {
            Ok(())
        }
    }
    pub fn get_request_extended_authorization(
        &self,
        intent_request_extended_authorization_optional: Option<
            types::RequestExtendedAuthorizationBool,
        >,
        always_request_extended_authorization_optional: Option<
            types::AlwaysRequestExtendedAuthorization,
        >,
        payment_method_optional: Option<PaymentMethod>,
        payment_method_type_optional: Option<common_enums::PaymentMethodType>,
    ) -> Option<types::RequestExtendedAuthorizationBool> {
        if always_request_extended_authorization_optional.is_some_and(
            |always_request_extended_authorization| *always_request_extended_authorization,
        ) {
            Some(true)
        } else if intent_request_extended_authorization_optional.is_some_and(
            |intent_request_extended_authorization| *intent_request_extended_authorization,
        ) {
            let supported_pms = self.get_payment_methods_supporting_extended_authorization();
            let supported_pmts = self.get_payment_method_types_supporting_extended_authorization();
            // check if payment method or payment method type is supported by the connector
            Some(
                match (payment_method_optional, payment_method_type_optional) {
                    (Some(payment_method), Some(payment_method_type)) => {
                        supported_pms.contains(&payment_method)
                            && supported_pmts.contains(&payment_method_type)
                    }
                    (Some(payment_method), None) => supported_pms.contains(&payment_method),
                    (None, Some(payment_method_type)) => {
                        supported_pmts.contains(&payment_method_type)
                    }
                    (None, None) => false,
                },
            )
        } else {
            None
        }
        .map(types::RequestExtendedAuthorizationBool::from)
    }
    fn get_payment_methods_supporting_extended_authorization(self) -> Vec<PaymentMethod> {
        vec![]
    }
    fn get_payment_method_types_supporting_extended_authorization(
        self,
    ) -> Vec<common_enums::PaymentMethodType> {
        vec![]
    }
}
