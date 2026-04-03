pub mod requests;
pub mod responses;
pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::{BytesExt, ValueExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, GenerateQr, PSync, PaymentMethodToken, PushNotification, Session,
            SetupMandate, Void,
        },
        refunds::{Execute, RSync},
        AuthorizeSessionToken, UpdateMetadata,
    },
    router_request_types::{
        AccessTokenRequestData, AuthorizeSessionTokenData, GenerateQrRequestData,
        PaymentMethodTokenizationData, PaymentsAuthorizeData, PaymentsCancelData,
        PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData, PaymentsUpdateMetadataData,
        PushNotificationRequestData, RefundsData, ResponseId, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsAuthorizeSessionTokenRouterData,
        PaymentsCancelRouterData, PaymentsCaptureRouterData, PaymentsGenerateQrRouterData,
        PaymentsPushNotificationRouterData, PaymentsSyncRouterData,
        PaymentsUpdateMetadataRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorAccessTokenSuffix, ConnectorCommon, ConnectorCommonExt,
        ConnectorIntegration, ConnectorSpecifications, ConnectorValidation, CurrentFlowInfo,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, RefreshTokenType, Response},
    webhooks,
};
use hyperswitch_masking::{Maskable, PeekInterface};

use crate::{
    connectors::santander::{
        requests::{
            AccessTokenUrlPath, SantanderAuthRequest, SantanderAuthType, SantanderMetadataObject,
            SantanderPaymentRequest, SantanderPixAutomaticSolicitationRequest,
            SantanderRefundRequest, SantanderRouterData, SantanderSetupMandateRequest,
        },
        responses::{
            SanatanderAccessTokenResponse, SantanderCreatePixPayloadLocationResponse,
            SantanderErrorResponse, SantanderGenericErrorResponse, SantanderPaymentsResponse,
            SantanderPaymentsSyncResponse, SantanderPixAutomaticRecResponse,
            SantanderPixAutomaticSolicitationResponse, SantanderRefundResponse,
            SantanderUpdateMetadataResponse, SantanderVoidResponse,
        },
    },
    constants::headers,
    types::{RefreshTokenRouterData, ResponseRouterData},
    utils::{self as connector_utils, convert_amount, PaymentsAuthorizeRequestData},
};

