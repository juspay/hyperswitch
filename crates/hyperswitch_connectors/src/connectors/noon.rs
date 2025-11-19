pub mod transformers;
use std::sync::LazyLock;

use api_models::webhooks::{ConnectorWebhookSecrets, IncomingWebhookEvent, ObjectReferenceId};
use base64::Engine;
use common_enums::enums;
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        mandate_revoke::MandateRevoke,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, MandateRevokeRequestData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, MandateRevokeResponseData, PaymentMethodDetails, PaymentsResponseData,
        RefundsResponseData, SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        MandateRevokeRouterData, PaymentsAuthorizeRouterData, PaymentsCancelRouterData,
        PaymentsCaptureRouterData, PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        MandateRevokeType, PaymentsAuthorizeType, PaymentsCaptureType, PaymentsSyncType,
        PaymentsVoidType, RefundExecuteType, RefundSyncType, Response,
    },
    webhooks,
};
use masking::{Mask, PeekInterface};
use router_env::logger;
use transformers as noon;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self as connector_utils, PaymentMethodDataType},
};

#[derive(Clone)]
pub struct Noon {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Noon {
    pub const fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

impl api::Payment for Noon {}
impl api::PaymentSession for Noon {}
impl api::ConnectorAccessToken for Noon {}
impl api::MandateSetup for Noon {}
impl api::PaymentAuthorize for Noon {}
impl api::PaymentSync for Noon {}
impl api::PaymentCapture for Noon {}
impl api::PaymentVoid for Noon {}
impl api::Refund for Noon {}
impl api::RefundExecute for Noon {}
impl api::RefundSync for Noon {}
impl api::PaymentToken for Noon {}
impl api::ConnectorMandateRevoke for Noon {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Noon
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Noon
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

fn get_auth_header(
    auth_type: &ConnectorAuthType,
) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
    let auth = noon::NoonAuthType::try_from(auth_type)?;

    let encoded_api_key = auth
        .business_identifier
        .zip(auth.application_identifier)
        .zip(auth.api_key)
        .map(|((business_identifier, application_identifier), api_key)| {
            common_utils::consts::BASE64_ENGINE.encode(format!(
                "{business_identifier}.{application_identifier}:{api_key}",
            ))
        });

    Ok(vec![(
        headers::AUTHORIZATION.to_string(),
        format!("Key {}", encoded_api_key.peek()).into_masked(),
    )])
}

impl ConnectorCommon for Noon {
    fn id(&self) -> &'static str {
        "noon"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.noon.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: Result<noon::NoonErrorResponse, Report<common_utils::errors::ParsingError>> =
            res.response.parse_struct("NoonErrorResponse");

