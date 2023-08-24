pub mod transformers;

use std::fmt::Debug;

use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use router_env::{instrument, tracing};
use transformers as stripe_connect;

use crate::{
    configs::settings,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, routes,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct StripeConnect;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for StripeConnect
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            Self::common_get_content_type(&self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for StripeConnect {
    fn id(&self) -> &'static str {
        "stripe_connect"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.stripe_connect.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: stripe_connect::StripeConnectAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_key.peek()).into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: stripe_connect::StripeConnectErrorResponse = res
            .response
            .parse_struct("StripeConnectErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message,
        })
    }
}

impl api::Payment for StripeConnect {}
impl api::PaymentAuthorize for StripeConnect {}
impl api::PaymentSync for StripeConnect {}
impl api::PaymentVoid for StripeConnect {}
impl api::PaymentCapture for StripeConnect {}
impl api::PreVerify for StripeConnect {}
impl api::ConnectorAccessToken for StripeConnect {}
impl api::PaymentToken for StripeConnect {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for StripeConnect
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for StripeConnect
{
}

impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for StripeConnect
{
}

impl api::PaymentSession for StripeConnect {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for StripeConnect
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for StripeConnect
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for StripeConnect
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for StripeConnect
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for StripeConnect
{
}

impl api::Refund for StripeConnect {}
impl api::RefundExecute for StripeConnect {}
impl api::RefundSync for StripeConnect {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for StripeConnect
{
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for StripeConnect
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for StripeConnect {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
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

impl api::Payouts for StripeConnect {}
#[cfg(feature = "payouts")]
impl api::PayoutCancel for StripeConnect {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for StripeConnect {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for StripeConnect {}
#[cfg(feature = "payouts")]
impl api::PayoutRecipient for StripeConnect {}
#[cfg(feature = "payouts")]
impl api::PayoutRecipientAccount for StripeConnect {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoCancel, types::PayoutsData, types::PayoutsResponseData>
    for StripeConnect
{
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let transfer_id = req.request.connector_payout_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "transfer_id",
            },
        )?;
        Ok(format!(
            "{}v1/transfers/{}/reversals",
            connectors.stripe_connect.base_url, transfer_id
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, _connectors)
    }

    fn get_request_body(
        &self,
        req: &types::RouterData<api::PoCancel, types::PayoutsData, types::PayoutsResponseData>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = stripe_connect::StripeConnectReversalRequest::try_from(req)?;
        let stripe_connect_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<stripe_connect::StripeConnectReversalRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_connect_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutCancelType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutCancelType::get_headers(self, req, connectors)?)
            .body(types::PayoutCancelType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoCancel>,
        res: Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoCancel>, errors::ConnectorError> {
        let response: stripe_connect::StripeConnectReversalResponse = res
            .response
            .parse_struct("StripeConnectReversalResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoCreate, types::PayoutsData, types::PayoutsResponseData>
    for StripeConnect
{
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v1/transfers",
            connectors.stripe_connect.base_url
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = stripe_connect::StripeConnectPayoutCreateRequest::try_from(req)?;
        let stripe_connect_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<stripe_connect::StripeConnectPayoutCreateRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_connect_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutCreateType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutCreateType::get_headers(self, req, connectors)?)
            .body(types::PayoutCreateType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoCreate>,
        res: Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoCreate>, errors::ConnectorError> {
        let response: stripe_connect::StripeConnectPayoutCreateResponse = res
            .response
            .parse_struct("StripeConnectPayoutCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for StripeConnect
{
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v1/payouts", connectors.stripe_connect.base_url,))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = self.build_headers(req, connectors)?;
        let customer_account = req
            .connector_customer
            .to_owned()
            .get_required_value("connector_customer")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_customer",
            })?;
        let mut customer_account_header = vec![(
            headers::STRIPE_COMPATIBLE_CONNECT_ACCOUNT.to_string(),
            customer_account.into_masked(),
        )];
        headers.append(&mut customer_account_header);
        Ok(headers)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = stripe_connect::StripeConnectPayoutFulfillRequest::try_from(req)?;
        let stripe_connect_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<stripe_connect::StripeConnectPayoutFulfillRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_connect_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutFulfillType::get_headers(
                self, req, connectors,
            )?)
            .body(types::PayoutFulfillType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoFulfill>,
        res: Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoFulfill>, errors::ConnectorError> {
        let response: stripe_connect::StripeConnectPayoutFulfillResponse = res
            .response
            .parse_struct("StripeConnectPayoutFulfillResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

#[async_trait::async_trait]
#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoRecipient, types::PayoutsData, types::PayoutsResponseData>
    for StripeConnect
{
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    async fn execute_posttasks(
        &self,
        router_data: &mut types::PayoutsRouterData<api::PoRecipient>,
        app_state: &routes::AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        // Create recipient's external account
        let recipient_router_data =
            &types::PayoutsRouterData::from((&router_data, router_data.request.clone()));
        let recipient_connector_integration: Box<
            &(dyn ConnectorIntegration<
                api::PoRecipientAccount,
                types::PayoutsData,
                types::PayoutsResponseData,
            > + Send
                  + Sync
                  + 'static),
        > = Box::new(self);
        services::execute_connector_processing_step(
            app_state,
            recipient_connector_integration,
            recipient_router_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await?;

        Ok(())
    }

    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoRecipient>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v1/accounts", connectors.stripe_connect.base_url))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipient>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipient>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = stripe_connect::StripeConnectRecipientCreateRequest::try_from(req)?;
        let wise_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<stripe_connect::StripeConnectRecipientCreateRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipient>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutRecipientType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutRecipientType::get_headers(
                self, req, connectors,
            )?)
            .body(types::PayoutRecipientType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoRecipient>,
        res: Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoRecipient>, errors::ConnectorError> {
        let response: stripe_connect::StripeConnectRecipientCreateResponse = res
            .response
            .parse_struct("StripeConnectRecipientCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoRecipientAccount, types::PayoutsData, types::PayoutsResponseData>
    for StripeConnect
{
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipientAccount>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_customer_id = req
            .connector_customer
            .clone()
            .get_required_value("connector_customer")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_customer",
            })?;
        Ok(format!(
            "{}v1/accounts/{}/external_accounts",
            connectors.stripe_connect.base_url, connector_customer_id
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipientAccount>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipientAccount>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req =
            stripe_connect::StripeConnectRecipientAccountCreateRequest::try_from(req)?;
        let wise_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<stripe_connect::StripeConnectRecipientAccountCreateRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipientAccount>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutRecipientAccountType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::PayoutRecipientAccountType::get_headers(
                self, req, connectors,
            )?)
            .body(types::PayoutRecipientAccountType::get_request_body(
                self, req,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoRecipientAccount>,
        res: Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoRecipientAccount>, errors::ConnectorError>
    {
        let response: stripe_connect::StripeConnectRecipientAccountCreateResponse = res
            .response
            .parse_struct("StripeConnectRecipientAccountCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
