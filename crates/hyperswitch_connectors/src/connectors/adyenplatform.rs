pub mod transformers;
use api_models::{self, webhooks::IncomingWebhookEvent};
#[cfg(feature = "payouts")]
use base64::Engine;
#[cfg(feature = "payouts")]
use common_utils::crypto;
use common_utils::errors::CustomResult;
#[cfg(feature = "payouts")]
use common_utils::ext_traits::{ByteSliceExt as _, BytesExt};
#[cfg(feature = "payouts")]
use common_utils::request::RequestContent;
#[cfg(feature = "payouts")]
use common_utils::request::{Method, Request, RequestBuilder};
#[cfg(feature = "payouts")]
use common_utils::types::MinorUnitForConnector;
#[cfg(feature = "payouts")]
use common_utils::types::{AmountConvertor, MinorUnit};
#[cfg(not(feature = "payouts"))]
use error_stack::report;
use error_stack::ResultExt;
#[cfg(feature = "payouts")]
use http::HeaderName;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::router_data::{ErrorResponse, RouterData};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::router_flow_types::PoFulfill;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::types::{PayoutsData, PayoutsResponseData, PayoutsRouterData};
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    router_data::{AccessToken, ConnectorAuthType},
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
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::events::connector_api_logs::ConnectorEvent;
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types::{PayoutFulfillType, Response};
use hyperswitch_interfaces::{
    api::{self, ConnectorCommon, ConnectorIntegration, ConnectorSpecifications},
    configs::Connectors,
    errors::ConnectorError,
    webhooks::{IncomingWebhook, IncomingWebhookFlowError, IncomingWebhookRequestDetails},
};
use masking::{Mask as _, Maskable, Secret};
#[cfg(feature = "payouts")]
use ring::hmac;
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};
#[cfg(feature = "payouts")]
use transformers::get_adyen_payout_webhook_event;

