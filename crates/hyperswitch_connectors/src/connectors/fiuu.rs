pub mod transformers;

use std::{
    any::type_name,
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use common_enums::{CaptureMethod, PaymentMethod, PaymentMethodType};
use common_utils::{
    crypto::{self, GenerateDigest},
    errors::{self as common_errors, CustomResult},
    ext_traits::{ByteSliceExt, BytesExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
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
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
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
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, PeekInterface, Secret};
use reqwest::multipart::Form;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use transformers::{self as fiuu, ExtraParameters, FiuuWebhooksResponse};

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, PaymentMethodDataType},
};

pub fn parse_and_log_keys_in_url_encoded_response<T>(data: &[u8]) {
    match std::str::from_utf8(data) {
        Ok(query_str) => {
            let loggable_keys = [
                "status",
                "orderid",
                "tranID",
                "nbcb",
                "amount",
                "currency",
                "paydate",
                "channel",
                "error_desc",
                "error_code",
                "extraP",
            ];
            let keys: Vec<(Cow<'_, str>, String)> =
                url::form_urlencoded::parse(query_str.as_bytes())
                    .map(|(key, value)| {
                        if loggable_keys.contains(&key.to_string().as_str()) {
                            (key, value.to_string())
                        } else {
                            (key, "SECRET".to_string())
                        }
                    })
                    .collect();
            router_env::logger::info!("Keys in {} response\n{:?}", type_name::<T>(), keys);
        }
        Err(err) => {
            router_env::logger::error!("Failed to convert bytes to string: {:?}", err);
        }
    }
}

fn parse_response<T>(data: &[u8]) -> Result<T, errors::ConnectorError>
where
    T: for<'de> Deserialize<'de>,
{
    let response_str = String::from_utf8(data.to_vec()).map_err(|e| {
        router_env::logger::error!("Error in Deserializing Response Data: {:?}", e);
        errors::ConnectorError::ResponseDeserializationFailed
    })?;

    let mut json = serde_json::Map::new();
    let mut miscellaneous: HashMap<String, Secret<String>> = HashMap::new();

    for line in response_str.lines() {
        if let Some((key, value)) = line.split_once('=') {
            if key.trim().is_empty() {
                router_env::logger::error!("Null or empty key encountered in response.");
                continue;
            }

            if let Some(old_value) = json.insert(key.to_string(), Value::String(value.to_string()))
            {
                router_env::logger::warn!("Repeated key encountered: {}", key);
                miscellaneous.insert(key.to_string(), Secret::new(old_value.to_string()));
            }
        }
    }
    if !miscellaneous.is_empty() {
        let misc_value = serde_json::to_value(miscellaneous).map_err(|e| {
            router_env::logger::error!("Error serializing miscellaneous data: {:?}", e);
            errors::ConnectorError::ResponseDeserializationFailed
        })?;
        json.insert("miscellaneous".to_string(), misc_value);
    }

    // TODO: Remove this after debugging
    let loggable_keys = [
        "StatCode",
        "StatName",
        "TranID",
        "ErrorCode",
        "ErrorDesc",
        "miscellaneous",
    ];
    let keys: Vec<(&str, Value)> = json
        .iter()
        .map(|(key, value)| {
            if loggable_keys.contains(&key.as_str()) {
                (key.as_str(), value.to_owned())
            } else {
                (key.as_str(), Value::String("SECRET".to_string()))
            }
        })
        .collect();
    router_env::logger::info!("Keys in response for type {}\n{:?}", type_name::<T>(), keys);

    let response: T = serde_json::from_value(Value::Object(json)).map_err(|e| {
        router_env::logger::error!("Error in Deserializing Response Data: {:?}", e);
        errors::ConnectorError::ResponseDeserializationFailed
    })?;

    Ok(response)
}
#[derive(Clone)]
pub struct Fiuu {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Fiuu {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

impl api::Payment for Fiuu {}
impl api::PaymentSession for Fiuu {}
impl api::ConnectorAccessToken for Fiuu {}
impl api::MandateSetup for Fiuu {}
impl api::PaymentAuthorize for Fiuu {}
impl api::PaymentSync for Fiuu {}
impl api::PaymentCapture for Fiuu {}
impl api::PaymentVoid for Fiuu {}
impl api::Refund for Fiuu {}
impl api::RefundExecute for Fiuu {}
impl api::RefundSync for Fiuu {}
impl api::PaymentToken for Fiuu {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Fiuu
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Fiuu
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        Ok(header)
    }
}

impl ConnectorCommon for Fiuu {
    fn id(&self) -> &'static str {
        "fiuu"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.fiuu.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: fiuu::FiuuErrorResponse = res
            .response
            .parse_struct("FiuuErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code.clone(),
            message: response.error_desc.clone(),
            reason: Some(response.error_desc.clone()),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}
pub fn build_form_from_struct<T: Serialize>(data: T) -> Result<Form, common_errors::ParsingError> {
    let mut form = Form::new();
    let serialized = serde_json::to_value(&data).map_err(|e| {
        router_env::logger::error!("Error serializing data to JSON value: {:?}", e);
        common_errors::ParsingError::EncodeError("json-value")
    })?;
    let serialized_object = serialized.as_object().ok_or_else(|| {
        router_env::logger::error!("Error: Expected JSON object but got something else");
        common_errors::ParsingError::EncodeError("Expected object")
    })?;
    for (key, values) in serialized_object {
        let value = match values {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(_) | Value::Object(_) | Value::Null => {
                router_env::logger::error!(serialization_error =? "Form Construction Failed.");
                "".to_string()
            }
        };
        form = form.text(key.clone(), value.clone());
    }
    Ok(form)
}

impl ConnectorValidation for Fiuu {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<CaptureMethod>,
        _payment_method: PaymentMethod,
        _pmt: Option<PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            CaptureMethod::Automatic
            | CaptureMethod::Manual
            | CaptureMethod::SequentialAutomatic => Ok(()),
            CaptureMethod::ManualMultiple | CaptureMethod::Scheduled => Err(
                utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }
    fn validate_mandate_payment(
        &self,
        pm_type: Option<PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd: HashSet<PaymentMethodDataType> =
            HashSet::from([PaymentMethodDataType::Card]);
        utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Fiuu {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Fiuu {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Fiuu {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Fiuu {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let optional_is_mit_flow = req.request.off_session;
        let optional_is_nti_flow = req
            .request
            .mandate_id
            .as_ref()
            .map(|mandate_id| mandate_id.is_network_transaction_id_flow());
        let url = match (optional_is_mit_flow, optional_is_nti_flow) {
            (Some(true), Some(false)) => format!(
                "{}/RMS/API/Recurring/input_v7.php",
                self.base_url(connectors)
            ),
            _ => {
                format!(
                    "{}RMS/API/Direct/1.4.0/index.php",
                    self.base_url(connectors)
                )
            }
        };
        Ok(url)
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = fiuu::FiuuRouterData::from((amount, req));
        let optional_is_mit_flow = req.request.off_session;
        let optional_is_nti_flow = req
            .request
            .mandate_id
            .as_ref()
            .map(|mandate_id| mandate_id.is_network_transaction_id_flow());

        let connector_req = match (optional_is_mit_flow, optional_is_nti_flow) {
            (Some(true), Some(false)) => {
                let recurring_request = fiuu::FiuuMandateRequest::try_from(&connector_router_data)?;
                build_form_from_struct(recurring_request)
                    .change_context(errors::ConnectorError::ParsingFailed)?
            }
            _ => {
                let payment_request = fiuu::FiuuPaymentRequest::try_from(&connector_router_data)?;
                build_form_from_struct(payment_request)
                    .change_context(errors::ConnectorError::ParsingFailed)?
            }
        };
        Ok(RequestContent::FormData(connector_req))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
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
        let response: fiuu::FiuuPaymentsResponse = res
            .response
            .parse_struct("Fiuu FiuuPaymentsResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Fiuu {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_url(
        &self,
        _req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = connectors.fiuu.secondary_base_url.clone();
        Ok(format!("{}RMS/API/gate-query/index.php", base_url))
    }
    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let sync_request = fiuu::FiuuPaymentSyncRequest::try_from(req)?;
        let connector_req = build_form_from_struct(sync_request)
            .change_context(errors::ConnectorError::ParsingFailed)?;
        Ok(RequestContent::FormData(connector_req))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .set_body(types::PaymentsSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        match res.headers {
            Some(headers) => {
                let content_header = utils::get_http_header("Content-type", &headers)
                    .attach_printable("Missing content type in headers")
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
                let response: fiuu::FiuuPaymentResponse = if content_header
                    == "text/plain;charset=UTF-8"
                {
                    parse_response(&res.response)
                } else {
                    Err(errors::ConnectorError::ResponseDeserializationFailed)
                        .attach_printable(format!("Expected content type to be text/plain;charset=UTF-8 , but received different content type as {content_header} in response"))?
                }?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
                .change_context(errors::ConnectorError::ResponseHandlingFailed)
            }
            None => {
                // We don't get headers for payment webhook response handling
                let response: fiuu::FiuuPaymentResponse = res
                    .response
                    .parse_struct("fiuu::FiuuPaymentResponse")
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
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Fiuu {
    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = connectors.fiuu.secondary_base_url.clone();
        Ok(format!("{}RMS/API/capstxn/index.php", base_url))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;

        let connector_router_data = fiuu::FiuuRouterData::from((amount, req));
        let connector_req = build_form_from_struct(fiuu::PaymentCaptureRequest::try_from(
            &connector_router_data,
        )?)
        .change_context(errors::ConnectorError::ParsingFailed)?;
        Ok(RequestContent::FormData(connector_req))
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
        let response: fiuu::PaymentCaptureResponse = res
            .response
            .parse_struct("Fiuu PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Fiuu {
    fn get_url(
        &self,
        _req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = connectors.fiuu.secondary_base_url.clone();
        Ok(format!("{}RMS/API/refundAPI/refund.php", base_url))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = build_form_from_struct(fiuu::FiuuPaymentCancelRequest::try_from(req)?)
            .change_context(errors::ConnectorError::ParsingFailed)?;
        Ok(RequestContent::FormData(connector_req))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        );

        Ok(request)
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: fiuu::FiuuPaymentCancelResponse = parse_response(&res.response)?;
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Fiuu {
    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = connectors.fiuu.secondary_base_url.clone();
        Ok(format!("{}RMS/API/refundAPI/index.php", base_url))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data = fiuu::FiuuRouterData::from((refund_amount, req));
        let connector_req =
            build_form_from_struct(fiuu::FiuuRefundRequest::try_from(&connector_router_data)?)
                .change_context(errors::ConnectorError::ParsingFailed)?;
        Ok(RequestContent::FormData(connector_req))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
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
        let response: fiuu::FiuuRefundResponse = res
            .response
            .parse_struct("fiuu FiuuRefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Fiuu {
    fn get_url(
        &self,
        _req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = connectors.fiuu.secondary_base_url.clone();
        Ok(format!("{}RMS/API/refundAPI/q_by_txn.php", base_url))
    }

    fn get_request_body(
        &self,
        req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = build_form_from_struct(fiuu::FiuuRefundSyncRequest::try_from(req)?)
            .change_context(errors::ConnectorError::ParsingFailed)?;
        Ok(RequestContent::FormData(connector_req))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
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
        let response: fiuu::FiuuRefundSyncResponse = res
            .response
            .parse_struct("fiuu FiuuRefundSyncResponse")
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
impl webhooks::IncomingWebhook for Fiuu {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Md5))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let header = utils::get_header_key_value("content-type", request.headers)?;
        let resource: FiuuWebhooksResponse = if header == "application/x-www-form-urlencoded" {
            parse_and_log_keys_in_url_encoded_response::<FiuuWebhooksResponse>(request.body);
            serde_urlencoded::from_bytes::<FiuuWebhooksResponse>(request.body)
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?
        } else {
            request
                .body
                .parse_struct("fiuu::FiuuWebhooksResponse")
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?
        };

        let signature = match resource {
            FiuuWebhooksResponse::FiuuWebhookPaymentResponse(webhooks_payment_response) => {
                webhooks_payment_response.skey
            }
            FiuuWebhooksResponse::FiuuWebhookRefundResponse(webhooks_refunds_response) => {
                webhooks_refunds_response.signature
            }
        };
        hex::decode(signature.expose())
            .change_context(errors::ConnectorError::WebhookVerificationSecretInvalid)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let header = utils::get_header_key_value("content-type", request.headers)?;
        let resource: FiuuWebhooksResponse = if header == "application/x-www-form-urlencoded" {
            parse_and_log_keys_in_url_encoded_response::<FiuuWebhooksResponse>(request.body);
            serde_urlencoded::from_bytes::<FiuuWebhooksResponse>(request.body)
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?
        } else {
            request
                .body
                .parse_struct("fiuu::FiuuWebhooksResponse")
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?
        };
        let verification_message = match resource {
            FiuuWebhooksResponse::FiuuWebhookPaymentResponse(webhooks_payment_response) => {
                let key0 = format!(
                    "{}{}{}{}{}{}",
                    webhooks_payment_response.tran_id,
                    webhooks_payment_response.order_id,
                    webhooks_payment_response.status,
                    webhooks_payment_response.domain.clone().peek(),
                    webhooks_payment_response.amount.get_amount_as_string(),
                    webhooks_payment_response.currency
                );
                let md5_key0 = hex::encode(
                    crypto::Md5
                        .generate_digest(key0.as_bytes())
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                );
                let key1 = format!(
                    "{}{}{}{}{}",
                    webhooks_payment_response.paydate,
                    webhooks_payment_response.domain.peek(),
                    md5_key0,
                    webhooks_payment_response
                        .appcode
                        .map_or("".to_string(), |appcode| appcode.expose()),
                    String::from_utf8_lossy(&connector_webhook_secrets.secret)
                );
                key1
            }
            FiuuWebhooksResponse::FiuuWebhookRefundResponse(webhooks_refunds_response) => {
                format!(
                    "{}{}{}{}{}{}{}{}",
                    webhooks_refunds_response.refund_type,
                    webhooks_refunds_response.merchant_id.peek(),
                    webhooks_refunds_response.ref_id,
                    webhooks_refunds_response.refund_id,
                    webhooks_refunds_response.txn_id,
                    webhooks_refunds_response.amount.get_amount_as_string(),
                    webhooks_refunds_response.status,
                    String::from_utf8_lossy(&connector_webhook_secrets.secret)
                )
            }
        };
        Ok(verification_message.as_bytes().to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let header = utils::get_header_key_value("content-type", request.headers)?;
        let resource: FiuuWebhooksResponse = if header == "application/x-www-form-urlencoded" {
            parse_and_log_keys_in_url_encoded_response::<FiuuWebhooksResponse>(request.body);
            serde_urlencoded::from_bytes::<FiuuWebhooksResponse>(request.body)
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?
        } else {
            request
                .body
                .parse_struct("fiuu::FiuuWebhooksResponse")
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?
        };
        let resource_id = match resource {
            FiuuWebhooksResponse::FiuuWebhookPaymentResponse(webhooks_payment_response) => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::PaymentAttemptId(
                        webhooks_payment_response.order_id,
                    ),
                )
            }
            FiuuWebhooksResponse::FiuuWebhookRefundResponse(webhooks_refunds_response) => {
                api_models::webhooks::ObjectReferenceId::RefundId(
                    api_models::webhooks::RefundIdType::ConnectorRefundId(
                        webhooks_refunds_response.refund_id,
                    ),
                )
            }
        };
        Ok(resource_id)
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let header = utils::get_header_key_value("content-type", request.headers)?;
        let resource: FiuuWebhooksResponse = if header == "application/x-www-form-urlencoded" {
            parse_and_log_keys_in_url_encoded_response::<FiuuWebhooksResponse>(request.body);
            serde_urlencoded::from_bytes::<FiuuWebhooksResponse>(request.body)
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?
        } else {
            request
                .body
                .parse_struct("fiuu::FiuuWebhooksResponse")
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?
        };

        match resource {
            FiuuWebhooksResponse::FiuuWebhookPaymentResponse(webhooks_payment_response) => Ok(
                api_models::webhooks::IncomingWebhookEvent::from(webhooks_payment_response.status),
            ),
            FiuuWebhooksResponse::FiuuWebhookRefundResponse(webhooks_refunds_response) => Ok(
                api_models::webhooks::IncomingWebhookEvent::from(webhooks_refunds_response.status),
            ),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let header = utils::get_header_key_value("content-type", request.headers)?;
        let payload: FiuuWebhooksResponse = if header == "application/x-www-form-urlencoded" {
            parse_and_log_keys_in_url_encoded_response::<FiuuWebhooksResponse>(request.body);
            serde_urlencoded::from_bytes::<FiuuWebhooksResponse>(request.body)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?
        } else {
            request
                .body
                .parse_struct("fiuu::FiuuWebhooksResponse")
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?
        };

        match payload.clone() {
            FiuuWebhooksResponse::FiuuWebhookPaymentResponse(webhook_payment_response) => Ok(
                Box::new(fiuu::FiuuPaymentResponse::FiuuWebhooksPaymentResponse(
                    webhook_payment_response,
                )),
            ),
            FiuuWebhooksResponse::FiuuWebhookRefundResponse(webhook_refund_response) => {
                Ok(Box::new(fiuu::FiuuRefundSyncResponse::Webhook(
                    webhook_refund_response,
                )))
            }
        }
    }

    fn get_mandate_details(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<hyperswitch_domain_models::router_flow_types::ConnectorMandateDetails>,
        errors::ConnectorError,
    > {
        parse_and_log_keys_in_url_encoded_response::<transformers::FiuuWebhooksPaymentResponse>(
            request.body,
        );
        let webhook_payment_response: transformers::FiuuWebhooksPaymentResponse =
            serde_urlencoded::from_bytes::<transformers::FiuuWebhooksPaymentResponse>(request.body)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        let mandate_reference = webhook_payment_response.extra_parameters.as_ref().and_then(|extra_p| {
                    let mandate_token: Result<ExtraParameters, _> = serde_json::from_str(&extra_p.clone().expose());
                    match mandate_token {
                        Ok(token) => {
                            token.token.as_ref().map(|token| hyperswitch_domain_models::router_flow_types::ConnectorMandateDetails {
                                connector_mandate_id:token.clone(),
                            })
                        }
                        Err(err) => {
                            router_env::logger::warn!(
                                "Failed to convert 'extraP' from fiuu webhook response to fiuu::ExtraParameters. \
                                 Input: '{:?}', Error: {}",
                                extra_p,
                                err
                            );
                            None
                        }
                    }
                });
        Ok(mandate_reference)
    }
}

impl ConnectorSpecifications for Fiuu {}