#[derive(Clone)]
pub struct Santander {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Santander {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

pub mod santander_constants {
    pub const SANTANDER_VERSION: &str = "v2";
    pub const PIX_MIN_LEN_PAYMENT_ID: usize = 26;
    pub const PIX_MAX_LEN_PAYMENT_ID: usize = 35;
    pub const BOLETO_MIN_LEN_PAYMENT_ID: usize = 13;
    pub const BOLETO_MAX_LEN_PAYMENT_ID: usize = 13;
}

impl api::Payment for Santander {}
impl api::PaymentSession for Santander {}
impl api::ConnectorAccessToken for Santander {}
impl api::MandateSetup for Santander {}
impl api::PaymentAuthorize for Santander {}
impl api::PaymentAuthorizeSessionToken for Santander {}
impl api::PaymentSync for Santander {}
impl api::PaymentCapture for Santander {}
impl api::PaymentVoid for Santander {}
impl api::Refund for Santander {}
impl api::RefundExecute for Santander {}
impl api::RefundSync for Santander {}
impl api::PaymentToken for Santander {}
impl api::PaymentUpdateMetadata for Santander {}
impl api::PaymentsPushNotification for Santander {}
impl api::PaymentsGenerateQr for Santander {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Santander
{
    // Not Implemented (R)
}

impl ConnectorIntegration<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>
    for Santander
{
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeSessionTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsAuthorizeSessionTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        match req.payment_method_type {
            Some(enums::PaymentMethodType::PixAutomaticoPush)
            | Some(enums::PaymentMethodType::PixAutomaticoQr) => {
                Ok(format!("{}api/v1/locrec", self.base_url(connectors)))
            }
            _ => Err(errors::ConnectorError::NotSupported {
                message: req.payment_method.to_string(),
                connector: "Santander",
            }
            .into()),
        }
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeSessionTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsPreAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .add_certificate(Some(auth_details.client_id))
                .add_certificate_key(Some(auth_details.client_secret))
                .attach_default_headers()
                .headers(types::PaymentsPreAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeSessionTokenRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeSessionTokenRouterData, errors::ConnectorError> {
        let response: SantanderCreatePixPayloadLocationResponse = res
            .response
            .parse_struct("SantanderCreatePixPayloadLocationResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>
    for Santander
{
    fn get_headers(
        &self,
        req: &RouterData<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsUpdateMetadataRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let santander_mca_metadata = SantanderMetadataObject::try_from(&req.connector_meta_data)?;

        match req.payment_method {
            enums::PaymentMethod::BankTransfer => match req.request.payment_method_type {
                Some(enums::PaymentMethodType::Pix) => {
                    let santander_variant =
                        transformers::get_qr_code_type(req.request.connector_meta.clone());

                    match santander_variant {
                        Some(enums::ExpiryType::Immediate) => Ok(format!(
                            "{}api/v1/cob/{}",
                            self.base_url(connectors),
                            req.request.connector_transaction_id
                        )),
                        Some(enums::ExpiryType::Scheduled) => Ok(format!(
                            "{}api/v1/cobv/{}",
                            self.base_url(connectors),
                            req.request.connector_transaction_id
                        )),
                        None => Err(errors::ConnectorError::ResponseDeserializationFailed.into()),
                    }
                }
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            },
            enums::PaymentMethod::Voucher => match req.request.payment_method_type {
                Some(enums::PaymentMethodType::Boleto) => {
                    let base_url = connectors
                        .santander
                        .secondary_base_url
                        .clone()
                        .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
                    let version = santander_constants::SANTANDER_VERSION;
                    let boleto_mca_metadata = santander_mca_metadata
                        .boleto
                        .ok_or(errors::ConnectorError::NoConnectorMetaData)?;
                    let workspace_id = boleto_mca_metadata.workspace_id.peek();
                    Ok(format!("{base_url}collection_bill_management/{version}/workspaces/{workspace_id}/bank_slips"))
                }
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            },
            _ => Err(errors::ConnectorError::NotSupported {
                message: req.payment_method.to_string(),
                connector: "Santander",
            }
            .into()),
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsUpdateMetadataRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = SantanderPaymentRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsUpdateMetadataRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Patch)
                .url(&types::PaymentsUpdateMetadataType::get_url(
                    self, req, connectors,
                )?)
                .add_certificate(Some(auth_details.client_id))
                .add_certificate_key(Some(auth_details.client_secret))
                .attach_default_headers()
                .headers(types::PaymentsUpdateMetadataType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsUpdateMetadataType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsUpdateMetadataRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsUpdateMetadataRouterData, errors::ConnectorError> {
        let response: SantanderUpdateMetadataResponse = res
            .response
            .parse_struct("Santander UpdateMetadataResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Santander
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let access_token =
            req.access_token
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "access_token",
                })?;
        let santander_mca_metadata = SantanderMetadataObject::try_from(&req.connector_meta_data)?;

        let client_id = match req.payment_method_type {
            Some(enums::PaymentMethodType::Pix) => {
                santander_mca_metadata
                    .pix
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)?
                    .client_id
            }
            Some(enums::PaymentMethodType::PixAutomaticoPush) => {
                santander_mca_metadata
                    .pix_automatico_push
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)?
                    .client_id
            }
            Some(enums::PaymentMethodType::PixAutomaticoQr) => {
                santander_mca_metadata
                    .pix_automatico_qr
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)?
                    .client_id
            }
            Some(enums::PaymentMethodType::Boleto) => {
                santander_mca_metadata
                    .boleto
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)?
                    .client_id
            }
            _ => {
                return Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into());
            }
        };

        let header = vec![
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.peek()).into(),
            ),
            (
                headers::CONTENT_TYPE.to_string(),
                self.common_get_content_type().to_string().into(),
            ),
            (
                headers::X_APPLICATION_KEY.to_string(),
                client_id.peek().to_owned().into(),
            ),
        ];

        Ok(header)
    }
}

impl ConnectorCommon for Santander {
    fn id(&self) -> &'static str {
        "santander"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.santander.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: SantanderErrorResponse = res
            .response
            .parse_struct("SantanderErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        match response {
            SantanderErrorResponse::PixQrCode(response) => {
                let message = response
                    .detail
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string());

                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: response.status.to_string(),
                    message,
                    // reason: response.detail.clone(),
                    reason: Some(response.title.clone()),
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            SantanderErrorResponse::PixAutomatico(response) => Ok(ErrorResponse {
                status_code: res.status_code,
                code: response.code.to_string(),
                message: response.message.clone(),
                reason: Some(response.description.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            }),
            SantanderErrorResponse::Boleto(response) => Ok(ErrorResponse {
                status_code: res.status_code,
                code: response.error_code.to_string(),
                message: response.error_message.clone(),
                reason: Some(
                    response
                        .errors
                        .as_ref()
                        .and_then(|v| v.first())
                        .map(|e| e.message.clone())
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                ),
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            }),
            SantanderErrorResponse::Generic(error_response) => match error_response {
                SantanderGenericErrorResponse::Pattern1(response) => {
                    let message = response
                        .detail
                        .clone()
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string());

                    Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: response
                            .status
                            .as_str()
                            .unwrap_or(NO_ERROR_CODE)
                            .to_string(),
                        message,
                        reason: response.detail.clone(),
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                        connector_response_reference_id: None,
                    })
                }
                SantanderGenericErrorResponse::Pattern2(response) => {
                    let message = response
                        .details
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string());

                    Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: NO_ERROR_CODE.to_string(),
                        message: message.clone(),
                        reason: Some(message),
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                        connector_response_reference_id: None,
                    })
                }
                SantanderGenericErrorResponse::Pattern3(response) => {
                    let detail = response.fault.detail.error_code;

                    Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: detail.clone(),
                        message: response.fault.fault_string,
                        reason: Some(detail),
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                        connector_response_reference_id: None,
                    })
                }
                SantanderGenericErrorResponse::Pattern4(response) => {
                    let detail = response.detail;

                    Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: detail.clone().unwrap_or(NO_ERROR_CODE.to_string()),
                        message: detail.unwrap_or(NO_ERROR_MESSAGE.to_string()),
                        reason: None,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                        connector_response_reference_id: None,
                    })
                }
            },
        }
    }
}

