pub mod transformers;

use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
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
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorAccessTokenSuffix, ConnectorCommon, ConnectorCommonExt,
        ConnectorIntegration, ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    consts::NO_ERROR_MESSAGE,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, RefreshTokenType, Response},
    webhooks,
};
use masking::{Mask, Maskable, PeekInterface, Secret};
use transformers as santander;

use crate::{
    constants::headers,
    types::{RefreshTokenRouterData, ResponseRouterData},
    utils::{self as connector_utils, convert_amount, RefundsRequestData},
};

#[derive(Clone)]
pub struct Santander {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Santander {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

pub mod santander_constants {
    pub const SANTANDER_VERSION: &str = "v2";
}

impl api::Payment for Santander {}
impl api::PaymentSession for Santander {}
impl api::ConnectorAccessToken for Santander {}
impl api::MandateSetup for Santander {}
impl api::PaymentAuthorize for Santander {}
impl api::PaymentSync for Santander {}
impl api::PaymentCapture for Santander {}
impl api::PaymentVoid for Santander {}
impl api::Refund for Santander {}
impl api::RefundExecute for Santander {}
impl api::RefundSync for Santander {}
impl api::PaymentToken for Santander {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Santander
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Santander
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Santander {
    fn id(&self) -> &'static str {
        "santander"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.santander.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: santander::SantanderErrorResponse = res
            .response
            .parse_struct("SantanderErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        match response {
            santander::SantanderErrorResponse::PixQrCode(response) => {
                let message = response
                    .detail
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string());

                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: response.status.to_string(),
                    message,
                    reason: response.detail.clone(),
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            santander::SantanderErrorResponse::Boleto(response) => Ok(ErrorResponse {
                status_code: res.status_code,
                code: response.error_code.to_string(),
                message: response.error_message.clone(),
                reason: Some(
                    response
                        .errors
                        .as_ref()
                        .and_then(|v| v.first())
                        .map(|e| e.message.clone())
                        .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                ),
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

impl ConnectorValidation for Santander {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Santander {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Santander {
    fn get_headers(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let client_id = req.request.app_id.clone();

        let client_secret = req.request.id.clone();

        let creds = format!(
            "{}:{}",
            client_id.peek(),
            client_secret.unwrap_or_default().peek()
        );
        let encoded_creds = common_utils::consts::BASE64_ENGINE.encode(creds);

        let auth_string = format!("Basic {encoded_creds}");
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                RefreshTokenType::get_content_type(self).to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                auth_string.into_masked(),
            ),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        _req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/oauth/token?grant_type=client_credentials",
            connectors.santander.base_url
        ))
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
        let response: santander::SantanderAuthUpdateResponse = res
            .response
            .parse_struct("santander SantanderAuthUpdateResponse")
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
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Santander
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Santander".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let santander_mca_metadata =
            santander::SantanderMetadataObject::try_from(&req.connector_meta_data)?;

        match req.payment_method {
            enums::PaymentMethod::BankTransfer => match req.request.payment_method_type {
                Some(enums::PaymentMethodType::Pix) => Ok(format!(
                    "{}cob/{}",
                    self.base_url(connectors),
                    req.payment_id
                )),
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            },
            enums::PaymentMethod::Voucher => match req.request.payment_method_type {
                Some(enums::PaymentMethodType::Boleto) => Ok(format!(
                    "{:?}{}/workspaces/{}/bank_slips",
                    connectors.santander.secondary_base_url.clone(),
                    santander_constants::SANTANDER_VERSION,
                    santander_mca_metadata.workspace_id
                )),
                _ => Err(errors::ConnectorError::NotSupported {
                    message: req.payment_method.to_string(),
                    connector: "Santander",
                }
                .into()),
            },
            _ => Err(errors::ConnectorError::NotSupported {
                message: req.payment_method.to_string(),
                connector: "Santander",
            }
            .into()),
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = santander::SantanderRouterData::from((amount, req));
        let connector_req = santander::SantanderPaymentRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Put)
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
        let response: santander::SantanderPaymentsResponse = res
            .response
            .parse_struct("Santander PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let original_amount = match response {
            santander::SantanderPaymentsResponse::PixQRCode(ref pix_data) => {
                pix_data.value.original.clone()
            }
            santander::SantanderPaymentsResponse::Boleto(_) => {
                convert_amount(
                    self.amount_converter,
                    MinorUnit::new(data.request.amount),
                    data.request.currency,
                )?
                // no amount field in the boleto response
            }
        };

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let response_integrity_object = connector_utils::get_authorise_integrity_object(
            self.amount_converter,
            original_amount,
            enums::Currency::BRL.to_string(),
        )?;

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
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
        match req.request.payment_method_type {
            Some(enums::PaymentMethodType::Pix) => {
                let connector_payment_id = req
                    .request
                    .connector_transaction_id
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

                Ok(format!(
                    "{}{}{}",
                    self.base_url(connectors),
                    "cob/",
                    connector_payment_id
                ))
            }
            Some(enums::PaymentMethodType::Boleto) => {
                let bill_id = req
                    .request
                    .connector_meta
                    .clone()
                    .and_then(|val| val.as_str().map(|s| s.to_string()))
                    .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                        field_name: "bill_id",
                    })?;

                Ok(format!(
                    "{:?}/{}/bills/{}/bank_slips",
                    connectors.santander.secondary_base_url.clone(),
                    santander_constants::SANTANDER_VERSION,
                    bill_id
                ))
            }
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_type",
            }
            .into()),
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.amount,
            req.request.currency,
        )?;

        let connector_router_data = santander::SantanderRouterData::from((amount, req));
        let connector_req =
            santander::SantanderPSyncBoletoRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        match req.request.payment_method_type {
            Some(enums::PaymentMethodType::Pix) => Ok(Some(
                RequestBuilder::new()
                    .method(Method::Get)
                    .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                    .build(),
            )),
            Some(enums::PaymentMethodType::Boleto) => Ok(Some(
                RequestBuilder::new()
                    .method(Method::Post)
                    .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                    .set_body(types::PaymentsSyncType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            )),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_type",
            }
            .into()),
        }
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: santander::SantanderPaymentsSyncResponse = res
            .response
            .parse_struct("santander SantanderPaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let original_amount = match response {
            santander::SantanderPaymentsSyncResponse::PixQRCode(ref pix_data) => {
                pix_data.base.value.original.clone()
            }
            santander::SantanderPaymentsSyncResponse::Boleto(_) => convert_amount(
                self.amount_converter,
                data.request.amount,
                data.request.currency,
            )?,
        };

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let response_integrity_object = connector_utils::get_sync_integrity_object(
            self.amount_converter,
            original_amount,
            enums::Currency::BRL.to_string(),
        )?;

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "Santander".to_string(),
        }
        .into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "Santander".to_string(),
        }
        .into())
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
        let response: santander::SantanderPaymentsResponse = res
            .response
            .parse_struct("Santander PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
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
        match req.payment_method {
            enums::PaymentMethod::BankTransfer => {
                let connector_payment_id = req.request.connector_transaction_id.clone();
                Ok(format!(
                    "{}cob/{}",
                    self.base_url(connectors),
                    connector_payment_id
                ))
            }
            _ => Err(errors::ConnectorError::NotSupported {
                message: req.payment_method.to_string(),
                connector: "Santander",
            }
            .into()),
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = santander::SantanderPaymentsCancelRequest::try_from(req)?;
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
        let response: santander::SantanderPixVoidResponse = res
            .response
            .parse_struct("Santander PaymentsResponse")
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        match req.payment_method {
            enums::PaymentMethod::BankTransfer => {
                let end_to_end_id = req
                    .request
                    .connector_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.get("end_to_end_id"))
                    .and_then(|val| val.as_str().map(|id| id.to_string()))
                    .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                        field_name: "end_to_end_id",
                    })?;

                let refund_id = req.request.connector_refund_id.clone();
                Ok(format!(
                    "{}{}{}{}{:?}",
                    self.base_url(connectors),
                    "pix/",
                    end_to_end_id,
                    "/refund/",
                    refund_id
                ))
            }
            _ => Err(errors::ConnectorError::NotSupported {
                message: req.payment_method.to_string(),
                connector: "Santander",
            }
            .into()),
        }
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data = santander::SantanderRouterData::from((refund_amount, req));
        let connector_req = santander::SantanderRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Put)
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
        let response: santander::SantanderRefundResponse = res
            .response
            .parse_struct("santander RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let original_amount = response.value.clone();

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let response_integrity_object = connector_utils::get_refund_integrity_object(
            self.amount_converter,
            original_amount,
            enums::Currency::BRL.to_string(),
        )?;

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Santander {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
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
        let connector_metadata = req.request.connector_metadata.clone();
        let end_to_end_id = match &connector_metadata {
            Some(metadata) => match metadata.get("end_to_end_id") {
                Some(val) => val.as_str().map(|id| id.to_string()),
                None => None,
            },
            None => None,
        }
        .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
            field_name: "end_to_end_id",
        })?;
        Ok(format!(
            "{}{}{}{}{}",
            self.base_url(connectors),
            "pix/",
            end_to_end_id,
            "/return/",
            req.request.get_connector_refund_id()?
        ))
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
        let response: santander::SantanderRefundResponse = res
            .response
            .parse_struct("santander RefundSyncResponse")
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

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<santander::SantanderWebhookBody, common_utils::errors::ParsingError> {
    let webhook: santander::SantanderWebhookBody = body.parse_struct("SantanderIncomingWebhook")?;

    Ok(webhook)
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Santander {
    async fn verify_webhook_source(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        _connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        Ok(true) // Hardcoded to true as the source verification algorithm for Santander remains to be unknown (in docs it is mentioned as MTLS)
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook_body = transformers::get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(
                webhook_body.participant_code,
            ),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let body = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(transformers::get_santander_webhook_event(body.function))
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook_body = transformers::get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(Box::new(webhook_body))
    }
}

static SANTANDER_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![enums::CaptureMethod::Automatic];

        let mut santander_supported_payment_methods = SupportedPaymentMethods::new();

        santander_supported_payment_methods.add(
            enums::PaymentMethod::BankTransfer,
            enums::PaymentMethodType::Pix,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        santander_supported_payment_methods.add(
            enums::PaymentMethod::Voucher,
            enums::PaymentMethodType::Boleto,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::NotSupported,
                supported_capture_methods,
                specific_features: None,
            },
        );

        santander_supported_payment_methods
    });

