mod transformers;

use common_utils::{date_time, crypto::{SignMessage, self}, ext_traits::Encode};
use hex::encode;
use time::{format_description::{self, well_known}, OffsetDateTime, PrimitiveDateTime, serde};
use std::fmt::Debug;
use error_stack::{ResultExt, IntoReport};

use crate::{
    configs::settings,
    utils::{self, BytesExt},
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger, services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    }
};


use transformers as dlocal;

#[derive(Debug, Clone)]
pub struct Dlocal;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Dlocal
where
    Self: ConnectorIntegration<Flow, Request, Response>,{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let dlocal_req = match self.get_request_body(req)? {
            Some(val) => val,
            None => "".to_string()
        };
        // #[serde(with = "common_utils::custom_serde::iso8601")]
        // let date:PrimitiveDateTime = date_time::now();
        let format = format_description::parse(
            "[year]-[month]-[day]T[hour]:[minute]:[second].000Z",
        )
        .into_report()
        .change_context(errors::ConnectorError::InvalidDateFormat)?;
        let date = date_time::now()
                    .format(&format)
                    .into_report()
                    .change_context(errors::ConnectorError::InvalidDateFormat)?;

        let auth = dlocal::DlocalAuthType::try_from(&req.connector_auth_type)?;
        let reqForSign: String = format!("{}{}{}",auth.xLogin.to_string(),date.to_string(),dlocal_req);
        let authz =
            crypto::HmacSha256::sign_message(
                &crypto::HmacSha256
                ,auth.secret.as_bytes()
                ,reqForSign.as_bytes())
                .change_context(errors::ConnectorError::RequestEncodingFailed)
                .attach_printable("Failed to sign the message")?;
        let auth_string: String = format!("{}{}","V2-HMAC-SHA256, Signature: ".to_string(),hex::encode(authz));
        let headers = vec![
            (headers::AUTHORIZATION.to_string(), auth_string),
            (headers::X_LOGIN.to_string(), auth.xLogin.to_string()),
            (headers::X_TRANS_KEY.to_string(), auth.xTransKey.to_string()),
            (headers::X_VERSION.to_string(), "2.1".to_string()),
            (headers::X_DATE.to_string(), date.to_string()),
            (headers::CONTENT_TYPE.to_string(), "application/json".to_string())
            ];
        Ok(headers)
    }
}

impl ConnectorCommon for Dlocal {
    fn id(&self) -> &'static str {
        "dlocal"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.dlocal.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: dlocal::DlocalErrorResponse = res
            .response
            .parse_struct("Dlocal ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code.to_string(),
            message: response.message,
            reason: response.param,
        })
    }
}

impl api::Payment for Dlocal {}

impl api::PreVerify for Dlocal {}
impl
    ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Dlocal
{
}

impl api::PaymentVoid for Dlocal {}

impl
    ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Dlocal
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
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let cancelData = dlocal::DlocalPaymentsCancelRequest::try_from(req)?;
        Ok(format!(
            "{}{}{}{}",
            self.base_url(connectors),
            "payments/",
            cancelData.cancel_id,
            "/cancel"
        ))
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .headers(types::PaymentsVoidType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: dlocal::DlocalPaymentsResponse = res
            .response
            .parse_struct("Dlocal PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(dlocalpayments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        println!("logging response void");
        println!("{:#?}",res.response);
        self.build_error_response(res)
    }
}

impl api::ConnectorAccessToken for Dlocal {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Dlocal
{
}

impl api::PaymentSync for Dlocal {}
impl
    ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Dlocal
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
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let syncData = dlocal::DlocalPaymentsSyncRequest::try_from(req)?;
        Ok(format!("{}{}{}{}",
            self.base_url(connectors)
            , "payments/"
            , syncData.authz_id
            , "/status"))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
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
        println!("logging response");
        println!("{:#?}",res.response);
        logger::debug!(payment_sync_response=?res);
        let response: dlocal:: DlocalPaymentsResponse = res
            .response
            .parse_struct("dlocal PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}


impl api::PaymentCapture for Dlocal {}
impl
    ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Dlocal
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
            "{}{}",
            self.base_url(connectors),
            "payments"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let dlocalReq = utils::Encode::<dlocal::DlocalPaymentsCaptureRequest>::convert_and_encode(req).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(dlocalReq))
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
        let response: dlocal::DlocalPaymentsResponse = res
            .response
            .parse_struct("Dlocal PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(dlocalpayments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        println!("logging response capture");
        println!("{:#?}",res.response);
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Dlocal {}

impl
    ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Dlocal
{
    //TODO: implement sessions flow
}

impl api::PaymentAuthorize for Dlocal {}

impl
    ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Dlocal {
    fn get_headers(&self, req: &types::PaymentsAuthorizeRouterData, connectors: &settings::Connectors,) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &types::PaymentsAuthorizeRouterData, connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "secure_payments"))
    }

    fn get_request_body(&self, req: &types::PaymentsAuthorizeRouterData) -> CustomResult<Option<String>,errors::ConnectorError> {
        let dlocal_req =
            utils::Encode::<dlocal::DlocalPaymentsRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(dlocal_req))
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
    ) -> CustomResult<types::PaymentsAuthorizeRouterData,errors::ConnectorError> {
        println!("logging response");
        println!("{:#?}",res.response);
        let response: dlocal::DlocalPaymentsResponse = res.response.parse_struct("DlocalPaymentsResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(dlocalpayments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        println!("logging response");
        println!("{:#?}",res.response);
        self.build_error_response(res)
    }
}

impl api::Refund for Dlocal {}
impl api::RefundExecute for Dlocal {}
impl api::RefundSync for Dlocal {}

impl
    ConnectorIntegration<
        api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > for Dlocal {
    fn get_headers(&self, req: &types::RefundsRouterData<api::Execute>, connectors: &settings::Connectors,) -> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &types::RefundsRouterData<api::Execute>, _connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(&self, req: &types::RefundsRouterData<api::Execute>) -> CustomResult<Option<String>,errors::ConnectorError> {
        let dlocal_req = utils::Encode::<dlocal::DlocalRefundRequest>::convert_and_encode(req).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(dlocal_req))
    }

    fn build_request(&self, req: &types::RefundsRouterData<api::Execute>, connectors: &settings::Connectors,) -> CustomResult<Option<services::Request>,errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(types::RefundExecuteType::get_headers(self, req, connectors)?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>,errors::ConnectorError> {
        logger::debug!(target: "router::connector::dlocal", response=?res);
        let response: dlocal::RefundResponse = res.response.parse_struct("dlocal RefundResponse").change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Dlocal {
    fn get_headers(&self, req: &types::RefundSyncRouterData,connectors: &settings::Connectors,) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &types::RefundSyncRouterData,_connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData,errors::ConnectorError,> {
        logger::debug!(target: "router::connector::dlocal", response=?res);
        let response: dlocal::RefundResponse = res.response.parse_struct("dlocal RefundResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Dlocal {
    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}

impl services::ConnectorRedirectResponse for Dlocal {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
