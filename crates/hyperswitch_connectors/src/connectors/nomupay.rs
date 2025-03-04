pub mod transformers;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    pii,
    request::{Method, RequestContent},
};
#[cfg(feature = "payouts")]
use common_utils::{
    request::{Request, RequestBuilder},
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
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
    router_response_types::{PaymentsResponseData, RefundsResponseData},
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::{
        PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount,
        PoSync,
    },
    router_request_types::PayoutsData,
    router_response_types::PayoutsResponseData,
    types::PayoutsRouterData,
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types;
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorRedirectResponse,
        ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks,
};
use josekit::{
    jws::{JwsHeader, ES256},
    jwt::{self, JwtPayload},
    Map, Value,
};
use masking::{ExposeInterface, Mask};
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};
use serde_json::json;
use transformers as nomupay;

use crate::{constants::headers, utils};
#[cfg(feature = "payouts")]
use crate::{types::ResponseRouterData, utils::RouterData as RouterDataTrait};

#[derive(Clone)]
pub struct Nomupay {
    #[cfg(feature = "payouts")]
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Nomupay {
    pub fn new() -> &'static Self {
        &Self {
            #[cfg(feature = "payouts")]
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

fn get_private_key(
    metadata: &Option<pii::SecretSerdeValue>,
) -> Result<String, errors::ConnectorError> {
    match nomupay::NomupayMetadata::try_from(metadata.as_ref()) {
        Ok(nomupay_metadata) => Ok(nomupay_metadata.private_key.expose()),
        Err(_e) => Err(errors::ConnectorError::NoConnectorMetaData),
    }
}

fn box_to_jwt_payload(
    body: Box<dyn masking::ErasedMaskSerialize + Send>,
) -> CustomResult<JwtPayload, errors::ConnectorError> {
    let str_result = serde_json::to_string(&body)
        .change_context(errors::ConnectorError::ProcessingStepFailed(None))?;

    let parsed_json: Map<String, Value> = serde_json::from_str(&str_result)
        .change_context(errors::ConnectorError::ProcessingStepFailed(None))?;

    let jwt_payload = JwtPayload::from_map(parsed_json)
        .change_context(errors::ConnectorError::ProcessingStepFailed(None))?;

    Ok(jwt_payload)
}

fn get_signature(
    metadata: &Option<pii::SecretSerdeValue>,
    auth: nomupay::NomupayAuthType,
    body: RequestContent,
    method: &str,
    path: String,
) -> CustomResult<String, errors::ConnectorError> {
    match body {
        RequestContent::Json(masked_json) => {
            let expiration_time = SystemTime::now() + Duration::from_secs(4 * 60);
            let expires_in = match expiration_time.duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_secs(),
                Err(_e) => 0,
            };

            let mut option_map = Map::new();
            option_map.insert("alg".to_string(), json!(format!("ES256")));
            option_map.insert("aud".to_string(), json!(format!("{} {}", method, path)));
            option_map.insert("exp".to_string(), json!(expires_in));
            option_map.insert("kid".to_string(), json!(auth.kid));

            let header = JwsHeader::from_map(option_map)
                .change_context(errors::ConnectorError::ProcessingStepFailed(None))?;

            let payload = match method {
                "GET" => JwtPayload::new(),
                _ => box_to_jwt_payload(masked_json)
                    .change_context(errors::ConnectorError::ProcessingStepFailed(None))?,
            };

            let private_key = get_private_key(metadata)?;

            let signer = ES256
                .signer_from_pem(&private_key)
                .change_context(errors::ConnectorError::ProcessingStepFailed(None))?;

            let nomupay_jwt = jwt::encode_with_signer(&payload, &header, &signer)
                .change_context(errors::ConnectorError::ProcessingStepFailed(None))?;

            let jws_blocks: Vec<&str> = nomupay_jwt.split('.').collect();

            let jws_detached = jws_blocks
                .first()
                .zip(jws_blocks.get(2))
                .map(|(first, third)| format!("{}..{}", first, third))
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "JWS blocks not sufficient for detached payload",
                })?;

            Ok(jws_detached)
        }
        _ => Err(errors::ConnectorError::ProcessingStepFailed(None).into()),
    }
}

impl api::Payment for Nomupay {}
impl api::PaymentSession for Nomupay {}
impl api::ConnectorAccessToken for Nomupay {}
impl api::MandateSetup for Nomupay {}
impl api::PaymentAuthorize for Nomupay {}
impl api::PaymentSync for Nomupay {}
impl api::PaymentCapture for Nomupay {}
impl api::PaymentVoid for Nomupay {}
impl api::Refund for Nomupay {}
impl api::RefundExecute for Nomupay {}
impl api::RefundSync for Nomupay {}
impl api::PaymentToken for Nomupay {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Nomupay
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nomupay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let is_post_req = matches!(self.get_http_method(), Method::Post);
        let body = self.get_request_body(req, connectors)?;
        let auth = nomupay::NomupayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let base_url = connectors.nomupay.base_url.as_str();
        let path: String = self
            .get_url(req, connectors)?
            .chars()
            .skip(base_url.len())
            .collect();
        let req_method = if is_post_req { "POST" } else { "GET" };

        let sign = get_signature(&req.connector_meta_data, auth, body, req_method, path)?;

        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        header.push((
            headers::X_SIGNATURE.to_string(),
            masking::Maskable::Normal(sign),
        ));

        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);

        Ok(header)
    }
}

impl TryFrom<Option<&pii::SecretSerdeValue>> for nomupay::NomupayMetadata {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: Option<&pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.cloned())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

