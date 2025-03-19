pub mod transformers;
use api_models::{self, webhooks::IncomingWebhookEvent};
#[cfg(feature = "payouts")]
use base64::Engine;
#[cfg(feature = "payouts")]
use common_utils::request::RequestContent;
#[cfg(feature = "payouts")]
use common_utils::types::MinorUnitForConnector;
#[cfg(feature = "payouts")]
use common_utils::types::{AmountConvertor, MinorUnit};
#[cfg(not(feature = "payouts"))]
use error_stack::report;
use error_stack::ResultExt;
#[cfg(feature = "payouts")]
use http::HeaderName;
use hyperswitch_interfaces::webhooks::IncomingWebhookFlowError;
use masking::Maskable;
#[cfg(feature = "payouts")]
use masking::Secret;
#[cfg(feature = "payouts")]
use ring::hmac;
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};

use self::transformers as adyenplatform;
#[cfg(feature = "payouts")]
use crate::connector::utils::convert_amount;
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers,
    services::{self, request::Mask, ConnectorSpecifications, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon},
    },
};
#[cfg(feature = "payouts")]
use crate::{
    consts,
    events::connector_api_logs::ConnectorEvent,
    types::transformers::ForeignFrom,
    utils::{crypto, ByteSliceExt, BytesExt},
};

#[derive(Clone)]
pub struct Adyenplatform {
    #[cfg(feature = "payouts")]
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}
impl Adyenplatform {
    pub const fn new() -> &'static Self {
        &Self {
            #[cfg(feature = "payouts")]
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl ConnectorCommon for Adyenplatform {
    fn id(&self) -> &'static str {
        "adyenplatform"
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = adyenplatform::AdyenplatformAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.adyenplatform.base_url.as_ref()
    }

    #[cfg(feature = "payouts")]
    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: adyenplatform::AdyenTransferErrorResponse = res
            .response
            .parse_struct("AdyenTransferErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.title,
            reason: response.detail,
            attempt_status: None,
            connector_transaction_id: None,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl api::Payment for Adyenplatform {}
impl api::PaymentAuthorize for Adyenplatform {}
impl api::PaymentSync for Adyenplatform {}
impl api::PaymentVoid for Adyenplatform {}
impl api::PaymentCapture for Adyenplatform {}
impl api::MandateSetup for Adyenplatform {}
impl api::ConnectorAccessToken for Adyenplatform {}
impl api::PaymentToken for Adyenplatform {}
impl ConnectorValidation for Adyenplatform {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl api::PaymentSession for Adyenplatform {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl api::Payouts for Adyenplatform {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Adyenplatform {}

#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for Adyenplatform
{
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}btl/v4/transfers",
            connectors.adyenplatform.base_url,
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutFulfillType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let auth = adyenplatform::AdyenplatformAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.destination_currency,
        )?;
        let connector_router_data =
            adyenplatform::AdyenPlatformRouterData::try_from((amount, req))?;
        let connector_req = adyenplatform::AdyenTransferRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutFulfillType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PayoutFulfillType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoFulfill>, errors::ConnectorError> {
        let response: adyenplatform::AdyenTransferResponse = res
            .response
            .parse_struct("AdyenTransferResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::Refund for Adyenplatform {}
impl api::RefundExecute for Adyenplatform {}
impl api::RefundSync for Adyenplatform {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Adyenplatform
{
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Adyenplatform
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Adyenplatform {
    #[cfg(feature = "payouts")]
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    #[cfg(feature = "payouts")]
    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let base64_signature = request
            .headers
            .get(HeaderName::from_static("hmacsignature"))
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(base64_signature.as_bytes().to_vec())
    }

    #[cfg(feature = "payouts")]
    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(request.body.to_vec())
    }

    #[cfg(feature = "payouts")]
    async fn verify_webhook_source(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_label,
                connector_webhook_details,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let raw_key = hex::decode(connector_webhook_secrets.secret)
            .change_context(errors::ConnectorError::WebhookVerificationSecretInvalid)?;

        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &raw_key);
        let signed_messaged = hmac::sign(&signing_key, &message);
        let payload_sign = consts::BASE64_ENGINE.encode(signed_messaged.as_ref());
        Ok(payload_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_object_reference_id(
        &self,
        #[cfg(feature = "payouts")] request: &api::IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            let webhook_body: adyenplatform::AdyenplatformIncomingWebhook = request
                .body
                .parse_struct("AdyenplatformIncomingWebhook")
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

            Ok(api_models::webhooks::ObjectReferenceId::PayoutId(
                api_models::webhooks::PayoutIdType::PayoutAttemptId(webhook_body.data.reference),
            ))
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(errors::ConnectorError::WebhooksNotImplemented))
        }
    }

    fn get_webhook_api_response(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
        error_kind: Option<IncomingWebhookFlowError>,
    ) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        if error_kind.is_some() {
            Ok(services::api::ApplicationResponse::JsonWithHeaders((
                serde_json::Value::Null,
                vec![(
                    "x-http-code".to_string(),
                    Maskable::Masked(Secret::new("404".to_string())),
                )],
            )))
        } else {
            Ok(services::api::ApplicationResponse::StatusOk)
        }
    }

    fn get_webhook_event_type(
        &self,
        #[cfg(feature = "payouts")] request: &api::IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            let webhook_body: adyenplatform::AdyenplatformIncomingWebhook = request
                .body
                .parse_struct("AdyenplatformIncomingWebhook")
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

            Ok(IncomingWebhookEvent::foreign_from((
                webhook_body.webhook_type,
                webhook_body.data.status,
                webhook_body.data.tracking,
            )))
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(errors::ConnectorError::WebhooksNotImplemented))
        }
    }

    fn get_webhook_resource_object(
        &self,
        #[cfg(feature = "payouts")] request: &api::IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            let webhook_body: adyenplatform::AdyenplatformIncomingWebhook = request
                .body
                .parse_struct("AdyenplatformIncomingWebhook")
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
            Ok(Box::new(webhook_body))
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(errors::ConnectorError::WebhooksNotImplemented))
        }
    }
}

impl ConnectorSpecifications for Adyenplatform {}
