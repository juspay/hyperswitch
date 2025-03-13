pub mod transformers;

use std::fmt::Write;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
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
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts::{ACCEPT_HEADER, NO_ERROR_CODE, NO_ERROR_MESSAGE, USER_AGENT},
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use lazy_static::lazy_static;
use masking::PeekInterface;
use transformers as itaubank;

use crate::{
    constants::headers,
    types::{RefreshTokenRouterData, ResponseRouterData},
    utils::{convert_amount, RefundsRequestData},
};

#[derive(Clone)]
pub struct Itaubank {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Itaubank {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

impl api::Payment for Itaubank {}
impl api::PaymentSession for Itaubank {}
impl api::ConnectorAccessToken for Itaubank {}
impl api::MandateSetup for Itaubank {}
impl api::PaymentAuthorize for Itaubank {}
impl api::PaymentSync for Itaubank {}
impl api::PaymentCapture for Itaubank {}
impl api::PaymentVoid for Itaubank {}
impl api::Refund for Itaubank {}
impl api::RefundExecute for Itaubank {}
impl api::RefundSync for Itaubank {}
impl api::PaymentToken for Itaubank {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Itaubank
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Itaubank
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let access_token =
            req.access_token
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "access_token",
                })?;

        let header = vec![
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.peek()).into(),
            ),
            (
                headers::ACCEPT.to_string(),
                ACCEPT_HEADER.to_string().into(),
            ),
            (
                headers::USER_AGENT.to_string(),
                USER_AGENT.to_string().into(),
            ),
            (
                headers::CONTENT_TYPE.to_string(),
                self.common_get_content_type().to_string().into(),
            ),
        ];

        Ok(header)
    }
}