impl ConnectorValidation for Santander {
    fn should_continue_further(
        &self,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
    ) -> Option<bool> {
        #[cfg(feature = "v1")]
        {
            Some(payment_intent.setup_future_usage == Some(common_enums::FutureUsage::OffSession))
        }
        #[cfg(feature = "v2")]
        {
            Some(payment_intent.setup_future_usage == common_enums::FutureUsage::OffSession)
        }
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Santander {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Santander {
    fn get_headers(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let path = transformers::decide_access_token_key_suffix(
            req.request.current_flow.clone(),
            req.payment_method_type,
            req.recurring_mandate_payment_data.is_some(),
        )
        .ok_or(errors::ConnectorError::GenericError {
            error_message: "AccessToken URL decision".to_string(),
            error_object: serde_json::Value::Null,
        })?;

        match path {
            AccessTokenUrlPath::Leg1 => Ok(format!(
                "{}oauth/token?grant_type=client_credentials",
                connectors.santander.base_url
            )),
            AccessTokenUrlPath::Leg2 => Ok(format!(
                "{}auth/oauth/v2/token",
                connectors.santander.base_url
            )),
            AccessTokenUrlPath::Boleto => {
                let secondary_base_url = connectors.santander.secondary_base_url.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "secondary_base_url for Santander",
                    },
                )?;
                Ok(format!("{}auth/oauth/v2/token", secondary_base_url))
            }
        }
    }

    fn get_request_body(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let santander_mca_metadata = SantanderMetadataObject::try_from(&req.connector_meta_data)?;
        let connector_req = SantanderAuthRequest::try_from((req, &santander_mca_metadata))?;

        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;
        let req = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .attach_default_headers()
                .headers(RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&RefreshTokenType::get_url(self, req, connectors)?)
                .add_certificate(Some(auth_details.client_id))
                .add_certificate_key(Some(auth_details.client_secret))
                .set_body(RefreshTokenType::get_request_body(self, req, connectors)?)
                .build(),
        );
        Ok(req)
    }