use self::transformers as adyenplatform;
use crate::constants::headers;
#[cfg(feature = "payouts")]
use crate::types::ResponseRouterData;
#[cfg(feature = "payouts")]
use crate::utils::convert_amount;

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
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = adyenplatform::AdyenplatformAuthType::try_from(auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.adyenplatform.base_url.as_ref()
    }

    #[cfg(feature = "payouts")]
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: adyenplatform::AdyenTransferErrorResponse = res
            .response
            .parse_struct("AdyenTransferErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let message = if let Some(invalid_fields) = &response.invalid_fields {
            match serde_json::to_string(invalid_fields) {
                Ok(invalid_fields_json) => format!(
                    "{}\nInvalid fields: {}",
                    response.title, invalid_fields_json
                ),
                Err(_) => response.title.clone(),
            }
        } else if let Some(detail) = &response.detail {
            format!("{}\nDetail: {}", response.title, detail)
        } else {
            response.title.clone()
        };

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message,
            reason: response.detail,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
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
impl api::ConnectorValidation for Adyenplatform {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Adyenplatform
{
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Adyenplatform {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Adyenplatform
{
}

impl api::PaymentSession for Adyenplatform {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Adyenplatform {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Adyenplatform {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Adyenplatform {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Adyenplatform
{
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Adyenplatform {}

impl api::Payouts for Adyenplatform {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Adyenplatform {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Adyenplatform {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}btl/v4/transfers",
            connectors.adyenplatform.base_url,
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PayoutFulfillType::get_content_type(self).to_string().into(),
        )];
        let auth = adyenplatform::AdyenplatformAuthType::try_from(&req.connector_auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
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
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutFulfillType::get_headers(self, req, connectors)?)
            .set_body(PayoutFulfillType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, ConnectorError> {
        let response: adyenplatform::AdyenTransferResponse = res
            .response
            .parse_struct("AdyenTransferResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::Refund for Adyenplatform {}
impl api::RefundExecute for Adyenplatform {}
impl api::RefundSync for Adyenplatform {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Adyenplatform {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Adyenplatform {}

#[async_trait::async_trait]
impl IncomingWebhook for Adyenplatform {
    #[cfg(feature = "payouts")]
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    #[cfg(feature = "payouts")]
    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        let base64_signature = request
            .headers
            .get(HeaderName::from_static("hmacsignature"))
            .ok_or(ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(base64_signature.as_bytes().to_vec())
    }

    #[cfg(feature = "payouts")]
    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        Ok(request.body.to_vec())
    }

    #[cfg(feature = "payouts")]
    async fn verify_webhook_source(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        connector_label: &str,
    ) -> CustomResult<bool, ConnectorError> {
        use common_utils::consts;

        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_label,
                connector_webhook_details,
            )
            .await
            .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

        let raw_key = hex::decode(connector_webhook_secrets.secret)
            .change_context(ConnectorError::WebhookVerificationSecretInvalid)?;

        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &raw_key);
        let signed_messaged = hmac::sign(&signing_key, &message);
        let payload_sign = consts::BASE64_ENGINE.encode(signed_messaged.as_ref());
        Ok(payload_sign.as_bytes().eq(&signature))
    }

    #[cfg(feature = "payouts")]
    fn get_payout_webhook_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::PayoutWebhookUpdate, ConnectorError> {
        let webhook_body: adyenplatform::AdyenplatformIncomingWebhook = request
            .body
            .parse_struct("AdyenplatformIncomingWebhook")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        let error_reason = webhook_body.data.reason.or(webhook_body
            .data
            .tracking
            .and_then(|tracking_data| tracking_data.reason));

        Ok(api_models::webhooks::PayoutWebhookUpdate {
            error_message: error_reason.clone(),
            error_code: error_reason,
        })
    }

    fn get_webhook_object_reference_id(
        &self,
        #[cfg(feature = "payouts")] request: &IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            let webhook_body: adyenplatform::AdyenplatformIncomingWebhook = request
                .body
                .parse_struct("AdyenplatformIncomingWebhook")
                .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

            Ok(api_models::webhooks::ObjectReferenceId::PayoutId(
                api_models::webhooks::PayoutIdType::PayoutAttemptId(webhook_body.data.reference),
            ))
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(ConnectorError::WebhooksNotImplemented))
        }
    }

    fn get_webhook_api_response(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        error_kind: Option<IncomingWebhookFlowError>,
    ) -> CustomResult<ApplicationResponse<serde_json::Value>, ConnectorError> {
        if error_kind.is_some() {
            Ok(ApplicationResponse::JsonWithHeaders((
                serde_json::Value::Null,
                vec![(
                    "x-http-code".to_string(),
                    Maskable::Masked(Secret::new("404".to_string())),
                )],
            )))
        } else {
            Ok(ApplicationResponse::StatusOk)
        }
    }

    fn get_webhook_event_type(
        &self,
        #[cfg(feature = "payouts")] request: &IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            let webhook_body: adyenplatform::AdyenplatformIncomingWebhook = request
                .body
                .parse_struct("AdyenplatformIncomingWebhook")
                .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

            Ok(get_adyen_payout_webhook_event(
                webhook_body.webhook_type,
                webhook_body.data.status,
                webhook_body.data.tracking,
            ))
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(ConnectorError::WebhooksNotImplemented))
        }
    }

    fn get_webhook_resource_object(
        &self,
        #[cfg(feature = "payouts")] request: &IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            let webhook_body: adyenplatform::AdyenplatformIncomingWebhook = request
                .body
                .parse_struct("AdyenplatformIncomingWebhook")
                .change_context(ConnectorError::WebhookSourceVerificationFailed)?;
            Ok(Box::new(webhook_body))
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(ConnectorError::WebhooksNotImplemented))
        }
    }
}

static ADYENPLATFORM_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Adyen Platform",
    description: "Adyen Platform for marketplace payouts and disbursements",
    connector_type: common_enums::HyperswitchConnectorCategory::PayoutProcessor,
    integration_status: common_enums::ConnectorIntegrationStatus::Sandbox,
};

impl ConnectorSpecifications for Adyenplatform {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&ADYENPLATFORM_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::enums::EventClass]> {
        None
    }
    #[cfg(feature = "v1")]
    fn generate_connector_customer_id(
        &self,
        customer_id: &Option<common_utils::id_type::CustomerId>,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> Option<String> {
        customer_id.as_ref().map(|cid| {
            format!(
                "{}_{}",
                merchant_id.get_string_repr(),
                cid.get_string_repr()
            )
        })
    }
}
