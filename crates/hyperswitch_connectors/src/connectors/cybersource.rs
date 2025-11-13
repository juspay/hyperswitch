pub mod transformers;
use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{report, Report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        mandate_revoke::MandateRevoke,
        payments::{
            Authorize, Capture, CompleteAuthorize, IncrementalAuthorization, PSync,
            PaymentMethodToken, Session, SetupMandate, Void,
        },
        refunds::{Execute, RSync},
        Authenticate, PostAuthenticate, PreAuthenticate, PreProcessing,
    },
    router_request_types::{
        AccessTokenRequestData, CompleteAuthorizeData, MandateRevokeRequestData,
        PaymentMethodTokenizationData, PaymentsAuthenticateData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsIncrementalAuthorizationData,
        PaymentsPostAuthenticateData, PaymentsPreAuthenticateData, PaymentsPreProcessingData,
        PaymentsSessionData, PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, MandateRevokeResponseData, PaymentMethodDetails, PaymentsResponseData,
        RefundsResponseData, SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        MandateRevokeRouterData, PaymentsAuthenticateRouterData, PaymentsAuthorizeRouterData,
        PaymentsCancelRouterData, PaymentsCaptureRouterData, PaymentsCompleteAuthorizeRouterData,
        PaymentsIncrementalAuthorizationRouterData, PaymentsPostAuthenticateRouterData,
        PaymentsPreAuthenticateRouterData, PaymentsPreProcessingRouterData, PaymentsSyncRouterData,
        RefundExecuteRouterData, RefundSyncRouterData, SetupMandateRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::PoFulfill,
    router_response_types::PayoutsResponseData,
    types::{PayoutsData, PayoutsRouterData},
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types::PayoutFulfillType;
use hyperswitch_interfaces::{
    api::{
        self,
        payments::PaymentSession,
        refunds::{Refund, RefundExecute, RefundSync},
        ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        IncrementalAuthorizationType, MandateRevokeType, PaymentsAuthenticateType,
        PaymentsAuthorizeType, PaymentsCaptureType, PaymentsCompleteAuthorizeType,
        PaymentsPostAuthenticateType, PaymentsPreAuthenticateType, PaymentsPreProcessingType,
        PaymentsSyncType, PaymentsVoidType, RefundExecuteType, RefundSyncType, Response,
        SetupMandateType,
    },
    webhooks,
};
use masking::{ExposeInterface, Mask, Maskable, PeekInterface};
use ring::{digest, hmac};
use time::OffsetDateTime;
use transformers as cybersource;
use url::Url;

use crate::{
    constants::{self, headers},
    types::ResponseRouterData,
    utils::{
        self, convert_amount, PaymentMethodDataType, PaymentsAuthorizeRequestData,
        RefundsRequestData, RouterData as OtherRouterData,
    },
};

#[derive(Clone)]
pub struct Cybersource {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Cybersource {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}
impl Cybersource {
    pub fn generate_digest(&self, payload: &[u8]) -> String {
        let payload_digest = digest::digest(&digest::SHA256, payload);
        consts::BASE64_ENGINE.encode(payload_digest)
    }

    pub fn generate_signature(
        &self,
        auth: cybersource::CybersourceAuthType,
        host: String,
        resource: &str,
        payload: &String,
        date: OffsetDateTime,
        http_method: Method,
    ) -> CustomResult<String, errors::ConnectorError> {
        let cybersource::CybersourceAuthType {
            api_key,
            merchant_account,
            api_secret,
        } = auth;
        let is_post_method = matches!(http_method, Method::Post);
        let is_patch_method = matches!(http_method, Method::Patch);
        let is_delete_method = matches!(http_method, Method::Delete);
        let digest_str = if is_post_method || is_patch_method {
            "digest "
        } else {
            ""
        };
        let headers = format!("host date (request-target) {digest_str}v-c-merchant-id");
        let request_target = if is_post_method {
            format!("(request-target): post {resource}\ndigest: SHA-256={payload}\n")
        } else if is_patch_method {
            format!("(request-target): patch {resource}\ndigest: SHA-256={payload}\n")
        } else if is_delete_method {
            format!("(request-target): delete {resource}\n")
        } else {
            format!("(request-target): get {resource}\n")
        };
        let signature_string = format!(
            "host: {host}\ndate: {date}\n{request_target}v-c-merchant-id: {}",
            merchant_account.peek()
        );
        let key_value = consts::BASE64_ENGINE
            .decode(api_secret.expose())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "connector_account_details.api_secret",
            })?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, &key_value);
        let signature_value =
            consts::BASE64_ENGINE.encode(hmac::sign(&key, signature_string.as_bytes()).as_ref());
        let signature_header = format!(
            r#"keyid="{}", algorithm="HmacSHA256", headers="{headers}", signature="{signature_value}""#,
            api_key.peek()
        );

        Ok(signature_header)
    }
}

