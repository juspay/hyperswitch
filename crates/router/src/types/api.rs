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

use error_stack::{report, ResultExt};
use hyperswitch_domain_models::router_data_v2::AccessTokenFlowData;
pub use hyperswitch_domain_models::router_flow_types::{
    access_token_auth::AccessTokenAuth, webhooks::VerifyWebhookSource,
};
pub use hyperswitch_interfaces::api::{ConnectorCommon, ConnectorCommonExt, CurrencyUnit};

#[cfg(feature = "frm")]
pub use self::fraud_check::*;
#[cfg(feature = "payouts")]
pub use self::payouts::*;
pub use self::{
    admin::*, api_keys::*, authentication::*, configs::*, customers::*, disputes::*, files::*,
    payment_link::*, payment_methods::*, payments::*, poll::*, refunds::*, webhooks::*,
};
use crate::{
    configs::settings::Connectors,
    connector,
    core::{
        errors::{self, CustomResult},
        payments::types as payments_types,
    },
    services::{
        connector_integration_interface::ConnectorEnum, ConnectorIntegration,
        ConnectorIntegrationV2, ConnectorRedirectResponse, ConnectorValidation,
    },
    types::{self, api::enums as api_enums},
};
pub trait ConnectorAccessToken:
    ConnectorIntegration<AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
{
}

pub trait ConnectorAccessTokenV2:
    ConnectorIntegrationV2<
    AccessTokenAuth,
    AccessTokenFlowData,
    types::AccessTokenRequestData,
    types::AccessToken,
>
{
}

#[derive(Clone)]
pub enum ConnectorCallType {
    PreDetermined(ConnectorData),
    Retryable(Vec<ConnectorData>),
    SessionMultiple(Vec<SessionConnectorData>),
}

pub trait ConnectorVerifyWebhookSource:
    ConnectorIntegration<
    VerifyWebhookSource,
    types::VerifyWebhookSourceRequestData,
    types::VerifyWebhookSourceResponseData,
>
{
}
pub trait ConnectorVerifyWebhookSourceV2:
    ConnectorIntegrationV2<
    VerifyWebhookSource,
    types::WebhookSourceVerifyData,
    types::VerifyWebhookSourceRequestData,
    types::VerifyWebhookSourceResponseData,
>
{
}

#[derive(Clone, Debug)]
pub struct MandateRevoke;

pub trait ConnectorMandateRevoke:
    ConnectorIntegration<
    MandateRevoke,
    types::MandateRevokeRequestData,
    types::MandateRevokeResponseData,
>
{
}

pub trait ConnectorMandateRevokeV2:
    ConnectorIntegrationV2<
    MandateRevoke,
    types::MandateRevokeFlowData,
    types::MandateRevokeRequestData,
    types::MandateRevokeResponseData,
>
{
}

pub trait ConnectorTransactionId: ConnectorCommon + Sync {
    fn connector_transaction_id(
        &self,
        payment_attempt: hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        Ok(payment_attempt.connector_transaction_id)
    }
}

pub trait Router {}

pub trait Connector:
    Send
    + Refund
    + RefundV2
    + Payment
    + PaymentV2
    + ConnectorRedirectResponse
    + IncomingWebhook
    + ConnectorAccessToken
    + ConnectorAccessTokenV2
    + Dispute
    + DisputeV2
    + FileUpload
    + FileUploadV2
    + ConnectorTransactionId
    + Payouts
    + PayoutsV2
    + ConnectorVerifyWebhookSource
    + ConnectorVerifyWebhookSourceV2
    + FraudCheck
    + FraudCheckV2
    + ConnectorMandateRevoke
    + ConnectorMandateRevokeV2
    + ExternalAuthentication
    + ExternalAuthenticationV2
{
}

