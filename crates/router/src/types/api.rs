pub mod admin;
pub mod api_keys;
pub mod authentication;
pub mod configs;
#[cfg(feature = "olap")]
pub mod connector_onboarding;
pub mod customers;
pub mod disputes;
pub mod enums;
pub mod ephemeral_key;
pub mod files;
#[cfg(feature = "frm")]
pub mod fraud_check;
pub mod mandates;
pub mod payment_link;
pub mod payment_methods;
pub mod payments;
#[cfg(feature = "payouts")]
pub mod payouts;
pub mod poll;
pub mod refunds;
pub mod routing;
#[cfg(feature = "olap")]
pub mod verify_connector;
#[cfg(feature = "olap")]
pub mod webhook_events;
pub mod webhooks;

pub mod authentication_v2;
pub mod disputes_v2;
pub mod files_v2;
#[cfg(feature = "frm")]
pub mod fraud_check_v2;
pub mod payments_v2;
#[cfg(feature = "payouts")]
pub mod payouts_v2;
pub mod refunds_v2;

use std::{fmt::Debug, str::FromStr};

use api_models::routing::{self as api_routing, RoutableConnectorChoice};
use common_enums::RoutableConnectors;
use error_stack::{report, ResultExt};
pub use hyperswitch_domain_models::router_flow_types::{
    access_token_auth::AccessTokenAuth, mandate_revoke::MandateRevoke,
    webhooks::VerifyWebhookSource,
};
pub use hyperswitch_interfaces::api::{
    ConnectorAccessToken, ConnectorAccessTokenV2, ConnectorCommon, ConnectorCommonExt,
    ConnectorMandateRevoke, ConnectorMandateRevokeV2, ConnectorVerifyWebhookSource,
    ConnectorVerifyWebhookSourceV2, CurrencyUnit,
};
use hyperswitch_interfaces::api::{UnifiedAuthenticationService, UnifiedAuthenticationServiceV2};

#[cfg(feature = "frm")]
pub use self::fraud_check::*;
#[cfg(feature = "payouts")]
pub use self::payouts::*;
pub use self::{
    admin::*, api_keys::*, authentication::*, configs::*, customers::*, disputes::*, files::*,
    payment_link::*, payment_methods::*, payments::*, poll::*, refunds::*, refunds_v2::*,
    webhooks::*,
};
use super::transformers::ForeignTryFrom;
use crate::{
    configs::settings::Connectors,
    connector,
    core::{
        errors::{self, CustomResult},
        payments::types as payments_types,
    },
    services::{connector_integration_interface::ConnectorEnum, ConnectorRedirectResponse},
    types::{self, api::enums as api_enums},
};
#[derive(Clone)]
pub enum ConnectorCallType {
    PreDetermined(ConnectorData),
    Retryable(Vec<ConnectorData>),
    SessionMultiple(Vec<SessionConnectorData>),
    #[cfg(feature = "v2")]
    Skip,
}

pub trait ConnectorTransactionId: ConnectorCommon + Sync {
    fn connector_transaction_id(
        &self,
        payment_attempt: hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        Ok(payment_attempt
            .get_connector_payment_id()
            .map(ToString::to_string))
    }
}
pub trait Connector:
    Send
    + Refund
    + Payment
    + ConnectorRedirectResponse
    + IncomingWebhook
    + ConnectorAccessToken
    + Dispute
    + FileUpload
    + ConnectorTransactionId
    + Payouts
    + ConnectorVerifyWebhookSource
    + FraudCheck
    + ConnectorMandateRevoke
    + ExternalAuthentication
    + TaxCalculation
    + UnifiedAuthenticationService
{
}

impl<
        T: Refund
            + Payment
            + ConnectorRedirectResponse
            + Send
            + IncomingWebhook
            + ConnectorAccessToken
            + Dispute
            + FileUpload
            + ConnectorTransactionId
            + Payouts
            + ConnectorVerifyWebhookSource
            + FraudCheck
            + ConnectorMandateRevoke
            + ExternalAuthentication
            + TaxCalculation
            + UnifiedAuthenticationService,
    > Connector for T
{
}