impl ConnectorCommon for Cybersource {
    fn id(&self) -> &'static str {
        "cybersource"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json;charset=utf-8"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.cybersource.base_url.as_ref()
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: Result<
            cybersource::CybersourceErrorResponse,
            Report<common_utils::errors::ParsingError>,
        > = res.response.parse_struct("Cybersource ErrorResponse");

        let error_message = if res.status_code == 401 {
            constants::CONNECTOR_UNAUTHORIZED_ERROR
        } else {
            hyperswitch_interfaces::consts::NO_ERROR_MESSAGE
        };
        match response {
            Ok(transformers::CybersourceErrorResponse::StandardError(response)) => {
                event_builder.map(|i| i.set_error_response_body(&response));
                router_env::logger::info!(connector_response=?response);

                let (code, message, reason) = match response.error_information {
                    Some(ref error_info) => {
                        let detailed_error_info = error_info.details.as_ref().map(|details| {
                            details
                                .iter()
                                .map(|det| format!("{} : {}", det.field, det.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });
                        (
                            error_info.reason.clone(),
                            error_info.message.clone(),
                            transformers::get_error_reason(
                                Some(error_info.message.clone()),
                                detailed_error_info,
                                None,
                            ),
                        )
                    }
                    None => {
                        let detailed_error_info = response.details.map(|details| {
                            details
                                .iter()
                                .map(|det| format!("{} : {}", det.field, det.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });
                        (
                            response.reason.clone().map_or(
                                hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                                |reason| reason.to_string(),
                            ),
                            response
                                .message
                                .clone()
                                .map_or(error_message.to_string(), |msg| msg.to_string()),
                            transformers::get_error_reason(
                                response.message,
                                detailed_error_info,
                                None,
                            ),
                        )
                    }
                };

                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code,
                    message,
                    reason,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Ok(transformers::CybersourceErrorResponse::AuthenticationError(response)) => {
                event_builder.map(|i| i.set_error_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                    message: response.response.rmsg.clone(),
                    reason: Some(response.response.rmsg),
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Ok(transformers::CybersourceErrorResponse::NotAvailableError(response)) => {
                event_builder.map(|i| i.set_error_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                let error_response = response
                    .errors
                    .iter()
                    .map(|error_info| {
                        format!(
                            "{}: {}",
                            error_info.error_type.clone().unwrap_or("".to_string()),
                            error_info.message.clone().unwrap_or("".to_string())
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(" & ");
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                    message: error_response.clone(),
                    reason: Some(error_response),
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Err(error_msg) => {
                event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
                router_env::logger::error!(deserialization_error =? error_msg);
                utils::handle_json_response_deserialization_failure(res, "cybersource")
            }
        }
    }
}

impl ConnectorValidation for Cybersource {
    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
            PaymentMethodDataType::ApplePay,
            PaymentMethodDataType::GooglePay,
            PaymentMethodDataType::SamsungPay,
        ]);
        utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Cybersource
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let date = OffsetDateTime::now_utc();
        let cybersource_req = self.get_request_body(req, connectors)?;
        let auth = cybersource::CybersourceAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account = auth.merchant_account.clone();
        let base_url = connectors.cybersource.base_url.as_str();
        let cybersource_host =
            Url::parse(base_url).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let host = cybersource_host
            .host_str()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let path: String = self
            .get_url(req, connectors)?
            .chars()
            .skip(base_url.len() - 1)
            .collect();
        let sha256 = self.generate_digest(cybersource_req.get_inner_value().expose().as_bytes());
        let http_method = self.get_http_method();
        let signature = self.generate_signature(
            auth,
            host.to_string(),
            path.as_str(),
            &sha256,
            date,
            http_method,
        )?;

        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                "application/hal+json;charset=utf-8".to_string().into(),
            ),
            (
                "v-c-merchant-id".to_string(),
                merchant_account.into_masked(),
            ),
            ("Date".to_string(), date.to_string().into()),
            ("Host".to_string(), host.to_string().into()),
            ("Signature".to_string(), signature.into_masked()),
        ];
        if matches!(http_method, Method::Post | Method::Put | Method::Patch) {
            headers.push((
                "Digest".to_string(),
                format!("SHA-256={sha256}").into_masked(),
            ));
        }
        Ok(headers)
    }
}

impl api::Payment for Cybersource {}
impl api::PaymentsPreAuthenticate for Cybersource {}
impl api::PaymentsPostAuthenticate for Cybersource {}
impl api::PaymentsAuthenticate for Cybersource {}
impl api::PaymentAuthorize for Cybersource {}
impl api::PaymentSync for Cybersource {}
impl api::PaymentVoid for Cybersource {}
impl api::PaymentCapture for Cybersource {}
impl api::PaymentIncrementalAuthorization for Cybersource {}
impl api::MandateSetup for Cybersource {}
impl api::ConnectorAccessToken for Cybersource {}
impl api::PaymentToken for Cybersource {}
impl api::PaymentsPreProcessing for Cybersource {}
impl api::PaymentsCompleteAuthorize for Cybersource {}
impl api::ConnectorMandateRevoke for Cybersource {}

impl api::Payouts for Cybersource {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Cybersource {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Cybersource
{
    // Not Implemented (R)
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Cybersource
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
        _req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}pts/v2/payments/", self.base_url(connectors)))
    }
    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = cybersource::CybersourceZeroMandateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&SetupMandateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(SetupMandateType::get_headers(self, req, connectors)?)
                .set_body(SetupMandateType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SetupMandateRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("CybersourceSetupMandatesResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        let attempt_status = match response.reason {
            Some(reason) => match reason {
                transformers::Reason::SystemError => Some(enums::AttemptStatus::Failure),
                transformers::Reason::ServerTimeout | transformers::Reason::ServiceTimeout => None,
            },
            None => None,
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &MandateRevokeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_http_method(&self) -> Method {
        Method::Delete
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        req: &MandateRevokeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}tms/v1/paymentinstruments/{}",
            self.base_url(connectors),
            utils::RevokeMandateRequestData::get_connector_mandate_id(&req.request)?
        ))
    }
    fn build_request(
        &self,
        req: &MandateRevokeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Delete)
                .url(&MandateRevokeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(MandateRevokeType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &MandateRevokeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<MandateRevokeRouterData, errors::ConnectorError> {
        if matches!(res.status_code, 204) {
            event_builder.map(|i| i.set_response_body(&serde_json::json!({"mandate_status": common_enums::MandateStatus::Revoked.to_string()})));
            Ok(MandateRevokeRouterData {
                response: Ok(MandateRevokeResponseData {
                    mandate_status: common_enums::MandateStatus::Revoked,
                }),
                ..data.clone()
            })
        } else {
            // If http_code != 204 || http_code != 4xx, we dont know any other response scenario yet.
            let response_value: serde_json::Value = serde_json::from_slice(&res.response)
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
            let response_string = response_value.to_string();

            event_builder.map(|i| {
                i.set_response_body(
                    &serde_json::json!({"response_string": response_string.clone()}),
                )
            });
            router_env::logger::info!(connector_response=?response_string);

            Ok(MandateRevokeRouterData {
                response: Err(ErrorResponse {
                    code: hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                    message: response_string.clone(),
                    reason: Some(response_string),
                    status_code: res.status_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..data.clone()
            })
        }
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}
impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Cybersource {
    // Not Implemented (R)
}

impl PaymentSession for Cybersource {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Cybersource {}

impl ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let redirect_response = req.request.redirect_response.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "redirect_response",
            },
        )?;
        match redirect_response.params {
            Some(param) if !param.clone().peek().is_empty() => Ok(format!(
                "{}risk/v1/authentications",
                self.base_url(connectors)
            )),
            Some(_) | None => Ok(format!(
                "{}risk/v1/authentication-results",
                self.base_url(connectors)
            )),
        }
    }
    fn get_request_body(
        &self,
        req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let minor_amount =
            req.request
                .minor_amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "minor_amount",
                })?;
        let currency =
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?;
        let amount = convert_amount(self.amount_converter, minor_amount, currency)?;
        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));
        let connector_req =
            cybersource::CybersourcePreProcessingRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsPreProcessingType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsPreProcessingType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(PaymentsPreProcessingType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsPreProcessingRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsPreProcessingRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePreProcessingResponse = res
            .response
            .parse_struct("Cybersource AuthEnrollmentResponse")
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

impl ConnectorIntegration<PreAuthenticate, PaymentsPreAuthenticateData, PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &PaymentsPreAuthenticateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &PaymentsPreAuthenticateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}risk/v1/authentication-setups",
            ConnectorCommon::base_url(self, connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &PaymentsPreAuthenticateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let minor_amount = req.request.minor_amount;
        let currency =
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?;

        let amount = convert_amount(self.amount_converter, minor_amount, currency)?;

        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));
        let connector_req =
            cybersource::CybersourceAuthSetupRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsPreAuthenticateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PaymentsPreAuthenticateType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(PaymentsPreAuthenticateType::get_headers(
                self, req, connectors,
            )?)
            .set_body(self.get_request_body(req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsPreAuthenticateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsPreAuthenticateRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceAuthSetupResponse = res
            .response
            .parse_struct("Cybersource AuthSetupResponse")
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

impl ConnectorIntegration<Authenticate, PaymentsAuthenticateData, PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &PaymentsAuthenticateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &PaymentsAuthenticateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}risk/v1/authentications",
            self.base_url(connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &PaymentsAuthenticateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let minor_amount =
            req.request
                .minor_amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "minor_amount",
                })?;
        let currency =
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?;
        let amount = convert_amount(self.amount_converter, minor_amount, currency)?;
        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));
        let connector_req =
            cybersource::CybersourceAuthEnrollmentRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsAuthenticateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsAuthenticateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsAuthenticateType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(PaymentsAuthenticateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthenticateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthenticateRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceAuthenticateResponse = res
            .response
            .parse_struct("Cybersource AuthEnrollmentResponse")
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

impl ConnectorIntegration<PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &PaymentsPostAuthenticateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &PaymentsPostAuthenticateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}risk/v1/authentication-results",
            self.base_url(connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &PaymentsPostAuthenticateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let minor_amount =
            req.request
                .minor_amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "minor_amount",
                })?;
        let currency =
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?;
        let amount = convert_amount(self.amount_converter, minor_amount, currency)?;
        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));
        let connector_req =
            cybersource::CybersourceAuthValidateRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsPostAuthenticateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsPostAuthenticateType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(PaymentsPostAuthenticateType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(PaymentsPostAuthenticateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsPostAuthenticateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsPostAuthenticateRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceAuthenticateResponse = res
            .response
            .parse_struct("Cybersource AuthEnrollmentResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Cybersource {
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
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{}/captures",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));
        let connector_req =
            cybersource::CybersourcePaymentsCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsCaptureType::get_headers(self, req, connectors)?)
                .set_body(PaymentsCaptureType::get_request_body(
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
    ) -> CustomResult<
        RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
        errors::ConnectorError,
    > {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Cybersource {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}tss/v2/transactions/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceTransactionResponse = res
            .response
            .parse_struct("Cybersource PaymentSyncResponse")
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

#[cfg(feature = "v1")]
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Cybersource {
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
        if req.is_three_ds()
            && req.request.is_card()
            && (req.request.connector_mandate_id().is_none()
                && req.request.get_optional_network_transaction_id().is_none())
            && req.request.authentication_data.is_none()
        {
            Ok(format!(
                "{}risk/v1/authentication-setups",
                ConnectorCommon::base_url(self, connectors)
            ))
        } else {
            Ok(format!(
                "{}pts/v2/payments/",
                ConnectorCommon::base_url(self, connectors)
            ))
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
        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));
        if req.is_three_ds()
            && req.request.is_card()
            && (req.request.connector_mandate_id().is_none()
                && req.request.get_optional_network_transaction_id().is_none())
            && req.request.authentication_data.is_none()
        {
            let connector_req =
                cybersource::CybersourceAuthSetupRequest::try_from(&connector_router_data)?;
            Ok(RequestContent::Json(Box::new(connector_req)))
        } else {
            let connector_req =
                cybersource::CybersourcePaymentsRequest::try_from(&connector_router_data)?;
            Ok(RequestContent::Json(Box::new(connector_req)))
        }
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
            .set_body(self.get_request_body(req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        if data.is_three_ds()
            && data.request.is_card()
            && (data.request.connector_mandate_id().is_none()
                && data.request.get_optional_network_transaction_id().is_none())
            && data.request.authentication_data.is_none()
        {
            let response: cybersource::CybersourceAuthSetupResponse = res
                .response
                .parse_struct("Cybersource AuthSetupResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);
            RouterData::try_from(ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        } else {
            let response: cybersource::CybersourcePaymentsResponse = res
                .response
                .parse_struct("Cybersource PaymentResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);
            RouterData::try_from(ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        }
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
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        let attempt_status = match response.reason {
            Some(reason) => match reason {
                transformers::Reason::SystemError => Some(enums::AttemptStatus::Failure),
                transformers::Reason::ServerTimeout | transformers::Reason::ServiceTimeout => None,
            },
            None => None,
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

#[cfg(feature = "v2")]
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Cybersource {
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
        Ok(format!(
            "{}pts/v2/payments/",
            ConnectorCommon::base_url(self, connectors)
        ))
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
        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));
        let connector_req =
            cybersource::CybersourcePaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
            .set_body(self.get_request_body(req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        let attempt_status = match response.reason {
            Some(reason) => match reason {
                transformers::Reason::SystemError => Some(enums::AttemptStatus::Failure),
                transformers::Reason::ServerTimeout | transformers::Reason::ServiceTimeout => None,
            },
            None => None,
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Cybersource {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}pts/v2/payouts", self.base_url(connectors)))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.destination_currency,
        )?;
        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));
        let connector_req =
            cybersource::CybersourcePayoutFulfillRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutFulfillType::get_headers(self, req, connectors)?)
            .set_body(PayoutFulfillType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, errors::ConnectorError> {
        let response: cybersource::CybersourceFulfillResponse = res
            .response
            .parse_struct("CybersourceFulfillResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        let attempt_status = match response.reason {
            Some(reason) => match reason {
                transformers::Reason::SystemError => Some(enums::AttemptStatus::Failure),
                transformers::Reason::ServerTimeout | transformers::Reason::ServiceTimeout => None,
            },
            None => None,
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &PaymentsCompleteAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &PaymentsCompleteAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}pts/v2/payments/",
            ConnectorCommon::base_url(self, connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &PaymentsCompleteAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));
        let connector_req =
            cybersource::CybersourcePaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsCompleteAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsCompleteAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(PaymentsCompleteAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(PaymentsCompleteAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCompleteAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        let attempt_status = match response.reason {
            Some(reason) => match reason {
                transformers::Reason::SystemError => Some(enums::AttemptStatus::Failure),
                transformers::Reason::ServerTimeout | transformers::Reason::ServiceTimeout => None,
            },
            None => None,
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Cybersource {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{connector_payment_id}/reversals",
            self.base_url(connectors)
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let minor_amount =
            req.request
                .minor_amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "Amount",
                })?;
        let currency =
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "Currency",
                })?;
        let amount = convert_amount(self.amount_converter, minor_amount, currency)?;
        let connector_router_data = cybersource::CybersourceRouterData::from((amount, req));

        let connector_req = cybersource::CybersourceVoidRequest::try_from(&connector_router_data)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl Refund for Cybersource {}
impl RefundExecute for Cybersource {}
impl RefundSync for Cybersource {}

#[allow(dead_code)]
impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Cybersource {
    fn get_headers(
        &self,
        req: &RefundExecuteRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundExecuteRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{}/refunds",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundExecuteRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let connector_router_data = cybersource::CybersourceRouterData::from((refund_amount, req));
        let connector_req =
            cybersource::CybersourceRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &RefundExecuteRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RefundExecuteType::get_headers(self, req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundExecuteRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundExecuteRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceRefundResponse = res
            .response
            .parse_struct("Cybersource RefundResponse")
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

#[allow(dead_code)]
impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Cybersource {
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
    fn get_http_method(&self) -> Method {
        Method::Get
    }
    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}tss/v2/transactions/{}",
            self.base_url(connectors),
            refund_id
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
                .url(&RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceRsyncResponse = res
            .response
            .parse_struct("Cybersource RefundSyncResponse")
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

impl
    ConnectorIntegration<
        IncrementalAuthorization,
        PaymentsIncrementalAuthorizationData,
        PaymentsResponseData,
    > for Cybersource
{
    fn get_headers(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_http_method(&self) -> Method {
        Method::Patch
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let minor_additional_amount = MinorUnit::new(req.request.additional_amount);
        let additional_amount = convert_amount(
            self.amount_converter,
            minor_additional_amount,
            req.request.currency,
        )?;
        let connector_router_data =
            cybersource::CybersourceRouterData::from((additional_amount, req));
        let connector_request =
            cybersource::CybersourcePaymentsIncrementalAuthorizationRequest::try_from(
                &connector_router_data,
            )?;
        Ok(RequestContent::Json(Box::new(connector_request)))
    }
    fn build_request(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Patch)
                .url(&IncrementalAuthorizationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(IncrementalAuthorizationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(IncrementalAuthorizationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &PaymentsIncrementalAuthorizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<
            IncrementalAuthorization,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        >,
        errors::ConnectorError,
    > {
        let response: cybersource::CybersourcePaymentsIncrementalAuthorizationResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
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
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Cybersource {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

static CYBERSOURCE_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::Manual,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let supported_card_network = vec![
            common_enums::CardNetwork::Visa,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::JCB,
            common_enums::CardNetwork::DinersClub,
            common_enums::CardNetwork::Discover,
            common_enums::CardNetwork::Visa,
            common_enums::CardNetwork::CartesBancaires,
            common_enums::CardNetwork::UnionPay,
            common_enums::CardNetwork::Maestro,
        ];

        let mut cybersource_supported_payment_methods = SupportedPaymentMethods::new();

        cybersource_supported_payment_methods.add(
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

        cybersource_supported_payment_methods.add(
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
                            supported_card_networks: supported_card_network.clone(),
                        }
                    }),
                ),
            },
        );

        cybersource_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::ApplePay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        cybersource_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::GooglePay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        cybersource_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::Paze,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        cybersource_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::SamsungPay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        cybersource_supported_payment_methods
    });

static CYBERSOURCE_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Cybersource",
    description: "Cybersource provides payments, fraud protection, integrations, and support to help businesses grow with a digital-first approach.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static CYBERSOURCE_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 0] = [];

impl ConnectorSpecifications for Cybersource {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&CYBERSOURCE_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*CYBERSOURCE_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&CYBERSOURCE_SUPPORTED_WEBHOOK_FLOWS)
    }
    fn get_preprocessing_flow_if_needed(
        &self,
        current_flow_info: api::CurrentFlowInfo<'_>,
    ) -> Option<api::PreProcessingFlowName> {
        match current_flow_info {
            api::CurrentFlowInfo::Authorize { .. } => {
                // during authorize flow, there is no pre processing flow. Only alternate PreAuthenticate flow
                None
            }
            api::CurrentFlowInfo::CompleteAuthorize { request_data } => {
                // TODO: add logic before deciding the pre processing flow Authenticate or PostAuthenticate
                let redirect_response = request_data.redirect_response.as_ref()?;
                match redirect_response.params.as_ref() {
                    Some(param) if !param.peek().is_empty() => {
                        Some(api::PreProcessingFlowName::Authenticate)
                    }
                    Some(_) | None => Some(api::PreProcessingFlowName::PostAuthenticate),
                }
            }
        }
    }
    fn decide_should_continue_after_preprocessing(
        &self,
        current_flow: api::CurrentFlowInfo<'_>,
        pre_processing_flow_name: api::PreProcessingFlowName,
        preprocessing_flow_response: api::PreProcessingFlowResponse<'_>,
    ) -> bool {
        match (current_flow, pre_processing_flow_name) {
            (api::CurrentFlowInfo::Authorize { .. }, _) => {
                // during authorize flow, there is no pre processing flow. Only alternate PreAuthenticate flow
                true
            }
            (
                api::CurrentFlowInfo::CompleteAuthorize { .. },
                api::PreProcessingFlowName::Authenticate,
            )
            | (
                api::CurrentFlowInfo::CompleteAuthorize { .. },
                api::PreProcessingFlowName::PostAuthenticate,
            ) => {
                (matches!(
                    preprocessing_flow_response.response,
                    Ok(PaymentsResponseData::TransactionResponse {
                        ref redirection_data,
                        ..
                    }) if redirection_data.is_none()
                ) && preprocessing_flow_response.attempt_status
                    != common_enums::AttemptStatus::AuthenticationFailed)
            }
        }
    }
    fn get_alternate_flow_if_needed(
        &self,
        current_flow: api::CurrentFlowInfo<'_>,
    ) -> Option<api::AlternateFlow> {
        match current_flow {
            api::CurrentFlowInfo::Authorize {
                request_data,
                auth_type,
            } => {
                if self.is_3ds_setup_required(request_data, *auth_type) {
                    Some(api::AlternateFlow::PreAuthenticate)
                } else {
                    None
                }
            }
            // No alternate flow for complete authorize
            api::CurrentFlowInfo::CompleteAuthorize { .. } => None,
        }
    }
}

impl Cybersource {
    pub fn is_3ds_setup_required(
        &self,
        request: &PaymentsAuthorizeData,
        auth_type: common_enums::AuthenticationType,
    ) -> bool {
        router_env::logger::info!(router_data_request=?request, auth_type=?auth_type, "Checking if 3DS setup is required for Cybersource");
        auth_type.is_three_ds()
            && request.is_card()
            && (request.connector_mandate_id().is_none()
                && request.get_optional_network_transaction_id().is_none())
            && request.authentication_data.is_none()
    }
}
