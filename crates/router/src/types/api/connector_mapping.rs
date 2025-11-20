use std::str::FromStr;

use error_stack::{report, ResultExt};
use hyperswitch_connectors::connectors::{Paytm, Phonepe};

use crate::{
    configs::settings::Connectors,
    connector,
    core::errors::{self, CustomResult},
    services::connector_integration_interface::ConnectorEnum,
    types::{self, api::enums},
};

/// Routing algorithm will output merchant connector identifier instead of connector name
/// In order to support backwards compatibility for older routing algorithms and merchant accounts
/// the support for connector name is retained
#[derive(Clone, Debug)]
pub struct ConnectorData {
    pub connector: ConnectorEnum,
    pub connector_name: types::Connector,
    pub get_token: GetToken,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

// Normal flow will call the connector and follow the flow specific operations (capture, authorize)
// SessionTokenFromMetadata will avoid calling the connector instead create the session token ( for sdk )
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum GetToken {
    GpayMetadata,
    SamsungPayMetadata,
    AmazonPayMetadata,
    ApplePayMetadata,
    PaypalSdkMetadata,
    PazeMetadata,
    Connector,
}

impl ConnectorData {
    pub fn get_connector_by_name(
        _connectors: &Connectors,
        name: &str,
        connector_type: GetToken,
        connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    ) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector = Self::convert_connector(name)?;
        let connector_name = enums::Connector::from_str(name)
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| format!("unable to parse connector name {name}"))?;
        Ok(Self {
            connector,
            connector_name,
            get_token: connector_type,
            merchant_connector_id: connector_id,
        })
    }

    #[cfg(feature = "payouts")]
    pub fn get_payout_connector_by_name(
        _connectors: &Connectors,
        name: &str,
        connector_type: GetToken,
        connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    ) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector = Self::convert_connector(name)?;
        let payout_connector_name = enums::PayoutConnectors::from_str(name)
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| format!("unable to parse payout connector name {name}"))?;
        let connector_name = enums::Connector::from(payout_connector_name);
        Ok(Self {
            connector,
            connector_name,
            get_token: connector_type,
            merchant_connector_id: connector_id,
        })
    }

    pub fn get_external_vault_connector_by_name(
        _connectors: &Connectors,
        connector: String,
        connector_type: GetToken,
        connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    ) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector_enum = Self::convert_connector(&connector)?;
        let external_vault_connector_name = enums::VaultConnectors::from_str(&connector)
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!("unable to parse external vault connector name {connector:?}")
            })?;
        let connector_name = enums::Connector::from(external_vault_connector_name);
        Ok(Self {
            connector: connector_enum,
            connector_name,
            get_token: connector_type,
            merchant_connector_id: connector_id,
        })
    }

    pub fn convert_connector(
        connector_name: &str,
    ) -> CustomResult<ConnectorEnum, errors::ApiErrorResponse> {
        match enums::Connector::from_str(connector_name) {
            Ok(name) => match name {
                enums::Connector::Aci => Ok(ConnectorEnum::Old(Box::new(connector::Aci::new()))),
                enums::Connector::Adyen => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Adyen::new())))
                }
                enums::Connector::Affirm => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Affirm::new())))
                }
                enums::Connector::Adyenplatform => Ok(ConnectorEnum::Old(Box::new(
                    connector::Adyenplatform::new(),
                ))),
                enums::Connector::Airwallex => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Airwallex::new())))
                }
                enums::Connector::Amazonpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Amazonpay::new())))
                }
                enums::Connector::Archipel => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Archipel::new())))
                }
                enums::Connector::Authipay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Authipay::new())))
                }
                enums::Connector::Authorizedotnet => Ok(ConnectorEnum::Old(Box::new(
                    connector::Authorizedotnet::new(),
                ))),
                enums::Connector::Bambora => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Bambora::new())))
                }
                enums::Connector::Bamboraapac => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Bamboraapac::new())))
                }
                enums::Connector::Bankofamerica => Ok(ConnectorEnum::Old(Box::new(
                    connector::Bankofamerica::new(),
                ))),
                enums::Connector::Barclaycard => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Barclaycard::new())))
                }
                enums::Connector::Billwerk => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Billwerk::new())))
                }
                enums::Connector::Bitpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Bitpay::new())))
                }
                enums::Connector::Blackhawknetwork => Ok(ConnectorEnum::Old(Box::new(
                    connector::Blackhawknetwork::new(),
                ))),
                enums::Connector::Bluesnap => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Bluesnap::new())))
                }
                enums::Connector::Calida => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Calida::new())))
                }
                enums::Connector::Boku => Ok(ConnectorEnum::Old(Box::new(connector::Boku::new()))),
                enums::Connector::Braintree => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Braintree::new())))
                }
                enums::Connector::Breadpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Breadpay::new())))
                }
                enums::Connector::Cashtocode => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Cashtocode::new())))
                }
                enums::Connector::Celero => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Celero::new())))
                }
                enums::Connector::Chargebee => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Chargebee::new())))
                }
                enums::Connector::Checkbook => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Checkbook::new())))
                }
                enums::Connector::Checkout => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Checkout::new())))
                }
                enums::Connector::Coinbase => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Coinbase::new())))
                }
                enums::Connector::Coingate => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Coingate::new())))
                }
                enums::Connector::Zift => Ok(ConnectorEnum::Old(Box::new(connector::Zift::new()))),
                enums::Connector::Cryptopay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Cryptopay::new())))
                }
                enums::Connector::CtpMastercard => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::CtpMastercard)))
                }
                enums::Connector::Custombilling => Ok(ConnectorEnum::Old(Box::new(
                    connector::Custombilling::new(),
                ))),
                enums::Connector::CtpVisa => Ok(ConnectorEnum::Old(Box::new(
                    connector::UnifiedAuthenticationService::new(),
                ))),
                enums::Connector::Cybersource => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Cybersource::new())))
                }
                enums::Connector::Datatrans => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Datatrans::new())))
                }
                enums::Connector::Deutschebank => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Deutschebank::new())))
                }
                enums::Connector::Digitalvirgo => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Digitalvirgo::new())))
                }
                enums::Connector::Dlocal => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Dlocal::new())))
                }
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector1 => Ok(ConnectorEnum::Old(Box::new(
                    connector::DummyConnector::<1>::new(),
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector2 => Ok(ConnectorEnum::Old(Box::new(
                    connector::DummyConnector::<2>::new(),
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector3 => Ok(ConnectorEnum::Old(Box::new(
                    connector::DummyConnector::<3>::new(),
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector4 => Ok(ConnectorEnum::Old(Box::new(
                    connector::DummyConnector::<4>::new(),
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector5 => Ok(ConnectorEnum::Old(Box::new(
                    connector::DummyConnector::<5>::new(),
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector6 => Ok(ConnectorEnum::Old(Box::new(
                    connector::DummyConnector::<6>::new(),
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector7 => Ok(ConnectorEnum::Old(Box::new(
                    connector::DummyConnector::<7>::new(),
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyBillingConnector => Ok(ConnectorEnum::Old(Box::new(
                    connector::DummyConnector::<8>::new(),
                ))),
                enums::Connector::Dwolla => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Dwolla::new())))
                }
                enums::Connector::Ebanx => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Ebanx::new())))
                }
                enums::Connector::Elavon => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Elavon::new())))
                }
                enums::Connector::Facilitapay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Facilitapay::new())))
                }
                enums::Connector::Finix => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Finix::new())))
                }
                enums::Connector::Fiserv => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Fiserv::new())))
                }
                enums::Connector::Fiservemea => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Fiservemea::new())))
                }
                enums::Connector::Fiuu => Ok(ConnectorEnum::Old(Box::new(connector::Fiuu::new()))),
                enums::Connector::Flexiti => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Flexiti::new())))
                }
                enums::Connector::Forte => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Forte::new())))
                }
                enums::Connector::Getnet => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Getnet::new())))
                }
                enums::Connector::Gigadat => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Gigadat::new())))
                }
                enums::Connector::Globalpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Globalpay::new())))
                }
                enums::Connector::Globepay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Globepay::new())))
                }
                enums::Connector::Gocardless => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Gocardless::new())))
                }
                enums::Connector::Hipay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Hipay::new())))
                }
                enums::Connector::Helcim => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Helcim::new())))
                }
                enums::Connector::HyperswitchVault => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::HyperswitchVault)))
                }
                enums::Connector::Iatapay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Iatapay::new())))
                }
                enums::Connector::Inespay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Inespay::new())))
                }
                enums::Connector::Itaubank => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Itaubank::new())))
                }
                enums::Connector::Jpmorgan => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Jpmorgan::new())))
                }
                enums::Connector::Juspaythreedsserver => Ok(ConnectorEnum::Old(Box::new(
                    connector::Juspaythreedsserver::new(),
                ))),
                enums::Connector::Klarna => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Klarna::new())))
                }
                enums::Connector::Loonio => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Loonio::new())))
                }
                enums::Connector::Mollie => {
                    // enums::Connector::Moneris => Ok(ConnectorEnum::Old(Box::new(connector::Moneris))),
                    Ok(ConnectorEnum::Old(Box::new(connector::Mollie::new())))
                }
                enums::Connector::Moneris => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Moneris::new())))
                }
                enums::Connector::Nexixpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Nexixpay::new())))
                }
                enums::Connector::Nmi => Ok(ConnectorEnum::Old(Box::new(connector::Nmi::new()))),
                enums::Connector::Nomupay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Nomupay::new())))
                }
                enums::Connector::Noon => Ok(ConnectorEnum::Old(Box::new(connector::Noon::new()))),
                enums::Connector::Nordea => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Nordea::new())))
                }
                enums::Connector::Novalnet => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Novalnet::new())))
                }
                enums::Connector::Nuvei => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Nuvei::new())))
                }
                enums::Connector::Opennode => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Opennode::new())))
                }
                enums::Connector::Paybox => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Paybox::new())))
                }
                // "payeezy" => Ok(ConnectorIntegrationEnum::Old(Box::new(&connector::Payeezy)), As psync and rsync are not supported by this connector, it is added as template code for future usage
                // enums::Connector::Payload => {
                //     Ok(ConnectorEnum::Old(Box::new(connector::Paybload::new())))
                // }
                enums::Connector::Payload => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Payload::new())))
                }
                enums::Connector::Payjustnow => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Payjustnow::new())))
                }
                enums::Connector::Payme => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Payme::new())))
                }
                enums::Connector::Payone => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Payone::new())))
                }
                enums::Connector::Payu => Ok(ConnectorEnum::Old(Box::new(connector::Payu::new()))),
                enums::Connector::Peachpayments => Ok(ConnectorEnum::Old(Box::new(
                    hyperswitch_connectors::connectors::Peachpayments::new(),
                ))),
                enums::Connector::Placetopay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Placetopay::new())))
                }
                enums::Connector::Powertranz => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Powertranz::new())))
                }
                enums::Connector::Prophetpay => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Prophetpay)))
                }
                enums::Connector::Razorpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Razorpay::new())))
                }
                enums::Connector::Rapyd => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Rapyd::new())))
                }
                enums::Connector::Recurly => {
                    Ok(ConnectorEnum::New(Box::new(connector::Recurly::new())))
                }
                enums::Connector::Redsys => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Redsys::new())))
                }
                enums::Connector::Santander => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Santander::new())))
                }
                enums::Connector::Shift4 => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Shift4::new())))
                }
                enums::Connector::Silverflow => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Silverflow::new())))
                }
                enums::Connector::Square => Ok(ConnectorEnum::Old(Box::new(&connector::Square))),
                enums::Connector::Stax => Ok(ConnectorEnum::Old(Box::new(&connector::Stax))),
                enums::Connector::Stripe => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Stripe::new())))
                }
                enums::Connector::Stripebilling => Ok(ConnectorEnum::Old(Box::new(
                    connector::Stripebilling::new(),
                ))),
                enums::Connector::Wise => Ok(ConnectorEnum::Old(Box::new(connector::Wise::new()))),
                enums::Connector::Worldline => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Worldline)))
                }
                enums::Connector::Worldpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Worldpay::new())))
                }
                enums::Connector::Worldpayvantiv => Ok(ConnectorEnum::Old(Box::new(
                    connector::Worldpayvantiv::new(),
                ))),
                enums::Connector::Worldpayxml => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Worldpayxml::new())))
                }
                enums::Connector::Xendit => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Xendit::new())))
                }
                enums::Connector::Mifinity => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Mifinity::new())))
                }
                enums::Connector::Multisafepay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Multisafepay::new())))
                }
                enums::Connector::Netcetera => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Netcetera)))
                }
                enums::Connector::Nexinets => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Nexinets)))
                }
                // enums::Connector::Nexixpay => {
                //     Ok(ConnectorEnum::Old(Box::new(&connector::Nexixpay)))
                // }
                enums::Connector::Paypal => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Paypal::new())))
                }
                enums::Connector::Paysafe => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Paysafe::new())))
                }
                enums::Connector::Paystack => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Paystack::new())))
                }
                // enums::Connector::Thunes => Ok(ConnectorEnum::Old(Box::new(connector::Thunes))),
                enums::Connector::Tesouro => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Tesouro::new())))
                }
                enums::Connector::Tokenex => Ok(ConnectorEnum::Old(Box::new(&connector::Tokenex))),
                enums::Connector::Tokenio => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Tokenio::new())))
                }
                enums::Connector::Trustpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Trustpay::new())))
                }
                enums::Connector::Trustpayments => Ok(ConnectorEnum::Old(Box::new(
                    connector::Trustpayments::new(),
                ))),
                enums::Connector::Tsys => Ok(ConnectorEnum::Old(Box::new(connector::Tsys::new()))),
                // enums::Connector::UnifiedAuthenticationService => Ok(ConnectorEnum::Old(Box::new(
                //     connector::UnifiedAuthenticationService,
                // ))),
                enums::Connector::Vgs => Ok(ConnectorEnum::Old(Box::new(&connector::Vgs))),
                enums::Connector::Volt => Ok(ConnectorEnum::Old(Box::new(connector::Volt::new()))),
                enums::Connector::Wellsfargo => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Wellsfargo::new())))
                }

                // enums::Connector::Wellsfargopayout => {
                //     Ok(Box::new(connector::Wellsfargopayout::new()))
                // }
                enums::Connector::Zen => Ok(ConnectorEnum::Old(Box::new(&connector::Zen))),
                enums::Connector::Zsl => Ok(ConnectorEnum::Old(Box::new(&connector::Zsl))),
                enums::Connector::Plaid => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Plaid::new())))
                }
                enums::Connector::Signifyd
                | enums::Connector::Riskified
                | enums::Connector::Gpayments
                | enums::Connector::Threedsecureio
                | enums::Connector::Cardinal
                | enums::Connector::Taxjar => {
                    Err(report!(errors::ConnectorError::InvalidConnectorName)
                        .attach_printable(format!("invalid connector name: {connector_name}")))
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                }
                enums::Connector::Phonepe => Ok(ConnectorEnum::Old(Box::new(Phonepe::new()))),
                enums::Connector::Paytm => Ok(ConnectorEnum::Old(Box::new(Paytm::new()))),
            },
            Err(_) => Err(report!(errors::ConnectorError::InvalidConnectorName)
                .attach_printable(format!("invalid connector name: {connector_name}")))
            .change_context(errors::ApiErrorResponse::InternalServerError),
        }
    }
}
