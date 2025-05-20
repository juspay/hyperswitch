pub mod transformers;

use base64::Engine;
use common_utils::{
    consts,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
};
use std::sync::LazyLock;
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
    router_response_types::{SupportedPaymentMethodsExt, PaymentMethodDetails, ConnectorInfo, SupportedPaymentMethods, PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData, PaymentsCancelRouterData,
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
use masking::{ExposeInterface, Mask};
use transformers as worldpayxml;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Worldpayxml {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Worldpayxml {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl api::Payment for Worldpayxml {}
impl api::PaymentSession for Worldpayxml {}
impl api::ConnectorAccessToken for Worldpayxml {}
impl api::MandateSetup for Worldpayxml {}
impl api::PaymentAuthorize for Worldpayxml {}
impl api::PaymentSync for Worldpayxml {}
impl api::PaymentCapture for Worldpayxml {}
impl api::PaymentVoid for Worldpayxml {}
impl api::Refund for Worldpayxml {}
impl api::RefundExecute for Worldpayxml {}
impl api::RefundSync for Worldpayxml {}
impl api::PaymentToken for Worldpayxml {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Worldpayxml
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Worldpayxml
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
            self.common_get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Worldpayxml {
    fn id(&self) -> &'static str {
        "worldpayxml"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "text/xml"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.worldpayxml.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = worldpayxml::WorldpayxmlAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let basic_auth_value = format!(
            "Basic {}",
            consts::BASE64_ENGINE.encode(format!(
                "{}:{}",
                auth.api_username.expose().to_string(),
                auth.api_password.expose().to_string()
            ))
        );

        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            Mask::into_masked(basic_auth_value),
        )])
    }
}

impl ConnectorValidation for Worldpayxml {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Worldpayxml {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Worldpayxml {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Worldpayxml
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Worldpayxml {
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
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = worldpayxml::WorldpayxmlRouterData::from((amount, req));
        let connector_req_object = worldpayxml::PaymentService::try_from(&connector_router_data)?;
        let connector_req = utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayxml::worldpayxml_constants::XML_VERSION,
            Some(worldpayxml::worldpayxml_constants::XML_ENCODING),
            None,
            worldpayxml::worldpayxml_constants::WORLDPAYXML_DOC_TYPE,
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
        router_env::logger::info!(connector_response=?res.response);
        let response: worldpayxml::PaymentService =
            utils::deserialize_xml_to_struct(&res.response)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Worldpayxml {
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
        _req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_owned())
    }

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req_object = worldpayxml::PaymentService::try_from(req)?;
        let connector_req = utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayxml::worldpayxml_constants::XML_VERSION,
            Some(worldpayxml::worldpayxml_constants::XML_ENCODING),
            None,
            worldpayxml::worldpayxml_constants::WORLDPAYXML_DOC_TYPE,
        )?;
        Ok(RequestContent::RawBytes(connector_req))
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
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        router_env::logger::info!(connector_response=?res.response);
        let response: worldpayxml::PaymentService =
            utils::deserialize_xml_to_struct(&res.response)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Worldpayxml {
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
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let connector_router_data = worldpayxml::WorldpayxmlRouterData::from((amount, req));
        let connector_req_object = worldpayxml::PaymentService::try_from(&connector_router_data)?;
        let connector_req = utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayxml::worldpayxml_constants::XML_VERSION,
            Some(worldpayxml::worldpayxml_constants::XML_ENCODING),
            None,
            worldpayxml::worldpayxml_constants::WORLDPAYXML_DOC_TYPE,
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
        router_env::logger::info!(connector_response=?res.response);
        let response: worldpayxml::PaymentService =
            utils::deserialize_xml_to_struct(&res.response)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Worldpayxml {
    fn get_url(
        &self,
        _req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(
            self.base_url(connectors).to_owned()
        )
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req_object = worldpayxml::PaymentService::try_from(req)?;
        let connector_req = utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayxml::worldpayxml_constants::XML_VERSION,
            Some(worldpayxml::worldpayxml_constants::XML_ENCODING),
            None,
            worldpayxml::worldpayxml_constants::WORLDPAYXML_DOC_TYPE,
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
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: worldpayxml::PaymentService =
            res.response
                .parse_struct("Redsys RedsysResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Worldpayxml {
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
        Ok(
            self.base_url(connectors).to_owned()
        )
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

        let connector_router_data = worldpayxml::WorldpayxmlRouterData::from((refund_amount, req));
        let connector_req =
            worldpayxml::PaymentService::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
        let response: worldpayxml::PaymentService = res
            .response
            .parse_struct("worldpayxml RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Worldpayxml {
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
        _req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_owned())
    }

    fn get_request_body(
        &self,
        req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req_object = worldpayxml::PaymentService::try_from(req)?;
        let connector_req = utils::XmlSerializer::serialize_to_xml_bytes(
            &connector_req_object,
            worldpayxml::worldpayxml_constants::XML_VERSION,
            Some(worldpayxml::worldpayxml_constants::XML_ENCODING),
            None,
            worldpayxml::worldpayxml_constants::WORLDPAYXML_DOC_TYPE,
        )?;
        Ok(RequestContent::RawBytes(connector_req))
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
        let response: worldpayxml::PaymentService = res
            .response
            .parse_struct("worldpayxml RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Worldpayxml {
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

static WORLDPAYXML_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            common_enums::CaptureMethod::Automatic,
            common_enums::CaptureMethod::Manual,
            common_enums::CaptureMethod::SequentialAutomatic,
        ];

        let supported_card_network = vec![
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::CartesBancaires,
            common_enums::CardNetwork::DinersClub,
            common_enums::CardNetwork::JCB,
            common_enums::CardNetwork::Maestro,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::Visa,
        ];

        let mut worldpayxml_supported_payment_methods = SupportedPaymentMethods::new();

        worldpayxml_supported_payment_methods.add(
            common_enums::PaymentMethod::Card,
            common_enums::PaymentMethodType::Credit,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported,
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

        worldpayxml_supported_payment_methods.add(
            common_enums::PaymentMethod::Card,
            common_enums::PaymentMethodType::Debit,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported,
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
        worldpayxml_supported_payment_methods
    });

static WORLDPAYXML_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Worldpay XML",
    description: "Worldpay is a payment gateway and PSP enabling secure online transactions",
    connector_type: common_enums::PaymentConnectorCategory::PaymentGateway,
};

static WORLDPAYXML_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 0] = [];

impl ConnectorSpecifications for Worldpayxml {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&WORLDPAYXML_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*WORLDPAYXML_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&WORLDPAYXML_SUPPORTED_WEBHOOK_FLOWS)
    }
}
