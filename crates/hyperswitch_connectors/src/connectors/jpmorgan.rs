pub mod transformers;
use base64::Engine;
use std::convert::TryFrom;
use common_enums::enums;
// use crate::types::RefreshTokenRouterData;
use common_utils::{
    errors::CustomResult, ext_traits::BytesExt, request::{Method, Request, RequestBuilder, RequestContent}, types::{AmountConvertor,MinorUnit, MinorUnitForConnector}
};
use masking::{Mask, PeekInterface};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData, RefundsData, SetupMandateRequestData
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration,
        ConnectorValidation,
    }, configs::Connectors, consts, errors, events::connector_api_logs::ConnectorEvent, types::{self, RefreshTokenType, Response}, webhooks
};
use transformers::{self as jpmorgan, JpmorganErrorResponse};

use crate::{
    constants::headers, types::{ResponseRouterData, RefreshTokenRouterData}, utils
};

#[derive(Clone)]
pub struct Jpmorgan {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Jpmorgan {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::Payment for Jpmorgan {}
impl api::PaymentSession for Jpmorgan {}
impl api::ConnectorAccessToken for Jpmorgan {}
impl api::MandateSetup for Jpmorgan {}
impl api::PaymentAuthorize for Jpmorgan {}
impl api::PaymentSync for Jpmorgan {}
impl api::PaymentCapture for Jpmorgan {}
impl api::PaymentVoid for Jpmorgan {}
impl api::Refund for Jpmorgan {}
impl api::RefundExecute for Jpmorgan {}
impl api::RefundSync for Jpmorgan {}
impl api::PaymentToken for Jpmorgan {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Jpmorgan
{
    // Not Implemented (R)
}

//use masking::Secret;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Jpmorgan
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let access_token = req.access_token.clone().ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_header = (
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", access_token.token.peek()).into_masked(),
        );

        headers.push(auth_header);
        Ok(headers)
    }
}

impl ConnectorCommon for Jpmorgan {
    fn id(&self) -> &'static str {
        "jpmorgan"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
        //todo!()
        //    TODO! Check connector documentation, on which unit they are processing the currency.
        //    If the connector accepts amount in lower unit ( i.e cents for USD) then return api::CurrencyUnit::Minor,
        //    if connector accepts amount in base unit (i.e dollars for USD) then return api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.jpmorgan.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        /*let auth = jpmorgan::JpmorganAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;*/
        let str_api_key : &str = "eyJ0eXAiOiJKV1QiLCJraWQiOiJJR05rNSthbHVNdy9FeHQ4ejc5Wmg5ZVpZL0U9IiwiYWxnIjoiUlMyNTYifQ.eyJzdWIiOiI1YWE0NmMwZi0xZDRkLTQ3OGMtYmJjOC1mZjA5MWJkZDg1NWIiLCJjdHMiOiJPQVVUSDJfU1RBVEVMRVNTX0dSQU5UIiwiYXVkaXRUcmFja2luZ0lkIjoiMzQ0ZjQ5YzItMGQzNi00YWFiLTg0ZmItODVlOWY3ODMzNTcyLTI1NTY3MDYiLCJzdWJuYW1lIjoiNWFhNDZjMGYtMWQ0ZC00NzhjLWJiYzgtZmYwOTFiZGQ4NTViIiwiaXNzIjoiaHR0cHM6Ly9pZC5wYXltZW50cy5qcG1vcmdhbi5jb206NDQzL2FtL29hdXRoMiIsInRva2VuTmFtZSI6ImFjY2Vzc190b2tlbiIsInRva2VuX3R5cGUiOiJCZWFyZXIiLCJhdXRoR3JhbnRJZCI6Ik5STkhpcnhieU1PWVRfZDB2NS10c0lBYkEybyIsImNsaWVudF9pZCI6IjVhYTQ2YzBmLTFkNGQtNDc4Yy1iYmM4LWZmMDkxYmRkODU1YiIsImF1ZCI6IjVhYTQ2YzBmLTFkNGQtNDc4Yy1iYmM4LWZmMDkxYmRkODU1YiIsIm5iZiI6MTczMTM5ODczNSwiZ3JhbnRfdHlwZSI6ImNsaWVudF9jcmVkZW50aWFscyIsInNjb3BlIjpbImpwbTpwYXltZW50czpzYW5kYm94Il0sImF1dGhfdGltZSI6MTczMTM5ODczNSwicmVhbG0iOiIvYWxwaGEiLCJleHAiOjE3MzE0MDIzMzUsImlhdCI6MTczMTM5ODczNSwiZXhwaXJlc19pbiI6MzYwMCwianRpIjoiUExzekxadjRFUWhKSW1LbTR6WFo5ckd2SktjIn0.VPs_g0y3KzRQAZIBQoS8n7JQsjKWN-Oek_AK-K6W3du7NfFWcefdmDdVjrpIPcJbB4GQyecJmjpyqdC2R0LzUmdyqe5nim4E3tNRxgDfSX9u3f6D4gqYrra5hH-FUU0Cu0JiOZIXN4sMKVtgvG_Ronpd33KuTzk4tmfJCQh1XBLyYGMfLoV9yrQZ0lEtUd5lKuS8Kq7isF5uyaB962nqznZhSOx0LIein2ZAN0dc-Q-u7Pe_0Hs1TFlUNP-cwC0IRMqyRfRNALO9_Q_P-uGPjoyNqUhRzvZEnIZToTzWjaPHCXCD1YjzhWnT-QzuLFr_ddMjiRTL5m2ghuTOc4_FXA";
        let api_key : String = str_api_key.to_string();
        let masked_api_key = api_key.into_masked();