impl<
        T: Refund
            + RefundV2
            + Payment
            + PaymentV2
            + ConnectorRedirectResponse
            + Send
            + IncomingWebhook
            + ConnectorAccessToken
            + ConnectorAccessTokenV2
            + Dispute
            + DisputeV2
            + FileUpload
            + FileUploadV2
            + ConnectorTransactionId
            + Payouts
            + PayoutsV2
            + ConnectorVerifyWebhookSource
            + ConnectorVerifyWebhookSourceV2
            + FraudCheck
            + FraudCheckV2
            + ConnectorMandateRevoke
            + ConnectorMandateRevokeV2
            + ExternalAuthentication
            + ExternalAuthenticationV2,
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
            + ExternalAuthenticationV2,
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
    ApplePayMetadata,
    PaypalSdkMetadata,
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
    pub merchant_connector_id: Option<String>,
}

#[derive(Clone)]
pub struct SessionConnectorData {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub connector: ConnectorData,
    pub business_sub_label: Option<String>,
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
        payment_method: &enums::PaymentMethod,
        payment_method_type: &enums::PaymentMethodType,
        card_network: Option<&enums::CardNetwork>,
    ) -> Option<payments_types::SurchargeDetails> {
        match self {
            Self::Calculated(surcharge_metadata) => surcharge_metadata
                .get_surcharge_details(payments_types::SurchargeKey::PaymentMethodData(
                    *payment_method,
                    *payment_method_type,
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
        connectors: &Connectors,
        name: &str,
        connector_type: GetToken,
        connector_id: Option<String>,
    ) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector = Self::convert_connector(connectors, name)?;
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
        connectors: &Connectors,
        name: &str,
        connector_type: GetToken,
        connector_id: Option<String>,
    ) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector = Self::convert_connector(connectors, name)?;
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
        _connectors: &Connectors,
        connector_name: &str,
    ) -> CustomResult<ConnectorEnum, errors::ApiErrorResponse> {
        match enums::Connector::from_str(connector_name) {
            Ok(name) => match name {
                enums::Connector::Aci => Ok(ConnectorEnum::Old(Box::new(&connector::Aci))),
                enums::Connector::Adyen => Ok(ConnectorEnum::Old(Box::new(&connector::Adyen))),
                enums::Connector::Adyenplatform => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Adyenplatform)))
                }
                enums::Connector::Airwallex => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Airwallex)))
                }
                enums::Connector::Authorizedotnet => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Authorizedotnet)))
                }
                enums::Connector::Bambora => Ok(ConnectorEnum::Old(Box::new(&connector::Bambora))),
                enums::Connector::Bankofamerica => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Bankofamerica)))
                }
                enums::Connector::Billwerk => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Billwerk)))
                }
                enums::Connector::Bitpay => Ok(ConnectorEnum::Old(Box::new(&connector::Bitpay))),
                enums::Connector::Bluesnap => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Bluesnap::new())))
                }
                enums::Connector::Boku => Ok(ConnectorEnum::Old(Box::new(&connector::Boku))),
                enums::Connector::Braintree => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Braintree)))
                }
                enums::Connector::Cashtocode => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Cashtocode::new())))
                }
                enums::Connector::Checkout => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Checkout)))
                }
                enums::Connector::Coinbase => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Coinbase)))
                }
                enums::Connector::Cryptopay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Cryptopay::new())))
                }
                enums::Connector::Cybersource => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Cybersource)))
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
                enums::Connector::Ebanx => Ok(ConnectorEnum::Old(Box::new(&connector::Ebanx))),
                enums::Connector::Fiserv => Ok(ConnectorEnum::Old(Box::new(&connector::Fiserv))),
                enums::Connector::Forte => Ok(ConnectorEnum::Old(Box::new(&connector::Forte))),
                enums::Connector::Globalpay => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Globalpay)))
                }
                enums::Connector::Globepay => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Globepay)))
                }
                enums::Connector::Gocardless => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Gocardless)))
                }
                enums::Connector::Helcim => Ok(ConnectorEnum::Old(Box::new(&connector::Helcim))),
                enums::Connector::Iatapay => Ok(ConnectorEnum::Old(Box::new(&connector::Iatapay))),
                enums::Connector::Klarna => Ok(ConnectorEnum::Old(Box::new(&connector::Klarna))),
                enums::Connector::Mollie => Ok(ConnectorEnum::Old(Box::new(&connector::Mollie))),
                enums::Connector::Nmi => Ok(ConnectorEnum::Old(Box::new(connector::Nmi::new()))),
                enums::Connector::Noon => Ok(ConnectorEnum::Old(Box::new(connector::Noon::new()))),
                enums::Connector::Nuvei => Ok(ConnectorEnum::Old(Box::new(&connector::Nuvei))),
                enums::Connector::Opennode => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Opennode)))
                }
                // "payeezy" => Ok(ConnectorIntegrationEnum::Old(Box::new(&connector::Payeezy)), As psync and rsync are not supported by this connector, it is added as template code for future usage
                enums::Connector::Payme => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Payme::new())))
                }
                enums::Connector::Payone => Ok(ConnectorEnum::Old(Box::new(&connector::Payone))),
                enums::Connector::Payu => Ok(ConnectorEnum::Old(Box::new(&connector::Payu))),
                enums::Connector::Placetopay => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Placetopay)))
                }
                enums::Connector::Powertranz => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Powertranz)))
                }
                enums::Connector::Prophetpay => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Prophetpay)))
                }
                enums::Connector::Rapyd => Ok(ConnectorEnum::Old(Box::new(&connector::Rapyd))),
                enums::Connector::Shift4 => Ok(ConnectorEnum::Old(Box::new(&connector::Shift4))),
                enums::Connector::Square => Ok(ConnectorEnum::Old(Box::new(&connector::Square))),
                enums::Connector::Stax => Ok(ConnectorEnum::Old(Box::new(&connector::Stax))),
                enums::Connector::Stripe => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Stripe::new())))
                }
                enums::Connector::Wise => Ok(ConnectorEnum::Old(Box::new(&connector::Wise))),
                enums::Connector::Worldline => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Worldline)))
                }
                enums::Connector::Worldpay => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Worldpay)))
                }
                enums::Connector::Mifinity => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Mifinity)))
                }
                enums::Connector::Multisafepay => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Multisafepay)))
                }
                enums::Connector::Netcetera => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Netcetera)))
                }
                enums::Connector::Nexinets => {
                    Ok(ConnectorEnum::Old(Box::new(&connector::Nexinets)))
                }
                enums::Connector::Paypal => Ok(ConnectorEnum::Old(Box::new(&connector::Paypal))),
                enums::Connector::Trustpay => {
                    Ok(ConnectorEnum::Old(Box::new(connector::Trustpay::new())))
                }
                enums::Connector::Tsys => Ok(ConnectorEnum::Old(Box::new(&connector::Tsys))),
                enums::Connector::Volt => Ok(ConnectorEnum::Old(Box::new(&connector::Volt))),
                enums::Connector::Zen => Ok(ConnectorEnum::Old(Box::new(&connector::Zen))),
                enums::Connector::Zsl => Ok(ConnectorEnum::Old(Box::new(&connector::Zsl))),
                enums::Connector::Signifyd
                | enums::Connector::Plaid
                | enums::Connector::Riskified
                | enums::Connector::Gpayments
                | enums::Connector::Threedsecureio => {
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

#[cfg(feature = "payouts")]
pub trait Payouts:
    ConnectorCommon
    + PayoutCancel
    + PayoutCreate
    + PayoutEligibility
    + PayoutFulfill
    + PayoutQuote
    + PayoutRecipient
    + PayoutRecipientAccount
{
}
#[cfg(not(feature = "payouts"))]
pub trait Payouts {}

#[cfg(not(feature = "payouts"))]
pub trait PayoutsV2 {}

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
