pub mod transformers;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts,
    crypto::Encryptable,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
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
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundsRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::{PoCreate, PoFulfill, PoQuote, PoSync},
    types::{PayoutsData, PayoutsResponseData, PayoutsRouterData},
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types::{
    PayoutCreateType, PayoutFulfillType, PayoutQuoteType, PayoutSyncType,
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts as api_consts, errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use lazy_static::lazy_static;
#[cfg(feature = "payouts")]
use masking::ExposeInterface;
use masking::{Mask, PeekInterface};
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};
use transformers as gigadat;
use url::form_urlencoded;
use uuid::Uuid;

#[cfg(feature = "payouts")]
use crate::utils::{to_payout_connector_meta, RouterData as RouterDataTrait};
use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Gigadat {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Gigadat {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

impl api::Payment for Gigadat {}
impl api::PaymentSession for Gigadat {}
impl api::ConnectorAccessToken for Gigadat {}
impl api::MandateSetup for Gigadat {}
impl api::PaymentAuthorize for Gigadat {}
impl api::PaymentSync for Gigadat {}
impl api::PaymentCapture for Gigadat {}
impl api::PaymentVoid for Gigadat {}
impl api::Refund for Gigadat {}
impl api::RefundExecute for Gigadat {}
impl api::RefundSync for Gigadat {}
impl api::PaymentToken for Gigadat {}
impl api::Payouts for Gigadat {}
#[cfg(feature = "payouts")]
impl api::PayoutQuote for Gigadat {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for Gigadat {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Gigadat {}
#[cfg(feature = "payouts")]
impl api::PayoutSync for Gigadat {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Gigadat
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Gigadat
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Gigadat {
    fn id(&self) -> &'static str {
        "gigadat"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.gigadat.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = gigadat::GigadatAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_key = format!(
            "{}:{}",
            auth.access_token.peek(),
            auth.security_token.peek()
        );
        let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth_header.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: gigadat::GigadatErrorResponse = res
            .response
            .parse_struct("GigadatErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.err.clone(),
            message: response.err.clone(),
            reason: Some(response.err).clone(),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Gigadat {
    fn validate_mandate_payment(
        &self,
        _pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        match pm_data {
            PaymentMethodData::Card(_) => Err(errors::ConnectorError::NotImplemented(
                "validate_mandate_payment does not support cards".to_string(),
            )
            .into()),
            _ => Ok(()),
        }
    }

    fn validate_psync_reference_id(
        &self,
        _data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Gigadat {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Gigadat {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Gigadat {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Gigadat {
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
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth = gigadat::GigadatAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}api/payment-token/{}",
            self.base_url(connectors),
            auth.campaign_id.peek()
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = gigadat::GigadatRouterData::from((amount, req));
        let connector_req = gigadat::GigadatCpiRequest::try_from(&connector_router_data)?;
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
        let response: gigadat::GigadatPaymentResponse = res
            .response
            .parse_struct("GigadatPaymentResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Gigadat {
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
        let transaction_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}api/transactions/{transaction_id}",
            self.base_url(connectors)
        ))
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
        let response: gigadat::GigadatTransactionStatusResponse = res
            .response
            .parse_struct("gigadat PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Gigadat {
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
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
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
        let response: gigadat::GigadatTransactionStatusResponse = res
            .response
            .parse_struct("Gigadat PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Gigadat {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Gigadat {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = gigadat::GigadatAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_key = format!(
            "{}:{}",
            auth.access_token.peek(),
            auth.security_token.peek()
        );
        let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
        Ok(vec![
            (
                headers::AUTHORIZATION.to_string(),
                auth_header.into_masked(),
            ),
            (
                headers::IDEMPOTENCY_KEY.to_string(),
                Uuid::new_v4().to_string().into_masked(),
            ),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}refunds", self.base_url(connectors),))
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

        let connector_router_data = gigadat::GigadatRouterData::from((refund_amount, req));
        let connector_req = gigadat::GigadatRefundRequest::try_from(&connector_router_data)?;
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
        let response: gigadat::RefundResponse = res
            .response
            .parse_struct("gigadat RefundResponse")
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
        let response: gigadat::GigadatRefundErrorResponse = res
            .response
            .parse_struct("GigadatRefundErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let code = response
            .error
            .first()
            .and_then(|error_detail| error_detail.code.clone())
            .unwrap_or(api_consts::NO_ERROR_CODE.to_string());
        let message = response
            .error
            .first()
            .map(|error_detail| error_detail.detail.clone())
            .unwrap_or(api_consts::NO_ERROR_MESSAGE.to_string());
        Ok(ErrorResponse {
            status_code: res.status_code,
            code,
            message,
            reason: Some(response.message).clone(),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Gigadat {
    //Gigadat does not support Refund Sync
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoQuote, PayoutsData, PayoutsResponseData> for Gigadat {
    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoQuote>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PayoutsRouterData<PoQuote>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth = gigadat::GigadatAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}api/payment-token/{}",
            self.base_url(connectors),
            auth.campaign_id.peek()
        ))
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoQuote>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.destination_currency,
        )?;

        let connector_router_data = gigadat::GigadatRouterData::from((amount, req));
        let connector_req = gigadat::GigadatPayoutQuoteRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoQuote>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
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
    ) -> CustomResult<PayoutsRouterData<PoQuote>, errors::ConnectorError> {
        let response: gigadat::GigadatPayoutQuoteResponse = res
            .response
            .parse_struct("GigadatPayoutQuoteResponse")
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
#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData> for Gigadat {
    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let transfer_id = req.get_quote_id()?;

        let metadata = Some(req.get_connector_meta()?.clone().expose());

        let gigatad_meta: gigadat::GigadatPayoutMeta = to_payout_connector_meta(metadata.clone())?;

        Ok(format!(
            "{}webflow?transaction={}&token={}",
            self.base_url(connectors),
            transfer_id,
            gigatad_meta.token.peek(),
        ))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutCreateType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutCreateType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoCreate>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoCreate>, errors::ConnectorError> {
        let response: gigadat::GigadatPayoutResponse = res
            .response
            .parse_struct("GigadatPayoutResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Gigadat {
    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let transfer_id = req.request.connector_payout_id.to_owned().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "transaction_id",
            },
        )?;
        let metadata = req
            .request
            .payout_connector_metadata
            .clone()
            .map(|secret| secret.peek().clone());

        let gigatad_meta: gigadat::GigadatPayoutMeta = to_payout_connector_meta(metadata.clone())?;

        Ok(format!(
            "{}webflow/deposit?transaction={}&token={}",
            self.base_url(connectors),
            transfer_id,
            gigatad_meta.token.peek(),
        ))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutFulfillType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, errors::ConnectorError> {
        let response: gigadat::GigadatPayoutResponse = res
            .response
            .parse_struct("GigadatPayoutResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoSync, PayoutsData, PayoutsResponseData> for Gigadat {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let transfer_id = req.request.connector_payout_id.to_owned().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "transaction_id",
            },
        )?;
        Ok(format!(
            "{}api/transactions/{}",
            connectors.gigadat.base_url, transfer_id
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&PayoutSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutSyncType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoSync>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoSync>, errors::ConnectorError> {
        let response: gigadat::GigadatPayoutSyncResponse = res
            .response
            .parse_struct("GigadatPayoutSyncResponse")
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

fn get_webhook_query_params(
    request: &webhooks::IncomingWebhookRequestDetails<'_>,
) -> CustomResult<transformers::GigadatWebhookQueryParameters, errors::ConnectorError> {
    let query_string = &request.query_params;

    let (transaction, status) = query_string
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .fold((None, None), |(mut txn, mut sts), (key, value)| {
            match key {
                "transaction" => txn = Some(value.to_string()),
                "status" => {
                    if let Ok(status) =
                        transformers::GigadatTransactionStatus::try_from(value.to_string())
                    {
                        sts = Some(status);
                    }
                }
                _ => {}
            }
            (txn, sts)
        });

    Ok(transformers::GigadatWebhookQueryParameters {
        transaction: transaction.ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?,
        status: status.ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?,
    })
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Gigadat {
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let query_params = get_webhook_query_params(request)?;
        let body_str = std::str::from_utf8(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let details: Vec<transformers::GigadatWebhookKeyValue> =
            form_urlencoded::parse(body_str.as_bytes())
                .map(|(key, value)| transformers::GigadatWebhookKeyValue {
                    key: key.to_string(),
                    value: value.to_string(),
                })
                .collect();

        let webhook_type = details
            .iter()
            .find(|&entry| entry.key == "type")
            .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let reference_id = match transformers::GigadatFlow::get_flow(webhook_type.value.as_str())? {
            transformers::GigadatFlow::Payment => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        query_params.transaction,
                    ),
                )
            }
            #[cfg(feature = "payouts")]
            transformers::GigadatFlow::Payout => api_models::webhooks::ObjectReferenceId::PayoutId(
                api_models::webhooks::PayoutIdType::ConnectorPayoutId(query_params.transaction),
            ),
        };
        Ok(reference_id)
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let query_params = get_webhook_query_params(request)?;
        let body_str = std::str::from_utf8(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let details: Vec<transformers::GigadatWebhookKeyValue> =
            form_urlencoded::parse(body_str.as_bytes())
                .map(|(key, value)| transformers::GigadatWebhookKeyValue {
                    key: key.to_string(),
                    value: value.to_string(),
                })
                .collect();

        let webhook_type = details
            .iter()
            .find(|&entry| entry.key == "type")
            .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let flow_type = transformers::GigadatFlow::get_flow(webhook_type.value.as_str())?;
        let event_type =
            transformers::get_gigadat_webhook_event_type(query_params.status, flow_type);
        Ok(event_type)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let body_str = std::str::from_utf8(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let details: Vec<transformers::GigadatWebhookKeyValue> =
            form_urlencoded::parse(body_str.as_bytes())
                .map(|(key, value)| transformers::GigadatWebhookKeyValue {
                    key: key.to_string(),
                    value: value.to_string(),
                })
                .collect();
        let resource_object = serde_json::to_string(&details)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(Box::new(resource_object))
    }
    async fn verify_webhook_source(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: Encryptable<masking::Secret<serde_json::Value>>,
        _connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        Ok(false)
    }
}

lazy_static! {
    static ref GIGADAT_SUPPORTED_PAYMENT_METHODS: SupportedPaymentMethods = {
        let supported_capture_methods = vec![enums::CaptureMethod::Automatic];

        let mut gigadat_supported_payment_methods = SupportedPaymentMethods::new();
        gigadat_supported_payment_methods.add(
            enums::PaymentMethod::BankRedirect,
            enums::PaymentMethodType::Interac,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods,
                specific_features: None,
            },
        );

        gigadat_supported_payment_methods
    };
    static ref GIGADAT_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name: "Gigadat",
        description: "Gigadat is a financial services product that offers a single API for payment integration. It provides Canadian businesses with a secure payment gateway and various pay-in and pay-out solutions, including Interac e-Transfer",
        connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
        integration_status: enums::ConnectorIntegrationStatus::Live,
    };
    static ref GIGADAT_SUPPORTED_WEBHOOK_FLOWS: Vec<enums::EventClass> = {
        #[cfg(feature = "payouts")]
        {
            let mut flows = vec![enums::EventClass::Payments];
            flows.push(enums::EventClass::Payouts);
            flows
        }
        #[cfg(not(feature = "payouts"))]
        {
            vec![enums::EventClass::Payments]
        }
    };
}

impl ConnectorSpecifications for Gigadat {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&*GIGADAT_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*GIGADAT_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&*GIGADAT_SUPPORTED_WEBHOOK_FLOWS)
    }
}