impl ConnectorCommon for Itaubank {
    fn id(&self) -> &'static str {
        "itaubank"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.itaubank.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: itaubank::ItaubankErrorResponse = res
            .response
            .parse_struct("ItaubankErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let reason = response
            .error
            .violacoes
            .map(|violacoes| {
                violacoes.iter().try_fold(String::new(), |mut acc, error| {
                    write!(
                        acc,
                        " razao - {}, propriedade - {}, valor - {};",
                        error.razao, error.propriedade, error.valor
                    )
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)
                    .attach_printable("Failed to concatenate error details")
                    .map(|_| acc)
                })
            })
            .transpose()?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.title.unwrap_or(NO_ERROR_CODE.to_string()),
            message: response
                .error
                .detail
                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
            reason,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Itaubank {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Itaubank {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Itaubank {
    fn get_url(
        &self,
        _req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/oauth/jwt", self.base_url(connectors)))
    }
    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }
    fn get_headers(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let flow_header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefreshTokenType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (
                headers::ACCEPT.to_string(),
                ACCEPT_HEADER.to_string().into(),
            ),
            (
                headers::USER_AGENT.to_string(),
                USER_AGENT.to_string().into(),
            ),
        ];
        Ok(flow_header)
    }
    fn get_request_body(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = itaubank::ItaubankAuthRequest::try_from(req)?;

        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = itaubank::ItaubankAuthType::try_from(&req.connector_auth_type)?;
        let req = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .attach_default_headers()
                .headers(types::RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&types::RefreshTokenType::get_url(self, req, connectors)?)
                .add_certificate(auth_details.certificate)
                .add_certificate_key(auth_details.certificate_key)
                .set_body(types::RefreshTokenType::get_request_body(
                    self, req, connectors,
                )?)
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
        let response: itaubank::ItaubankUpdateTokenResponse = res
            .response
            .parse_struct("ItaubankUpdateTokenResponse")
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
        let response: itaubank::ItaubankTokenErrorResponse = res
            .response
            .parse_struct("ItaubankTokenErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.title.unwrap_or(NO_ERROR_CODE.to_string()),
            message: response
                .detail
                .to_owned()
                .or(response.user_message.to_owned())
                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
            reason: response.detail.or(response.user_message),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Itaubank
{
    fn build_request(
        &self,
        _req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "setup mandate".to_string(),
            connector: "itaubank".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Itaubank {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}itau-ep9-gtw-pix-recebimentos-ext-v2/v2/cob",
            self.base_url(connectors)
        ))
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

        let connector_router_data = itaubank::ItaubankRouterData::from((amount, req));
        let connector_req = itaubank::ItaubankPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = itaubank::ItaubankAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .add_certificate(auth_details.certificate)
                .add_certificate_key(auth_details.certificate_key)
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
        let response: itaubank::ItaubankPaymentsResponse = res
            .response
            .parse_struct("Itaubank PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Itaubank {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        Ok(format!(
            "{}itau-ep9-gtw-pix-recebimentos-ext-v2/v2/cob/{}",
            self.base_url(connectors),
            req.request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = itaubank::ItaubankAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .add_certificate(auth_details.certificate)
                .add_certificate_key(auth_details.certificate_key)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: itaubank::ItaubankPaymentsSyncResponse = res
            .response
            .parse_struct("itaubank PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Itaubank {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = itaubank::ItaubankAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .add_certificate(auth_details.certificate)
                .add_certificate_key(auth_details.certificate_key)
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
        let response: itaubank::ItaubankPaymentsResponse = res
            .response
            .parse_struct("Itaubank PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Itaubank {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "void".to_string(),
            connector: "itaubank".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Itaubank {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        let itaubank_metadata = req.request.get_connector_metadata()?;
        let pix_data: itaubank::ItaubankMetaData = serde_json::from_value(itaubank_metadata)
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "itaubank_metadata",
            })?;
        Ok(format!(
            "{}itau-ep9-gtw-pix-recebimentos-ext-v2/v2/pix/{}/devolucao/{}",
            self.base_url(connectors),
            pix_data
                .pix_id
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "pix_id"
                })?,
            req.request.connector_transaction_id
        ))
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

        let connector_router_data = itaubank::ItaubankRouterData::from((refund_amount, req));
        let connector_req = itaubank::ItaubankRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = itaubank::ItaubankAuthType::try_from(&req.connector_auth_type)?;
        let request = RequestBuilder::new()
            .method(Method::Put)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .add_certificate(auth_details.certificate)
            .add_certificate_key(auth_details.certificate_key)
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
        let response: itaubank::RefundResponse = res
            .response
            .parse_struct("itaubank RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Itaubank {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        let itaubank_metadata = req.request.get_connector_metadata()?;
        let pix_data: itaubank::ItaubankMetaData = serde_json::from_value(itaubank_metadata)
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "itaubank_metadata",
            })?;
        Ok(format!(
            "{}itau-ep9-gtw-pix-recebimentos-ext-v2/v2/pix/{}/devolucao/{}",
            self.base_url(connectors),
            pix_data
                .pix_id
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "pix_id"
                })?,
            req.request.connector_transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = itaubank::ItaubankAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .add_certificate(auth_details.certificate)
                .add_certificate_key(auth_details.certificate_key)
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
        let response: itaubank::RefundResponse = res
            .response
            .parse_struct("itaubank RefundResponse")
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
impl IncomingWebhook for Itaubank {
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

lazy_static! {
    static ref ITAUBANK_SUPPORTED_PAYMENT_METHODS: SupportedPaymentMethods = {
        let supported_capture_methods = Vec::new();

        let mut itaubank_supported_payment_methods = SupportedPaymentMethods::new();

        itaubank_supported_payment_methods.add(
            enums::PaymentMethod::BankTransfer,
            enums::PaymentMethodType::Pix,
            PaymentMethodDetails{
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            }
        );

        itaubank_supported_payment_methods
    };

    static ref ITAUBANK_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name: "Itaubank",
        description: "Itau Bank is a leading Brazilian financial institution offering a wide range of banking services, including retail banking, loans, and investment solutions.",
        connector_type: enums::PaymentConnectorCategory::PaymentGateway,
    };

    static ref ITAUBANK_SUPPORTED_WEBHOOK_FLOWS: Vec<enums::EventClass> = Vec::new();
}

impl ConnectorSpecifications for Itaubank {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&*ITAUBANK_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*ITAUBANK_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&*ITAUBANK_SUPPORTED_WEBHOOK_FLOWS)
    }
}
