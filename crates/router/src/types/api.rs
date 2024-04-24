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

use std::{fmt::Debug, str::FromStr};

use error_stack::{report, ResultExt};

#[cfg(feature = "frm")]
pub use self::fraud_check::*;
#[cfg(feature = "payouts")]
pub use self::payouts::*;
pub use self::{
    admin::*, api_keys::*, authentication::*, configs::*, customers::*, disputes::*, files::*,
    payment_link::*, payment_methods::*, payments::*, poll::*, refunds::*, webhooks::*,
};
use super::ErrorResponse;
use crate::{
    configs::settings::Connectors,
    connector, consts,
    core::{
        errors::{self, CustomResult},
        payments::types as payments_types,
    },
    events::connector_api_logs::ConnectorEvent,
    services::{request, ConnectorIntegration, ConnectorRedirectResponse, ConnectorValidation},
    types::{self, api::enums as api_enums},
};

#[derive(Clone, Debug)]
pub struct AccessTokenAuth;

pub trait ConnectorAccessToken:
    ConnectorIntegration<AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
{
}

#[derive(Clone)]
pub enum ConnectorCallType {
    PreDetermined(ConnectorData),
    Retryable(Vec<ConnectorData>),
    SessionMultiple(Vec<SessionConnectorData>),
}

#[derive(Clone, Debug)]
pub struct VerifyWebhookSource;

pub trait ConnectorVerifyWebhookSource:
    ConnectorIntegration<
    VerifyWebhookSource,
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

pub trait ConnectorTransactionId: ConnectorCommon + Sync {
    fn connector_transaction_id(
        &self,
        payment_attempt: data_models::payments::payment_attempt::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        Ok(payment_attempt.connector_transaction_id)
    }
}

pub enum CurrencyUnit {
    Base,
    Minor,
}

pub trait ConnectorCommon {
    /// Name of the connector (in lowercase).
    fn id(&self) -> &'static str;

    /// Connector accepted currency unit as either "Base" or "Minor"
    fn get_currency_unit(&self) -> CurrencyUnit {
        CurrencyUnit::Minor // Default implementation should be remove once it is implemented in all connectors
    }

    /// HTTP header used for authorization.
    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    /// HTTP `Content-Type` to be used for POST requests.
    /// Defaults to `application/json`.
    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    // FIXME write doc - think about this
    // fn headers(&self) -> Vec<(&str, &str)>;

    /// The base URL for interacting with the connector's API.
    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str;