pub trait ConnectorV2:
    Send
    + RefundV2
    + PaymentV2
    + ConnectorRedirectResponse
    + IncomingWebhook
    + ConnectorAccessTokenV2
    + DisputeV2
    + FileUploadV2
    + ConnectorTransactionId
    + PayoutsV2
    + ConnectorVerifyWebhookSourceV2
    + FraudCheckV2
    + ConnectorMandateRevokeV2
    + ExternalAuthenticationV2
    + UnifiedAuthenticationServiceV2
{
}
impl<
        T: RefundV2
            + PaymentV2
            + ConnectorRedirectResponse
            + Send
            + IncomingWebhook
            + ConnectorAccessTokenV2
            + DisputeV2
            + FileUploadV2
            + ConnectorTransactionId
            + PayoutsV2
            + ConnectorVerifyWebhookSourceV2
            + FraudCheckV2
            + ConnectorMandateRevokeV2
            + ExternalAuthenticationV2
            + UnifiedAuthenticationServiceV2,
    > ConnectorV2 for T
{
}

pub type BoxedConnector = Box<&'static (dyn Connector + Sync)>;
pub type BoxedConnectorV2 = Box<&'static (dyn ConnectorV2 + Sync)>;

// Normal flow will call the connector and follow the flow specific operations (capture, authorize)
// SessionTokenFromMetadata will avoid calling the connector instead create the session token ( for sdk )
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum GetToken {
    GpayMetadata,
    SamsungPayMetadata,
    ApplePayMetadata,
    PaypalSdkMetadata,
    PazeMetadata,
    Connector,
}

