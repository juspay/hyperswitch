pub mod transformers;
use std::fmt::Write;

use base64::Engine;
use common_enums::{enums, CallConnectorAction, PaymentAction};
use common_utils::{
    consts,
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, PSync, PaymentMethodToken, PostSessionTokens, PreProcessing,
            SdkSessionUpdate, Session, SetupMandate, Void,
        },
        refunds::{Execute, RSync},
        CompleteAuthorize, VerifyWebhookSource,
    },
    router_request_types::{
        AccessTokenRequestData, CompleteAuthorizeData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsPostSessionTokensData, PaymentsPreProcessingData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, ResponseId, SdkPaymentsSessionUpdateData,
        SetupMandateRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        PaymentsResponseData, RefundsResponseData, VerifyWebhookSourceResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPostSessionTokensRouterData,
        PaymentsPreProcessingRouterData, PaymentsSyncRouterData, RefreshTokenRouterData,
        RefundSyncRouterData, RefundsRouterData, SdkSessionUpdateRouterData,
        SetupMandateRouterData, VerifyWebhookSourceRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::{PoCreate, PoFulfill, PoSync},
    router_response_types::PayoutsResponseData,
    types::{PayoutsData, PayoutsRouterData},
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types::{PayoutFulfillType, PayoutSyncType};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorRedirectResponse,
        ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    disputes, errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        PaymentsAuthorizeType, PaymentsCaptureType, PaymentsCompleteAuthorizeType,
        PaymentsPostSessionTokensType, PaymentsPreProcessingType, PaymentsSyncType,
        PaymentsVoidType, RefreshTokenType, RefundExecuteType, RefundSyncType, Response,
        SdkSessionUpdateType, SetupMandateType, VerifyWebhookSourceType,
    },
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::{ExposeInterface, Mask, Maskable, PeekInterface, Secret};
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};
use transformers::{
    self as paypal, auth_headers, PaypalAuthResponse, PaypalMeta, PaypalWebhookEventType,
};

use crate::{
    constants::{self, headers},
    types::ResponseRouterData,
    utils::{
        self as connector_utils, to_connector_meta, ConnectorErrorType, ConnectorErrorTypeMapping,
        ForeignTryFrom, PaymentMethodDataType, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, RefundsRequestData,
    },
};

#[derive(Clone)]
pub struct Paypal {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Paypal {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

impl api::Payment for Paypal {}
impl api::PaymentSession for Paypal {}
impl api::PaymentToken for Paypal {}
impl api::ConnectorAccessToken for Paypal {}
impl api::MandateSetup for Paypal {}
impl api::PaymentAuthorize for Paypal {}
impl api::PaymentsCompleteAuthorize for Paypal {}
impl api::PaymentSync for Paypal {}
impl api::PaymentCapture for Paypal {}
impl api::PaymentVoid for Paypal {}
impl api::Refund for Paypal {}
impl api::RefundExecute for Paypal {}
impl api::RefundSync for Paypal {}
impl api::ConnectorVerifyWebhookSource for Paypal {}
impl api::PaymentPostSessionTokens for Paypal {}
impl api::PaymentSessionUpdate for Paypal {}

impl api::Payouts for Paypal {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for Paypal {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Paypal {}
#[cfg(feature = "payouts")]
impl api::PayoutSync for Paypal {}

impl Paypal {
    pub fn get_order_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        //Handled error response separately for Orders as the end point is different for Orders - (Authorize) and Payments - (Capture, void, refund, rsync).
        //Error response have different fields for Orders and Payments.
        let response: paypal::PaypalOrderErrorResponse = res
            .response
            .parse_struct("Paypal ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_reason = response.details.clone().map(|order_errors| {
            order_errors
                .iter()
                .map(|error| {
                    let mut reason = format!("description - {}", error.description);
                    if let Some(value) = &error.value {
                        reason.push_str(&format!(", value - {value}"));
                    }
                    if let Some(field) = error
                        .field
                        .as_ref()
                        .and_then(|field| field.split('/').last())
                    {
                        reason.push_str(&format!(", field - {field}"));
                    }
                    reason.push(';');
                    reason
                })
                .collect::<String>()
        });
        let errors_list = response.details.unwrap_or_default();
        let option_error_code_message =
            connector_utils::get_error_code_error_message_based_on_priority(
                self.clone(),
                errors_list
                    .into_iter()
                    .map(|errors| errors.into())
                    .collect(),
            );
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: option_error_code_message
                .clone()
                .map(|error_code_message| error_code_message.error_code)
                .unwrap_or(NO_ERROR_CODE.to_string()),
            message: option_error_code_message
                .map(|error_code_message| error_code_message.error_message)
                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
            reason: error_reason.or(Some(response.message)),
            attempt_status: None,
            connector_transaction_id: response.debug_id,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Paypal
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let access_token = req
            .access_token
            .clone()
            .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
        let key = &req.connector_request_reference_id;
        let auth = paypal::PaypalAuthType::try_from(&req.connector_auth_type)?;
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.peek()).into_masked(),
            ),
            (
                auth_headers::PREFER.to_string(),
                "return=representation".to_string().into(),
            ),
            (
                auth_headers::PAYPAL_REQUEST_ID.to_string(),
                key.to_string().into_masked(),
            ),
        ];
        if let Ok(paypal::PaypalConnectorCredentials::PartnerIntegration(credentials)) =
            auth.get_credentials()
        {
            let auth_assertion_header =
                construct_auth_assertion_header(&credentials.payer_id, &credentials.client_id);
            headers.extend(vec![
                (
                    auth_headers::PAYPAL_AUTH_ASSERTION.to_string(),
                    auth_assertion_header.to_string().into_masked(),
                ),
                (
                    auth_headers::PAYPAL_PARTNER_ATTRIBUTION_ID.to_string(),
                    "HyperSwitchPPCP_SP".to_string().into(),
                ),
            ])
        } else {
            headers.extend(vec![(
                auth_headers::PAYPAL_PARTNER_ATTRIBUTION_ID.to_string(),
                "HyperSwitchlegacy_Ecom".to_string().into(),
            )])
        }
        Ok(headers)
    }
}

