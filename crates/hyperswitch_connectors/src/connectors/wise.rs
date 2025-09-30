pub mod transformers;

use api_models::webhooks::IncomingWebhookEvent;
#[cfg(feature = "payouts")]
use common_utils::request::{Method, RequestBuilder, RequestContent};
#[cfg(feature = "payouts")]
use common_utils::types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector};
use common_utils::{errors::CustomResult, ext_traits::ByteSliceExt, request::Request};
#[cfg(not(feature = "payouts"))]
use error_stack::report;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
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
use hyperswitch_domain_models::{
    router_flow_types::{
        PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoSync,
    },
    types::{PayoutsData, PayoutsResponseData, PayoutsRouterData},
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types::PayoutQuoteType;
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types::{
    PayoutCancelType, PayoutCreateType, PayoutFulfillType, PayoutRecipientType, PayoutSyncType,
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        Refund, RefundExecute, RefundSync,
    },
    configs::Connectors,
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
#[cfg(feature = "payouts")]
use masking::PeekInterface;
use masking::{Mask as _, Maskable};
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};

use self::transformers as wise;
use crate::constants::headers;
#[cfg(feature = "payouts")]
use crate::{types::ResponseRouterData, utils::convert_amount};

#[derive(Clone)]
pub struct Wise {
    #[cfg(feature = "payouts")]
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Wise {
    pub fn new() -> &'static Self {
        &Self {
            #[cfg(feature = "payouts")]
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Wise
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    #[cfg(feature = "payouts")]
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        use masking::Mask as _;

        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PayoutQuoteType::get_content_type(self).to_string().into(),
        )];
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )];
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Wise {
    fn id(&self) -> &'static str {
        "wise"
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = wise::WiseAuthType::try_from(auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.wise.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: wise::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let default_status = response.status.unwrap_or_default().get_status();
        match response.errors {
            Some(errs) => {
                if let Some(e) = errs.first() {
                    Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: e.code.clone(),
                        message: e.message.clone(),
                        reason: None,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: default_status,
                        message: response.message.unwrap_or_default(),
                        reason: None,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                }
            }
            None => Ok(ErrorResponse {
                status_code: res.status_code,
                code: default_status,
                message: response.message.unwrap_or_default(),
                reason: None,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            }),
        }
    }
}