/// Routing algorithm will output merchant connector identifier instead of connector name
/// In order to support backwards compatibility for older routing algorithms and merchant accounts
/// the support for connector name is retained
#[derive(Clone)]
pub struct ConnectorData {
    pub connector: ConnectorEnum,
    pub connector_name: types::Connector,
    pub get_token: GetToken,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

#[derive(Clone)]
pub struct SessionConnectorData {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub connector: ConnectorData,
    pub business_sub_label: Option<String>,
}

impl SessionConnectorData {
    pub fn new(
        payment_method_type: api_enums::PaymentMethodType,
        connector: ConnectorData,
        business_sub_label: Option<String>,
    ) -> Self {
        Self {
            payment_method_type,
            connector,
            business_sub_label,
        }
    }
}

pub fn convert_connector_data_to_routable_connectors(
    connectors: &[ConnectorData],
) -> CustomResult<Vec<RoutableConnectorChoice>, common_utils::errors::ValidationError> {
    connectors
        .iter()
        .map(|connector_data| RoutableConnectorChoice::foreign_try_from(connector_data.clone()))
        .collect()
}

impl ForeignTryFrom<ConnectorData> for RoutableConnectorChoice {
    type Error = error_stack::Report<common_utils::errors::ValidationError>;
    fn foreign_try_from(from: ConnectorData) -> Result<Self, Self::Error> {
        match RoutableConnectors::foreign_try_from(from.connector_name) {
            Ok(connector) => Ok(Self {
                choice_kind: api_routing::RoutableChoiceKind::FullStruct,
                connector,
                merchant_connector_id: from.merchant_connector_id,
            }),
            Err(e) => Err(common_utils::errors::ValidationError::InvalidValue {
                message: format!("This is not a routable connector: {:?}", e),
            })?,
        }
    }
}

/// Session Surcharge type
pub enum SessionSurchargeDetails {
    /// Surcharge is calculated by hyperswitch
    Calculated(payments_types::SurchargeMetadata),
    /// Surcharge is sent by merchant
    PreDetermined(payments_types::SurchargeDetails),
}

impl SessionSurchargeDetails {
    pub fn fetch_surcharge_details(
        &self,
        payment_method: enums::PaymentMethod,
        payment_method_type: enums::PaymentMethodType,
        card_network: Option<&enums::CardNetwork>,
    ) -> Option<payments_types::SurchargeDetails> {
        match self {
            Self::Calculated(surcharge_metadata) => surcharge_metadata
                .get_surcharge_details(payments_types::SurchargeKey::PaymentMethodData(
                    payment_method,
                    payment_method_type,
                    card_network.cloned(),
                ))
                .cloned(),
            Self::PreDetermined(surcharge_details) => Some(surcharge_details.clone()),
        }
    }
}

pub enum ConnectorChoice {
    SessionMultiple(Vec<SessionConnectorData>),
    StraightThrough(serde_json::Value),
    Decide,
}

impl ConnectorData {
    pub fn get_connector_by_name(
        _connectors: &Connectors,
        name: &str,
        connector_type: GetToken,
        connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    ) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector = Self::convert_connector(name)?;
        let connector_name = api_enums::Connector::from_str(name)
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
        let payout_connector_name = api_enums::PayoutConnectors::from_str(name)
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| format!("unable to parse payout connector name {name}"))?;
        let connector_name = api_enums::Connector::from(payout_connector_name);
        Ok(Self {
            connector,
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
                enums::Connector::Adyenplatform => Ok(ConnectorEnum::Old(Box::new(
                    connector::Adyenplatform::new(),
                ))),
                enums::Connector::Airwallex => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Airwallex)))
                }
                // enums::Connector::Amazonpay => {
                //     Ok(ConnectorEnum::Old(Box::new(connector::Amazonpay)))
                // }
                enums::Connector::Authorizedotnet => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Authorizedotnet)))
                }
                enums::Connector::Bambora => Ok(ConnectorEnum::Old(Box::new(&connector::Bambora))),
                enums::Connector::Bamboraapac => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Bamboraapac::new())))
                }
                enums::Connector::Bankofamerica => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Bankofamerica)))
                }
                enums::Connector::Billwerk => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Billwerk::new())))
                }
                enums::Connector::Bitpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Bitpay::new())))
                }
                enums::Connector::Bluesnap => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Bluesnap::new())))
                }
                enums::Connector::Boku => Ok(ConnectorEnum::Old(Box::new(connector::Boku::new()))),
                enums::Connector::Braintree => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Braintree::new())))
                }
                enums::Connector::Cashtocode => {
                    // enums::Connector::Chargebee => Ok(ConnectorEnum::Old(Box::new(connector::Chargebee))),
                    Ok(ConnectorEnum::Old(Box::new(connector::Cashtocode::new())))
                }
                enums::Connector::Checkout => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Checkout::new())))
                }
                enums::Connector::Coinbase => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Coinbase)))
                }
                // enums::Connector::Coingate => Ok(ConnectorEnum::Old(Box::new(connector::Coingate))),
                enums::Connector::Cryptopay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Cryptopay::new())))
                }
                enums::Connector::CtpMastercard => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::CtpMastercard)))
                }
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
                enums::Connector::Dlocal => Ok(ConnectorEnum::Old(Box::new(&connector::Dlocal))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector1 => Ok(ConnectorEnum::Old(Box::new(
                    &connector::DummyConnector::<1>,
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector2 => Ok(ConnectorEnum::Old(Box::new(
                    &connector::DummyConnector::<2>,
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector3 => Ok(ConnectorEnum::Old(Box::new(
                    &connector::DummyConnector::<3>,
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector4 => Ok(ConnectorEnum::Old(Box::new(
                    &connector::DummyConnector::<4>,
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector5 => Ok(ConnectorEnum::Old(Box::new(
                    &connector::DummyConnector::<5>,
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector6 => Ok(ConnectorEnum::Old(Box::new(
                    &connector::DummyConnector::<6>,
                ))),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector7 => Ok(ConnectorEnum::Old(Box::new(
                    &connector::DummyConnector::<7>,
                ))),
                enums::Connector::Ebanx => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Ebanx::new())))
                }
                enums::Connector::Elavon => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Elavon::new())))
                }
                enums::Connector::Fiserv => Ok(ConnectorEnum::Old(Box::new(&connector::Fiserv))),
                enums::Connector::Fiservemea => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Fiservemea::new())))
                }
                enums::Connector::Fiuu => Ok(ConnectorEnum::Old(Box::new(connector::Fiuu::new()))),
                enums::Connector::Forte => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Forte::new())))
                }
                // enums::Connector::Getnet => {
                //     Ok(ConnectorEnum::Old(Box::new(connector::Getnet::new())))
                // }
                enums::Connector::Globalpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Globalpay::new())))
                }
                enums::Connector::Globepay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Globepay::new())))
                }
                enums::Connector::Gocardless => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Gocardless)))
                }
                enums::Connector::Helcim => Ok(ConnectorEnum::Old(Box::new(&connector::Helcim))),
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
                enums::Connector::Klarna => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Klarna::new())))
                }
                enums::Connector::Mollie => {
                    // enums::Connector::Moneris => Ok(ConnectorEnum::Old(Box::new(connector::Moneris))),
                    Ok(ConnectorEnum::Old(Box::new(connector::Mollie::new())))
                }
                enums::Connector::Nexixpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Nexixpay::new())))
                }
                enums::Connector::Nmi => Ok(ConnectorEnum::Old(Box::new(connector::Nmi::new()))),
                // enums::Connector::Nomupay => Ok(ConnectorEnum::Old(Box::new(connector::Nomupay))),
                enums::Connector::Noon => Ok(ConnectorEnum::Old(Box::new(connector::Noon::new()))),
                enums::Connector::Novalnet => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Novalnet::new())))
                }
                enums::Connector::Nuvei => Ok(ConnectorEnum::Old(Box::new(&connector::Nuvei))),
                enums::Connector::Opennode => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Opennode)))
                }
                enums::Connector::Paybox => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Paybox::new())))
                }
                // "payeezy" => Ok(ConnectorIntegrationEnum::Old(Box::new(&connector::Payeezy)), As psync and rsync are not supported by this connector, it is added as template code for future usage
                enums::Connector::Payme => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Payme::new())))
                }
                enums::Connector::Payone => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Payone::new())))
                }
                enums::Connector::Payu => Ok(ConnectorEnum::Old(Box::new(connector::Payu::new()))),
                enums::Connector::Placetopay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Placetopay::new())))
                }
                enums::Connector::Powertranz => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Powertranz)))
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
                // enums::Connector::Redsys => Ok(ConnectorEnum::Old(Box::new(connector::Redsys))),
                enums::Connector::Shift4 => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Shift4::new())))
                }
                enums::Connector::Square => Ok(ConnectorEnum::Old(Box::new(&connector::Square))),
                enums::Connector::Stax => Ok(ConnectorEnum::Old(Box::new(&connector::Stax))),
                enums::Connector::Stripe => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Stripe::new())))
                }
                enums::Connector::Wise => Ok(ConnectorEnum::Old(Box::new(connector::Wise::new()))),
                enums::Connector::Worldline => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Worldline)))
                }
                enums::Connector::Worldpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Worldpay::new())))
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
                enums::Connector::Paystack => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Paystack::new())))
                }
                // enums::Connector::Thunes => Ok(ConnectorEnum::Old(Box::new(connector::Thunes))),
                enums::Connector::Trustpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Trustpay::new())))
                }
                enums::Connector::Tsys => Ok(ConnectorEnum::Old(Box::new(connector::Tsys::new()))),
                // enums::Connector::UnifiedAuthenticationService => Ok(ConnectorEnum::Old(Box::new(
                //     connector::UnifiedAuthenticationService,
                // ))),
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
                | enums::Connector::Taxjar => {
                    Err(report!(errors::ConnectorError::InvalidConnectorName)
                        .attach_printable(format!("invalid connector name: {connector_name}")))
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                }
            },
            Err(_) => Err(report!(errors::ConnectorError::InvalidConnectorName)
                .attach_printable(format!("invalid connector name: {connector_name}")))
            .change_context(errors::ApiErrorResponse::InternalServerError),
        }
    }
}