        //sending the hardcoded access token in the api key (just for testing flows)
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
            &self,
            res: Response,
            event_builder: Option<&mut ConnectorEvent>,
        ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
            let response : JpmorganErrorResponse = res.response.parse_struct("JpmorganErrorResponse").
            change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

            router_env::logger::info!(connector_response=?response);
            event_builder.map(|i| i.set_response_body(&response));

            let response_message = response.response_message.as_ref().map_or_else(|| consts::NO_ERROR_MESSAGE.to_string(), ToString::to_string);

            Ok(ErrorResponse{
                status_code : res.status_code,
                code : response.response_code,
                message : response_message.clone(),
                reason : Some(response_message),
                attempt_status : None,
                connector_transaction_id : None,
            })
    }
}

impl ConnectorValidation for Jpmorgan {
    //TODO: implement functions when support enabled
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            //enums::CaptureMethod::ManualMultiple | 
            enums::CaptureMethod::Scheduled => Err(      
                utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
            enums::CaptureMethod::ManualMultiple => Err(errors::ConnectorError::NotImplemented(     //ManualMultiple unimplemented
                utils::get_unimplemented_payment_method_error_message("Jpmorgan"),
            ))?,
        }
    }

    fn validate_psync_reference_id(
            &self,
            data: &PaymentsSyncData,
            _is_three_ds: bool,
            _status: enums::AttemptStatus,
            _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
        ) -> CustomResult<(), errors::ConnectorError> {
            if data.encoded_data.is_some() || data.connector_transaction_id.get_connector_transaction_id().is_ok(){
                return Ok(());
            }
            Err(errors::ConnectorError::MissingConnectorTransactionID.into())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Jpmorgan {
    //TODO: implement sessions flow
}

// use masking::Secret;

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Jpmorgan {
    fn get_url(
        &self,
        _req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let access_token_url = connectors.jpmorgan.secondary_base_url.as_ref().ok_or( errors::ConnectorError::FailedToObtainIntegrationUrl)?;
        Ok(format!("{}", access_token_url))
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_headers(
        &self, 
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let client_id = req.request.app_id.clone();

        let client_secret = req.request.id.clone();

        let creds = format!("{}:{}", client_id.peek(), client_secret.unwrap().peek());
        let encoded_creds = common_utils::consts::BASE64_ENGINE.encode(creds);

        let auth_string = format!("Basic {}", encoded_creds);
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            RefreshTokenType::get_content_type(self).to_string().into(),
        ),
        (
            headers::AUTHORIZATION.to_string(),
            auth_string.into_masked(),
        )])
    }

    fn get_request_body(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = jpmorgan::JpmorganAuthUpdateRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let req = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .attach_default_headers()
                .headers(RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&RefreshTokenType::get_url(self, req, connectors)?)
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
        let response: jpmorgan::JpmorganAuthUpdateResponse = res
            .response
            .parse_struct("jpmorgan JpmorganAuthUpdateResponse")
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

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Jpmorgan
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Jpmorgan {
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
        let endpoint = self.base_url(connectors);
        Ok(format!("{}/payments", endpoint))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount: MinorUnit = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = jpmorgan::JpmorganRouterData::from((amount, req));
        let connector_req = jpmorgan::JpmorganPaymentsRequest::try_from(&connector_router_data)?;
        let printrequest = common_utils::ext_traits::Encode::encode_to_string_of_json(&connector_req)
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
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
        let response: jpmorgan::JpmorganPaymentsResponse = res
            .response
            .parse_struct("Jpmorgan PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Jpmorgan {
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
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = self.base_url(connectors);
        let tid = req.request.connector_transaction_id.clone();
        Ok(format!("{}/payments/{}/captures", endpoint, tid))
        // Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount: MinorUnit = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
                
        let connector_router_data = jpmorgan::JpmorganRouterData::from((amount, req));
        let connector_req = jpmorgan::JpmorganCaptureRequest::try_from(&connector_router_data)?;
        let printrequest = common_utils::ext_traits::Encode::encode_to_string_of_json(&connector_req)
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
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
        let response: jpmorgan::JpmorganPaymentsResponse = res
            .response
            .parse_struct("Jpmorgan PaymentsCaptureResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Jpmorgan {
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
        let tid = req.request.connector_transaction_id.get_connector_transaction_id().change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let endpoint = self.base_url(connectors);
        Ok(format!("{}/payments/{}", endpoint, tid))
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
            let response: jpmorgan::JpmorganPaymentsResponse = res
                .response
                .parse_struct("jpmorgan PaymentsSyncResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Jpmorgan {
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
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = self.base_url(connectors);
        let tid = req.request.connector_transaction_id.clone();
        Ok(format!("{}/payments/{}", endpoint, tid))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount: MinorUnit = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount.unwrap(),
            req.request.currency.unwrap(),
        )?;

        let connector_router_data = jpmorgan::JpmorganRouterData::from((amount, req));
        let connector_req = jpmorgan::JpmorganCancelRequest::try_from(connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Patch)
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
        let response: jpmorgan::JpmorganCancelResponse = res
            .response
            .parse_struct("JpmrorganPaymentsVoidResponse")
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Jpmorgan {
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
        let endpoint = self.base_url(connectors);
        Ok(format!("{}/refunds", endpoint))
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

        let connector_router_data = jpmorgan::JpmorganRouterData::from((refund_amount, req));
        let connector_req = jpmorgan::JpmorganRefundRequest::try_from(&connector_router_data)?;
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
        let response: jpmorgan::JpmorganRefundResponse = res
            .response
            .parse_struct( "JpmorganRefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Jpmorgan {
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
        let endpoint = self.base_url(connectors);
        let tid = req.request.connector_transaction_id.clone();
        Ok(format!("{}/refunds/{}", endpoint, tid))
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
        let response: jpmorgan::JpmorganRefundSyncResponse = res
            .response
            .parse_struct("jpmorgan RefundSyncResponse")
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
impl webhooks::IncomingWebhook for Jpmorgan {
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
