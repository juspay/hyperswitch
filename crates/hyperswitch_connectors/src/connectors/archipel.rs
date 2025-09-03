use std::sync::LazyLock;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, ValueExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        IncrementalAuthorization,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsIncrementalAuthorizationData,
        PaymentsSessionData, PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsIncrementalAuthorizationRouterData, PaymentsSyncRouterData, RefundSyncRouterData,
        RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts::NO_ERROR_MESSAGE,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::Maskable;
use router_env::{error, info};
use transformers::{
    self as archipel, ArchipelCardAuthorizationRequest, ArchipelIncrementalAuthorizationRequest,
    ArchipelPaymentsCancelRequest, ArchipelRefundRequest, ArchipelWalletAuthorizationRequest,
};

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{is_mandate_supported, PaymentMethodDataType, PaymentsAuthorizeRequestData},
};

pub mod transformers;

#[derive(Clone)]
pub struct Archipel {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Archipel {
    pub const fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::PaymentAuthorize for Archipel {}
impl api::PaymentSync for Archipel {}
impl api::PaymentVoid for Archipel {}
impl api::PaymentCapture for Archipel {}
impl api::MandateSetup for Archipel {}
impl api::ConnectorAccessToken for Archipel {}
impl api::PaymentToken for Archipel {}
impl api::PaymentSession for Archipel {}
impl api::Refund for Archipel {}
impl api::RefundExecute for Archipel {}
impl api::RefundSync for Archipel {}
impl api::Payment for Archipel {}
impl api::PaymentIncrementalAuthorization for Archipel {}

fn build_env_specific_endpoint(
    base_url: &str,
    connector_metadata: &Option<common_utils::pii::SecretSerdeValue>,
) -> CustomResult<String, errors::ConnectorError> {
    let archipel_connector_metadata_object =
        transformers::ArchipelConfigData::try_from(connector_metadata)?;
    let endpoint_prefix = archipel_connector_metadata_object.platform_url;
    Ok(base_url.replace("{{merchant_endpoint_prefix}}", &endpoint_prefix))
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Archipel
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

impl ConnectorCommon for Archipel {
    fn id(&self) -> &'static str {
        "archipel"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.archipel.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let archipel_error: CustomResult<
            archipel::ArchipelErrorMessage,
            common_utils::errors::ParsingError,
        > = res.response.parse_struct("ArchipelErrorMessage");

        match archipel_error {
            Ok(err) => {
                event_builder.map(|i| i.set_error_response_body(&err));
                info!(connector_response=?err);

                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: err.code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    message: err
                        .description
                        .clone()
                        .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                    reason: err.description,
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Err(error) => {
                event_builder.map(|event| {
                    event.set_error(serde_json::json!({
                        "error": res.response.escape_ascii().to_string(),
                        "status_code": res.status_code
                    }))
                });
                error!(deserialization_error=?error);
                crate::utils::handle_json_response_deserialization_failure(res, "archipel")
            }
        }
    }
}

impl ConnectorValidation for Archipel {
    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([PaymentMethodDataType::Card]);

        is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Archipel {
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
        let capture_method = req
            .request
            .capture_method
            .ok_or(errors::ConnectorError::CaptureMethodNotSupported)?;
        let base_url =
            build_env_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::SequentialAutomatic => {
                Ok(format!("{}{}", base_url, "/pay"))
            }
            enums::CaptureMethod::Manual => Ok(format!("{}{}", base_url, "/authorize")),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => {
                Err(report!(errors::ConnectorError::CaptureMethodNotSupported))
            }
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let config_data: archipel::ArchipelConfigData = (&req.connector_meta_data).try_into()?;
        let amount = crate::utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let router_data: archipel::ArchipelRouterData<_> =
            (amount, config_data.tenant_id, req).into();

        if req.request.is_wallet() {
            let request: ArchipelWalletAuthorizationRequest = router_data.try_into()?;
            Ok(RequestContent::Json(Box::new(request)))
        } else {
            let request: ArchipelCardAuthorizationRequest = router_data.try_into()?;
            Ok(RequestContent::Json(Box::new(request)))
        }
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = archipel::ArchipelAuthType::try_from(&req.connector_auth_type)?;
        let url = &self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;
        let body = self.get_request_body(req, connectors)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(url)
                .attach_default_headers()
                .headers(headers)
                .add_ca_certificate_pem(auth_details.ca_certificate)
                .set_body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: archipel::ArchipelPaymentsResponse = res
            .response
            .parse_struct("ArchipelPaymentsResponse for Authorize flow")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        info!(connector_response=?response);

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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<
        IncrementalAuthorization,
        PaymentsIncrementalAuthorizationData,
        PaymentsResponseData,
    > for Archipel
{
    fn get_headers(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url =
            build_env_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        let connector_payment_id = req.request.connector_transaction_id.clone();

        Ok(format!(
            "{}{}{}",
            base_url, "/incrementAuthorization/", connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let config_data: archipel::ArchipelConfigData = (&req.connector_meta_data).try_into()?;
        let router_data: archipel::ArchipelRouterData<_> = (
            MinorUnit::new(req.request.additional_amount),
            config_data.tenant_id,
            req,
        )
            .into();
        let request: ArchipelIncrementalAuthorizationRequest = router_data.into();

        Ok(RequestContent::Json(Box::new(request)))
    }

    fn build_request(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = archipel::ArchipelAuthType::try_from(&req.connector_auth_type)?;
        let url = &self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;
        let body = self.get_request_body(req, connectors)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(url)
                .attach_default_headers()
                .headers(headers)
                .add_ca_certificate_pem(auth_details.ca_certificate)
                .set_body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsIncrementalAuthorizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsIncrementalAuthorizationRouterData, errors::ConnectorError> {
        let response: archipel::ArchipelPaymentsResponse = res
            .response
            .parse_struct("ArchipelPaymentsResponse for IncrementalAuthorization flow")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        info!(connector_response=?response);

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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Archipel {
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
        let base_url =
            build_env_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        let metadata: archipel::ArchipelTransactionMetadata = req
            .request
            .connector_meta
            .clone()
            .and_then(|value| value.parse_value("ArchipelTransactionMetadata").ok())
            .ok_or_else(|| errors::ConnectorError::MissingConnectorTransactionID)?;

        Ok(format!(
            "{}{}{}",
            base_url, "/transactions/", metadata.transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = archipel::ArchipelAuthType::try_from(&req.connector_auth_type)?;
        let url = &self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(url)
                .attach_default_headers()
                .headers(headers)
                .add_ca_certificate_pem(auth_details.ca_certificate)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: archipel::ArchipelPaymentsResponse = res
            .response
            .parse_struct("ArchipelPaymentsResponse for PSync flow")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        info!(connector_response=?response);

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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Archipel {
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
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url =
            build_env_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        Ok(format!(
            "{}{}{}",
            base_url, "/capture/", req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let config_data: archipel::ArchipelConfigData = (&req.connector_meta_data).try_into()?;
        let amount_to_capture = crate::utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let router_data: archipel::ArchipelRouterData<_> =
            (amount_to_capture, config_data.tenant_id, req).into();
        let request: archipel::ArchipelCaptureRequest = router_data.into();

        Ok(RequestContent::Json(Box::new(request)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = archipel::ArchipelAuthType::try_from(&req.connector_auth_type)?;
        let url = &self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;
        let body = self.get_request_body(req, connectors)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(url)
                .attach_default_headers()
                .headers(headers)
                .add_ca_certificate_pem(auth_details.ca_certificate)
                .set_body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: archipel::ArchipelPaymentsResponse = res
            .response
            .parse_struct("ArchipelPaymentsResponse for Capture flow")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        info!(connector_response=?response);

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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Archipel
{
    fn get_headers(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url =
            build_env_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        Ok(format!("{}{}", base_url, "/verify"))
    }

    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let config_data: archipel::ArchipelConfigData = (&req.connector_meta_data).try_into()?;
        let router_data: archipel::ArchipelRouterData<_> =
            (MinorUnit::zero(), config_data.tenant_id, req).into();
        let request: ArchipelCardAuthorizationRequest = router_data.try_into()?;

        Ok(RequestContent::Json(Box::new(request)))
    }

    fn build_request(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = archipel::ArchipelAuthType::try_from(&req.connector_auth_type)?;
        let url = &self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;
        let body = self.get_request_body(req, connectors)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(url)
                .attach_default_headers()
                .headers(headers)
                .add_ca_certificate_pem(auth_details.ca_certificate)
                .set_body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SetupMandateRouterData, errors::ConnectorError> {
        let response: archipel::ArchipelPaymentsResponse = res
            .response
            .parse_struct("ArchipelPaymentsResponse for SetupMandate flow")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        info!(connector_response=?response);

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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Archipel {
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
        let base_url =
            build_env_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        Ok(format!(
            "{}{}{}",
            base_url, "/refund/", req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let config_data: archipel::ArchipelConfigData = (&req.connector_meta_data).try_into()?;
        let refund_amount = crate::utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let router_data: archipel::ArchipelRouterData<_> =
            (refund_amount, config_data.tenant_id, req).into();
        let request: ArchipelRefundRequest = router_data.into();

        Ok(RequestContent::Json(Box::new(request)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = archipel::ArchipelAuthType::try_from(&req.connector_auth_type)?;
        let url = &self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;
        let body = self.get_request_body(req, connectors)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(url)
                .attach_default_headers()
                .headers(headers)
                .add_ca_certificate_pem(auth_details.ca_certificate)
                .set_body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: archipel::ArchipelRefundResponse = res
            .response
            .parse_struct("ArchipelRefundResponse for Execute flow")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        info!(connector_response=?response);

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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Archipel {
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
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let config_data: archipel::ArchipelConfigData = (&req.connector_meta_data).try_into()?;
        let platform_url = &config_data.platform_url;
        let metadata: archipel::ArchipelTransactionMetadata = req
            .request
            .connector_metadata
            .clone()
            .and_then(|value| value.parse_value("ArchipelTransactionMetadata").ok())
            .ok_or_else(|| errors::ConnectorError::MissingConnectorTransactionID)?;

        Ok(format!(
            "{platform_url}{}{}",
            "Transaction/v1/transactions/", metadata.transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = archipel::ArchipelAuthType::try_from(&req.connector_auth_type)?;
        let url = &self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(url)
                .attach_default_headers()
                .headers(headers)
                .add_ca_certificate_pem(auth_details.ca_certificate)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: archipel::ArchipelRefundResponse = res
            .response
            .parse_struct("ArchipelRefundResponse for RSync flow")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        info!(connector_response=?response);

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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Archipel
{
    // Not Implemented (R)
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Archipel {
    // Not Implemented (R)
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Archipel {
    // Not Implemented (R)
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Archipel {
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
        let base_url =
            build_env_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        Ok(format!(
            "{}{}{}",
            base_url, "/cancel/", req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let config_data: archipel::ArchipelConfigData = (&req.connector_meta_data).try_into()?;
        let router_data: archipel::ArchipelRouterData<_> = (
            req.request
                .minor_amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "Amount",
                })?,
            config_data.tenant_id,
            req,
        )
            .into();
        let request: ArchipelPaymentsCancelRequest = router_data.into();

        Ok(RequestContent::Json(Box::new(request)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth_details = archipel::ArchipelAuthType::try_from(&req.connector_auth_type)?;
        let url = &self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;
        let body = self.get_request_body(req, connectors)?;

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(url)
                .attach_default_headers()
                .headers(headers)
                .add_ca_certificate_pem(auth_details.ca_certificate)
                .set_body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: archipel::ArchipelPaymentsResponse = res
            .response
            .parse_struct("ArchipelPaymentsResponse for Void flow")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        info!(connector_response=?response);

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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl IncomingWebhook for Archipel {
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

static ARCHIPEL_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::Manual,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let supported_card_network = vec![
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::Visa,
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::DinersClub,
            common_enums::CardNetwork::Discover,
            common_enums::CardNetwork::CartesBancaires,
        ];

        let mut archipel_supported_payment_methods = SupportedPaymentMethods::new();

        archipel_supported_payment_methods.add(
            enums::PaymentMethod::Card,
            enums::PaymentMethodType::Credit,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                        api_models::feature_matrix::CardSpecificFeatures {
                            three_ds: common_enums::FeatureStatus::Supported,
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_network.clone(),
                        }
                    }),
                ),
            },
        );

        archipel_supported_payment_methods.add(
            enums::PaymentMethod::Card,
            enums::PaymentMethodType::Debit,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                        api_models::feature_matrix::CardSpecificFeatures {
                            three_ds: common_enums::FeatureStatus::Supported,
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_network,
                        }
                    }),
                ),
            },
        );

        archipel_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::ApplePay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods,
                specific_features: None,
            },
        );

        archipel_supported_payment_methods
    });

static ARCHIPEL_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Archipel",
    description: "Full-service processor offering secure payment solutions and innovative banking technologies for businesses of all sizes.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static ARCHIPEL_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Archipel {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&ARCHIPEL_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*ARCHIPEL_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&ARCHIPEL_SUPPORTED_WEBHOOK_FLOWS)
    }
}
