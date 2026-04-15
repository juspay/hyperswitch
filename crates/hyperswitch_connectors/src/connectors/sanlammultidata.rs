pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
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
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors, webhooks,
};
use hyperswitch_masking::{Mask, PeekInterface};
use transformers as sanlammultidata;

use crate::constants::headers;

#[derive(Clone)]
pub struct Sanlammultidata {}

impl Sanlammultidata {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for Sanlammultidata {}
impl api::PaymentSession for Sanlammultidata {}
impl api::ConnectorAccessToken for Sanlammultidata {}
impl api::MandateSetup for Sanlammultidata {}
impl api::PaymentAuthorize for Sanlammultidata {}
impl api::PaymentSync for Sanlammultidata {}
impl api::PaymentCapture for Sanlammultidata {}
impl api::PaymentVoid for Sanlammultidata {}
impl api::Refund for Sanlammultidata {}
impl api::RefundExecute for Sanlammultidata {}
impl api::RefundSync for Sanlammultidata {}
impl api::PaymentToken for Sanlammultidata {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Sanlammultidata
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Sanlammultidata {
    fn id(&self) -> &'static str {
        "sanlammultidata"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.sanlammultidata.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = sanlammultidata::SanlammultidataAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            (
                headers::AUTHORIZATION.to_string(),
                auth.api_key.peek().to_owned().into_masked(),
            ),
            (
                headers::MERCHANT_ID.to_string(),
                auth.merchant_id.peek().to_owned().into(),
            ),
        ])
    }
}

impl ConnectorValidation for Sanlammultidata {
    fn validate_mandate_payment(
        &self,
        _pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        match pm_data {
            PaymentMethodData::Card(_) => Err(errors::ConnectorError::NotImplemented(
                "validate_mandate_payment does not support cards".to_string(),
            )
            .into()),
            _ => Ok(()),
        }
    }

    fn validate_psync_reference_id(
        &self,
        _data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Sanlammultidata {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Sanlammultidata
{
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for Sanlammultidata
{
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Sanlammultidata
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Sanlammultidata
{
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Sanlammultidata {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Sanlammultidata {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Sanlammultidata {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Sanlammultidata {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Sanlammultidata {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Sanlammultidata {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

static SANLAMMULTIDATA_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(SupportedPaymentMethods::new);

static SANLAMMULTIDATA_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Sanlammultidata",
    description: "Sanlammultidata connector",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static SANLAMMULTIDATA_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Sanlammultidata {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&SANLAMMULTIDATA_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*SANLAMMULTIDATA_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&SANLAMMULTIDATA_SUPPORTED_WEBHOOK_FLOWS)
    }
}
