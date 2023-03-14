mod transformers;

use std::fmt::Debug;

use ::common_utils::{
    errors::ReportSwitchExt,
    ext_traits::{BytesExt, StringExt, ValueExt},
};
use error_stack::{IntoReport, ResultExt};
use transformers as nuvei;

use super::utils::{self, to_boolean, RouterData};
use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers,
    services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt, InitPayment},
        storage::enums,
        ErrorResponse, Response,
    },
    utils::{self as common_utils},
};

#[derive(Debug, Clone)]
pub struct Nuvei;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nuvei
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let headers = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string(),
        )];
        Ok(headers)
    }
}

impl ConnectorCommon for Nuvei {
    fn id(&self) -> &'static str {
        "nuvei"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.nuvei.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![])
    }
}

impl api::Payment for Nuvei {}
impl api::PreVerify for Nuvei {}
impl api::PaymentVoid for Nuvei {}
impl api::PaymentSync for Nuvei {}
impl api::PaymentCapture for Nuvei {}
impl api::PaymentSession for Nuvei {}
impl api::PaymentAuthorize for Nuvei {}
impl api::Refund for Nuvei {}
impl api::RefundExecute for Nuvei {}
impl api::RefundSync for Nuvei {}
impl api::PaymentsCompleteAuthorize for Nuvei {}
impl api::ConnectorAccessToken for Nuvei {}

impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Nuvei
{
}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/payment.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let meta: nuvei::NuveiMeta = utils::to_connector_meta(req.request.connector_meta.clone())?;
        let req_obj = nuvei::NuveiPaymentsRequest::try_from((req, meta.session_token))?;
        let req =
            common_utils::Encode::<nuvei::NuveiPaymentsRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(req))
    }
    fn build_request(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsComeplteAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .headers(types::PaymentsComeplteAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsComeplteAuthorizeType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::PaymentsCompleteAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/voidTransaction.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentFlowRequest::try_from(req)?;
        let req = common_utils::Encode::<nuvei::NuveiPaymentFlowRequest>::encode_to_string_of_json(
            &req_obj,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
            .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
            .body(types::PaymentsVoidType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
        types::PaymentsCancelRouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Nuvei
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/getPaymentStatus.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentSyncRequest::try_from(req)?;
        let req = common_utils::Encode::<nuvei::NuveiPaymentSyncRequest>::encode_to_string_of_json(
            &req_obj,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }
    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .body(types::PaymentsSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/settleTransaction.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentFlowRequest::try_from(req)?;
        let req = common_utils::Encode::<nuvei::NuveiPaymentFlowRequest>::encode_to_string_of_json(
            &req_obj,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Nuvei
{
}

#[async_trait::async_trait]
impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/payment.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    async fn execute_pretasks(
        &self,
        router_data: &mut types::PaymentsAuthorizeRouterData,
        app_state: &crate::routes::AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        let integ: Box<
            &(dyn ConnectorIntegration<
                api::AuthorizeSessionToken,
                types::AuthorizeSessionTokenData,
                types::PaymentsResponseData,
            > + Send
                  + Sync
                  + 'static),
        > = Box::new(&Self);
        let authorize_data = &types::PaymentsAuthorizeSessionTokenRouterData::from((
            &router_data,
            types::AuthorizeSessionTokenData::from(&router_data),
        ));
        let resp = services::execute_connector_processing_step(
            app_state,
            integ,
            authorize_data,
            payments::CallConnectorAction::Trigger,
        )
        .await?;
        router_data.session_token = resp.session_token;
        let (enrolled_for_3ds, related_transaction_id) = match router_data.auth_type {
            storage_models::enums::AuthenticationType::ThreeDs => {
                let integ: Box<
                    &(dyn ConnectorIntegration<
                        InitPayment,
                        types::PaymentsAuthorizeData,
                        types::PaymentsResponseData,
                    > + Send
                          + Sync
                          + 'static),
                > = Box::new(&Self);
                let init_data = &types::PaymentsInitRouterData::from((
                    &router_data,
                    router_data.request.clone(),
                ));
                let init_resp = services::execute_connector_processing_step(
                    app_state,
                    integ,
                    init_data,
                    payments::CallConnectorAction::Trigger,
                )
                .await?;
                (
                    init_resp.request.enrolled_for_3ds,
                    init_resp.request.related_transaction_id,
                )
            }
            storage_models::enums::AuthenticationType::NoThreeDs => (false, None),
        };

        router_data.request.enrolled_for_3ds = enrolled_for_3ds;
        router_data.request.related_transaction_id = related_transaction_id;
        Ok(())
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentsRequest::try_from((req, req.get_session_token()?))?;
        let req =
            common_utils::Encode::<nuvei::NuveiPaymentsRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<
        api::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
    > for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeSessionTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeSessionTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/getSessionToken.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeSessionTokenRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiSessionRequest::try_from(req)?;
        let req =
            common_utils::Encode::<nuvei::NuveiSessionRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeSessionTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsPreAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .headers(types::PaymentsPreAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsPreAuthorizeType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeSessionTokenRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeSessionTokenRouterData, errors::ConnectorError> {
        let response: nuvei::NuveiSessionResponse =
            res.response.parse_struct("NuveiSessionResponse").switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<InitPayment, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsInitRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsInitRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/initPayment.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsInitRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentsRequest::try_from((req, req.get_session_token()?))?;
        let req =
            common_utils::Encode::<nuvei::NuveiSessionRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsInitRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsInitType::get_url(self, req, connectors)?)
                .headers(types::PaymentsInitType::get_headers(self, req, connectors)?)
                .body(types::PaymentsInitType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsInitRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsInitRouterData, errors::ConnectorError> {
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
        let response_data = types::RouterData::try_from(types::ResponseRouterData {
            response: response.clone(),
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        let is_enrolled_for_3ds = response
            .clone()
            .payment_option
            .and_then(|po| po.card)
            .and_then(|c| c.three_d)
            .and_then(|t| t.v2supported)
            .map(to_boolean)
            .unwrap_or_default();
        Ok(types::RouterData {
            request: types::PaymentsAuthorizeData {
                enrolled_for_3ds: is_enrolled_for_3ds,
                related_transaction_id: response.transaction_id,
                ..response_data.request
            },
            ..response_data
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Nuvei {
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/refundTransaction.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentFlowRequest::try_from(req)?;
        let req = common_utils::Encode::<nuvei::NuveiPaymentFlowRequest>::encode_to_string_of_json(
            &req_obj,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Nuvei {}

#[async_trait::async_trait]
impl api::IncomingWebhook for Nuvei {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}

impl services::ConnectorRedirectResponse for Nuvei {
    fn get_flow_type(
        &self,
        _query_params: &str,
        json_payload: Option<serde_json::Value>,
        action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        match action {
            services::PaymentAction::PSync => Ok(payments::CallConnectorAction::Trigger),
            services::PaymentAction::CompleteAuthorize => {
                if let Some(payload) = json_payload {
                    let redirect_response: nuvei::NuveiRedirectionResponse =
                        payload.parse_value("NuveiRedirectionResponse").switch()?;
                    let acs_response: nuvei::NuveiACSResponse = redirect_response
                        .cres
                        .parse_struct("NuveiACSResponse")
                        .switch()?;
                    match acs_response.trans_status {
                        None | Some(nuvei::LiabilityShift::Failed) => {
                            Ok(payments::CallConnectorAction::StatusUpdate(
                                enums::AttemptStatus::AuthenticationFailed,
                            ))
                        }
                        _ => Ok(payments::CallConnectorAction::Trigger),
                    }
                } else {
                    Ok(payments::CallConnectorAction::Trigger)
                }
            }
        }
    }
}
