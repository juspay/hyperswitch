mod transformers;
use std::fmt::Debug;

use base64::Engine;
use common_utils::date_time;
use error_stack::{IntoReport, ResultExt};
use rand::distributions::{Alphanumeric, DistString};
use ring::hmac;
use transformers as rapyd;

use crate::{
    configs::settings,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger, services,
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Rapyd;

impl Rapyd {
    pub fn generate_signature(
        &self,
        auth: &rapyd::RapydAuthType,
        http_method: &str,
        url_path: &str,
        body: &str,
        timestamp: &i64,
        salt: &str,
    ) -> CustomResult<String, errors::ConnectorError> {
        let rapyd::RapydAuthType {
            access_key,
            secret_key,
        } = auth;
        let to_sign =
            format!("{http_method}{url_path}{salt}{timestamp}{access_key}{secret_key}{body}");
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret_key.as_bytes());
        let tag = hmac::sign(&key, to_sign.as_bytes());
        let hmac_sign = hex::encode(tag);
        let signature_value = consts::BASE64_ENGINE_URL_SAFE.encode(hmac_sign);
        Ok(signature_value)
    }
}

impl ConnectorCommon for Rapyd {
    fn id(&self) -> &'static str {
        "rapyd"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.rapyd.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![])
    }
}

impl api::PaymentAuthorize for Rapyd {}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self).to_string(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1/payments", self.base_url(connectors)))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let rapyd_req = utils::Encode::<rapyd::RapydPaymentsRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let signature =
            self.generate_signature(&auth, "post", "/v1/payments", &rapyd_req, &timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key),
            ("salt".to_string(), salt),
            ("timestamp".to_string(), timestamp.to_string()),
            ("signature".to_string(), signature),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsAuthorizeType::get_url(
                self, req, connectors,
            )?)
            .headers(types::PaymentsAuthorizeType::get_headers(
                self, req, connectors,
            )?)
            .headers(headers)
            .body(Some(rapyd_req))
            .build();
        Ok(Some(request))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let rapyd_req = utils::Encode::<rapyd::RapydPaymentsRequest>::convert_and_url_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(rapyd_req))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(rapydpayments_create_response=?response);
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
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status.error_code,
            message: response.status.status,
            reason: response.status.message,
        })
    }
}

impl api::Payment for Rapyd {}

impl api::PreVerify for Rapyd {}
impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Rapyd
{
}

impl api::PaymentVoid for Rapyd {}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsVoidType::get_content_type(self).to_string(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1/payments/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let url_path = format!("/v1/payments/{}", req.request.connector_transaction_id);
        let signature =
            self.generate_signature(&auth, "delete", &url_path, "", &timestamp, &salt)?;

        let headers = vec![
            ("access_key".to_string(), auth.access_key),
            ("salt".to_string(), salt),
            ("timestamp".to_string(), timestamp.to_string()),
            ("signature".to_string(), signature),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Delete)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
            .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
            .headers(headers)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(rapydpayments_create_response=?response);
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
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status.error_code,
            message: response.status.status,
            reason: response.status.message,
        })
    }
}

impl api::PaymentSync for Rapyd {}
impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("PSync".to_string()).into())
    }

    fn build_request(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(None)
    }

    fn get_error_response(
        &self,
        _res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("PSync".to_string()).into())
    }

    fn handle_response(
        &self,
        _data: &types::PaymentsSyncRouterData,
        _res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("PSync".to_string()).into())
    }
}

impl api::PaymentCapture for Rapyd {}
impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsCaptureType::get_content_type(self).to_string(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let rapyd_req = utils::Encode::<rapyd::CaptureRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(rapyd_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let rapyd_req = utils::Encode::<rapyd::CaptureRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let url_path = format!(
            "/v1/payments/{}/capture",
            req.request.connector_transaction_id
        );
        let signature =
            self.generate_signature(&auth, "post", &url_path, &rapyd_req, &timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key),
            ("salt".to_string(), salt),
            ("timestamp".to_string(), timestamp.to_string()),
            ("signature".to_string(), signature),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
            .headers(types::PaymentsCaptureType::get_headers(
                self, req, connectors,
            )?)
            .headers(headers)
            .body(Some(rapyd_req))
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("RapydPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1/payments/{}/capture",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status.error_code,
            message: response.status.status,
            reason: response.status.message,
        })
    }
}

impl api::PaymentSession for Rapyd {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Rapyd
{
    //TODO: implement sessions flow
}

impl api::Refund for Rapyd {}
impl api::RefundExecute for Rapyd {}
impl api::RefundSync for Rapyd {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefundExecuteType::get_content_type(self).to_string(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        api::ConnectorCommon::common_get_content_type(self)
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1/refunds", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let rapyd_req = utils::Encode::<rapyd::RapydRefundRequest>::convert_and_url_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(rapyd_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let rapyd_req = utils::Encode::<rapyd::RapydRefundRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let signature =
            self.generate_signature(&auth, "post", "/v1/refunds", &rapyd_req, &timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key),
            ("salt".to_string(), salt),
            ("timestamp".to_string(), timestamp.to_string()),
            ("signature".to_string(), signature),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(headers)
            .body(Some(rapyd_req))
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        logger::debug!(target: "router::connector::rapyd", response=?res);
        let response: rapyd::RefundResponse = res
            .response
            .parse_struct("rapyd RefundResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
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
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status.error_code,
            message: response.status.status,
            reason: response.status.message,
        })
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("RSync".to_string()).into())
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        logger::debug!(target: "router::connector::rapyd", response=?res);
        let response: rapyd::RefundResponse = res
            .response
            .parse_struct("rapyd RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
        _res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("RSync".to_string()).into())
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Rapyd {
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

impl services::ConnectorRedirectResponse for Rapyd {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
