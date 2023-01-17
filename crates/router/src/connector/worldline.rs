mod transformers;

use std::fmt::Debug;

use base64::Engine;
use bytes::Bytes;
use error_stack::{IntoReport, ResultExt};
use ring::hmac;
use time::{format_description, OffsetDateTime};
use transformers as worldline;

use crate::{
    configs::settings::Connectors,
    consts,
    core::errors::{self, CustomResult},
    headers, logger,
    services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct Worldline;

impl Worldline {
    pub fn generate_authorization_token(
        &self,
        auth: worldline::AuthType,
        http_method: &services::Method,
        content_type: &str,
        date: &str,
        endpoint: &str,
    ) -> CustomResult<String, errors::ConnectorError> {
        let signature_data: String = format!(
            "{}\n{}\n{}\n/{}\n",
            http_method,
            content_type.trim(),
            date.trim(),
            endpoint.trim()
        );
        let worldline::AuthType {
            api_key,
            api_secret,
            ..
        } = auth;
        let key = hmac::Key::new(hmac::HMAC_SHA256, api_secret.as_bytes());
        let signed_data = consts::BASE64_ENGINE.encode(hmac::sign(&key, signature_data.as_bytes()));

        Ok(format!("GCS v1HMAC:{api_key}:{signed_data}"))
    }

    pub fn get_current_date_time() -> CustomResult<String, errors::ConnectorError> {
        let format = format_description::parse(
            "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT",
        )
        .into_report()
        .change_context(errors::ConnectorError::InvalidDateFormat)?;
        OffsetDateTime::now_utc()
            .format(&format)
            .into_report()
            .change_context(errors::ConnectorError::InvalidDateFormat)
    }
}

impl ConnectorCommon for Worldline {
    fn id(&self) -> &'static str {
        "worldline"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.worldline.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: worldline::ErrorResponse = res
            .parse_struct("Worldline ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let error = response.errors.into_iter().next().unwrap_or_default();
        Ok(ErrorResponse {
            code: error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: error
                .message
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            ..Default::default()
        })
    }
}

impl api::Payment for Worldline {}

impl api::PreVerify for Worldline {}
impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Worldline
{
}

impl api::PaymentVoid for Worldline {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Worldline
{
    fn get_headers(
        &self,
        req: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let url = &types::PaymentsVoidType::get_url(self, req, connectors)?;
        let endpoint = url.clone().replace(base_url, "");
        let http_method = services::Method::Post;
        let auth = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let date = Self::get_current_date_time()?;
        let content_type = types::PaymentsAuthorizeType::get_content_type(self);
        let signed_data: String =
            self.generate_authorization_token(auth, &http_method, content_type, &date, &endpoint)?;

        Ok(vec![
            (headers::DATE.to_string(), date),
            (headers::AUTHORIZATION.to_string(), signed_data),
            (headers::CONTENT_TYPE.to_string(), content_type.to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let auth: worldline::AuthType = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id;
        let payment_id: &str = req.request.connector_transaction_id.as_ref();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}/cancel"
        ))
    }

    fn build_request(
        &self,
        req: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: worldline::PaymentResponse = res
            .response
            .parse_struct("PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(payments_cancel_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSync for Worldline {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Worldline
{
    fn get_headers(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let url = &types::PaymentsSyncType::get_url(self, req, connectors)?;
        let endpoint = url.clone().replace(base_url, "");
        let auth = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let date = Self::get_current_date_time()?;
        let signed_data: String =
            self.generate_authorization_token(auth, &services::Method::Get, "", &date, &endpoint)?;
        Ok(vec![
            (headers::DATE.to_string(), date),
            (headers::AUTHORIZATION.to_string(), signed_data),
        ])
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let base_url = self.base_url(connectors);
        let auth = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id;
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}"
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &Connectors,
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
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        logger::debug!(payment_sync_response=?res);
        let response: worldline::Payment = res
            .response
            .parse_struct("Payment")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PaymentCapture for Worldline {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Worldline
{
    // Not Implemented
}

impl api::PaymentSession for Worldline {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Worldline
{
    // Not Implemented
}

impl api::PaymentAuthorize for Worldline {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Worldline
{
    fn get_headers(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let url = &types::PaymentsAuthorizeType::get_url(self, req, connectors)?;
        let endpoint = url.clone().replace(base_url, "");
        let auth = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let date = Self::get_current_date_time()?;
        let content_type = types::PaymentsAuthorizeType::get_content_type(self);
        let signed_data: String = self.generate_authorization_token(
            auth,
            &services::Method::Post,
            content_type,
            &date,
            &endpoint,
        )?;

        Ok(vec![
            (headers::DATE.to_string(), date),
            (headers::AUTHORIZATION.to_string(), signed_data),
            (headers::CONTENT_TYPE.to_string(), content_type.to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let auth = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id;
        Ok(format!("{base_url}v1/{merchant_account_id}/payments"))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let worldline_req = utils::Encode::<worldline::PaymentsRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(worldline_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        connectors: &Connectors,
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
        let response: worldline::PaymentResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(worldlinepayments_create_response=?response);
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
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Worldline {}
impl api::RefundExecute for Worldline {}
impl api::RefundSync for Worldline {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Worldline
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let url = &types::RefundExecuteType::get_url(self, req, connectors)?;
        let endpoint = url.clone().replace(base_url, "");
        let auth = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let date = Self::get_current_date_time()?;
        let content_type = types::RefundExecuteType::get_content_type(self);
        let signed_data: String = self.generate_authorization_token(
            auth,
            &services::Method::Post,
            content_type,
            &date,
            &endpoint,
        )?;

        Ok(vec![
            (headers::DATE.to_string(), date),
            (headers::AUTHORIZATION.to_string(), signed_data),
            (headers::CONTENT_TYPE.to_string(), content_type.to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = req.request.connector_transaction_id.clone();
        let base_url = self.base_url(connectors);
        let auth = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id;
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}/refund"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let refund_req =
            utils::Encode::<worldline::WorldlineRefundRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(refund_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &Connectors,
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
        logger::debug!(target: "router::connector::worldline", response=?res);
        let response: worldline::RefundResponse = res
            .response
            .parse_struct("worldline RefundResponse")
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
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Worldline
{
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let url = &types::RefundSyncType::get_url(self, req, connectors)?;
        let endpoint = url.clone().replace(base_url, "");
        let auth = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let date = Self::get_current_date_time()?;
        let signed_data: String =
            self.generate_authorization_token(auth, &services::Method::Get, "", &date, &endpoint)?;

        Ok(vec![
            (headers::DATE.to_string(), date),
            (headers::AUTHORIZATION.to_string(), signed_data),
        ])
    }

    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req
            .response
            .as_ref()
            .ok()
            .get_required_value("response")
            .change_context(errors::ConnectorError::FailedToObtainIntegrationUrl)?
            .connector_refund_id
            .clone();
        let base_url = self.base_url(connectors);
        let auth: worldline::AuthType = worldline::AuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id;
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/refunds/{refund_id}/"
        ))
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        logger::debug!(target: "router::connector::worldline", response=?res);
        let response: worldline::RefundResponse = res
            .response
            .parse_struct("worldline RefundResponse")
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
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Worldline {
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

impl services::ConnectorRedirectResponse for Worldline {}