static SANTANDER_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Santander",
    description:
        "Santander is a leading private bank in Brazil, offering a wide range of financial services across retail and corporate segments. It is part of the global Santander Group, one of Europe’s largest financial institutions.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Alpha,
};

static SANTANDER_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 2] =
    [enums::EventClass::Payments, enums::EventClass::Refunds];

impl ConnectorSpecifications for Santander {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&SANTANDER_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*SANTANDER_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&SANTANDER_SUPPORTED_WEBHOOK_FLOWS)
    }
}

impl ConnectorAccessTokenSuffix for Santander {
    fn get_access_token_key<F, Req, Res>(
        &self,
        router_data: &RouterData<F, Req, Res>,
        merchant_connector_id_or_connector_name: String,
    ) -> CustomResult<String, errors::ConnectorError> {
        let key_suffix = router_data
            .payment_method_type
            .as_ref()
            .map(|pmt| pmt.to_string());

        match key_suffix {
            Some(key) => Ok(format!(
                "access_token_{}_{}_{}",
                router_data.merchant_id.get_string_repr(),
                merchant_connector_id_or_connector_name,
                key
            )),
            None => Ok(common_utils::access_token::get_default_access_token_key(
                &router_data.merchant_id,
                merchant_connector_id_or_connector_name,
            )),
        }
    }
}