#[cfg(feature = "frm")]
pub trait FraudCheck:
    ConnectorCommon
    + FraudCheckSale
    + FraudCheckTransaction
    + FraudCheckCheckout
    + FraudCheckFulfillment
    + FraudCheckRecordReturn
{
}

#[cfg(not(feature = "frm"))]
pub trait FraudCheck {}

#[cfg(not(feature = "frm"))]
pub trait FraudCheckV2 {}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_convert_connector_parsing_success() {
        let result = enums::Connector::from_str("aci");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), enums::Connector::Aci);

        let result = enums::Connector::from_str("shift4");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), enums::Connector::Shift4);

        let result = enums::Connector::from_str("authorizedotnet");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), enums::Connector::Authorizedotnet);
    }

    #[test]
    fn test_convert_connector_parsing_fail_for_unknown_type() {
        let result = enums::Connector::from_str("unknowntype");
        assert!(result.is_err());

        let result = enums::Connector::from_str("randomstring");
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_connector_parsing_fail_for_camel_case() {
        let result = enums::Connector::from_str("Paypal");
        assert!(result.is_err());

        let result = enums::Connector::from_str("Authorizedotnet");
        assert!(result.is_err());

        let result = enums::Connector::from_str("Opennode");
        assert!(result.is_err());
    }
}

#[derive(Clone)]
pub struct TaxCalculateConnectorData {
    pub connector: ConnectorEnum,
    pub connector_name: enums::TaxConnectors,
}

impl TaxCalculateConnectorData {
    pub fn get_connector_by_name(name: &str) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector_name = enums::TaxConnectors::from_str(name)
            .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
            .attach_printable_lazy(|| format!("unable to parse connector: {name}"))?;
        let connector = Self::convert_connector(connector_name)?;
        Ok(Self {
            connector,
            connector_name,
        })
    }

    fn convert_connector(
        connector_name: enums::TaxConnectors,
    ) -> CustomResult<ConnectorEnum, errors::ApiErrorResponse> {
        match connector_name {
            enums::TaxConnectors::Taxjar => {
                Ok(ConnectorEnum::Old(Box::new(connector::Taxjar::new())))
            }
        }
    }
}