    fn handle_response(
        &self,
        data: &RefreshTokenRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefreshTokenRouterData, errors::ConnectorError> {
        let response: SanatanderAccessTokenResponse = res
            .response
            .parse_struct("santander SanatanderAccessTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Santander
{
    fn get_headers(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/v1/rec", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let rec_amount_in_minor = req
            .request
            .connector_intent_metadata
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_intent_metadata",
            })?
            .get_mandatory_pix_automatico_maximum_permissible_mandate_amount()
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name:
                    "connector_metadata.santander.pix_automatico.cit.mandate_details.amount",
            })?;
        let rec_amount = convert_amount(
            self.amount_converter,
            rec_amount_in_minor,
            req.request.currency,
        )?;
        let final_amount = if req.request.minor_amount.get_amount_as_i64()
            < rec_amount_in_minor.get_amount_as_i64()
        {
            rec_amount
        } else {
            amount
        };
        let connector_router_data = SantanderRouterData::from((final_amount, req));
        let connector_req = SantanderSetupMandateRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;

        let request = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .url(&self.get_url(req, connectors)?)
                .add_certificate(Some(auth_details.client_id))
                .add_certificate_key(Some(auth_details.client_secret))
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        );
        Ok(request)
    }

    fn handle_response(
        &self,
        data: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        errors::ConnectorError,
    > {
        let response: responses::SantanderSetupMandateResponse = res
            .response
            .parse_struct("santander SantanderSetupMandateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PushNotification, PushNotificationRequestData, PaymentsResponseData>
    for Santander
{
    fn get_headers(
        &self,
        req: &PaymentsPushNotificationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsPushNotificationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/v1/solicrec", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsPushNotificationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = SantanderPixAutomaticSolicitationRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsPushNotificationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;

        let builder = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PaymentsPushNotificationType::get_url(
                self, req, connectors,
            )?)
            .add_certificate(Some(auth_details.client_id))
            .add_certificate_key(Some(auth_details.client_secret))
            .attach_default_headers()
            .headers(types::PaymentsPushNotificationType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PaymentsPushNotificationType::get_request_body(
                self, req, connectors,
            )?);

        Ok(Some(builder.build()))
    }

    fn handle_response(
        &self,
        data: &PaymentsPushNotificationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsPushNotificationRouterData, errors::ConnectorError> {
        let response: SantanderPixAutomaticSolicitationResponse = res
            .response
            .parse_struct("SantanderPixAutomaticSolicitationResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let santander_mca_metadata = SantanderMetadataObject::try_from(&req.connector_meta_data)?;

        match req.payment_method {
            enums::PaymentMethod::BankTransfer => match req.request.payment_method_type {
                Some(enums::PaymentMethodType::Pix)
                | Some(enums::PaymentMethodType::PixAutomaticoQr) => {
                    // Check if this is a MIT (Merchant Initiated Transaction) for PixAutomaticoPush or PixAutomaticoQr
                    if req.request.is_mit_payment() {
                        // For MIT payments, use cobr endpoint (recurring charge endpoint)
                        return Ok(format!(
                            "{}api/v1/cobr/{}",
                            self.base_url(connectors),
                            req.connector_request_reference_id,
                        ));
                    }
                    match &req
                        .request
                        .feature_metadata
                        .as_ref()
                        .and_then(|f| f.pix_additional_details.as_ref())
                    {
                        Some(api_models::payments::PixAdditionalDetails::Immediate(_immediate)) => {
                            Ok(format!(
                                "{}api/v1/cob/{}",
                                self.base_url(connectors),
                                req.connector_request_reference_id
                            ))
                        }
                        Some(api_models::payments::PixAdditionalDetails::Scheduled(_scheduled)) => {
                            Ok(format!(
                                "{}api/v1/cobv/{}",
                                self.base_url(connectors),
                                req.connector_request_reference_id
                            ))
                        }
                        None => Err(errors::ConnectorError::MissingRequiredField {
                            field_name: "pix_additional_details",
                        }
                        .into()),
                    }
                }
                Some(enums::PaymentMethodType::PixAutomaticoPush) => {
                    // For PixAutomaticoPush MIT, use cobr endpoint
                    if req.request.is_mit_payment() {
                        Ok(format!(
                            "{}api/v1/cobr/{}",
                            self.base_url(connectors),
                            req.connector_request_reference_id,
                        ))
                    } else {
                        Err(errors::ConnectorError::NotSupported {
                            message: req.payment_method.to_string(),
                            connector: "Santander",
                        }
                        .into())
                    }
                }
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            },
            enums::PaymentMethod::Voucher => match req.request.payment_method_type {
                Some(enums::PaymentMethodType::Boleto) => {
                    let boleto_mca_metadata = santander_mca_metadata
                        .boleto
                        .ok_or(errors::ConnectorError::NoConnectorMetaData)?;
                    let secondary_base_url =
                        connectors.santander.secondary_base_url.clone().ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "secondary_base_url for Santander",
                            },
                        )?;
                    Ok(format!(
                        "{}collection_bill_management/{}/workspaces/{}/bank_slips",
                        secondary_base_url,
                        santander_constants::SANTANDER_VERSION,
                        boleto_mca_metadata.workspace_id.peek(),
                    ))
                }
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            },
            _ => Err(errors::ConnectorError::NotSupported {
                message: req.payment_method.to_string(),
                connector: "Santander",
            }
            .into()),
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = SantanderRouterData::from((amount, req));
        let connector_req = SantanderPaymentRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;
        let method: Result<Method, error_stack::Report<errors::ConnectorError>> =
            match req.payment_method_type {
                Some(enums::PaymentMethodType::Pix)
                | Some(enums::PaymentMethodType::PixAutomaticoQr)
                | Some(enums::PaymentMethodType::PixAutomaticoPush) => Ok(Method::Put),
                Some(enums::PaymentMethodType::Boleto) => Ok(Method::Post),
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            };
        Ok(Some(
            RequestBuilder::new()
                .method(method?)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .add_certificate(Some(auth_details.client_id))
                .add_certificate_key(Some(auth_details.client_secret))
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: SantanderPaymentsResponse = res
            .response
            .parse_struct("Santander PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let original_amount = match response {
            SantanderPaymentsResponse::PixQRCode(ref pix_data) => pix_data.valor.original.clone(),
            SantanderPaymentsResponse::Boleto(ref boleto_data) => boleto_data.nominal_value.clone(),
            SantanderPaymentsResponse::PixAutomaticoCobr(ref automatico_data) => {
                automatico_data.valor.original.clone()
            }
        };

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let response_integrity_object = connector_utils::get_authorise_integrity_object(
            self.amount_converter,
            original_amount,
            enums::Currency::BRL.to_string(),
        )?;

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
            })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let santander_mca_metadata = SantanderMetadataObject::try_from(&req.connector_meta_data)?;

        let connector_transaction_id = match req.request.connector_transaction_id {
            ResponseId::ConnectorTransactionId(ref id) => Some(id.clone()),
            _ => None,
        };

        // MIT recurring charge sync - use cobr endpoint
        if req.request.connector_meta.is_none()
            && matches!(
                req.payment_method_type,
                Some(enums::PaymentMethodType::PixAutomaticoPush)
                    | Some(enums::PaymentMethodType::PixAutomaticoQr)
            )
        {
            let txid =
                connector_transaction_id.ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "connector_transaction_id",
                })?;
            return Ok(format!("{}api/v1/cobr/{}", self.base_url(connectors), txid,));
        }

        let is_journey_2_cit = req.request.amount.get_amount_as_i64() == 0
            && matches!(
                req.payment_method_type,
                Some(enums::PaymentMethodType::PixAutomaticoQr)
            );

        if is_journey_2_cit {
            let mandate_id = req.request.connector_reference_id.as_ref().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "connector_reference_id for Journey 2 CIT",
                },
            )?;
            // Journey 2 CIT flow
            Ok(format!(
                "{}api/v1/rec/{}",
                self.base_url(connectors),
                mandate_id,
            ))
        } else {
            match req.payment_method {
                enums::PaymentMethod::BankTransfer => match req.request.payment_method_type {
                    Some(enums::PaymentMethodType::Pix)
                    | Some(enums::PaymentMethodType::PixAutomaticoQr) => {
                        let santander_variant =
                            transformers::get_qr_code_type(req.request.connector_meta.clone());
                        match santander_variant {
                            // Pix One-off Immediate type or Journey 3 CIT Flow
                            Some(enums::ExpiryType::Immediate) => Ok(format!(
                                "{}api/v1/cob/{}",
                                self.base_url(connectors),
                                connector_transaction_id.ok_or(
                                    errors::ConnectorError::MissingRequiredField {
                                        field_name: "connector_transaction_id"
                                    }
                                )?
                            )),
                            // Pix One-off Scheduled type or Journey 4 CIT flow
                            Some(enums::ExpiryType::Scheduled) => Ok(format!(
                                "{}api/v1/cobv/{}",
                                self.base_url(connectors),
                                connector_transaction_id.ok_or(
                                    errors::ConnectorError::MissingRequiredField {
                                        field_name: "connector_transaction_id"
                                    }
                                )?
                            )),
                            // Journey 1/2 CIT flow (no QR code type found)
                            None => {
                                let mandate_id = req
                                    .request
                                    .mandate_id
                                    .as_ref()
                                    .and_then(|ids| match &ids.mandate_reference_id {
                                        Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                                            connector_mandate_ids,
                                        )) => connector_mandate_ids.get_connector_mandate_id(),
                                        _ => None,
                                    })
                                    .ok_or(errors::ConnectorError::MissingConnectorMandateID)?;
                                Ok(format!(
                                    "{}api/v1/rec/{}",
                                    self.base_url(connectors),
                                    mandate_id,
                                ))
                            }
                        }
                    }
                    // Journey 1 CIT flow
                    Some(enums::PaymentMethodType::PixAutomaticoPush) => {
                        let mandate_id = req
                            .request
                            .connector_reference_id
                            .clone()
                            .ok_or(errors::ConnectorError::MissingConnectorMandateID)?;
                        Ok(format!(
                            "{}api/v1/rec/{}",
                            self.base_url(connectors),
                            mandate_id,
                        ))
                    }
                    _ => Err(errors::ConnectorError::NotSupported {
                        message: req.payment_method.to_string(),
                        connector: "Santander",
                    }
                    .into()),
                },
                enums::PaymentMethod::Voucher => match req.request.payment_method_type {
                    Some(enums::PaymentMethodType::Boleto) => {
                        let boleto_mca_metadata = santander_mca_metadata
                            .boleto
                            .ok_or(errors::ConnectorError::NoConnectorMetaData)?;
                        let boleto_base_url = connectors
                            .santander
                            .secondary_base_url
                            .clone()
                            .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
                        let connector_transaction_id = connector_transaction_id.ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "connector_transaction_id",
                            },
                        )?;
                        let workspace_id = boleto_mca_metadata.workspace_id.peek();
                        let version = santander_constants::SANTANDER_VERSION;
                        let voucher_data = req
                            .request
                            .connector_meta
                            .clone()
                            .map(|data| {
                                data.parse_value::<api_models::payments::VoucherNextStepData>(
                                    "VoucherNextStepData",
                                )
                                .change_context(errors::ConnectorError::ParsingFailed)
                            })
                            .transpose()?;

                        let (expiry_date, issue_date) = voucher_data
                            .as_ref()
                            .and_then(|data| data.expiry_date.zip(data.entry_date.clone()))
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "issue_date/due_date",
                            })?;

                        Ok(format!(
    "{boleto_base_url}collection_bill_management/{version}/workspaces/{workspace_id}/bank_slips?\
    paymentDateFinal={due_date}&\
    paymentDateInitial={issue_date}&\
    status=LIQUIDADO&\
    bankNumber={connector_transaction_id}",
    boleto_base_url = boleto_base_url,
    version = version,
    workspace_id = workspace_id,
    due_date = expiry_date,
    issue_date = issue_date,
    connector_transaction_id = connector_transaction_id
))
                    }
                    _ => Err(errors::ConnectorError::NotSupported {
                        message: req.payment_method.to_string(),
                        connector: "Santander",
                    }
                    .into()),
                },
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            }
        }
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .add_certificate(Some(auth_details.client_id))
                .add_certificate_key(Some(auth_details.client_secret))
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: SantanderPaymentsSyncResponse = res
            .response
            .parse_struct("santander SantanderPaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let original_amount = match response {
            SantanderPaymentsSyncResponse::PixQRCode(ref pix_data) => {
                pix_data.valor.original.clone()
            }
            // No amount is sent back in Boleto response
            SantanderPaymentsSyncResponse::Boleto(_) => convert_amount(
                self.amount_converter,
                data.request.amount,
                data.request.currency,
            )?,
            SantanderPaymentsSyncResponse::PixAutomaticoConsultAndActivateJourney(_) => {
                convert_amount(
                    self.amount_converter,
                    data.request.amount,
                    data.request.currency,
                )?
            }
            SantanderPaymentsSyncResponse::PixAutomaticoCobrSync(ref cobr_data) => {
                cobr_data.valor.original.clone()
            }
        };

        let response_integrity_object = connector_utils::get_sync_integrity_object(
            self.amount_converter,
            original_amount,
            enums::Currency::BRL.to_string(),
        )?;

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
            })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "Santander".to_string(),
        }
        .into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "Santander".to_string(),
        }
        .into())
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsCaptureType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: SantanderPaymentsResponse = res
            .response
            .parse_struct("Santander PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Santander".to_string()).into())
        // let santander_mca_metadata = SantanderMetadataObject::try_from(&req.connector_meta_data)?;

        // match req.payment_method {
        //     enums::PaymentMethod::BankTransfer => match req.request.payment_method_type {
        //         Some(enums::PaymentMethodType::Pix) => {
        //             let santander_variant =
        //                 transformers::get_qr_code_type(req.request.connector_meta.clone())?;

        //             match santander_variant {
        //                 ExpiryType::Immediate => Ok(format!(
        //                     "{}api/v1/cob/{}",
        //                     self.base_url(connectors),
        //                     req.request.connector_transaction_id
        //                 )),
        //                 ExpiryType::Scheduled => Ok(format!(
        //                     "{}api/v1/cobv/{}",
        //                     self.base_url(connectors),
        //                     req.request.connector_transaction_id
        //                 )),
        //             }
        //         }
        //         _ => Err(errors::ConnectorError::NotSupported {
        //             message: req.payment_method.to_string(),
        //             connector: "Santander",
        //         }
        //         .into()),
        //     },
        //     enums::PaymentMethod::Voucher => match req.request.payment_method_type {
        //         Some(enums::PaymentMethodType::Boleto) => {
        //             let base_url = connectors
        //                 .santander
        //                 .secondary_base_url
        //                 .clone()
        //                 .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;

        //             let version = santander_constants::SANTANDER_VERSION;

        //             let boleto_mca_metadata = santander_mca_metadata
        //                 .boleto
        //                 .ok_or(errors::ConnectorError::NoConnectorMetaData)?;

        //             Ok(format!(
        //                 "{base_url}collection_bill_management/{version}/workspaces/{}/bank_slips",
        //                 boleto_mca_metadata.workspace_id.peek(),
        //             ))
        //         }
        //         _ => Err(errors::ConnectorError::NotSupported {
        //             message: req.payment_method.to_string(),
        //             connector: "Santander",
        //         }
        //         .into()),
        //     },
        //     _ => Err(errors::ConnectorError::NotSupported {
        //         message: req.payment_method.to_string(),
        //         connector: "Santander",
        //     }
        //     .into()),
        // }
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Santander".to_string()).into())
        // let connector_req = SantanderPaymentsCancelRequest::try_from(req)?;
        // Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Santander".to_string()).into())
        // let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;
        // Ok(Some(
        //     RequestBuilder::new()
        //         .method(Method::Patch)
        //         .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
        //         .add_certificate(Some(auth_details.client_id))
        //         .add_certificate_key(Some(auth_details.client_secret))
        //         .attach_default_headers()
        //         .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
        //         .set_body(types::PaymentsVoidType::get_request_body(
        //             self, req, connectors,
        //         )?)
        //         .build(),
        // ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: SantanderVoidResponse =
            res.response
                .parse_struct("Santander VoidResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        match req.payment_method {
            enums::PaymentMethod::BankTransfer => match req.payment_method_type {
                Some(enums::PaymentMethodType::Pix) => {
                    let end_to_end_id = req
                        .request
                        .connector_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.get("end_to_end_id"))
                        .and_then(|val| val.as_str().map(|id| id.to_string()))
                        .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                            field_name: "end_to_end_id",
                        })?;
                    let refund_id = req.request.refund_id.clone();
                    let base_url = self.base_url(connectors);
                    Ok(format!(
                        "{base_url}api/v1/pix/{end_to_end_id}/devolucao/{refund_id}"
                    ))
                }
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            },
            _ => Err(errors::ConnectorError::NotSupported {
                message: req.payment_method.to_string(),
                connector: "Santander",
            }
            .into()),
        }
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data = SantanderRouterData::from((refund_amount, req));
        let connector_req = SantanderRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;
        let method: Result<Method, error_stack::Report<errors::ConnectorError>> =
            match req.payment_method_type {
                Some(enums::PaymentMethodType::Pix) => Ok(Method::Put),
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            };
        let request = RequestBuilder::new()
            .method(method?)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .add_certificate(Some(auth_details.client_id))
            .add_certificate_key(Some(auth_details.client_secret))
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::RefundExecuteType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: SantanderRefundResponse = res
            .response
            .parse_struct("santander RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let original_amount = response.valor.clone();

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let response_integrity_object = connector_utils::get_refund_integrity_object(
            self.amount_converter,
            original_amount,
            enums::Currency::BRL.to_string(),
        )?;

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
            })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_metadata = req.request.connector_metadata.clone();
        let end_to_end_id = match &connector_metadata {
            Some(metadata) => match metadata.get("end_to_end_id") {
                Some(val) => val.as_str().map(|id| id.to_string()),
                None => None,
            },
            None => None,
        }
        .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
            field_name: "end_to_end_id",
        })?;
        let base_url = self.base_url(connectors);
        let refund_id = &req.request.refund_id;

        Ok(format!(
            "{base_url}api/v1/pix/{end_to_end_id}/devolucao/{refund_id}"
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .add_certificate(Some(auth_details.client_id))
                .add_certificate_key(Some(auth_details.client_secret))
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(types::RefundSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: SantanderRefundResponse = res
            .response
            .parse_struct("santander RefundSyncResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Santander {
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

static SANTANDER_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![enums::CaptureMethod::Automatic];

        let mut santander_supported_payment_methods = SupportedPaymentMethods::new();

        santander_supported_payment_methods.add(
            enums::PaymentMethod::BankTransfer,
            enums::PaymentMethodType::Pix,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        santander_supported_payment_methods.add(
            enums::PaymentMethod::BankTransfer,
            enums::PaymentMethodType::PixAutomaticoQr,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::NotSupported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        santander_supported_payment_methods.add(
            enums::PaymentMethod::BankTransfer,
            enums::PaymentMethodType::PixAutomaticoPush,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::NotSupported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        santander_supported_payment_methods.add(
            enums::PaymentMethod::Voucher,
            enums::PaymentMethodType::Boleto,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::NotSupported,
                supported_capture_methods,
                specific_features: None,
            },
        );

        santander_supported_payment_methods
    });

static SANTANDER_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Santander",
    description:
        "Santander is a leading private bank in Brazil, offering a wide range of financial services across retail and corporate segments. It is part of the global Santander Group, one of Europe’s largest financial institutions.",
    connector_type: enums::HyperswitchConnectorCategory::BankAcquirer,
    integration_status: enums::ConnectorIntegrationStatus::Alpha,
};

static SANTANDER_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Santander {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&SANTANDER_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*SANTANDER_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&SANTANDER_SUPPORTED_WEBHOOK_FLOWS)
    }

    #[cfg(feature = "v1")]
    fn generate_connector_request_reference_id(
        &self,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
        is_config_enabled_to_send_payment_id_as_connector_request_id: bool,
    ) -> String {
        match payment_attempt.payment_method_type {
            Some(enums::PaymentMethodType::Pix)
            | Some(enums::PaymentMethodType::PixAutomaticoQr)
            | Some(enums::PaymentMethodType::PixAutomaticoPush) => {
                if is_config_enabled_to_send_payment_id_as_connector_request_id
                    && payment_intent.is_payment_id_from_merchant.unwrap_or(false)
                {
                    payment_attempt.payment_id.get_string_repr().to_owned()
                } else {
                    connector_utils::generate_alphanumeric_code(
                        santander_constants::PIX_MIN_LEN_PAYMENT_ID,
                        santander_constants::PIX_MAX_LEN_PAYMENT_ID,
                    )
                }
            }
            Some(enums::PaymentMethodType::Boleto) => {
                if is_config_enabled_to_send_payment_id_as_connector_request_id
                    && payment_intent.is_payment_id_from_merchant.unwrap_or(false)
                {
                    payment_attempt.payment_id.get_string_repr().to_owned()
                } else {
                    connector_utils::generate_random_string_containing_digits(
                        santander_constants::BOLETO_MIN_LEN_PAYMENT_ID,
                        santander_constants::BOLETO_MAX_LEN_PAYMENT_ID,
                    )
                }
            }
            _ => payment_attempt.payment_id.get_string_repr().to_owned(),
        }
    }

    fn is_authorize_session_token_call_required(
        &self,
        current_flow: Option<CurrentFlowInfo>,
    ) -> bool {
        match current_flow {
            // Journey 1/2/3/4 CIT
            Some(CurrentFlowInfo::SetupMandate { .. }) => true,
            Some(CurrentFlowInfo::CompleteAuthorize { .. })
            | Some(CurrentFlowInfo::Authorize { .. })
            | Some(CurrentFlowInfo::Psync { .. })
            | None => false,
        }
    }
    fn is_push_notification_flow_required(&self, current_flow: CurrentFlowInfo) -> bool {
        match current_flow {
            CurrentFlowInfo::SetupMandate { request_data, .. } => {
                matches!(
                    request_data.payment_method_type,
                    Some(enums::PaymentMethodType::PixAutomaticoPush)
                ) && request_data.mandate_id.is_none()
            }
            CurrentFlowInfo::Authorize { .. }
            | CurrentFlowInfo::CompleteAuthorize { .. }
            | CurrentFlowInfo::Psync { .. } => false,
        }
    }
    fn is_generate_qr_flow_required(&self, current_flow: CurrentFlowInfo) -> bool {
        match current_flow {
            CurrentFlowInfo::SetupMandate { request_data, .. } => {
                matches!(
                    request_data.payment_method_type,
                    Some(enums::PaymentMethodType::PixAutomaticoQr)
                ) && request_data.mandate_id.is_none()
            }
            CurrentFlowInfo::Authorize { .. }
            | CurrentFlowInfo::CompleteAuthorize { .. }
            | CurrentFlowInfo::Psync { .. } => false,
        }
    }
}

impl ConnectorAccessTokenSuffix for Santander {
    fn get_access_token_key(
        &self,
        router_data: &dyn api::AccessTokenData,
        merchant_connector_id_or_connector_name: String,
        current_flow: Option<CurrentFlowInfo>,
    ) -> CustomResult<String, errors::ConnectorError> {
        let merchant_id = &router_data.get_merchant_id();
        let url_path = transformers::decide_access_token_key_suffix(
            current_flow.clone(),
            router_data.get_payment_method_type(),
            router_data.is_mit_payment(),
        );

        let suffix = url_path.map(|path| match path {
            AccessTokenUrlPath::Leg1 => "pix",
            AccessTokenUrlPath::Leg2 => "pix_automatico",
            AccessTokenUrlPath::Boleto => "boleto",
        });

        match suffix {
            Some(suffix) => Ok(format!(
                "access_token_{}_{}_{}",
                merchant_id.get_string_repr(),
                merchant_connector_id_or_connector_name,
                suffix,
            )),
            None => Ok(common_utils::access_token::get_default_access_token_key(
                merchant_id,
                merchant_connector_id_or_connector_name,
            )),
        }
    }
}

impl ConnectorIntegration<GenerateQr, GenerateQrRequestData, PaymentsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &PaymentsGenerateQrRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsGenerateQrRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let mandate_id = req
            .request
            .get_connector_mandate_id()
            .ok_or(errors::ConnectorError::MissingConnectorMandateID)?;

        // Control enters when it is PixAutomaticoQr and Journey 2 CIT
        if req.request.amount.unwrap_or(0) > 0 {
            Ok(format!(
                "{}api/v1/rec/{}?txid={}",
                self.base_url(connectors),
                mandate_id,
                req.connector_request_reference_id
            ))
        }
        // Control enters when it is PixAutomaticoQr and Journey 3/4 CIT
        else {
            Ok(format!(
                "{}api/v1/rec/{}",
                self.base_url(connectors),
                mandate_id
            ))
        }
    }

    fn build_request(
        &self,
        req: &PaymentsGenerateQrRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = SantanderAuthType::try_from(&req.connector_auth_type)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsGenerateQrType::get_url(
                    self, req, connectors,
                )?)
                .add_certificate(Some(auth_details.client_id))
                .add_certificate_key(Some(auth_details.client_secret))
                .attach_default_headers()
                .headers(types::PaymentsGenerateQrType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsGenerateQrRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsGenerateQrRouterData, errors::ConnectorError> {
        let response: SantanderPixAutomaticRecResponse = res
            .response
            .parse_struct("SantanderPixAutomaticRecResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}
