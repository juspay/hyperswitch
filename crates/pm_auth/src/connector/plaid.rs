pub mod transformers;

use std::fmt::Debug;

use common_utils::{
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use masking::{Mask, Maskable};
use transformers as plaid;

use crate::{
    core::errors,
    types::{
        self as auth_types,
        api::{
            auth_service::{
                self, BankAccountCredentials, ExchangeToken, LinkToken, RecipientCreate,
            },
            ConnectorCommon, ConnectorCommonExt, ConnectorIntegration,
        },
    },
};

#[derive(Debug, Clone)]
pub struct Plaid;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Plaid
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &auth_types::PaymentAuthRouterData<Flow, Request, Response>,
        _connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            "Content-Type".to_string(),
            self.get_content_type().to_string().into(),
        )];

        let mut auth = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut auth);
        Ok(header)
    }
}

impl ConnectorCommon for Plaid {
    fn id(&self) -> &'static str {
        "plaid"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
    fn base_url<'a>(&self, _connectors: &'a auth_types::PaymentMethodAuthConnectors) -> &'a str {
        "https://sandbox.plaid.com"
    }

    fn get_auth_header(
        &self,
        auth_type: &auth_types::ConnectorAuthType,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = plaid::PlaidAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let client_id = auth.client_id.into_masked();
        let secret = auth.secret.into_masked();

        Ok(vec![
            ("PLAID-CLIENT-ID".to_string(), client_id),
            ("PLAID-SECRET".to_string(), secret),
        ])
    }

    fn build_error_response(
        &self,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ErrorResponse, errors::ConnectorError> {
        let response: plaid::PlaidErrorResponse =
            res.response
                .parse_struct("PlaidErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(auth_types::ErrorResponse {
            status_code: res.status_code,
            code: crate::consts::NO_ERROR_CODE.to_string(),
            message: response.error_message,
            reason: response.display_message,
        })
    }
}

impl auth_service::AuthService for Plaid {}
impl auth_service::PaymentInitiationRecipientCreate for Plaid {}
impl auth_service::PaymentInitiation for Plaid {}
impl auth_service::AuthServiceLinkToken for Plaid {}

impl ConnectorIntegration<LinkToken, auth_types::LinkTokenRequest, auth_types::LinkTokenResponse>
    for Plaid
{
    fn get_headers(
        &self,
        req: &auth_types::LinkTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &auth_types::LinkTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/link/token/create"
        ))
    }

    fn get_request_body(
        &self,
        req: &auth_types::LinkTokenRouterData,
    ) -> errors::CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = plaid::PlaidLinkTokenRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &auth_types::LinkTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&auth_types::PaymentAuthLinkTokenType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(auth_types::PaymentAuthLinkTokenType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(auth_types::PaymentAuthLinkTokenType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &auth_types::LinkTokenRouterData,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::LinkTokenRouterData, errors::ConnectorError> {
        let response: plaid::PlaidLinkTokenResponse = res
            .response
            .parse_struct("PlaidLinkTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        <auth_types::LinkTokenRouterData>::try_from(auth_types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl auth_service::AuthServiceExchangeToken for Plaid {}

impl
    ConnectorIntegration<
        ExchangeToken,
        auth_types::ExchangeTokenRequest,
        auth_types::ExchangeTokenResponse,
    > for Plaid
{
    fn get_headers(
        &self,
        req: &auth_types::ExchangeTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &auth_types::ExchangeTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/item/public_token/exchange"
        ))
    }

    fn get_request_body(
        &self,
        req: &auth_types::ExchangeTokenRouterData,
    ) -> errors::CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = plaid::PlaidExchangeTokenRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &auth_types::ExchangeTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&auth_types::PaymentAuthExchangeTokenType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(auth_types::PaymentAuthExchangeTokenType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(auth_types::PaymentAuthExchangeTokenType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &auth_types::ExchangeTokenRouterData,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ExchangeTokenRouterData, errors::ConnectorError> {
        let response: plaid::PlaidExchangeTokenResponse = res
            .response
            .parse_struct("PlaidExchangeTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        <auth_types::ExchangeTokenRouterData>::try_from(auth_types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl auth_service::AuthServiceBankAccountCredentials for Plaid {}

impl
    ConnectorIntegration<
        BankAccountCredentials,
        auth_types::BankAccountCredentialsRequest,
        auth_types::BankAccountCredentialsResponse,
    > for Plaid
{
    fn get_headers(
        &self,
        req: &auth_types::BankDetailsRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &auth_types::BankDetailsRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "/auth/get"))
    }

    fn get_request_body(
        &self,
        req: &auth_types::BankDetailsRouterData,
    ) -> errors::CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = plaid::PlaidBankAccountCredentialsRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &auth_types::BankDetailsRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&auth_types::PaymentAuthBankAccountDetailsType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(auth_types::PaymentAuthBankAccountDetailsType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(
                    auth_types::PaymentAuthBankAccountDetailsType::get_request_body(self, req)?,
                )
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &auth_types::BankDetailsRouterData,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::BankDetailsRouterData, errors::ConnectorError> {
        let response: plaid::PlaidBankAccountCredentialsResponse = res
            .response
            .parse_struct("PlaidBankAccountCredentialsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        <auth_types::BankDetailsRouterData>::try_from(auth_types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<
        RecipientCreate,
        auth_types::RecipientCreateRequest,
        auth_types::RecipientCreateResponse,
    > for Plaid
{
    fn get_headers(
        &self,
        req: &auth_types::RecipientCreateRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &auth_types::RecipientCreateRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/payment_initiation/recipient/create"
        ))
    }

    fn get_request_body(
        &self,
        req: &auth_types::RecipientCreateRouterData,
    ) -> errors::CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = plaid::PlaidRecipientCreateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &auth_types::RecipientCreateRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&auth_types::PaymentInitiationRecipientCreateType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(
                    auth_types::PaymentInitiationRecipientCreateType::get_headers(
                        self, req, connectors,
                    )?,
                )
                .set_body(
                    auth_types::PaymentInitiationRecipientCreateType::get_request_body(self, req)?,
                )
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &auth_types::RecipientCreateRouterData,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::RecipientCreateRouterData, errors::ConnectorError> {
        let response: plaid::PlaidRecipientCreateResponse = res
            .response
            .parse_struct("PlaidRecipientCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        <auth_types::RecipientCreateRouterData>::try_from(auth_types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}