        match response {
            Ok(noon_error_response) => {
                event_builder.map(|i| i.set_error_response_body(&noon_error_response));
                router_env::logger::info!(connector_response=?noon_error_response);
                // Adding in case of timeouts, if psync gives 4xx with this code, fail the payment
                let attempt_status = if noon_error_response.result_code == 19001 {
                    Some(enums::AttemptStatus::Failure)
                } else {
                    None
                };
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: noon_error_response.result_code.to_string(),
                    message: noon_error_response.message.clone(),
                    reason: Some(noon_error_response.message),
                    attempt_status,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Err(error_message) => {
                event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
                logger::error!(deserialization_error =? error_message);
                connector_utils::handle_json_response_deserialization_failure(res, "noon")
            }
        }
    }
}

impl ConnectorValidation for Noon {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _payment_method: enums::PaymentMethod,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic
            | enums::CaptureMethod::Manual
            | enums::CaptureMethod::SequentialAutomatic => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
            PaymentMethodDataType::ApplePay,
            PaymentMethodDataType::GooglePay,
        ]);
        connector_utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }

    fn validate_psync_reference_id(
        &self,
        _data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        // since we can make psync call with our reference_id, having connector_transaction_id is not an mandatory criteria
        Ok(())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Noon {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Noon {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Noon {
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Noon".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Noon {
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
        Ok(format!("{}payment/v1/order", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let mandate_details =
            connector_utils::get_mandate_details(req.request.setup_mandate_details.clone())?;
        let mandate_amount = mandate_details
            .map(|mandate| {
                connector_utils::convert_amount(
                    self.amount_converter,
                    mandate.amount,
                    mandate.currency,
                )
            })
            .transpose()?;

        let connector_router_data = noon::NoonRouterData::from((amount, req, mandate_amount));
        let connector_req = noon::NoonPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                .set_body(PaymentsAuthorizeType::get_request_body(
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
        let response: noon::NoonPaymentsResponse = res
            .response
            .parse_struct("Noon PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Noon {
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
            "{}payment/v1/order/getbyreference/{}",
            self.base_url(connectors),
            req.connector_request_reference_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: noon::NoonPaymentsResponse = res
            .response
            .parse_struct("noon PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Noon {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}payment/v1/order", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let connector_router_data = noon::NoonRouterData::from((amount, req, None));
        let connector_req = noon::NoonPaymentsActionRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsCaptureType::get_headers(self, req, connectors)?)
                .set_body(PaymentsCaptureType::get_request_body(
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
        let response: noon::NoonPaymentsResponse = res
            .response
            .parse_struct("Noon PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Noon {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}payment/v1/order", self.base_url(connectors)))
    }
    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = noon::NoonPaymentsCancelRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(PaymentsVoidType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: noon::NoonPaymentsResponse = res
            .response
            .parse_struct("Noon PaymentsCancelResponse")
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

impl ConnectorIntegration<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>
    for Noon
{
    fn get_headers(
        &self,
        req: &MandateRevokeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &MandateRevokeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}payment/v1/order", self.base_url(connectors)))
    }
    fn build_request(
        &self,
        req: &MandateRevokeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&MandateRevokeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(MandateRevokeType::get_headers(self, req, connectors)?)
                .set_body(MandateRevokeType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }
    fn get_request_body(
        &self,
        req: &MandateRevokeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = noon::NoonRevokeMandateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &MandateRevokeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<MandateRevokeRouterData, errors::ConnectorError> {
        let response: noon::NoonRevokeMandateResponse = res
            .response
            .parse_struct("Noon NoonRevokeMandateResponse")
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Noon {
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
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}payment/v1/order", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let connector_router_data = noon::NoonRouterData::from((refund_amount, req, None));
        let connector_req = noon::NoonPaymentsActionRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(RefundExecuteType::get_headers(self, req, connectors)?)
            .set_body(RefundExecuteType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: noon::RefundResponse = res
            .response
            .parse_struct("noon RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Noon {
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
        Ok(format!(
            "{}payment/v1/order/getbyreference/{}",
            self.base_url(connectors),
            req.connector_request_reference_id
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: noon::RefundSyncResponse = res
            .response
            .parse_struct("noon RefundSyncResponse")
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
}

impl api::ConnectorRedirectResponse for Noon {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: enums::PaymentAction,
    ) -> CustomResult<enums::CallConnectorAction, errors::ConnectorError> {
        match action {
            enums::PaymentAction::PSync
            | enums::PaymentAction::CompleteAuthorize
            | enums::PaymentAction::PaymentAuthenticateCompleteAuthorize => {
                Ok(enums::CallConnectorAction::Trigger)
            }
        }
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Noon {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha512))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let webhook_body: noon::NoonWebhookSignature = request
            .body
            .parse_struct("NoonWebhookSignature")
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
        let signature = webhook_body.signature;
        common_utils::consts::BASE64_ENGINE
            .decode(signature)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let webhook_body: noon::NoonWebhookBody = request
            .body
            .parse_struct("NoonWebhookBody")
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
        let message = format!(
            "{},{},{},{},{}",
            webhook_body.order_id,
            webhook_body.order_status,
            webhook_body.event_id,
            webhook_body.event_type,
            webhook_body.time_stamp,
        );
        Ok(message.into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
        let details: noon::NoonWebhookOrderId = request
            .body
            .parse_struct("NoonWebhookOrderId")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(
                details.order_id.to_string(),
            ),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let details: noon::NoonWebhookEvent = request
            .body
            .parse_struct("NoonWebhookEvent")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(match &details.event_type {
            noon::NoonWebhookEventTypes::Sale | noon::NoonWebhookEventTypes::Capture => {
                match &details.order_status {
                    noon::NoonPaymentStatus::Captured => IncomingWebhookEvent::PaymentIntentSuccess,
                    _ => Err(errors::ConnectorError::WebhookEventTypeNotFound)?,
                }
            }
            noon::NoonWebhookEventTypes::Fail => IncomingWebhookEvent::PaymentIntentFailure,
            noon::NoonWebhookEventTypes::Authorize
            | noon::NoonWebhookEventTypes::Authenticate
            | noon::NoonWebhookEventTypes::Refund
            | noon::NoonWebhookEventTypes::Unknown => IncomingWebhookEvent::EventNotSupported,
        })
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let resource: noon::NoonWebhookObject = request
            .body
            .parse_struct("NoonWebhookObject")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(Box::new(noon::NoonPaymentsResponse::from(resource)))
    }
}

static NOON_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::Manual,
        enums::CaptureMethod::SequentialAutomatic,
    ];

    let supported_card_network = vec![
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::Visa,
    ];

    let mut noon_supported_payment_methods = SupportedPaymentMethods::new();

    noon_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    noon_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Debit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    noon_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::ApplePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    noon_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::GooglePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    noon_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::Paypal,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    noon_supported_payment_methods
});

static NOON_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Noon",
    description: "Noon is a payment gateway and PSP enabling secure online transactions",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static NOON_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] = [enums::EventClass::Payments];

impl ConnectorSpecifications for Noon {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&NOON_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*NOON_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&NOON_SUPPORTED_WEBHOOK_FLOWS)
    }
}