impl api::Payment for Wise {}
impl api::PaymentAuthorize for Wise {}
impl api::PaymentSync for Wise {}
impl api::PaymentVoid for Wise {}
impl api::PaymentCapture for Wise {}
impl api::MandateSetup for Wise {}
impl api::ConnectorAccessToken for Wise {}
impl api::PaymentToken for Wise {}
impl api::ConnectorValidation for Wise {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Wise
{
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Wise {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Wise {
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Err(ConnectorError::NotImplemented("Setup Mandate flow for Wise".to_string()).into())
    }
}

impl api::PaymentSession for Wise {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Wise {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Wise {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Wise {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Wise {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Wise {}

impl api::Payouts for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutCancel for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutEligibility for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutQuote for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutRecipient for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutSync for Wise {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCancel, PayoutsData, PayoutsResponseData> for Wise {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let transfer_id = req.request.connector_payout_id.clone().ok_or(
            ConnectorError::MissingRequiredField {
                field_name: "transfer_id",
            },
        )?;
        Ok(format!(
            "{}v1/transfers/{}/cancel",
            connectors.wise.base_url, transfer_id
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, _connectors)
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Put)
            .url(&PayoutCancelType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutCancelType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoCancel>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoCancel>, ConnectorError> {
        let response: wise::WisePayoutResponse = res
            .response
            .parse_struct("WisePayoutResponse")
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
        let response: wise::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let def_res = response.status.unwrap_or_default().get_status();
        let errors = response.errors.unwrap_or_default();
        let (code, message) = if let Some(e) = errors.first() {
            (e.code.clone(), e.message.clone())
        } else {
            (def_res, response.message.unwrap_or_default())
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            code,
            message,
            reason: None,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoQuote, PayoutsData, PayoutsResponseData> for Wise {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoQuote>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}v3/profiles/{}/quotes",
            connectors.wise.base_url,
            auth.profile_id.peek()
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoQuote>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoQuote>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.source_currency,
        )?;
        let connector_router_data = wise::WiseRouterData::from((amount, req));
        let connector_req = wise::WisePayoutQuoteRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoQuote>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutQuoteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutQuoteType::get_headers(self, req, connectors)?)
            .set_body(PayoutQuoteType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoQuote>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoQuote>, ConnectorError> {
        let response: wise::WisePayoutQuoteResponse = res
            .response
            .parse_struct("WisePayoutQuoteResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoSync, PayoutsData, PayoutsResponseData> for Wise {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let transfer_id = req.request.connector_payout_id.to_owned().ok_or(
            ConnectorError::MissingRequiredField {
                field_name: "transfer_id",
            },
        )?;
        Ok(format!(
            "{}/v1/transfers/{}",
            connectors.wise.base_url, transfer_id
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&PayoutSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutSyncType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoSync>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoSync>, ConnectorError> {
        let response: wise::WisePayoutSyncResponse = res
            .response
            .parse_struct("WisePayoutSyncResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoRecipient, PayoutsData, PayoutsResponseData> for Wise {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}v1/accounts", connectors.wise.base_url))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.source_currency,
        )?;
        let connector_router_data = wise::WiseRouterData::from((amount, req));
        let connector_req = wise::WiseRecipientCreateRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutRecipientType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutRecipientType::get_headers(self, req, connectors)?)
            .set_body(PayoutRecipientType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoRecipient>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoRecipient>, ConnectorError> {
        let response: wise::WiseRecipientCreateResponse = res
            .response
            .parse_struct("WiseRecipientCreateResponse")
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

#[async_trait::async_trait]
#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData> for Wise {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}/v1/transfers", connectors.wise.base_url))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = wise::WisePayoutCreateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutCreateType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutCreateType::get_headers(self, req, connectors)?)
            .set_body(PayoutCreateType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoCreate>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoCreate>, ConnectorError> {
        let response: wise::WisePayoutResponse = res
            .response
            .parse_struct("WisePayoutResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoEligibility, PayoutsData, PayoutsResponseData> for Wise {
    fn build_request(
        &self,
        _req: &PayoutsRouterData<PoEligibility>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        // Eligibility check for cards is not implemented
        Err(ConnectorError::NotImplemented("Payout Eligibility for Wise".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Wise {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        let transfer_id = req.request.connector_payout_id.to_owned().ok_or(
            ConnectorError::MissingRequiredField {
                field_name: "transfer_id",
            },
        )?;
        Ok(format!(
            "{}v3/profiles/{}/transfers/{}/payments",
            connectors.wise.base_url,
            auth.profile_id.peek(),
            transfer_id
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = wise::WisePayoutFulfillRequest::try_from(req)?;
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
        let response: wise::WiseFulfillResponse = res
            .response
            .parse_struct("WiseFulfillResponse")
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

impl Refund for Wise {}
impl RefundExecute for Wise {}
impl RefundSync for Wise {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Wise {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Wise {}

#[cfg(feature = "payouts")]
fn is_setup_webhook_event(request: &IncomingWebhookRequestDetails<'_>) -> bool {
    let test_webhook_header = request
        .headers
        .get("X-Test-Notification")
        .and_then(|header_value| String::from_utf8(header_value.as_bytes().to_vec()).ok());

    test_webhook_header == Some("true".to_string())
}

#[async_trait::async_trait]
impl IncomingWebhook for Wise {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn common_utils::crypto::VerifySignature + Send>, ConnectorError> {
        Ok(Box::new(common_utils::crypto::RsaSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        #[cfg(feature = "payouts")] request: &IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            request
                .headers
                .get("X-Signature-SHA256")
                .map(|header_value| header_value.as_bytes().to_vec())
                .ok_or(ConnectorError::WebhookSignatureNotFound.into())
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(ConnectorError::WebhooksNotImplemented))
        }
    }

    fn get_webhook_source_verification_message(
        &self,
        #[cfg(feature = "payouts")] request: &IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            Ok(request.body.to_vec())
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(ConnectorError::WebhooksNotImplemented))
        }
    }

    fn get_webhook_object_reference_id(
        &self,
        #[cfg(feature = "payouts")] request: &IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            let payload: wise::WisePayoutsWebhookBody = request
                .body
                .parse_struct("WisePayoutsWebhookBody")
                .change_context(ConnectorError::WebhookReferenceIdNotFound)?;

            Ok(api_models::webhooks::ObjectReferenceId::PayoutId(
                api_models::webhooks::PayoutIdType::ConnectorPayoutId(
                    payload.data.resource.id.to_string(),
                ),
            ))
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(ConnectorError::WebhooksNotImplemented))
        }
    }

    fn get_webhook_event_type(
        &self,
        #[cfg(feature = "payouts")] request: &IncomingWebhookRequestDetails<'_>,
        #[cfg(not(feature = "payouts"))] _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        #[cfg(feature = "payouts")]
        {
            if is_setup_webhook_event(request) {
                return Ok(IncomingWebhookEvent::SetupWebhook);
            }

            let payload: wise::WisePayoutsWebhookBody = request
                .body
                .parse_struct("WisePayoutsWebhookBody")
                .change_context(ConnectorError::WebhookReferenceIdNotFound)?;

            Ok(transformers::get_wise_webhooks_event(
                payload.data.current_state,
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
            let payload: wise::WisePayoutsWebhookBody = request
                .body
                .parse_struct("WisePayoutsWebhookBody")
                .change_context(ConnectorError::WebhookReferenceIdNotFound)?;

            Ok(Box::new(wise::WisePayoutSyncResponse::from(payload.data)))
        }
        #[cfg(not(feature = "payouts"))]
        {
            Err(report!(ConnectorError::WebhooksNotImplemented))
        }
    }
}

static WISE_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Wise",
    description: "The Wise connector enables cross-border money transfers by integrating with Wise's API to initiate, track, and manage international payouts efficiently.",
    connector_type: common_enums::HyperswitchConnectorCategory::PayoutProcessor,
    integration_status: common_enums::ConnectorIntegrationStatus::Sandbox,
};

impl ConnectorSpecifications for Wise {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&WISE_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::enums::EventClass]> {
        None
    }
}