impl ConnectorCommon for Nomupay {
    fn id(&self) -> &'static str {
        "nomupay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.nomupay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = nomupay::NomupayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.kid.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: nomupay::NomupayErrorResponse = res
            .response
            .parse_struct("NomupayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        match (
            response.status,
            response.code,
            response.error,
            response.status_code,
            response.detail,
        ) {
            (Some(status), Some(code), _, _, _) => Ok(ErrorResponse {
                status_code: res.status_code,
                code: code.to_string(),
                message: status,
                reason: None,
                attempt_status: None,
                connector_transaction_id: None,
            }),
            (None, None, Some(nomupay_inner_error), _, _) => {
                match (
                    nomupay_inner_error.error_description,
                    nomupay_inner_error.validation_errors,
                ) {
                    (Some(error_description), _) => Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: nomupay_inner_error.error_code,
                        message: error_description,
                        reason: None,
                        attempt_status: None,
                        connector_transaction_id: None,
                    }),
                    (_, Some(validation_errors)) => Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: nomupay_inner_error.error_code,
                        message: validation_errors
                            .first()
                            .map(|m| m.message.clone())
                            .unwrap_or_default(),
                        reason: Some(
                            validation_errors
                                .first()
                                .map(|m| m.field.clone())
                                .unwrap_or_default(),
                        ),
                        attempt_status: None,
                        connector_transaction_id: None,
                    }),
                    (None, None) => Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: NO_ERROR_CODE.to_string(),
                        message: NO_ERROR_MESSAGE.to_string(),
                        reason: None,
                        attempt_status: None,
                        connector_transaction_id: None,
                    }),
                }
            }
            (None, None, None, Some(status_code), Some(detail)) => Ok(ErrorResponse {
                status_code,
                code: detail
                    .get(1)
                    .map(|d| d.error_type.clone())
                    .unwrap_or_default(),
                message: status_code.to_string(),
                reason: None,
                attempt_status: None,
                connector_transaction_id: None,
            }),
            _ => Ok(ErrorResponse {
                status_code: res.status_code,
                code: NO_ERROR_CODE.to_string(),
                message: NO_ERROR_MESSAGE.to_string(),
                reason: None,
                attempt_status: None,
                connector_transaction_id: None,
            }),
        }
    }
}

impl ConnectorValidation for Nomupay {}
impl ConnectorRedirectResponse for Nomupay {}
impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Nomupay {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Nomupay {}
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Nomupay {}
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Nomupay {}
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Nomupay {}
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Nomupay {}
impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Nomupay {}
impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Nomupay {}
impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Nomupay {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoQuote, PayoutsData, PayoutsResponseData> for Nomupay {}
#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData> for Nomupay {}
#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCancel, PayoutsData, PayoutsResponseData> for Nomupay {}
#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoEligibility, PayoutsData, PayoutsResponseData> for Nomupay {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoSync, PayoutsData, PayoutsResponseData> for Nomupay {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let nomupay_payout_id = req
            .request
            .connector_payout_id
            .clone()
            .ok_or_else(utils::missing_field_err("connector_payout_id"))?;
        Ok(format!(
            "{}/v1alpha1/payments/{}",
            self.base_url(connectors),
            nomupay_payout_id
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::PayoutSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutSyncType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoSync>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoSync>, errors::ConnectorError> {
        let response: nomupay::NomupayPaymentResponse = res
            .response
            .parse_struct("NomupayPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl api::Payouts for Nomupay {}
#[cfg(feature = "payouts")]
impl api::PayoutCancel for Nomupay {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for Nomupay {}
#[cfg(feature = "payouts")]
impl api::PayoutEligibility for Nomupay {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Nomupay {}
#[cfg(feature = "payouts")]
impl api::PayoutQuote for Nomupay {}
#[cfg(feature = "payouts")]
impl api::PayoutRecipient for Nomupay {}
#[cfg(feature = "payouts")]
impl api::PayoutRecipientAccount for Nomupay {}
#[cfg(feature = "payouts")]
impl api::PayoutSync for Nomupay {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoRecipient, PayoutsData, PayoutsResponseData> for Nomupay {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1alpha1/sub-account",
            connectors.nomupay.base_url
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nomupay::OnboardSubAccountRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PayoutRecipientType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutRecipientType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PayoutRecipientType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoRecipient>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoRecipient>, errors::ConnectorError> {
        let response: nomupay::OnboardSubAccountResponse = res
            .response
            .parse_struct("OnboardSubAccountResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoRecipientAccount, PayoutsData, PayoutsResponseData> for Nomupay {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let sid = req.get_connector_customer_id()?;

        Ok(format!(
            "{}/v1alpha1/sub-account/{}/transfer-method",
            connectors.nomupay.base_url, sid
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nomupay::OnboardTransferMethodRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PayoutRecipientAccountType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::PayoutRecipientAccountType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PayoutRecipientAccountType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoRecipientAccount>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoRecipientAccount>, errors::ConnectorError> {
        let response: nomupay::OnboardTransferMethodResponse = res
            .response
            .parse_struct("OnboardTransferMethodResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Nomupay {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1alpha1/payments", connectors.nomupay.base_url))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.destination_currency,
        )?;
        let connector_req = nomupay::NomupayPaymentRequest::try_from((req, amount))?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
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

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, errors::ConnectorError> {
        let response: nomupay::NomupayPaymentResponse = res
            .response
            .parse_struct("NomupayPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Nomupay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

impl ConnectorSpecifications for Nomupay {}
