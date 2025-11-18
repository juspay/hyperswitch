use api_models::enums as api_enums;
use common_utils::pii;
use error_stack::ResultExt;
use external_services::http_client::client;
use masking::PeekInterface;
use pm_auth::connector::plaid::transformers::PlaidAuthType;

use crate::{core::errors, types, types::transformers::ForeignTryFrom};

pub struct ConnectorAuthTypeAndMetadataValidation<'a> {
    pub connector_name: &'a api_models::enums::Connector,
    pub auth_type: &'a types::ConnectorAuthType,
    pub connector_meta_data: &'a Option<pii::SecretSerdeValue>,
}

impl ConnectorAuthTypeAndMetadataValidation<'_> {
    pub fn validate_auth_and_metadata_type(
        &self,
    ) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
        let connector_auth_type_validation = ConnectorAuthTypeValidation {
            auth_type: self.auth_type,
        };
        connector_auth_type_validation.validate_connector_auth_type()?;
        self.validate_auth_and_metadata_type_with_connector()
            .map_err(|err| match *err.current_context() {
                errors::ConnectorError::InvalidConnectorName => {
                    err.change_context(errors::ApiErrorResponse::InvalidRequestData {
                        message: "The connector name is invalid".to_string(),
                    })
                }
                errors::ConnectorError::InvalidConnectorConfig { config: field_name } => err
                    .change_context(errors::ApiErrorResponse::InvalidRequestData {
                        message: format!("The {field_name} is invalid"),
                    }),
                errors::ConnectorError::FailedToObtainAuthType => {
                    err.change_context(errors::ApiErrorResponse::InvalidRequestData {
                        message: "The auth type is invalid for the connector".to_string(),
                    })
                }
                _ => err.change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: "The request body is invalid".to_string(),
                }),
            })
    }

    fn validate_auth_and_metadata_type_with_connector(
        &self,
    ) -> Result<(), error_stack::Report<errors::ConnectorError>> {
        use crate::connector::*;

        match self.connector_name {
            api_enums::Connector::Vgs => {
                vgs::transformers::VgsAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Adyenplatform => {
                adyenplatform::transformers::AdyenplatformAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            #[cfg(feature = "dummy_connector")]
            api_enums::Connector::DummyBillingConnector
            | api_enums::Connector::DummyConnector1
            | api_enums::Connector::DummyConnector2
            | api_enums::Connector::DummyConnector3
            | api_enums::Connector::DummyConnector4
            | api_enums::Connector::DummyConnector5
            | api_enums::Connector::DummyConnector6
            | api_enums::Connector::DummyConnector7 => {
                hyperswitch_connectors::connectors::dummyconnector::transformers::DummyConnectorAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Aci => {
                aci::transformers::AciAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Adyen => {
                adyen::transformers::AdyenAuthType::try_from(self.auth_type)?;
                adyen::transformers::AdyenConnectorMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Affirm => {
                affirm::transformers::AffirmAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Airwallex => {
                airwallex::transformers::AirwallexAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Amazonpay => {
                amazonpay::transformers::AmazonpayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Archipel => {
                archipel::transformers::ArchipelAuthType::try_from(self.auth_type)?;
                archipel::transformers::ArchipelConfigData::try_from(self.connector_meta_data)?;
                Ok(())
            }
            api_enums::Connector::Authipay => {
                authipay::transformers::AuthipayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Authorizedotnet => {
                authorizedotnet::transformers::AuthorizedotnetAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Bankofamerica => {
                bankofamerica::transformers::BankOfAmericaAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Barclaycard => {
                barclaycard::transformers::BarclaycardAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Billwerk => {
                billwerk::transformers::BillwerkAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Bitpay => {
                bitpay::transformers::BitpayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_models::connector_enums::Connector::Zift => {
                zift::transformers::ZiftAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Bambora => {
                bambora::transformers::BamboraAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Bamboraapac => {
                bamboraapac::transformers::BamboraapacAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Boku => {
                boku::transformers::BokuAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Bluesnap => {
                bluesnap::transformers::BluesnapAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Blackhawknetwork => {
                blackhawknetwork::transformers::BlackhawknetworkAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Calida => {
                calida::transformers::CalidaAuthType::try_from(self.auth_type)?;
                calida::transformers::CalidaMetadataObject::try_from(self.connector_meta_data)?;
                Ok(())
            }
            api_enums::Connector::Braintree => {
                braintree::transformers::BraintreeAuthType::try_from(self.auth_type)?;
                braintree::transformers::BraintreeMeta::try_from(self.connector_meta_data)?;
                Ok(())
            }
            api_enums::Connector::Breadpay => {
                breadpay::transformers::BreadpayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Cardinal => Ok(()),
            api_enums::Connector::Cashtocode => {
                cashtocode::transformers::CashtocodeAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Chargebee => {
                chargebee::transformers::ChargebeeAuthType::try_from(self.auth_type)?;
                chargebee::transformers::ChargebeeMetadata::try_from(self.connector_meta_data)?;
                Ok(())
            }
            api_enums::Connector::Celero => {
                celero::transformers::CeleroAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Checkbook => {
                checkbook::transformers::CheckbookAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Checkout => {
                checkout::transformers::CheckoutAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Coinbase => {
                coinbase::transformers::CoinbaseAuthType::try_from(self.auth_type)?;
                coinbase::transformers::CoinbaseConnectorMeta::try_from(self.connector_meta_data)?;
                Ok(())
            }
            api_enums::Connector::Coingate => {
                coingate::transformers::CoingateAuthType::try_from(self.auth_type)?;
                coingate::transformers::CoingateConnectorMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Cryptopay => {
                cryptopay::transformers::CryptopayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::CtpMastercard => Ok(()),
            api_enums::Connector::Custombilling => Ok(()),
            api_enums::Connector::CtpVisa => Ok(()),
            api_enums::Connector::Cybersource => {
                cybersource::transformers::CybersourceAuthType::try_from(self.auth_type)?;
                cybersource::transformers::CybersourceConnectorMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Datatrans => {
                datatrans::transformers::DatatransAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Deutschebank => {
                deutschebank::transformers::DeutschebankAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Digitalvirgo => {
                digitalvirgo::transformers::DigitalvirgoAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Dlocal => {
                dlocal::transformers::DlocalAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Dwolla => {
                dwolla::transformers::DwollaAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Ebanx => {
                ebanx::transformers::EbanxAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Elavon => {
                elavon::transformers::ElavonAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Facilitapay => {
                facilitapay::transformers::FacilitapayAuthType::try_from(self.auth_type)?;
                facilitapay::transformers::FacilitapayConnectorMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Fiserv => {
                fiserv::transformers::FiservAuthType::try_from(self.auth_type)?;
                fiserv::transformers::FiservSessionObject::try_from(self.connector_meta_data)?;
                Ok(())
            }
            api_enums::Connector::Fiservemea => {
                fiservemea::transformers::FiservemeaAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Fiuu => {
                fiuu::transformers::FiuuAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Flexiti => {
                flexiti::transformers::FlexitiAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Forte => {
                forte::transformers::ForteAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Getnet => {
                getnet::transformers::GetnetAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Gigadat => {
                gigadat::transformers::GigadatAuthType::try_from(self.auth_type)?;
                gigadat::transformers::GigadatConnectorMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Globalpay => {
                globalpay::transformers::GlobalpayAuthType::try_from(self.auth_type)?;
                globalpay::transformers::GlobalPayMeta::try_from(self.connector_meta_data)?;
                Ok(())
            }
            api_enums::Connector::Globepay => {
                globepay::transformers::GlobepayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Gocardless => {
                gocardless::transformers::GocardlessAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Gpayments => {
                gpayments::transformers::GpaymentsAuthType::try_from(self.auth_type)?;
                gpayments::transformers::GpaymentsMetaData::try_from(self.connector_meta_data)?;
                Ok(())
            }
            api_enums::Connector::Hipay => {
                hipay::transformers::HipayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Helcim => {
                helcim::transformers::HelcimAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::HyperswitchVault => {
                hyperswitch_vault::transformers::HyperswitchVaultAuthType::try_from(
                    self.auth_type,
                )?;
                Ok(())
            }
            api_enums::Connector::Iatapay => {
                iatapay::transformers::IatapayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Inespay => {
                inespay::transformers::InespayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Itaubank => {
                itaubank::transformers::ItaubankAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Jpmorgan => {
                jpmorgan::transformers::JpmorganAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Juspaythreedsserver => Ok(()),
            api_enums::Connector::Klarna => {
                klarna::transformers::KlarnaAuthType::try_from(self.auth_type)?;
                klarna::transformers::KlarnaConnectorMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Loonio => {
                loonio::transformers::LoonioAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Mifinity => {
                mifinity::transformers::MifinityAuthType::try_from(self.auth_type)?;
                mifinity::transformers::MifinityConnectorMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Mollie => {
                mollie::transformers::MollieAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Moneris => {
                moneris::transformers::MonerisAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Multisafepay => {
                multisafepay::transformers::MultisafepayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Netcetera => {
                netcetera::transformers::NetceteraAuthType::try_from(self.auth_type)?;
                netcetera::transformers::NetceteraMetaData::try_from(self.connector_meta_data)?;
                Ok(())
            }
            api_enums::Connector::Nexinets => {
                nexinets::transformers::NexinetsAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Nexixpay => {
                nexixpay::transformers::NexixpayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Nmi => {
                nmi::transformers::NmiAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Nomupay => {
                nomupay::transformers::NomupayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Noon => {
                noon::transformers::NoonAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Nordea => {
                nordea::transformers::NordeaAuthType::try_from(self.auth_type)?;
                nordea::transformers::NordeaConnectorMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Novalnet => {
                novalnet::transformers::NovalnetAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Nuvei => {
                nuvei::transformers::NuveiAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Opennode => {
                opennode::transformers::OpennodeAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Paybox => {
                paybox::transformers::PayboxAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Payload => {
                payload::transformers::PayloadAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Payjustnow => {
                payjustnow::transformers::PayjustnowAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Payme => {
                payme::transformers::PaymeAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Paypal => {
                paypal::transformers::PaypalAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Paysafe => {
                paysafe::transformers::PaysafeAuthType::try_from(self.auth_type)?;
                paysafe::transformers::PaysafeConnectorMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Payone => {
                payone::transformers::PayoneAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Paystack => {
                paystack::transformers::PaystackAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Payu => {
                payu::transformers::PayuAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Peachpayments => {
                peachpayments::transformers::PeachpaymentsAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Placetopay => {
                placetopay::transformers::PlacetopayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Powertranz => {
                powertranz::transformers::PowertranzAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Prophetpay => {
                prophetpay::transformers::ProphetpayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Rapyd => {
                rapyd::transformers::RapydAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Razorpay => {
                razorpay::transformers::RazorpayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Recurly => {
                recurly::transformers::RecurlyAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Redsys => {
                redsys::transformers::RedsysAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Santander => {
                santander::transformers::SantanderAuthType::try_from(self.auth_type)?;
                santander::transformers::SantanderMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Shift4 => {
                shift4::transformers::Shift4AuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Silverflow => {
                silverflow::transformers::SilverflowAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Square => {
                square::transformers::SquareAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Stax => {
                stax::transformers::StaxAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Taxjar => {
                taxjar::transformers::TaxjarAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Stripe => {
                stripe::transformers::StripeAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Stripebilling => {
                stripebilling::transformers::StripebillingAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Tesouro => {
                tesouro::transformers::TesouroAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Trustpay => {
                trustpay::transformers::TrustpayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Trustpayments => {
                trustpayments::transformers::TrustpaymentsAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Tokenex => {
                tokenex::transformers::TokenexAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Tokenio => {
                tokenio::transformers::TokenioAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Tsys => {
                tsys::transformers::TsysAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Volt => {
                volt::transformers::VoltAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Wellsfargo => {
                wellsfargo::transformers::WellsfargoAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Wise => {
                wise::transformers::WiseAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Worldline => {
                worldline::transformers::WorldlineAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Worldpay => {
                worldpay::transformers::WorldpayAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Worldpayvantiv => {
                worldpayvantiv::transformers::WorldpayvantivAuthType::try_from(self.auth_type)?;
                worldpayvantiv::transformers::WorldpayvantivMetadataObject::try_from(
                    self.connector_meta_data,
                )?;
                Ok(())
            }
            api_enums::Connector::Worldpayxml => {
                worldpayxml::transformers::WorldpayxmlAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Xendit => {
                xendit::transformers::XenditAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Zen => {
                zen::transformers::ZenAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Zsl => {
                zsl::transformers::ZslAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Signifyd => {
                signifyd::transformers::SignifydAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Riskified => {
                riskified::transformers::RiskifiedAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Plaid => {
                PlaidAuthType::foreign_try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Threedsecureio => {
                threedsecureio::transformers::ThreedsecureioAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Phonepe => {
                phonepe::transformers::PhonepeAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Paytm => {
                paytm::transformers::PaytmAuthType::try_from(self.auth_type)?;
                Ok(())
            }
            api_enums::Connector::Finix => {
                finix::transformers::FinixAuthType::try_from(self.auth_type)?;
                Ok(())
            }
        }
    }
}

struct ConnectorAuthTypeValidation<'a> {
    auth_type: &'a types::ConnectorAuthType,
}

impl ConnectorAuthTypeValidation<'_> {
    fn validate_connector_auth_type(
        &self,
    ) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
        let validate_non_empty_field = |field_value: &str, field_name: &str| {
            if field_value.trim().is_empty() {
                Err(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: format!("connector_account_details.{field_name}"),
                    expected_format: "a non empty String".to_string(),
                }
                .into())
            } else {
                Ok(())
            }
        };
        match self.auth_type {
            hyperswitch_domain_models::router_data::ConnectorAuthType::TemporaryAuth => Ok(()),
            hyperswitch_domain_models::router_data::ConnectorAuthType::HeaderKey { api_key } => {
                validate_non_empty_field(api_key.peek(), "api_key")
            }
            hyperswitch_domain_models::router_data::ConnectorAuthType::BodyKey {
                api_key,
                key1,
            } => {
                validate_non_empty_field(api_key.peek(), "api_key")?;
                validate_non_empty_field(key1.peek(), "key1")
            }
            hyperswitch_domain_models::router_data::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => {
                validate_non_empty_field(api_key.peek(), "api_key")?;
                validate_non_empty_field(key1.peek(), "key1")?;
                validate_non_empty_field(api_secret.peek(), "api_secret")
            }
            hyperswitch_domain_models::router_data::ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => {
                validate_non_empty_field(api_key.peek(), "api_key")?;
                validate_non_empty_field(key1.peek(), "key1")?;
                validate_non_empty_field(api_secret.peek(), "api_secret")?;
                validate_non_empty_field(key2.peek(), "key2")
            }
            hyperswitch_domain_models::router_data::ConnectorAuthType::CurrencyAuthKey {
                auth_key_map,
            } => {
                if auth_key_map.is_empty() {
                    Err(errors::ApiErrorResponse::InvalidDataFormat {
                        field_name: "connector_account_details.auth_key_map".to_string(),
                        expected_format: "a non empty map".to_string(),
                    }
                    .into())
                } else {
                    Ok(())
                }
            }
            hyperswitch_domain_models::router_data::ConnectorAuthType::CertificateAuth {
                certificate,
                private_key,
            } => {
                client::create_identity_from_certificate_and_key(
                    certificate.to_owned(),
                    private_key.to_owned(),
                )
                .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name:
                        "connector_account_details.certificate or connector_account_details.private_key"
                            .to_string(),
                    expected_format:
                        "a valid base64 encoded string of PEM encoded Certificate and Private Key"
                            .to_string(),
                })?;
                Ok(())
            }
            hyperswitch_domain_models::router_data::ConnectorAuthType::NoKey => Ok(()),
        }
    }
}
