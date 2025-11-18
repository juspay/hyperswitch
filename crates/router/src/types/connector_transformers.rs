use api_models::enums as api_enums;

use super::ForeignTryFrom;

impl ForeignTryFrom<api_enums::Connector> for common_enums::RoutableConnectors {
    type Error = error_stack::Report<common_utils::errors::ValidationError>;

    fn foreign_try_from(from: api_enums::Connector) -> Result<Self, Self::Error> {
        Ok(match from {
            api_enums::Connector::Aci => Self::Aci,
            api_enums::Connector::Adyen => Self::Adyen,
            api_enums::Connector::Affirm => Self::Affirm,
            api_enums::Connector::Adyenplatform => Self::Adyenplatform,
            api_enums::Connector::Airwallex => Self::Airwallex,
            api_enums::Connector::Amazonpay => Self::Amazonpay,
            api_enums::Connector::Archipel => Self::Archipel,
            api_enums::Connector::Authipay => Self::Authipay,
            api_enums::Connector::Authorizedotnet => Self::Authorizedotnet,
            api_enums::Connector::Bambora => Self::Bambora,
            api_enums::Connector::Bamboraapac => Self::Bamboraapac,
            api_enums::Connector::Bankofamerica => Self::Bankofamerica,
            api_enums::Connector::Barclaycard => Self::Barclaycard,
            api_enums::Connector::Billwerk => Self::Billwerk,
            api_enums::Connector::Bitpay => Self::Bitpay,
            api_enums::Connector::Bluesnap => Self::Bluesnap,
            api_enums::Connector::Blackhawknetwork => Self::Blackhawknetwork,
            api_enums::Connector::Calida => Self::Calida,
            api_enums::Connector::Boku => Self::Boku,
            api_enums::Connector::Braintree => Self::Braintree,
            api_enums::Connector::Breadpay => Self::Breadpay,
            api_enums::Connector::Cardinal => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "cardinal is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Cashtocode => Self::Cashtocode,
            api_enums::Connector::Celero => Self::Celero,
            api_enums::Connector::Chargebee => Self::Chargebee,
            api_enums::Connector::Checkbook => Self::Checkbook,
            api_enums::Connector::Checkout => Self::Checkout,
            api_enums::Connector::Coinbase => Self::Coinbase,
            api_enums::Connector::Coingate => Self::Coingate,
            api_enums::Connector::Cryptopay => Self::Cryptopay,
            api_enums::Connector::Custombilling => Self::Custombilling,
            api_enums::Connector::CtpVisa => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "ctp visa is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::CtpMastercard => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "ctp mastercard is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Cybersource => Self::Cybersource,
            api_enums::Connector::Datatrans => Self::Datatrans,
            api_enums::Connector::Deutschebank => Self::Deutschebank,
            api_enums::Connector::Digitalvirgo => Self::Digitalvirgo,
            api_enums::Connector::Dlocal => Self::Dlocal,
            api_enums::Connector::Dwolla => Self::Dwolla,
            api_enums::Connector::Ebanx => Self::Ebanx,
            api_enums::Connector::Elavon => Self::Elavon,
            api_enums::Connector::Facilitapay => Self::Facilitapay,
            api_enums::Connector::Finix => Self::Finix,
            api_enums::Connector::Fiserv => Self::Fiserv,
            api_enums::Connector::Fiservemea => Self::Fiservemea,
            api_enums::Connector::Fiuu => Self::Fiuu,
            api_enums::Connector::Flexiti => Self::Flexiti,
            api_enums::Connector::Forte => Self::Forte,
            api_enums::Connector::Getnet => Self::Getnet,
            api_enums::Connector::Gigadat => Self::Gigadat,
            api_enums::Connector::Globalpay => Self::Globalpay,
            api_enums::Connector::Globepay => Self::Globepay,
            api_enums::Connector::Gocardless => Self::Gocardless,
            api_enums::Connector::Gpayments => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "gpayments is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Hipay => Self::Hipay,
            api_enums::Connector::Helcim => Self::Helcim,
            api_enums::Connector::HyperswitchVault => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "Hyperswitch Vault is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Iatapay => Self::Iatapay,
            api_enums::Connector::Inespay => Self::Inespay,
            api_enums::Connector::Itaubank => Self::Itaubank,
            api_enums::Connector::Jpmorgan => Self::Jpmorgan,
            api_enums::Connector::Juspaythreedsserver => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "juspaythreedsserver is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Klarna => Self::Klarna,
            api_enums::Connector::Loonio => Self::Loonio,
            api_enums::Connector::Mifinity => Self::Mifinity,
            api_enums::Connector::Mollie => Self::Mollie,
            api_enums::Connector::Moneris => Self::Moneris,
            api_enums::Connector::Multisafepay => Self::Multisafepay,
            api_enums::Connector::Netcetera => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "netcetera is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Nexinets => Self::Nexinets,
            api_enums::Connector::Nexixpay => Self::Nexixpay,
            api_enums::Connector::Nmi => Self::Nmi,
            api_enums::Connector::Nomupay => Self::Nomupay,
            api_enums::Connector::Noon => Self::Noon,
            api_enums::Connector::Nordea => Self::Nordea,
            api_enums::Connector::Novalnet => Self::Novalnet,
            api_enums::Connector::Nuvei => Self::Nuvei,
            api_enums::Connector::Opennode => Self::Opennode,
            api_enums::Connector::Paybox => Self::Paybox,
            api_enums::Connector::Payjustnow => Self::Payjustnow,
            api_enums::Connector::Payload => Self::Payload,
            api_enums::Connector::Payme => Self::Payme,
            api_enums::Connector::Payone => Self::Payone,
            api_enums::Connector::Paypal => Self::Paypal,
            api_enums::Connector::Paysafe => Self::Paysafe,
            api_enums::Connector::Paystack => Self::Paystack,
            api_enums::Connector::Payu => Self::Payu,
            api_enums::Connector::Peachpayments => Self::Peachpayments,
            api_models::enums::Connector::Placetopay => Self::Placetopay,
            api_enums::Connector::Plaid => Self::Plaid,
            api_enums::Connector::Powertranz => Self::Powertranz,
            api_enums::Connector::Prophetpay => Self::Prophetpay,
            api_enums::Connector::Rapyd => Self::Rapyd,
            api_enums::Connector::Razorpay => Self::Razorpay,
            api_enums::Connector::Recurly => Self::Recurly,
            api_enums::Connector::Redsys => Self::Redsys,
            api_enums::Connector::Santander => Self::Santander,
            api_enums::Connector::Shift4 => Self::Shift4,
            api_enums::Connector::Zift => Self::Zift,
            api_enums::Connector::Silverflow => Self::Silverflow,
            api_enums::Connector::Signifyd => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "signifyd is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Riskified => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "riskified is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Square => Self::Square,
            api_enums::Connector::Stax => Self::Stax,
            api_enums::Connector::Stripe => Self::Stripe,
            api_enums::Connector::Stripebilling => Self::Stripebilling,
            // api_enums::Connector::Thunes => Self::Thunes,
            api_enums::Connector::Tesouro => Self::Tesouro,
            api_enums::Connector::Tokenex => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "Tokenex is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Tokenio => Self::Tokenio,
            api_enums::Connector::Trustpay => Self::Trustpay,
            api_enums::Connector::Trustpayments => Self::Trustpayments,
            api_enums::Connector::Tsys => Self::Tsys,
            // api_enums::Connector::UnifiedAuthenticationService => {
            //     Self::UnifiedAuthenticationService
            // }
            api_enums::Connector::Vgs => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "Vgs is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Volt => Self::Volt,
            api_enums::Connector::Wellsfargo => Self::Wellsfargo,
            // api_enums::Connector::Wellsfargopayout => Self::Wellsfargopayout,
            api_enums::Connector::Wise => Self::Wise,
            api_enums::Connector::Worldline => Self::Worldline,
            api_enums::Connector::Worldpay => Self::Worldpay,
            api_enums::Connector::Worldpayvantiv => Self::Worldpayvantiv,
            api_enums::Connector::Worldpayxml => Self::Worldpayxml,
            api_enums::Connector::Xendit => Self::Xendit,
            api_enums::Connector::Zen => Self::Zen,
            api_enums::Connector::Zsl => Self::Zsl,
            #[cfg(feature = "dummy_connector")]
            api_enums::Connector::DummyBillingConnector => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "stripe_billing_test is not a routable connector".to_string(),
                })?
            }
            #[cfg(feature = "dummy_connector")]
            api_enums::Connector::DummyConnector1 => Self::DummyConnector1,
            #[cfg(feature = "dummy_connector")]
            api_enums::Connector::DummyConnector2 => Self::DummyConnector2,
            #[cfg(feature = "dummy_connector")]
            api_enums::Connector::DummyConnector3 => Self::DummyConnector3,
            #[cfg(feature = "dummy_connector")]
            api_enums::Connector::DummyConnector4 => Self::DummyConnector4,
            #[cfg(feature = "dummy_connector")]
            api_enums::Connector::DummyConnector5 => Self::DummyConnector5,
            #[cfg(feature = "dummy_connector")]
            api_enums::Connector::DummyConnector6 => Self::DummyConnector6,
            #[cfg(feature = "dummy_connector")]
            api_enums::Connector::DummyConnector7 => Self::DummyConnector7,
            api_enums::Connector::Threedsecureio => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "threedsecureio is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Taxjar => {
                Err(common_utils::errors::ValidationError::InvalidValue {
                    message: "Taxjar is not a routable connector".to_string(),
                })?
            }
            api_enums::Connector::Phonepe => Self::Phonepe,
            api_enums::Connector::Paytm => Self::Paytm,
        })
    }
}
