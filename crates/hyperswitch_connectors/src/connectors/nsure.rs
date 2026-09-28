//! nSure.ai fraud and risk management.
//!
//! nSure has no in-process integration: its risk evaluation runs on the
//! Unified Connector Service, which owns the provider-specific transformation.
//! This type exists so nSure is a first-class `Connector` for routing, MCA
//! validation and the FRM gateway, the same way UCS-only payment connectors
//! ship a stub. Every flow below falls through to the `ConnectorIntegration`
//! defaults, which return `NotImplemented`.

use common_utils::{errors::CustomResult, request::Request};
use error_stack::report;
use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
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
    router_flow_types::{Checkout, Fulfillment, PoFrm, RecordReturn, Sale, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckPayoutData,
        FraudCheckRecordReturnData, FraudCheckSaleData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};
#[cfg(feature = "frm")]
use hyperswitch_interfaces::api::{
    FraudCheck, FraudCheckCheckout, FraudCheckFulfillment, FraudCheckPayout,
    FraudCheckRecordReturn, FraudCheckSale, FraudCheckTransaction,
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
impl FraudCheckPayout for Nsure {}

// The FRM flows must fail loudly on the direct path rather than inherit the
// trait default, which returns `Ok(None)` and lets the flow complete as a no-op
// with the seeded `Pending` status. That would let a payment through unscored
// even under `PreFrmFailureMode::FailClosed`. Returning an error here routes
// the case through the existing fail-open/fail-closed handling instead.
#[cfg(feature = "frm")]
macro_rules! ucs_only_frm_flow {
    ($flow:ty, $request:ty) => {
        impl ConnectorIntegration<$flow, $request, FraudCheckResponseData> for Nsure {
            fn build_request(
                &self,
                _req: &RouterData<$flow, $request, FraudCheckResponseData>,
                _connectors: &Connectors,
            ) -> CustomResult<Option<Request>, ConnectorError> {
                Err(ConnectorError::NotImplemented(
                    "nsure runs on the Unified Connector Service and has no direct integration"
                        .to_string(),
                )
                .into())
            }
        }
    };
}

#[cfg(feature = "frm")]
ucs_only_frm_flow!(Sale, FraudCheckSaleData);
#[cfg(feature = "frm")]
ucs_only_frm_flow!(Checkout, FraudCheckCheckoutData);
#[cfg(feature = "frm")]
ucs_only_frm_flow!(Transaction, FraudCheckTransactionData);
#[cfg(feature = "frm")]
ucs_only_frm_flow!(Fulfillment, FraudCheckFulfillmentData);
#[cfg(feature = "frm")]
ucs_only_frm_flow!(RecordReturn, FraudCheckRecordReturnData);
#[cfg(feature = "frm")]
ucs_only_frm_flow!(PoFrm, FraudCheckPayoutData);

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
