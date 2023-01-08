#![allow(dead_code)]
mod transformers;

use std::fmt::Debug;

use bytes::Bytes;
use common_utils::ext_traits::{Encode, ValueExt};
use error_stack::{IntoReport, ResultExt};
use transformers as authorizedotnet;

use crate::{
    configs::settings,
    consts,
    core::{
        errors::{self, ConnectorErrorExt, CustomResult},
        payments,
    },
    headers, routes,
    services::{self, logger},
    types::{
        self,
        api::{self, ConnectorCommon},
    },
    utils::{self, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct Authorizedotnet;

impl ConnectorCommon for Authorizedotnet {
    fn id(&self) -> &'static str {
        "authorizedotnet"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.authorizedotnet.base_url.as_ref()
    }
}

impl api::Payment for Authorizedotnet {}
impl api::PaymentAuthorize for Authorizedotnet {}
impl api::PaymentSync for Authorizedotnet {}
impl api::PaymentVoid for Authorizedotnet {}
impl api::PaymentCapture for Authorizedotnet {}
impl api::PaymentSession for Authorizedotnet {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    // Not Implemented (R)
}

impl api::PreVerify for Authorizedotnet {}

impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    // Issue: #173
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsSyncType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let sync_request =
            utils::Encode::<authorizedotnet::AuthorizedotnetCreateSyncRequest>::convert_and_encode(
                req,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(sync_request))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
            .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
            .body(types::PaymentsSyncType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetSyncResponse = intermediate_response
            .parse_struct("AuthorizedotnetSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        logger::debug!(request=?req);
        let authorizedotnet_req =
            utils::Encode::<authorizedotnet::CreateTransactionRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
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
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .header(headers::X_ROUTER, "test")
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        use bytes::Buf;
        logger::debug!(authorizedotnetpayments_create_response=?res);

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = intermediate_response
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        logger::debug!(authorizedotnetpayments_create_error_response=?res);
        get_error_response(res)
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let authorizedotnet_req =
            utils::Encode::<authorizedotnet::CancelTransactionRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
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
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .header(headers::X_ROUTER, "test")
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = intermediate_response
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(authorizedotnetpayments_create_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

impl api::Refund for Authorizedotnet {}
impl api::RefundExecute for Authorizedotnet {}
impl api::RefundSync for Authorizedotnet {}

#[async_trait::async_trait]
impl api::RefundCommon for Authorizedotnet {
    async fn refund_execute_update_tracker(
        &self,
        state: &routes::AppState,
        connector: &api::ConnectorData,
        router_data: types::RefundsRouterData<api::Execute>,
        payment_attempt: &storage_models::payment_attempt::PaymentAttempt,
    ) -> errors::RouterResult<types::RefundsRouterData<api::Execute>> {
        let request = types::PaymentsSyncData {
            connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                payment_attempt
                    .connector_transaction_id
                    .as_ref()
                    .get_required_value("connector_transaction_id")?
                    .clone(),
            ),
            encoded_data: None,
        };

        let response = Err(types::ErrorResponse::default());
        let (router_data_inner, request, response) =
            services::router_data_conversion::<_, api::PSync, _, _, _, _>(
                router_data,
                request,
                response,
            );
        let connector_integration = connector.connector.get_connector_integration();

        let router_data_inner = services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data_inner,
            payments::CallConnectorAction::Trigger,
        )
        .await
        .map_err(|err| err.to_payment_failed_response())?;
        let (mut router_data, _request, response) =
            services::router_data_conversion(router_data_inner, request, response);
        let connector_payment_data = match response.ok().and_then(|inner_resp| match inner_resp {
            types::PaymentsResponseData::TransactionResponse {
                connector_specific_metadata,
                ..
            } => connector_specific_metadata,
            _ => None,
        }) {
            None => None,
            Some(val) => {
                let payment_details: transformers::PaymentDetails = val
                    .parse_value("payment_details")
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;
                Some(
                    Encode::<'_, transformers::PaymentDetails>::encode_to_value(&payment_details)
                        .change_context(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "payment_details",
                    })?,
                )
            }
        };

        // let connector_payment_data = Some(serde_json::to_value(match payments::helpers::Vault::get_payment_method_data_from_locker(state, payment_attempt.payment_method_id.as_ref().get_required_value("payment_method_id")?).await?.0.get_required_value("payment_method")? {
        //     api::PaymentMethod::Card(ref ccard) => {
        //         let expiry_month = ccard.card_exp_month.peek().clone();
        //         let expiry_year = ccard.card_exp_year.peek().clone();

        //         PaymentDetails::CreditCard(transformers::CreditCardDetails {
        //             card_number: ccard.card_number.peek().clone(),
        //             expiration_date: Some(format!("{expiry_year}-{expiry_month}")),
        //             card_code: Some(ccard.card_cvc.peek().clone()),
        //         })
        //     }
        //     api::PaymentMethod::BankTransfer => PaymentDetails::BankAccount(transformers::BankAccountDetails {
        //         account_number: "XXXXX".to_string(),
        //     }),
        //     api::PaymentMethod::PayLater(_) => PaymentDetails::Klarna,
        //     api::PaymentMethod::Wallet(_) => PaymentDetails::Wallet,
        //     api::PaymentMethod::Paypal => PaymentDetails::Paypal,
        // }).into_report().change_context(errors::ParsingError).change_context(errors::ApiErrorResponse::InternalServerError)?);

        router_data.request.connector_specific_data = connector_payment_data;
        Ok(router_data)
    }
}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        logger::debug!(refund_request=?req);
        let authorizedotnet_req =
            utils::Encode::<authorizedotnet::CreateRefundRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
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
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        use bytes::Buf;
        logger::debug!(response=?res);

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetRefundResponse = intermediate_response
            .parse_struct("AuthorizedotnetRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);

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
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::RefundsRouterData<api::RSync>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefundSyncType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::RSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let sync_request =
            utils::Encode::<authorizedotnet::AuthorizedotnetCreateSyncRequest>::convert_and_encode(
                req,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(sync_request))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundSyncType::get_url(self, req, connectors)?)
            .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
            .body(types::RefundSyncType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::RSync>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::RSync>, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetSyncResponse = intermediate_response
            .parse_struct("AuthorizedotnetSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Authorizedotnet {
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

impl services::ConnectorRedirectResponse for Authorizedotnet {}

#[inline]
fn get_error_response(bytes: Bytes) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
    let response: authorizedotnet::AuthorizedotnetPaymentsResponse = bytes
        .parse_struct("AuthorizedotnetPaymentsResponse")
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

    logger::info!(response=?response);

    Ok(response
        .transaction_response
        .errors
        .and_then(|errors| {
            errors.into_iter().next().map(|error| types::ErrorResponse {
                code: error.error_code,
                message: error.error_text,
                reason: None,
            })
        })
        .unwrap_or_else(|| types::ErrorResponse {
            code: consts::NO_ERROR_CODE.to_string(),
            message: consts::NO_ERROR_MESSAGE.to_string(),
            reason: None,
        }))
}