fn construct_auth_assertion_header(
    payer_id: &Secret<String>,
    client_id: &Secret<String>,
) -> String {
    let algorithm = consts::BASE64_ENGINE
        .encode("{\"alg\":\"none\"}")
        .to_string();
    let merchant_credentials = format!(
        "{{\"iss\":\"{}\",\"payer_id\":\"{}\"}}",
        client_id.clone().expose(),
        payer_id.clone().expose()
    );
    let encoded_credentials = consts::BASE64_ENGINE
        .encode(merchant_credentials)
        .to_string();
    format!("{algorithm}.{encoded_credentials}.")
}

impl ConnectorCommon for Paypal {
    fn id(&self) -> &'static str {
        "paypal"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.paypal.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = paypal::PaypalAuthType::try_from(auth_type)?;
        let credentials = auth.get_credentials()?;

        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            credentials.get_client_secret().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: paypal::PaypalPaymentErrorResponse = res
            .response
            .parse_struct("Paypal ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_reason = response
            .details
            .clone()
            .map(|error_details| {
                error_details
                    .iter()
                    .try_fold(String::new(), |mut acc, error| {
                        if let Some(description) = &error.description {
                            write!(acc, "description - {} ;", description)
                                .change_context(
                                    errors::ConnectorError::ResponseDeserializationFailed,
                                )
                                .attach_printable("Failed to concatenate error details")
                                .map(|_| acc)
                        } else {
                            Ok(acc)
                        }
                    })
            })
            .transpose()?;
        let reason = match error_reason {
            Some(err_reason) => err_reason
                .is_empty()
                .then(|| response.message.to_owned())
                .or(Some(err_reason)),
            None => Some(response.message.to_owned()),
        };
        let errors_list = response.details.unwrap_or_default();
        let option_error_code_message =
            connector_utils::get_error_code_error_message_based_on_priority(
                self.clone(),
                errors_list
                    .into_iter()
                    .map(|errors| errors.into())
                    .collect(),
            );

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: option_error_code_message
                .clone()
                .map(|error_code_message| error_code_message.error_code)
                .unwrap_or(NO_ERROR_CODE.to_string()),
            message: option_error_code_message
                .map(|error_code_message| error_code_message.error_message)
                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
            reason,
            attempt_status: None,
            connector_transaction_id: response.debug_id,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl ConnectorValidation for Paypal {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _payment_method: enums::PaymentMethod,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic
            | enums::CaptureMethod::Manual
            | enums::CaptureMethod::SequentialAutomatic => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }
    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
            PaymentMethodDataType::PaypalRedirect,
            PaymentMethodDataType::PaypalSdk,
        ]);
        connector_utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Paypal
{
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Paypal {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Paypal {
    fn get_url(
        &self,
        _req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v1/oauth2/token", self.base_url(connectors)))
    }
    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }
    fn get_headers(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = paypal::PaypalAuthType::try_from(&req.connector_auth_type)?;
        let credentials = auth.get_credentials()?;
        let auth_val = credentials.generate_authorization_value();

        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                RefreshTokenType::get_content_type(self).to_string().into(),
            ),
            (headers::AUTHORIZATION.to_string(), auth_val.into_masked()),
        ])
    }
    fn get_request_body(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = paypal::PaypalAuthUpdateRequest::try_from(req)?;

        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let req = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .headers(RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&RefreshTokenType::get_url(self, req, connectors)?)
                .set_body(RefreshTokenType::get_request_body(self, req, connectors)?)
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
        let response: paypal::PaypalAuthUpdateResponse = res
            .response
            .parse_struct("Paypal PaypalAuthUpdateResponse")
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
        let response: paypal::PaypalAccessTokenErrorResponse = res
            .response
            .parse_struct("Paypal AccessTokenErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.clone(),
            message: response.error.clone(),
            reason: Some(response.error_description),
            attempt_status: None,
            connector_transaction_id: None,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Paypal {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v1/payments/payouts", self.base_url(connectors)))
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
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.destination_currency,
        )?;
        let connector_router_data =
            paypal::PaypalRouterData::try_from((amount, None, None, None, req))?;
        let connector_req = paypal::PaypalFulfillRequest::try_from(&connector_router_data)?;
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

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, errors::ConnectorError> {
        let response: paypal::PaypalFulfillResponse = res
            .response
            .parse_struct("PaypalFulfillResponse")
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
impl ConnectorIntegration<PoSync, PayoutsData, PayoutsResponseData> for Paypal {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let batch_id = req.request.connector_payout_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_payout_id",
            },
        )?;
        Ok(format!(
            "{}v1/payments/payouts/{}",
            self.base_url(connectors),
            batch_id
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
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

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoSync>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoSync>, errors::ConnectorError> {
        let response: paypal::PaypalFulfillResponse = res
            .response
            .parse_struct("PaypalFulfillResponse")
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
impl ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData> for Paypal {
    fn build_request(
        &self,
        _req: &PayoutsRouterData<PoCreate>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        // Eligibility check for wallet is not implemented
        Err(
            errors::ConnectorError::NotImplemented("Payout Eligibility for Paypal".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Paypal {
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
        Ok(format!(
            "{}v3/vault/payment-tokens/",
            self.base_url(connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = paypal::PaypalZeroMandateRequest::try_from(req)?;
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
        let response: paypal::PaypalSetupMandatesResponse = res
            .response
            .parse_struct("PaypalSetupMandatesResponse")
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

impl ConnectorIntegration<PostSessionTokens, PaymentsPostSessionTokensData, PaymentsResponseData>
    for Paypal
{
    fn get_headers(
        &self,
        req: &PaymentsPostSessionTokensRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &PaymentsPostSessionTokensRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v2/checkout/orders", self.base_url(connectors)))
    }
    fn build_request(
        &self,
        req: &PaymentsPostSessionTokensRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsPostSessionTokensType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(PaymentsPostSessionTokensType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(PaymentsPostSessionTokensType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsPostSessionTokensRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.amount,
            req.request.currency,
        )?;
        let shipping_cost = connector_utils::convert_amount(
            self.amount_converter,
            req.request.shipping_cost.unwrap_or(MinorUnit::zero()),
            req.request.currency,
        )?;
        let order_amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.order_amount,
            req.request.currency,
        )?;
        let connector_router_data = paypal::PaypalRouterData::try_from((
            amount,
            Some(shipping_cost),
            None,
            Some(order_amount),
            req,
        ))?;
        let connector_req = paypal::PaypalPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &PaymentsPostSessionTokensRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsPostSessionTokensRouterData, errors::ConnectorError> {
        let response: paypal::PaypalRedirectResponse = res
            .response
            .parse_struct("PaypalRedirectResponse")
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
        self.get_order_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<SdkSessionUpdate, SdkPaymentsSessionUpdateData, PaymentsResponseData>
    for Paypal
{
    fn get_headers(
        &self,
        req: &SdkSessionUpdateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &SdkSessionUpdateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let session_id =
            req.request
                .session_id
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "session_id",
                })?;
        Ok(format!(
            "{}v2/checkout/orders/{}",
            self.base_url(connectors),
            session_id
        ))
    }

    fn build_request(
        &self,
        req: &SdkSessionUpdateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Patch)
                .url(&SdkSessionUpdateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(SdkSessionUpdateType::get_headers(self, req, connectors)?)
                .set_body(SdkSessionUpdateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn get_request_body(
        &self,
        req: &SdkSessionUpdateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let order_amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.order_amount,
            req.request.currency,
        )?;
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.amount,
            req.request.currency,
        )?;
        let order_tax_amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.order_tax_amount,
            req.request.currency,
        )?;
        let shipping_cost = connector_utils::convert_amount(
            self.amount_converter,
            req.request.shipping_cost.unwrap_or(MinorUnit::zero()),
            req.request.currency,
        )?;
        let connector_router_data = paypal::PaypalRouterData::try_from((
            amount,
            Some(shipping_cost),
            Some(order_tax_amount),
            Some(order_amount),
            req,
        ))?;

        let connector_req = paypal::PaypalUpdateOrderRequest::try_from(&connector_router_data)?;
        // encode only for for urlencoded things.
        Ok(RequestContent::Json(Box::new(
            connector_req.get_inner_value(),
        )))
    }

    fn handle_response(
        &self,
        data: &SdkSessionUpdateRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SdkSessionUpdateRouterData, errors::ConnectorError> {
        router_env::logger::debug!("Expected zero bytes response, skipped parsing of the response");
        // https://developer.paypal.com/docs/api/orders/v2/#orders_patch
        // If 204 status code, then the session was updated successfully.
        let status = if res.status_code == 204 {
            enums::SessionUpdateStatus::Success
        } else {
            enums::SessionUpdateStatus::Failure
        };
        Ok(SdkSessionUpdateRouterData {
            response: Ok(PaymentsResponseData::SessionUpdateResponse { status }),
            ..data.clone()
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

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Paypal {
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
        match &req.request.payment_method_data {
            PaymentMethodData::Wallet(WalletData::PaypalSdk(paypal_wallet_data)) => {
                let authorize_url = if req.request.is_auto_capture()? {
                    "capture".to_string()
                } else {
                    "authorize".to_string()
                };
                Ok(format!(
                    "{}v2/checkout/orders/{}/{authorize_url}",
                    self.base_url(connectors),
                    paypal_wallet_data.token
                ))
            }
            _ => Ok(format!("{}v2/checkout/orders", self.base_url(connectors))),
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let shipping_cost = connector_utils::convert_amount(
            self.amount_converter,
            req.request.shipping_cost.unwrap_or(MinorUnit::zero()),
            req.request.currency,
        )?;
        let connector_router_data =
            paypal::PaypalRouterData::try_from((amount, Some(shipping_cost), None, None, req))?;
        let connector_req = paypal::PaypalPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let payment_method_data = req.request.payment_method_data.clone();
        let req = match payment_method_data {
            PaymentMethodData::Wallet(WalletData::PaypalSdk(_)) => RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                .build(),
            _ => RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                .set_body(PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        };

        Ok(Some(req))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: PaypalAuthResponse =
            res.response
                .parse_struct("paypal PaypalAuthResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        match response {
            PaypalAuthResponse::PaypalOrdersResponse(response) => {
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);

                RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            PaypalAuthResponse::PaypalRedirectResponse(response) => {
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            PaypalAuthResponse::PaypalThreeDsResponse(response) => {
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.get_order_error_response(res, event_builder)
    }
}

impl api::PaymentsPreProcessing for Paypal {}

impl ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
    for Paypal
{
    fn get_headers(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let order_id = req
            .request
            .connector_transaction_id
            .to_owned()
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}v2/checkout/orders/{}?fields=payment_source",
            self.base_url(connectors),
            order_id,
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&PaymentsPreProcessingType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsPreProcessingType::get_headers(
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
        let response: paypal::PaypalPreProcessingResponse = res
            .response
            .parse_struct("paypal PaypalPreProcessingResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        match response {
            // if card supports 3DS check for liability
            paypal::PaypalPreProcessingResponse::PaypalLiabilityResponse(liability_response) => {
                // permutation for status to continue payment
                match (
                    liability_response
                        .payment_source
                        .card
                        .authentication_result
                        .three_d_secure
                        .enrollment_status
                        .as_ref(),
                    liability_response
                        .payment_source
                        .card
                        .authentication_result
                        .three_d_secure
                        .authentication_status
                        .as_ref(),
                    liability_response
                        .payment_source
                        .card
                        .authentication_result
                        .liability_shift
                        .clone(),
                ) {
                    (
                        Some(paypal::EnrollmentStatus::Ready),
                        Some(paypal::AuthenticationStatus::Success),
                        paypal::LiabilityShift::Possible,
                    )
                    | (
                        Some(paypal::EnrollmentStatus::Ready),
                        Some(paypal::AuthenticationStatus::Attempted),
                        paypal::LiabilityShift::Possible,
                    )
                    | (Some(paypal::EnrollmentStatus::NotReady), None, paypal::LiabilityShift::No)
                    | (Some(paypal::EnrollmentStatus::Unavailable), None, paypal::LiabilityShift::No)
                    | (Some(paypal::EnrollmentStatus::Bypassed), None, paypal::LiabilityShift::No) => {
                        Ok(PaymentsPreProcessingRouterData {
                            status: enums::AttemptStatus::AuthenticationSuccessful,
                            response: Ok(PaymentsResponseData::TransactionResponse {
                                resource_id: ResponseId::NoResponseId,
                                redirection_data: Box::new(None),
                                mandate_reference: Box::new(None),
                                connector_metadata: None,
                                network_txn_id: None,
                                connector_response_reference_id: None,
                                incremental_authorization_allowed: None,
                                charges: None,
                            }),
                            ..data.clone()
                        })
                    }
                    _ => Ok(PaymentsPreProcessingRouterData {
                        response: Err(ErrorResponse {
                            attempt_status: Some(enums::AttemptStatus::Failure),
                            code: NO_ERROR_CODE.to_string(),
                            message: NO_ERROR_MESSAGE.to_string(),
                            connector_transaction_id: None,
                            reason: Some(format!("{} Connector Responsded with LiabilityShift: {:?}, EnrollmentStatus: {:?}, and AuthenticationStatus: {:?}",
                            constants::CANNOT_CONTINUE_AUTH,
                            liability_response
                                .payment_source
                                .card
                                .authentication_result
                                .liability_shift,
                            liability_response
                                .payment_source
                                .card
                                .authentication_result
                                .three_d_secure
                                .enrollment_status
                                .unwrap_or(paypal::EnrollmentStatus::Null),
                            liability_response
                                .payment_source
                                .card
                                .authentication_result
                                .three_d_secure
                                .authentication_status
                                .unwrap_or(paypal::AuthenticationStatus::Null),
                            )),
                            status_code: res.status_code,
                            issuer_error_code: None,
                            issuer_error_message: None,
                        }),
                        ..data.clone()
                    }),
                }
            }
            // if card does not supports 3DS check for liability
            paypal::PaypalPreProcessingResponse::PaypalNonLiabilityResponse(_) => {
                Ok(PaymentsPreProcessingRouterData {
                    status: enums::AttemptStatus::AuthenticationSuccessful,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::NoResponseId,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..data.clone()
                })
            }
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

impl ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>
    for Paypal
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
        req: &PaymentsCompleteAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let complete_authorize_url = if req.request.is_auto_capture()? {
            "capture".to_string()
        } else {
            "authorize".to_string()
        };
        Ok(format!(
            "{}v2/checkout/orders/{}/{complete_authorize_url}",
            self.base_url(connectors),
            req.request
                .connector_transaction_id
                .clone()
                .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?
        ))
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
                .headers(PaymentsCompleteAuthorizeType::get_headers(
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
        let response: paypal::PaypalOrdersResponse = res
            .response
            .parse_struct("paypal PaypalOrdersResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Paypal {
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
        let paypal_meta: PaypalMeta = to_connector_meta(req.request.connector_meta.clone())?;
        match req.payment_method {
            enums::PaymentMethod::Wallet | enums::PaymentMethod::BankRedirect => Ok(format!(
                "{}v2/checkout/orders/{}",
                self.base_url(connectors),
                req.request
                    .connector_transaction_id
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
            )),
            _ => {
                let psync_url = match paypal_meta.psync_flow {
                    transformers::PaypalPaymentIntent::Authorize => {
                        let authorize_id = paypal_meta.authorize_id.ok_or(
                            errors::ConnectorError::RequestEncodingFailedWithReason(
                                "Missing Authorize id".to_string(),
                            ),
                        )?;
                        format!("v2/payments/authorizations/{authorize_id}",)
                    }
                    transformers::PaypalPaymentIntent::Capture => {
                        let capture_id = paypal_meta.capture_id.ok_or(
                            errors::ConnectorError::RequestEncodingFailedWithReason(
                                "Missing Capture id".to_string(),
                            ),
                        )?;
                        format!("v2/payments/captures/{capture_id}")
                    }
                    // only set when payment is done through card 3DS
                    //because no authorize or capture id is generated during payment authorize call for card 3DS
                    transformers::PaypalPaymentIntent::Authenticate => {
                        format!(
                            "v2/checkout/orders/{}",
                            req.request
                                .connector_transaction_id
                                .get_connector_transaction_id()
                                .change_context(
                                    errors::ConnectorError::MissingConnectorTransactionID
                                )?
                        )
                    }
                };
                Ok(format!("{}{psync_url}", self.base_url(connectors)))
            }
        }
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
        let response: paypal::PaypalSyncResponse = res
            .response
            .parse_struct("paypal SyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::foreign_try_from((
            ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            data.request.payment_experience,
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Paypal {
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
        let paypal_meta: PaypalMeta = to_connector_meta(req.request.connector_meta.clone())?;
        let authorize_id = paypal_meta.authorize_id.ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Missing Authorize id".to_string(),
            ),
        )?;
        Ok(format!(
            "{}v2/payments/authorizations/{}/capture",
            self.base_url(connectors),
            authorize_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount_to_capture = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let connector_router_data =
            paypal::PaypalRouterData::try_from((amount_to_capture, None, None, None, req))?;
        let connector_req = paypal::PaypalPaymentsCaptureRequest::try_from(&connector_router_data)?;
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
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: paypal::PaypalCaptureResponse = res
            .response
            .parse_struct("Paypal PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Paypal {
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
        let paypal_meta: PaypalMeta = to_connector_meta(req.request.connector_meta.clone())?;
        let authorize_id = paypal_meta.authorize_id.ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Missing Authorize id".to_string(),
            ),
        )?;
        Ok(format!(
            "{}v2/payments/authorizations/{}/void",
            self.base_url(connectors),
            authorize_id,
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PaymentsVoidType::get_url(self, req, connectors)?)
            .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: paypal::PaypalPaymentsCancelResponse = res
            .response
            .parse_struct("PaymentCancelResponse")
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Paypal {
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
        let paypal_meta: PaypalMeta = to_connector_meta(req.request.connector_metadata.clone())?;
        let capture_id = paypal_meta.capture_id.ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Missing Capture id".to_string(),
            ),
        )?;
        Ok(format!(
            "{}v2/payments/captures/{}/refund",
            self.base_url(connectors),
            capture_id,
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let connector_router_data =
            paypal::PaypalRouterData::try_from((amount, None, None, None, req))?;
        let connector_req = paypal::PaypalRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&RefundExecuteType::get_url(self, req, connectors)?)
            .headers(RefundExecuteType::get_headers(self, req, connectors)?)
            .set_body(RefundExecuteType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: paypal::RefundResponse =
            res.response
                .parse_struct("paypal RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Paypal {
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
        Ok(format!(
            "{}v2/payments/refunds/{}",
            self.base_url(connectors),
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
                .url(&RefundSyncType::get_url(self, req, connectors)?)
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
        let response: paypal::RefundSyncResponse = res
            .response
            .parse_struct("paypal RefundSyncResponse")
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
        VerifyWebhookSource,
        VerifyWebhookSourceRequestData,
        VerifyWebhookSourceResponseData,
    > for Paypal
{
    fn get_headers(
        &self,
        req: &RouterData<
            VerifyWebhookSource,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = paypal::PaypalAuthType::try_from(&req.connector_auth_type)?;
        let credentials = auth.get_credentials()?;
        let auth_val = credentials.generate_authorization_value();

        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                VerifyWebhookSourceType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (headers::AUTHORIZATION.to_string(), auth_val.into_masked()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<
            VerifyWebhookSource,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v1/notifications/verify-webhook-signature",
            self.base_url(connectors)
        ))
    }

    fn build_request(
        &self,
        req: &VerifyWebhookSourceRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&VerifyWebhookSourceType::get_url(self, req, connectors)?)
            .headers(VerifyWebhookSourceType::get_headers(self, req, connectors)?)
            .set_body(VerifyWebhookSourceType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn get_request_body(
        &self,
        req: &RouterData<
            VerifyWebhookSource,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = paypal::PaypalSourceVerificationRequest::try_from(&req.request)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &VerifyWebhookSourceRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<VerifyWebhookSourceRouterData, errors::ConnectorError> {
        let response: paypal::PaypalSourceVerificationResponse = res
            .response
            .parse_struct("paypal PaypalSourceVerificationResponse")
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
impl IncomingWebhook for Paypal {
    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let payload: paypal::PaypalWebhooksBody =
            request
                .body
                .parse_struct("PaypalWebhooksBody")
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        match payload.resource {
            paypal::PaypalResource::PaypalCardWebhooks(resource) => {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        resource.supplementary_data.related_ids.order_id,
                    ),
                ))
            }
            paypal::PaypalResource::PaypalRedirectsWebhooks(resource) => {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::PaymentAttemptId(
                        resource
                            .purchase_units
                            .first()
                            .and_then(|unit| unit.invoice_id.clone().or(unit.reference_id.clone()))
                            .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                    ),
                ))
            }
            paypal::PaypalResource::PaypalRefundWebhooks(resource) => {
                Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                    api_models::webhooks::RefundIdType::ConnectorRefundId(resource.id),
                ))
            }
            paypal::PaypalResource::PaypalDisputeWebhooks(resource) => {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        resource
                            .disputed_transactions
                            .first()
                            .map(|transaction| transaction.seller_transaction_id.clone())
                            .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                    ),
                ))
            }
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let payload: paypal::PaypalWebooksEventType = request
            .body
            .parse_struct("PaypalWebooksEventType")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let outcome = match payload.event_type {
            PaypalWebhookEventType::CustomerDisputeResolved => Some(
                request
                    .body
                    .parse_struct::<paypal::DisputeOutcome>("PaypalWebooksEventType")
                    .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?
                    .outcome_code,
            ),
            PaypalWebhookEventType::CustomerDisputeCreated
            | PaypalWebhookEventType::RiskDisputeCreated
            | PaypalWebhookEventType::CustomerDisputedUpdated
            | PaypalWebhookEventType::PaymentAuthorizationCreated
            | PaypalWebhookEventType::PaymentAuthorizationVoided
            | PaypalWebhookEventType::PaymentCaptureDeclined
            | PaypalWebhookEventType::PaymentCaptureCompleted
            | PaypalWebhookEventType::PaymentCapturePending
            | PaypalWebhookEventType::PaymentCaptureRefunded
            | PaypalWebhookEventType::CheckoutOrderApproved
            | PaypalWebhookEventType::CheckoutOrderCompleted
            | PaypalWebhookEventType::CheckoutOrderProcessed
            | PaypalWebhookEventType::Unknown => None,
        };

        Ok(transformers::get_payapl_webhooks_event(
            payload.event_type,
            outcome,
        ))
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let details: paypal::PaypalWebhooksBody =
            request
                .body
                .parse_struct("PaypalWebhooksBody")
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(match details.resource {
            paypal::PaypalResource::PaypalCardWebhooks(resource) => Box::new(
                paypal::PaypalPaymentsSyncResponse::try_from((*resource, details.event_type))?,
            ),
            paypal::PaypalResource::PaypalRedirectsWebhooks(resource) => Box::new(
                paypal::PaypalOrdersResponse::try_from((*resource, details.event_type))?,
            ),
            paypal::PaypalResource::PaypalRefundWebhooks(resource) => Box::new(
                paypal::RefundSyncResponse::try_from((*resource, details.event_type))?,
            ),
            paypal::PaypalResource::PaypalDisputeWebhooks(_) => Box::new(details),
        })
    }

    fn get_dispute_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<disputes::DisputePayload, errors::ConnectorError> {
        let webhook_payload: paypal::PaypalWebhooksBody = request
            .body
            .parse_struct("PaypalWebhooksBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        match webhook_payload.resource {
            transformers::PaypalResource::PaypalCardWebhooks(_)
            | transformers::PaypalResource::PaypalRedirectsWebhooks(_)
            | transformers::PaypalResource::PaypalRefundWebhooks(_) => {
                Err(errors::ConnectorError::ResponseDeserializationFailed)
                    .attach_printable("Expected Dispute webhooks,but found other webhooks")?
            }
            transformers::PaypalResource::PaypalDisputeWebhooks(payload) => {
                Ok(disputes::DisputePayload {
                    amount: connector_utils::to_currency_lower_unit(
                        payload.dispute_amount.value.get_amount_as_string(),
                        payload.dispute_amount.currency_code,
                    )?,
                    currency: payload.dispute_amount.currency_code,
                    dispute_stage: api_models::enums::DisputeStage::from(
                        payload.dispute_life_cycle_stage.clone(),
                    ),
                    connector_status: payload.status.to_string(),
                    connector_dispute_id: payload.dispute_id,
                    connector_reason: payload.reason.clone(),
                    connector_reason_code: payload.reason,
                    challenge_required_by: None,
                    created_at: payload.create_time,
                    updated_at: payload.update_time,
                })
            }
        }
    }
}

impl ConnectorRedirectResponse for Paypal {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: PaymentAction,
    ) -> CustomResult<CallConnectorAction, errors::ConnectorError> {
        match action {
            PaymentAction::PSync
            | PaymentAction::CompleteAuthorize
            | PaymentAction::PaymentAuthenticateCompleteAuthorize => {
                Ok(CallConnectorAction::Trigger)
            }
        }
    }
}

impl ConnectorErrorTypeMapping for Paypal {
    fn get_connector_error_type(
        &self,
        error_code: String,
        _error_message: String,
    ) -> ConnectorErrorType {
        match error_code.as_str() {
            "CANNOT_BE_NEGATIVE" => ConnectorErrorType::UserError,
            "CANNOT_BE_ZERO_OR_NEGATIVE" => ConnectorErrorType::UserError,
            "CARD_EXPIRED" => ConnectorErrorType::UserError,
            "DECIMAL_PRECISION" => ConnectorErrorType::UserError,
            "DUPLICATE_INVOICE_ID" => ConnectorErrorType::UserError,
            "INSTRUMENT_DECLINED" => ConnectorErrorType::BusinessError,
            "INTERNAL_SERVER_ERROR" => ConnectorErrorType::TechnicalError,
            "INVALID_ACCOUNT_STATUS" => ConnectorErrorType::BusinessError,
            "INVALID_CURRENCY_CODE" => ConnectorErrorType::UserError,
            "INVALID_PARAMETER_SYNTAX" => ConnectorErrorType::UserError,
            "INVALID_PARAMETER_VALUE" => ConnectorErrorType::UserError,
            "INVALID_RESOURCE_ID" => ConnectorErrorType::UserError,
            "INVALID_STRING_LENGTH" => ConnectorErrorType::UserError,
            "MISSING_REQUIRED_PARAMETER" => ConnectorErrorType::UserError,
            "PAYER_ACCOUNT_LOCKED_OR_CLOSED" => ConnectorErrorType::BusinessError,
            "PAYER_ACCOUNT_RESTRICTED" => ConnectorErrorType::BusinessError,
            "PAYER_CANNOT_PAY" => ConnectorErrorType::BusinessError,
            "PERMISSION_DENIED" => ConnectorErrorType::BusinessError,
            "INVALID_ARRAY_MAX_ITEMS" => ConnectorErrorType::UserError,
            "INVALID_ARRAY_MIN_ITEMS" => ConnectorErrorType::UserError,
            "INVALID_COUNTRY_CODE" => ConnectorErrorType::UserError,
            "NOT_SUPPORTED" => ConnectorErrorType::BusinessError,
            "PAYPAL_REQUEST_ID_REQUIRED" => ConnectorErrorType::UserError,
            "MALFORMED_REQUEST_JSON" => ConnectorErrorType::UserError,
            "PERMISSION_DENIED_FOR_DONATION_ITEMS" => ConnectorErrorType::BusinessError,
            "MALFORMED_REQUEST" => ConnectorErrorType::TechnicalError,
            "AMOUNT_MISMATCH" => ConnectorErrorType::UserError,
            "BILLING_ADDRESS_INVALID" => ConnectorErrorType::UserError,
            "CITY_REQUIRED" => ConnectorErrorType::UserError,
            "DONATION_ITEMS_NOT_SUPPORTED" => ConnectorErrorType::BusinessError,
            "DUPLICATE_REFERENCE_ID" => ConnectorErrorType::UserError,
            "INVALID_PAYER_ID" => ConnectorErrorType::UserError,
            "ITEM_TOTAL_REQUIRED" => ConnectorErrorType::UserError,
            "MAX_VALUE_EXCEEDED" => ConnectorErrorType::UserError,
            "MISSING_PICKUP_ADDRESS" => ConnectorErrorType::UserError,
            "MULTI_CURRENCY_ORDER" => ConnectorErrorType::BusinessError,
            "MULTIPLE_ITEM_CATEGORIES" => ConnectorErrorType::UserError,
            "MULTIPLE_SHIPPING_ADDRESS_NOT_SUPPORTED" => ConnectorErrorType::UserError,
            "MULTIPLE_SHIPPING_TYPE_NOT_SUPPORTED" => ConnectorErrorType::BusinessError,
            "PAYEE_ACCOUNT_INVALID" => ConnectorErrorType::UserError,
            "PAYEE_ACCOUNT_LOCKED_OR_CLOSED" => ConnectorErrorType::UserError,
            "REFERENCE_ID_REQUIRED" => ConnectorErrorType::UserError,
            "PAYMENT_SOURCE_CANNOT_BE_USED" => ConnectorErrorType::BusinessError,
            "PAYMENT_SOURCE_DECLINED_BY_PROCESSOR" => ConnectorErrorType::BusinessError,
            "PAYMENT_SOURCE_INFO_CANNOT_BE_VERIFIED" => ConnectorErrorType::BusinessError,
            "POSTAL_CODE_REQUIRED" => ConnectorErrorType::UserError,
            "SHIPPING_ADDRESS_INVALID" => ConnectorErrorType::UserError,
            "TAX_TOTAL_MISMATCH" => ConnectorErrorType::UserError,
            "TAX_TOTAL_REQUIRED" => ConnectorErrorType::UserError,
            "UNSUPPORTED_INTENT" => ConnectorErrorType::BusinessError,
            "UNSUPPORTED_PAYMENT_INSTRUCTION" => ConnectorErrorType::UserError,
            "SHIPPING_TYPE_NOT_SUPPORTED_FOR_CLIENT" => ConnectorErrorType::BusinessError,
            "UNSUPPORTED_SHIPPING_TYPE" => ConnectorErrorType::BusinessError,
            "PREFERRED_SHIPPING_OPTION_AMOUNT_MISMATCH" => ConnectorErrorType::UserError,
            "CARD_CLOSED" => ConnectorErrorType::BusinessError,
            "ORDER_CANNOT_BE_SAVED" => ConnectorErrorType::BusinessError,
            "SAVE_ORDER_NOT_SUPPORTED" => ConnectorErrorType::BusinessError,
            "FIELD_NOT_PATCHABLE" => ConnectorErrorType::UserError,
            "AMOUNT_NOT_PATCHABLE" => ConnectorErrorType::UserError,
            "INVALID_PATCH_OPERATION" => ConnectorErrorType::UserError,
            "PAYEE_ACCOUNT_NOT_SUPPORTED" => ConnectorErrorType::UserError,
            "PAYEE_ACCOUNT_NOT_VERIFIED" => ConnectorErrorType::UserError,
            "PAYEE_NOT_CONSENTED" => ConnectorErrorType::UserError,
            "INVALID_JSON_POINTER_FORMAT" => ConnectorErrorType::BusinessError,
            "INVALID_PARAMETER" => ConnectorErrorType::UserError,
            "NOT_PATCHABLE" => ConnectorErrorType::BusinessError,
            "PATCH_VALUE_REQUIRED" => ConnectorErrorType::UserError,
            "PATCH_PATH_REQUIRED" => ConnectorErrorType::UserError,
            "REFERENCE_ID_NOT_FOUND" => ConnectorErrorType::UserError,
            "SHIPPING_OPTION_NOT_SELECTED" => ConnectorErrorType::UserError,
            "SHIPPING_OPTIONS_NOT_SUPPORTED" => ConnectorErrorType::BusinessError,
            "MULTIPLE_SHIPPING_OPTION_SELECTED" => ConnectorErrorType::UserError,
            "ORDER_ALREADY_COMPLETED" => ConnectorErrorType::BusinessError,
            "ACTION_DOES_NOT_MATCH_INTENT" => ConnectorErrorType::BusinessError,
            "AGREEMENT_ALREADY_CANCELLED" => ConnectorErrorType::BusinessError,
            "BILLING_AGREEMENT_NOT_FOUND" => ConnectorErrorType::BusinessError,
            "DOMESTIC_TRANSACTION_REQUIRED" => ConnectorErrorType::BusinessError,
            "ORDER_NOT_APPROVED" => ConnectorErrorType::UserError,
            "MAX_NUMBER_OF_PAYMENT_ATTEMPTS_EXCEEDED" => ConnectorErrorType::TechnicalError,
            "PAYEE_BLOCKED_TRANSACTION" => ConnectorErrorType::BusinessError,
            "TRANSACTION_LIMIT_EXCEEDED" => ConnectorErrorType::UserError,
            "TRANSACTION_RECEIVING_LIMIT_EXCEEDED" => ConnectorErrorType::BusinessError,
            "TRANSACTION_REFUSED" => ConnectorErrorType::TechnicalError,
            "ORDER_ALREADY_AUTHORIZED" => ConnectorErrorType::BusinessError,
            "AUTH_CAPTURE_NOT_ENABLED" => ConnectorErrorType::BusinessError,
            "AMOUNT_CANNOT_BE_SPECIFIED" => ConnectorErrorType::BusinessError,
            "AUTHORIZATION_AMOUNT_EXCEEDED" => ConnectorErrorType::UserError,
            "AUTHORIZATION_CURRENCY_MISMATCH" => ConnectorErrorType::UserError,
            "MAX_AUTHORIZATION_COUNT_EXCEEDED" => ConnectorErrorType::BusinessError,
            "ORDER_COMPLETED_OR_VOIDED" => ConnectorErrorType::BusinessError,
            "ORDER_EXPIRED" => ConnectorErrorType::BusinessError,
            "INVALID_PICKUP_ADDRESS" => ConnectorErrorType::UserError,
            "CONSENT_NEEDED" => ConnectorErrorType::UserError,
            "COMPLIANCE_VIOLATION" => ConnectorErrorType::BusinessError,
            "REDIRECT_PAYER_FOR_ALTERNATE_FUNDING" => ConnectorErrorType::TechnicalError,
            "ORDER_ALREADY_CAPTURED" => ConnectorErrorType::UserError,
            "TRANSACTION_BLOCKED_BY_PAYEE" => ConnectorErrorType::BusinessError,
            "NOT_ENABLED_FOR_CARD_PROCESSING" => ConnectorErrorType::BusinessError,
            "PAYEE_NOT_ENABLED_FOR_CARD_PROCESSING" => ConnectorErrorType::BusinessError,
            _ => ConnectorErrorType::UnknownError,
        }
    }
}

impl ConnectorSpecifications for Paypal {}
