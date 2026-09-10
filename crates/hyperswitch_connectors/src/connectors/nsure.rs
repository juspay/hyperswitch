//! nSure.ai fraud and risk management.
//!
//! nSure has no in-process integration: its risk evaluation runs on the
//! Unified Connector Service, which owns the provider-specific transformation.
//! This type exists so nSure is a first-class `Connector` for routing, MCA
//! validation and the FRM gateway, the same way UCS-only payment connectors
//! ship a stub. Every flow below falls through to the `ConnectorIntegration`
//! defaults, which return `NotImplemented`.

use common_utils::errors::CustomResult;
use error_stack::report;
use hyperswitch_domain_models::{
    router_data::AccessToken,
    router_flow_types::{
        AccessTokenAuth, Authorize, Capture, Execute, PSync, PaymentMethodToken, RSync, Session,
        SetupMandate, Void,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentsResponseData, RefundsResponseData, SupportedPaymentMethods,
    },
};
#[cfg(feature = "frm")]
use hyperswitch_domain_models::{
    router_flow_types::{Checkout, Fulfillment, RecordReturn, Sale, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
        FraudCheckSaleData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};
#[cfg(feature = "frm")]
use hyperswitch_interfaces::api::{
    FraudCheck, FraudCheckCheckout, FraudCheckFulfillment, FraudCheckRecordReturn, FraudCheckSale,
    FraudCheckTransaction,
};
use hyperswitch_interfaces::{
    api::{
        ConnectorAccessToken, ConnectorCommon, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation, MandateSetup, Payment, PaymentAuthorize, PaymentCapture,
        PaymentSession, PaymentSync, PaymentToken, PaymentVoid, Refund, RefundExecute, RefundSync,
    },
    configs::Connectors,
    errors::ConnectorError,
    webhooks,
};

#[derive(Clone)]
pub struct Nsure;

impl Nsure {
    pub fn new() -> &'static Self {
        &Self
    }
}

impl ConnectorCommon for Nsure {
    fn id(&self) -> &'static str {
        "nsure"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.nsure.base_url.as_ref()
    }
}

impl Payment for Nsure {}
impl PaymentAuthorize for Nsure {}
impl PaymentSync for Nsure {}
impl PaymentVoid for Nsure {}
impl PaymentCapture for Nsure {}
impl PaymentSession for Nsure {}
impl MandateSetup for Nsure {}
impl ConnectorAccessToken for Nsure {}
impl PaymentToken for Nsure {}
impl Refund for Nsure {}
impl RefundExecute for Nsure {}
impl RefundSync for Nsure {}
impl ConnectorValidation for Nsure {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Nsure
{
}
impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Nsure {}
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Nsure {}
impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Nsure {}
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Nsure {}
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Nsure {}
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Nsure {}
impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Nsure {}
impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Nsure {}
impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Nsure {}

#[cfg(feature = "frm")]
impl FraudCheck for Nsure {}
#[cfg(feature = "frm")]
impl FraudCheckSale for Nsure {}
#[cfg(feature = "frm")]
impl FraudCheckCheckout for Nsure {}
#[cfg(feature = "frm")]
impl FraudCheckTransaction for Nsure {}
#[cfg(feature = "frm")]
impl FraudCheckFulfillment for Nsure {}
#[cfg(feature = "frm")]
impl FraudCheckRecordReturn for Nsure {}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Sale, FraudCheckSaleData, FraudCheckResponseData> for Nsure {}
#[cfg(feature = "frm")]
impl ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData> for Nsure {}
#[cfg(feature = "frm")]
impl ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData>
    for Nsure
{
}
#[cfg(feature = "frm")]
impl ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
    for Nsure
{
}
#[cfg(feature = "frm")]
impl ConnectorIntegration<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
    for Nsure
{
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Nsure {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }
}

static NSURE_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "nSure",
    description:
        "nSure.ai fraud and risk management provider. Executed via the Unified Connector Service.",
    connector_type: common_enums::HyperswitchConnectorCategory::FraudAndRiskManagementProvider,
    integration_status: common_enums::ConnectorIntegrationStatus::Sandbox,
};

impl ConnectorSpecifications for Nsure {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&NSURE_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::enums::EventClass]> {
        None
    }
}
