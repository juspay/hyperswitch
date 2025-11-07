pub mod transformers;

use std::sync::LazyLock;

use base64::Engine;
use common_utils::{
    consts::BASE64_ENGINE,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, PSync, PaymentMethodToken, PostCaptureVoid, Session, SetupMandate,
            Void,
        },
        refunds::{Execute, RSync},
        Accept, Dsync, Evidence, Fetch, Retrieve, Upload,
    },
    router_request_types::{
        AcceptDisputeRequestData, AccessTokenRequestData, DisputeSyncData,
        FetchDisputesRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCancelPostCaptureData, PaymentsCaptureData,
        PaymentsSessionData, PaymentsSyncData, RefundsData, RetrieveFileRequestData,
        SetupMandateRequestData, SubmitEvidenceRequestData, UploadFileRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, ConnectorInfo, DisputeSyncResponse, FetchDisputesResponse,
        PaymentMethodDetails, PaymentsResponseData, RefundsResponseData, RetrieveFileResponse,
        SubmitEvidenceResponse, SupportedPaymentMethods, SupportedPaymentMethodsExt,
        UploadFileResponse,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelPostCaptureRouterData, PaymentsCancelRouterData,
        PaymentsCaptureRouterData, PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
        SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self,
        disputes::{AcceptDispute, Dispute, DisputeSync, FetchDisputes, SubmitEvidence},
        files::{FilePurpose, FileUpload, RetrieveFile, UploadFile},
        ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts::NO_ERROR_CODE,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{Mask, PeekInterface};
use transformers as worldpayvantiv;

use crate::{
    constants::headers,
    types::{
        AcceptDisputeRouterData, DisputeSyncRouterData, FetchDisputeRouterData, ResponseRouterData,
        RetrieveFileRouterData, SubmitEvidenceRouterData, UploadFileRouterData,
    },
    utils as connector_utils,
};

#[derive(Clone)]
pub struct Worldpayvantiv {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Worldpayvantiv {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::Payment for Worldpayvantiv {}
impl api::PaymentSession for Worldpayvantiv {}
impl api::ConnectorAccessToken for Worldpayvantiv {}
impl api::MandateSetup for Worldpayvantiv {}
impl api::PaymentAuthorize for Worldpayvantiv {}
impl api::PaymentSync for Worldpayvantiv {}
impl api::PaymentCapture for Worldpayvantiv {}
impl api::PaymentVoid for Worldpayvantiv {}
impl api::Refund for Worldpayvantiv {}
impl api::RefundExecute for Worldpayvantiv {}
impl api::RefundSync for Worldpayvantiv {}
impl api::PaymentToken for Worldpayvantiv {}
impl api::PaymentPostCaptureVoid for Worldpayvantiv {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Worldpayvantiv
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Worldpayvantiv
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

impl ConnectorCommon for Worldpayvantiv {
    fn id(&self) -> &'static str {
        "worldpayvantiv"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "text/xml"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.worldpayvantiv.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = worldpayvantiv::WorldpayvantivAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_key = format!("{}:{}", auth.user.peek(), auth.password.peek());
        let auth_header = format!("Basic {}", BASE64_ENGINE.encode(auth_key));
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth_header.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: Result<worldpayvantiv::CnpOnlineResponse, _> =
            connector_utils::deserialize_xml_to_struct(&res.response);

        match response {
            Ok(response_data) => {
                event_builder.map(|i| i.set_response_body(&response_data));
                router_env::logger::info!(connector_response=?response_data);
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: response_data.response_code,
                    message: response_data.message.clone(),
                    reason: Some(response_data.message.clone()),
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Err(error_msg) => {
                event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
                router_env::logger::error!(deserialization_error =? error_msg);
                connector_utils::handle_json_response_deserialization_failure(res, "worldpayvantiv")
            }
        }
    }
}

impl ConnectorValidation for Worldpayvantiv {
    fn validate_mandate_payment(
        &self,
        pm_type: Option<api_models::enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            connector_utils::PaymentMethodDataType::Card,
            connector_utils::PaymentMethodDataType::ApplePay,
            connector_utils::PaymentMethodDataType::GooglePay,
        ]);
        connector_utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Worldpayvantiv {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Worldpayvantiv {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Worldpayvantiv
{
    fn get_headers(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_owned())
    }

    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req_object = worldpayvantiv::CnpOnlineRequest::try_from(req)?;

        router_env::logger::info!(raw_connector_request=?connector_req_object);
        let connector_req = connector_utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayvantiv::worldpayvantiv_constants::XML_VERSION,
            Some(worldpayvantiv::worldpayvantiv_constants::XML_ENCODING),
            None,
            None,
        )?;
        Ok(RequestContent::RawBytes(connector_req))
    }

    fn build_request(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::SetupMandateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::SetupMandateType::get_headers(self, req, connectors)?)
                .set_body(types::SetupMandateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SetupMandateRouterData, errors::ConnectorError> {
        let response: worldpayvantiv::CnpOnlineResponse =
            connector_utils::deserialize_xml_to_struct(&res.response)?;
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

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Worldpayvantiv
{
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
        Ok(self.base_url(connectors).to_owned())
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

        let connector_router_data = worldpayvantiv::WorldpayvantivRouterData::from((amount, req));
        let connector_req_object =
            worldpayvantiv::CnpOnlineRequest::try_from(&connector_router_data)?;

        router_env::logger::info!(raw_connector_request=?connector_req_object);
        let connector_req = connector_utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayvantiv::worldpayvantiv_constants::XML_VERSION,
            Some(worldpayvantiv::worldpayvantiv_constants::XML_ENCODING),
            None,
            None,
        )?;
        Ok(RequestContent::RawBytes(connector_req))
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
        let response: worldpayvantiv::CnpOnlineResponse =
            connector_utils::deserialize_xml_to_struct(&res.response)?;
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Worldpayvantiv {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/reports/dtrPaymentStatus/{}",
            connectors.worldpayvantiv.secondary_base_url.to_owned(),
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
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
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
        let response: worldpayvantiv::VantivSyncResponse = res
            .response
            .parse_struct("VantivSyncResponse")
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
        handle_vantiv_json_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Worldpayvantiv {
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
        Ok(self.base_url(connectors).to_owned())
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

        let connector_router_data = worldpayvantiv::WorldpayvantivRouterData::from((amount, req));
        let connector_req_object =
            worldpayvantiv::CnpOnlineRequest::try_from(&connector_router_data)?;
        router_env::logger::info!(raw_connector_request=?connector_req_object);
        let connector_req = connector_utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayvantiv::worldpayvantiv_constants::XML_VERSION,
            Some(worldpayvantiv::worldpayvantiv_constants::XML_ENCODING),
            None,
            None,
        )?;

        Ok(RequestContent::RawBytes(connector_req))
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
        let response: worldpayvantiv::CnpOnlineResponse =
            connector_utils::deserialize_xml_to_struct(&res.response)?;
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Worldpayvantiv {
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
        Ok(self.base_url(connectors).to_owned())
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req_object = worldpayvantiv::CnpOnlineRequest::try_from(req)?;
        router_env::logger::info!(raw_connector_request=?connector_req_object);

        let connector_req = connector_utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayvantiv::worldpayvantiv_constants::XML_VERSION,
            Some(worldpayvantiv::worldpayvantiv_constants::XML_ENCODING),
            None,
            None,
        )?;

        Ok(RequestContent::RawBytes(connector_req))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: worldpayvantiv::CnpOnlineResponse =
            connector_utils::deserialize_xml_to_struct(&res.response)?;
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

impl ConnectorIntegration<PostCaptureVoid, PaymentsCancelPostCaptureData, PaymentsResponseData>
    for Worldpayvantiv
{
    fn get_headers(
        &self,
        req: &PaymentsCancelPostCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCancelPostCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_owned())
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelPostCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req_object = worldpayvantiv::CnpOnlineRequest::try_from(req)?;
        router_env::logger::info!(raw_connector_request=?connector_req_object);

        let connector_req = connector_utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayvantiv::worldpayvantiv_constants::XML_VERSION,
            Some(worldpayvantiv::worldpayvantiv_constants::XML_ENCODING),
            None,
            None,
        )?;

        Ok(RequestContent::RawBytes(connector_req))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelPostCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsPostCaptureVoidType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsPostCaptureVoidType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsPostCaptureVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelPostCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelPostCaptureRouterData, errors::ConnectorError> {
        let response: worldpayvantiv::CnpOnlineResponse =
            connector_utils::deserialize_xml_to_struct(&res.response)?;
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Worldpayvantiv {
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
        Ok(self.base_url(connectors).to_owned())
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

        let connector_router_data =
            worldpayvantiv::WorldpayvantivRouterData::from((refund_amount, req));
        let connector_req_object =
            worldpayvantiv::CnpOnlineRequest::try_from(&connector_router_data)?;
        router_env::logger::info!(connector_request=?connector_req_object);
        let connector_req = connector_utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayvantiv::worldpayvantiv_constants::XML_VERSION,
            Some(worldpayvantiv::worldpayvantiv_constants::XML_ENCODING),
            None,
            None,
        )?;

        Ok(RequestContent::RawBytes(connector_req))
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
        let response: worldpayvantiv::CnpOnlineResponse =
            connector_utils::deserialize_xml_to_struct(&res.response)?;
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Worldpayvantiv {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/reports/dtrPaymentStatus/{}",
            connectors.worldpayvantiv.secondary_base_url.to_owned(),
            req.request.connector_transaction_id
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
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: worldpayvantiv::VantivSyncResponse = res
            .response
            .parse_struct("VantivSyncResponse")
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
        handle_vantiv_json_error_response(res, event_builder)
    }
}

impl Dispute for Worldpayvantiv {}
impl FetchDisputes for Worldpayvantiv {}
impl DisputeSync for Worldpayvantiv {}
impl SubmitEvidence for Worldpayvantiv {}
impl AcceptDispute for Worldpayvantiv {}

impl ConnectorIntegration<Fetch, FetchDisputesRequestData, FetchDisputesResponse>
    for Worldpayvantiv
{
    fn get_headers(
        &self,
        req: &FetchDisputeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::FetchDisputesType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (
                headers::ACCEPT.to_string(),
                types::FetchDisputesType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
        ];

        let mut auth_header = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut auth_header);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/com.vantivcnp.services-v2+xml"
    }

    fn get_url(
        &self,
        req: &FetchDisputeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let date = req.request.created_from.date();
        let day = date.day();
        let month = u8::from(date.month());
        let year = date.year();

        Ok(format!(
            "{}/services/chargebacks/?date={year}-{month}-{day}",
            connectors.worldpayvantiv.third_base_url.to_owned()
        ))
    }

    fn build_request(
        &self,
        req: &FetchDisputeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::FetchDisputesType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::FetchDisputesType::get_headers(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &FetchDisputeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FetchDisputeRouterData, errors::ConnectorError> {
        let response: worldpayvantiv::ChargebackRetrievalResponse =
            connector_utils::deserialize_xml_to_struct(&res.response)?;
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
        handle_vantiv_dispute_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Dsync, DisputeSyncData, DisputeSyncResponse> for Worldpayvantiv {
    fn get_headers(
        &self,
        req: &DisputeSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::FetchDisputesType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (
                headers::ACCEPT.to_string(),
                types::FetchDisputesType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
        ];

        let mut auth_header = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut auth_header);
        Ok(headers)
    }

    fn get_url(
        &self,
        req: &DisputeSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/services/chargebacks/{}",
            connectors.worldpayvantiv.third_base_url.to_owned(),
            req.request.connector_dispute_id
        ))
    }

    fn build_request(
        &self,
        req: &DisputeSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::DisputeSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::DisputeSyncType::get_headers(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &DisputeSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<DisputeSyncRouterData, errors::ConnectorError> {
        let response: worldpayvantiv::ChargebackRetrievalResponse =
            connector_utils::deserialize_xml_to_struct(&res.response)?;
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
        handle_vantiv_dispute_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>
    for Worldpayvantiv
{
    fn get_headers(
        &self,
        req: &SubmitEvidenceRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::FetchDisputesType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (
                headers::ACCEPT.to_string(),
                types::FetchDisputesType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
        ];

        let mut auth_header = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut auth_header);
        Ok(headers)
    }

    fn get_url(
        &self,
        req: &SubmitEvidenceRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/services/chargebacks/{}",
            connectors.worldpayvantiv.third_base_url.to_owned(),
            req.request.connector_dispute_id
        ))
    }

    fn get_request_body(
        &self,
        req: &SubmitEvidenceRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req_object = worldpayvantiv::ChargebackUpdateRequest::from(req);
        router_env::logger::info!(raw_connector_request=?connector_req_object);
        let connector_req = connector_utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayvantiv::worldpayvantiv_constants::XML_VERSION,
            Some(worldpayvantiv::worldpayvantiv_constants::XML_ENCODING),
            None,
            None,
        )?;

        Ok(RequestContent::RawBytes(connector_req))
    }

    fn build_request(
        &self,
        req: &SubmitEvidenceRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Put)
                .url(&types::SubmitEvidenceType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::SubmitEvidenceType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::SubmitEvidenceType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &SubmitEvidenceRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        _res: Response,
    ) -> CustomResult<SubmitEvidenceRouterData, errors::ConnectorError> {
        Ok(SubmitEvidenceRouterData {
            response: Ok(SubmitEvidenceResponse {
                dispute_status: data.request.dispute_status,
                connector_status: None,
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        handle_vantiv_dispute_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>
    for Worldpayvantiv
{
    fn get_headers(
        &self,
        req: &AcceptDisputeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::FetchDisputesType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (
                headers::ACCEPT.to_string(),
                types::FetchDisputesType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
        ];

        let mut auth_header = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut auth_header);
        Ok(headers)
    }

    fn get_url(
        &self,
        req: &AcceptDisputeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/services/chargebacks/{}",
            connectors.worldpayvantiv.third_base_url.to_owned(),
            req.request.connector_dispute_id
        ))
    }

    fn get_request_body(
        &self,
        req: &AcceptDisputeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req_object = worldpayvantiv::ChargebackUpdateRequest::from(req);
        router_env::logger::info!(raw_connector_request=?connector_req_object);
        let connector_req = connector_utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayvantiv::worldpayvantiv_constants::XML_VERSION,
            Some(worldpayvantiv::worldpayvantiv_constants::XML_ENCODING),
            Some(worldpayvantiv::worldpayvantiv_constants::XML_STANDALONE),
            None,
        )?;

        Ok(RequestContent::RawBytes(connector_req))
    }

    fn build_request(
        &self,
        req: &AcceptDisputeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Put)
                .url(&types::AcceptDisputeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::AcceptDisputeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::AcceptDisputeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &AcceptDisputeRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        _res: Response,
    ) -> CustomResult<AcceptDisputeRouterData, errors::ConnectorError> {
        Ok(AcceptDisputeRouterData {
            response: Ok(AcceptDisputeResponse {
                dispute_status: data.request.dispute_status,
                connector_status: None,
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        handle_vantiv_dispute_error_response(res, event_builder)
    }
}

impl UploadFile for Worldpayvantiv {}

impl ConnectorIntegration<Upload, UploadFileRequestData, UploadFileResponse> for Worldpayvantiv {
    fn get_headers(
        &self,
        req: &RouterData<Upload, UploadFileRequestData, UploadFileResponse>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![(
            headers::CONTENT_TYPE.to_string(),
            req.request.file_type.to_string().into(),
        )];

        let mut auth_header = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut auth_header);
        Ok(headers)
    }

    fn get_url(
        &self,
        req: &UploadFileRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let file_type = if req.request.file_type == mime::IMAGE_GIF {
            "gif"
        } else if req.request.file_type == mime::IMAGE_JPEG {
            "jpeg"
        } else if req.request.file_type == mime::IMAGE_PNG {
            "png"
        } else if req.request.file_type == mime::APPLICATION_PDF {
            "pdf"
        } else {
            return Err(errors::ConnectorError::FileValidationFailed {
                reason: "file_type does not match JPEG, JPG, PNG, or PDF format".to_owned(),
            })?;
        };
        let file_name = req.request.file_key.split('/').next_back().ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Failed fetching file_id from file_key".to_string(),
            ),
        )?;
        Ok(format!(
            "{}/services/chargebacks/upload/{}/{file_name}.{file_type}",
            connectors.worldpayvantiv.third_base_url.to_owned(),
            req.request.connector_dispute_id,
        ))
    }

    fn get_request_body(
        &self,
        req: &UploadFileRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Ok(RequestContent::RawBytes(req.request.file.clone()))
    }

    fn build_request(
        &self,
        req: &UploadFileRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::UploadFileType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::UploadFileType::get_headers(self, req, connectors)?)
                .set_body(types::UploadFileType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &UploadFileRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<Upload, UploadFileRequestData, UploadFileResponse>,
        errors::ConnectorError,
    > {
        let response: worldpayvantiv::ChargebackDocumentUploadResponse =
            connector_utils::deserialize_xml_to_struct(&res.response)?;
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
        handle_vantiv_dispute_error_response(res, event_builder)
    }
}

impl RetrieveFile for Worldpayvantiv {}

impl ConnectorIntegration<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>
    for Worldpayvantiv
{
    fn get_headers(
        &self,
        req: &RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_url(
        &self,
        req: &RetrieveFileRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_dispute_id = req.request.connector_dispute_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "dispute_id",
            },
        )?;
        Ok(format!(
            "{}/services/chargebacks/retrieve/{connector_dispute_id}/{}",
            connectors.worldpayvantiv.third_base_url.to_owned(),
            req.request.provider_file_id,
        ))
    }

    fn build_request(
        &self,
        req: &RetrieveFileRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RetrieveFileType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RetrieveFileType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RetrieveFileRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RetrieveFileRouterData, errors::ConnectorError> {
        let response: Result<worldpayvantiv::ChargebackDocumentUploadResponse, _> =
            connector_utils::deserialize_xml_to_struct(&res.response);
        match response {
            Ok(response) => {
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            Err(_) => {
                event_builder.map(|event| event.set_response_body(&serde_json::json!({"connector_response_type": "file", "status_code": res.status_code})));
                router_env::logger::info!(connector_response_type=?"file");
                let response = res.response;
                Ok(RetrieveFileRouterData {
                    response: Ok(RetrieveFileResponse {
                        file_data: response.to_vec(),
                    }),
                    ..data.clone()
                })
            }
        }
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        handle_vantiv_dispute_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Worldpayvantiv {
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

fn handle_vantiv_json_error_response(
    res: Response,
    event_builder: Option<&mut ConnectorEvent>,
) -> CustomResult<ErrorResponse, errors::ConnectorError> {
    let response: Result<worldpayvantiv::VantivSyncErrorResponse, _> = res
        .response
        .parse_struct("VantivSyncErrorResponse")
        .change_context(errors::ConnectorError::ResponseDeserializationFailed);

    match response {
        Ok(response_data) => {
            event_builder.map(|i| i.set_response_body(&response_data));
            router_env::logger::info!(connector_response=?response_data);
            let error_reason = response_data.error_messages.join(" & ");

            Ok(ErrorResponse {
                status_code: res.status_code,
                code: NO_ERROR_CODE.to_string(),
                message: error_reason.clone(),
                reason: Some(error_reason.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        }
        Err(error_msg) => {
            event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
            router_env::logger::error!(deserialization_error =? error_msg);
            connector_utils::handle_json_response_deserialization_failure(res, "worldpayvantiv")
        }
    }
}

fn handle_vantiv_dispute_error_response(
    res: Response,
    event_builder: Option<&mut ConnectorEvent>,
) -> CustomResult<ErrorResponse, errors::ConnectorError> {
    let response: Result<worldpayvantiv::VantivDisputeErrorResponse, _> =
        connector_utils::deserialize_xml_to_struct::<worldpayvantiv::VantivDisputeErrorResponse>(
            &res.response,
        );

    match response {
        Ok(response_data) => {
            event_builder.map(|i| i.set_response_body(&response_data));
            router_env::logger::info!(connector_response=?response_data);
            let error_reason = response_data
                .errors
                .iter()
                .map(|error_info| error_info.error.clone())
                .collect::<Vec<String>>()
                .join(" & ");

            Ok(ErrorResponse {
                status_code: res.status_code,
                code: NO_ERROR_CODE.to_string(),
                message: error_reason.clone(),
                reason: Some(error_reason.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        }
        Err(error_msg) => {
            event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
            router_env::logger::error!(deserialization_error =? error_msg);
            connector_utils::handle_json_response_deserialization_failure(res, "worldpayvantiv")
        }
    }
}

#[async_trait::async_trait]
impl FileUpload for Worldpayvantiv {
    fn validate_file_upload(
        &self,
        purpose: FilePurpose,
        file_size: i32,
        file_type: mime::Mime,
    ) -> CustomResult<(), errors::ConnectorError> {
        match purpose {
            FilePurpose::DisputeEvidence => {
                let supported_file_types = [
                    "image/gif",
                    "image/jpeg",
                    "image/jpg",
                    "application/pdf",
                    "image/png",
                    "image/tiff",
                ];
                if file_size > 2000000 {
                    Err(errors::ConnectorError::FileValidationFailed {
                        reason: "file_size exceeded the max file size of 2MB".to_owned(),
                    })?
                }
                if !supported_file_types.contains(&file_type.to_string().as_str()) {
                    Err(errors::ConnectorError::FileValidationFailed {
                        reason: "file_type does not match JPEG, JPG, PNG, or PDF format".to_owned(),
                    })?
                }
            }
        }
        Ok(())
    }
}

static WORLDPAYVANTIV_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            common_enums::CaptureMethod::Automatic,
            common_enums::CaptureMethod::Manual,
            common_enums::CaptureMethod::SequentialAutomatic,
        ];

        let supported_card_network = vec![
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::DinersClub,
            common_enums::CardNetwork::JCB,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::Visa,
            common_enums::CardNetwork::Discover,
        ];

        let mut worldpayvantiv_supported_payment_methods = SupportedPaymentMethods::new();

        worldpayvantiv_supported_payment_methods.add(
            common_enums::PaymentMethod::Card,
            common_enums::PaymentMethodType::Credit,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                        api_models::feature_matrix::CardSpecificFeatures {
                            three_ds: common_enums::FeatureStatus::NotSupported,
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_network.clone(),
                        }
                    }),
                ),
            },
        );

        worldpayvantiv_supported_payment_methods.add(
            common_enums::PaymentMethod::Card,
            common_enums::PaymentMethodType::Debit,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                        api_models::feature_matrix::CardSpecificFeatures {
                            three_ds: common_enums::FeatureStatus::NotSupported,
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_network.clone(),
                        }
                    }),
                ),
            },
        );

        #[cfg(feature = "v2")]
        worldpayvantiv_supported_payment_methods.add(
            common_enums::PaymentMethod::Card,
            common_enums::PaymentMethodType::Card,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                        api_models::feature_matrix::CardSpecificFeatures {
                            three_ds: common_enums::FeatureStatus::NotSupported,
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_network.clone(),
                        }
                    }),
                ),
            },
        );

        worldpayvantiv_supported_payment_methods.add(
            common_enums::PaymentMethod::Wallet,
            common_enums::PaymentMethodType::ApplePay,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        worldpayvantiv_supported_payment_methods.add(
            common_enums::PaymentMethod::Wallet,
            common_enums::PaymentMethodType::GooglePay,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        worldpayvantiv_supported_payment_methods
    });

static WORLDPAYVANTIV_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Worldpay Vantiv",
    description: "Worldpay Vantiv, also known as the Worldpay CNP API, is a robust XML-based interface used to process online (card-not-present) transactions such as e-commerce purchases, subscription billing, and digital payments",
    connector_type: common_enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: common_enums::ConnectorIntegrationStatus::Sandbox,
};

static WORLDPAYVANTIV_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 0] = [];

impl ConnectorSpecifications for Worldpayvantiv {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&WORLDPAYVANTIV_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*WORLDPAYVANTIV_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&WORLDPAYVANTIV_SUPPORTED_WEBHOOK_FLOWS)
    }
    #[cfg(feature = "v1")]
    fn generate_connector_request_reference_id(
        &self,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
        is_config_enabled_to_send_payment_id_as_connector_request_id: bool,
    ) -> String {
        if is_config_enabled_to_send_payment_id_as_connector_request_id
            && payment_intent.is_payment_id_from_merchant.unwrap_or(false)
        {
            payment_attempt.payment_id.get_string_repr().to_owned()
        } else {
            let max_payment_reference_id_length =
                worldpayvantiv::worldpayvantiv_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH;
            nanoid::nanoid!(max_payment_reference_id_length)
        }
    }
    #[cfg(feature = "v2")]
    fn generate_connector_request_reference_id(
        &self,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> String {
        if payment_intent.is_payment_id_from_merchant.unwrap_or(false) {
            payment_attempt.payment_id.get_string_repr().to_owned()
        } else {
            connector_utils::generate_12_digit_number().to_string()
        }
    }
}
