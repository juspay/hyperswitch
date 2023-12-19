pub mod transformers;
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    services::{ConnectorIntegration, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon},
    },
};
use error_stack::IntoReport;

#[derive(Debug, Clone)]
pub struct Kount;

#[cfg(feature = "frm")]
impl api::FraudCheckSale for Kount {}
#[cfg(feature = "frm")]
impl api::FraudCheckCheckout for Kount {}
#[cfg(feature = "frm")]
impl api::FraudCheckTransaction for Kount {}
#[cfg(feature = "frm")]
impl api::FraudCheckFulfillment for Kount {}
#[cfg(feature = "frm")]
impl api::FraudCheck for Kount {}
#[cfg(feature = "frm")]
impl api::FraudCheckRecordReturn for Kount {}

impl ConnectorCommon for Kount {
    fn id(&self) -> &'static str {
        "kount"
    }

    fn base_url<'a>(&self, _connectors: &'a settings::Connectors) -> &'a str {
        "https://api.kount.net/rpc/v1"
    }
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        api::Sale,
        types::fraud_check::FraudCheckSaleData,
        types::fraud_check::FraudCheckResponseData,
    > for Kount
{
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        api::Checkout,
        types::fraud_check::FraudCheckCheckoutData,
        types::fraud_check::FraudCheckResponseData,
    > for Kount
{
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        api::RecordReturn,
        types::fraud_check::FraudCheckRecordReturnData,
        types::fraud_check::FraudCheckResponseData,
    > for Kount
{
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        api::Transaction,
        types::fraud_check::FraudCheckTransactionData,
        types::fraud_check::FraudCheckResponseData,
    > for Kount
{
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        api::Fulfillment,
        types::fraud_check::FraudCheckFulfillmentData,
        types::fraud_check::FraudCheckResponseData,
    > for Kount
{
}

impl api::Payment for Kount {}
impl api::PaymentAuthorize for Kount {}
impl api::PaymentSync for Kount {}
impl api::PaymentVoid for Kount {}
impl api::PaymentCapture for Kount {}
impl api::MandateSetup for Kount {}
impl api::ConnectorAccessToken for Kount {}
impl api::PaymentToken for Kount {}
impl api::Refund for Kount {}
impl api::RefundExecute for Kount {}
impl api::RefundSync for Kount {}
impl ConnectorValidation for Kount {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Kount
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Kount
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Kount
{
}

impl api::PaymentSession for Kount {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Kount
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Kount
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Kount
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Kount
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Kount
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Kount {}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Kount {}

#[async_trait::async_trait]
impl api::IncomingWebhook for Kount {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