    /// common error response for a connector if it is same in all case
    fn build_error_response(
        &self,
        res: types::Response,
        _event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: consts::NO_ERROR_CODE.to_string(),
            message: consts::NO_ERROR_MESSAGE.to_string(),
            reason: None,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

/// Extended trait for connector common to allow functions with generic type
pub trait ConnectorCommonExt<Flow, Req, Resp>:
    ConnectorCommon + ConnectorIntegration<Flow, Req, Resp>
{
    /// common header builder when every request for the connector have same headers
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(Vec::new())
    }
}

pub trait Router {}

pub trait Connector:
    Send
    + Refund
    + Payment
    + Debug
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
{
}

pub struct Re;

pub struct Pe;

impl<
        T: Refund
            + Payment
            + Debug
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
            + ExternalAuthentication,
    > Connector for T
{
}

type BoxedConnector = Box<&'static (dyn Connector + Sync)>;

// Normal flow will call the connector and follow the flow specific operations (capture, authorize)
// SessionTokenFromMetadata will avoid calling the connector instead create the session token ( for sdk )
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum GetToken {
    GpayMetadata,
    ApplePayMetadata,
    Connector,
}

/// Routing algorithm will output merchant connector identifier instead of connector name
/// In order to support backwards compatibility for older routing algorithms and merchant accounts
/// the support for connector name is retained
#[derive(Clone, Debug)]
pub struct ConnectorData {
    pub connector: BoxedConnector,
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
            .attach_printable_lazy(|| format!("unable to parse connector name {connector:?}"))?;
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
            .attach_printable_lazy(|| {
                format!("unable to parse payout connector name {connector:?}")
            })?;
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
    ) -> CustomResult<BoxedConnector, errors::ApiErrorResponse> {
        match enums::Connector::from_str(connector_name) {
            Ok(name) => match name {
                enums::Connector::Aci => Ok(Box::new(&connector::Aci)),
                enums::Connector::Adyen => Ok(Box::new(&connector::Adyen)),
                enums::Connector::Airwallex => Ok(Box::new(&connector::Airwallex)),
                enums::Connector::Authorizedotnet => Ok(Box::new(&connector::Authorizedotnet)),
                enums::Connector::Bambora => Ok(Box::new(&connector::Bambora)),
                enums::Connector::Bankofamerica => Ok(Box::new(&connector::Bankofamerica)),
                enums::Connector::Billwerk => Ok(Box::new(&connector::Billwerk)),
                enums::Connector::Bitpay => Ok(Box::new(&connector::Bitpay)),
                enums::Connector::Bluesnap => Ok(Box::new(&connector::Bluesnap)),
                enums::Connector::Boku => Ok(Box::new(&connector::Boku)),
                enums::Connector::Braintree => Ok(Box::new(&connector::Braintree)),
                enums::Connector::Cashtocode => Ok(Box::new(&connector::Cashtocode)),
                enums::Connector::Checkout => Ok(Box::new(&connector::Checkout)),
                enums::Connector::Coinbase => Ok(Box::new(&connector::Coinbase)),
                enums::Connector::Cryptopay => Ok(Box::new(&connector::Cryptopay)),
                enums::Connector::Cybersource => Ok(Box::new(&connector::Cybersource)),
                enums::Connector::Dlocal => Ok(Box::new(&connector::Dlocal)),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector1 => Ok(Box::new(&connector::DummyConnector::<1>)),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector2 => Ok(Box::new(&connector::DummyConnector::<2>)),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector3 => Ok(Box::new(&connector::DummyConnector::<3>)),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector4 => Ok(Box::new(&connector::DummyConnector::<4>)),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector5 => Ok(Box::new(&connector::DummyConnector::<5>)),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector6 => Ok(Box::new(&connector::DummyConnector::<6>)),
                #[cfg(feature = "dummy_connector")]
                enums::Connector::DummyConnector7 => Ok(Box::new(&connector::DummyConnector::<7>)),
                enums::Connector::Fiserv => Ok(Box::new(&connector::Fiserv)),
                enums::Connector::Forte => Ok(Box::new(&connector::Forte)),
                enums::Connector::Globalpay => Ok(Box::new(&connector::Globalpay)),
                enums::Connector::Globepay => Ok(Box::new(&connector::Globepay)),
                enums::Connector::Gocardless => Ok(Box::new(&connector::Gocardless)),
                enums::Connector::Helcim => Ok(Box::new(&connector::Helcim)),
                enums::Connector::Iatapay => Ok(Box::new(&connector::Iatapay)),
                enums::Connector::Klarna => Ok(Box::new(&connector::Klarna)),
                // enums::Connector::Mifinity => Ok(Box::new(&connector::Mifinity)), Added as template code for future usage
                enums::Connector::Mollie => Ok(Box::new(&connector::Mollie)),
                enums::Connector::Nmi => Ok(Box::new(&connector::Nmi)),
                enums::Connector::Noon => Ok(Box::new(&connector::Noon)),
                enums::Connector::Nuvei => Ok(Box::new(&connector::Nuvei)),
                enums::Connector::Opennode => Ok(Box::new(&connector::Opennode)),
                // "payeezy" => Ok(Box::new(&connector::Payeezy)), As psync and rsync are not supported by this connector, it is added as template code for future usage
                enums::Connector::Payme => Ok(Box::new(&connector::Payme)),
                enums::Connector::Payu => Ok(Box::new(&connector::Payu)),
                enums::Connector::Placetopay => Ok(Box::new(&connector::Placetopay)),
                enums::Connector::Powertranz => Ok(Box::new(&connector::Powertranz)),
                enums::Connector::Prophetpay => Ok(Box::new(&connector::Prophetpay)),
                enums::Connector::Rapyd => Ok(Box::new(&connector::Rapyd)),
                enums::Connector::Shift4 => Ok(Box::new(&connector::Shift4)),
                enums::Connector::Square => Ok(Box::new(&connector::Square)),
                enums::Connector::Stax => Ok(Box::new(&connector::Stax)),
                enums::Connector::Stripe => Ok(Box::new(&connector::Stripe)),
                enums::Connector::Wise => Ok(Box::new(&connector::Wise)),
                enums::Connector::Worldline => Ok(Box::new(&connector::Worldline)),
                enums::Connector::Worldpay => Ok(Box::new(&connector::Worldpay)),
                enums::Connector::Multisafepay => Ok(Box::new(&connector::Multisafepay)),
                enums::Connector::Netcetera => Ok(Box::new(&connector::Netcetera)),
                enums::Connector::Nexinets => Ok(Box::new(&connector::Nexinets)),
                enums::Connector::Paypal => Ok(Box::new(&connector::Paypal)),
                enums::Connector::Trustpay => Ok(Box::new(&connector::Trustpay)),
                enums::Connector::Tsys => Ok(Box::new(&connector::Tsys)),
                enums::Connector::Volt => Ok(Box::new(&connector::Volt)),
                enums::Connector::Zen => Ok(Box::new(&connector::Zen)),
                enums::Connector::Zsl => Ok(Box::new(&connector::Zsl)),
                enums::Connector::Signifyd
                | enums::Connector::Plaid
                | enums::Connector::Riskified
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

#[cfg(feature = "payouts")]
pub trait Payouts:
    ConnectorCommon
    + PayoutCancel
    + PayoutCreate
    + PayoutEligibility
    + PayoutFulfill
    + PayoutQuote
    + PayoutRecipient
{
}
#[cfg(not(feature = "payouts"))]
pub trait Payouts {}

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
