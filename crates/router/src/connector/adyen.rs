pub mod transformers;
use api_models::{enums::PaymentMethodType, webhooks::IncomingWebhookEvent};
use base64::Engine;
use common_utils::{
    request::RequestContent,
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use diesel_models::{enums as storage_enums, enums};
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::webhooks::IncomingWebhookFlowError;
use masking::{ExposeInterface, Secret};
use ring::hmac;
use router_env::{instrument, tracing};

use self::transformers as adyen;
use super::utils::is_mandate_supported;
#[cfg(feature = "payouts")]
use crate::connector::utils::PayoutsData;
use crate::{
    capture_method_not_supported,
    configs::settings,
    connector::utils::{convert_amount, PaymentMethodDataType},
    consts,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers, logger,
    services::{
        self,
        request::{self, Mask},
        ConnectorSpecifications, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon},
        domain,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::{crypto, ByteSliceExt, BytesExt, OptionExt},
};
const ADYEN_API_VERSION: &str = "v68";

#[derive(Clone)]
pub struct Adyen {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Adyen {
    pub const fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}
impl ConnectorCommon for Adyen {
    fn id(&self) -> &'static str {
        "adyen"
    }
    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }
    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = adyen::AdyenAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::X_API_KEY.to_string(),
            auth.api_key.into_masked(),
        )])
    }
    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.adyen.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: adyen::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.message.to_owned(),
            reason: Some(response.message),
            attempt_status: None,
            connector_transaction_id: response.psp_reference,
        })
    }
}

impl ConnectorValidation for Adyen {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<storage_enums::CaptureMethod>,
        _payment_method: enums::PaymentMethod,
        pmt: Option<PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        let connector = self.id();
        match pmt {
            Some(payment_method_type) => match payment_method_type {
                PaymentMethodType::Affirm
                | PaymentMethodType::AfterpayClearpay
                | PaymentMethodType::ApplePay
                | PaymentMethodType::Credit
                | PaymentMethodType::Debit
                | PaymentMethodType::GooglePay
                | PaymentMethodType::MobilePay
                | PaymentMethodType::PayBright
                | PaymentMethodType::Sepa
                | PaymentMethodType::Vipps
                | PaymentMethodType::Venmo
                | PaymentMethodType::Paypal => match capture_method {
                    enums::CaptureMethod::Automatic
                    | enums::CaptureMethod::SequentialAutomatic
                    | enums::CaptureMethod::Manual
                    | enums::CaptureMethod::ManualMultiple => Ok(()),
                    enums::CaptureMethod::Scheduled => {
                        capture_method_not_supported!(
                            connector,
                            capture_method,
                            payment_method_type
                        )
                    }
                },
                PaymentMethodType::Ach
                | PaymentMethodType::SamsungPay
                | PaymentMethodType::Paze
                | PaymentMethodType::Alma
                | PaymentMethodType::Bacs
                | PaymentMethodType::Givex
                | PaymentMethodType::Klarna
                | PaymentMethodType::Twint
                | PaymentMethodType::Walley => match capture_method {
                    enums::CaptureMethod::Automatic
                    | enums::CaptureMethod::Manual
                    | enums::CaptureMethod::SequentialAutomatic => Ok(()),
                    enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => {
                        capture_method_not_supported!(
                            connector,
                            capture_method,
                            payment_method_type
                        )
                    }
                },

                PaymentMethodType::AliPay
                | PaymentMethodType::AliPayHk
                | PaymentMethodType::Atome
                | PaymentMethodType::BancontactCard
                | PaymentMethodType::Benefit
                | PaymentMethodType::Bizum
                | PaymentMethodType::Blik
                | PaymentMethodType::Boleto
                | PaymentMethodType::Dana
                | PaymentMethodType::Eps
                | PaymentMethodType::OnlineBankingFpx
                | PaymentMethodType::Gcash
                | PaymentMethodType::GoPay
                | PaymentMethodType::Ideal
                | PaymentMethodType::KakaoPay
                | PaymentMethodType::Knet
                | PaymentMethodType::MbWay
                | PaymentMethodType::Momo
                | PaymentMethodType::MomoAtm
                | PaymentMethodType::OnlineBankingFinland
                | PaymentMethodType::OnlineBankingPoland
                | PaymentMethodType::OnlineBankingSlovakia
                | PaymentMethodType::OnlineBankingThailand
                | PaymentMethodType::Oxxo
                | PaymentMethodType::PaySafeCard
                | PaymentMethodType::Pix
                | PaymentMethodType::Swish
                | PaymentMethodType::TouchNGo
                | PaymentMethodType::Trustly
                | PaymentMethodType::WeChatPay
                | PaymentMethodType::DanamonVa
                | PaymentMethodType::BcaBankTransfer
                | PaymentMethodType::BriVa
                | PaymentMethodType::BniVa
                | PaymentMethodType::CimbVa
                | PaymentMethodType::MandiriVa
                | PaymentMethodType::Alfamart
                | PaymentMethodType::Indomaret
                | PaymentMethodType::FamilyMart
                | PaymentMethodType::Seicomart
                | PaymentMethodType::PayEasy
                | PaymentMethodType::MiniStop
                | PaymentMethodType::Lawson
                | PaymentMethodType::SevenEleven
                | PaymentMethodType::OpenBankingUk
                | PaymentMethodType::OnlineBankingCzechRepublic
                | PaymentMethodType::PermataBankTransfer => match capture_method {
                    enums::CaptureMethod::Automatic | enums::CaptureMethod::SequentialAutomatic => {
                        Ok(())
                    }
                    enums::CaptureMethod::Manual
                    | enums::CaptureMethod::ManualMultiple
                    | enums::CaptureMethod::Scheduled => {
                        capture_method_not_supported!(
                            connector,
                            capture_method,
                            payment_method_type
                        )
                    }
                },
                PaymentMethodType::AmazonPay
                | PaymentMethodType::CardRedirect
                | PaymentMethodType::DirectCarrierBilling
                | PaymentMethodType::Fps
                | PaymentMethodType::DuitNow
                | PaymentMethodType::Interac
                | PaymentMethodType::Multibanco
                | PaymentMethodType::Przelewy24
                | PaymentMethodType::Becs
                | PaymentMethodType::ClassicReward
                | PaymentMethodType::Pse
                | PaymentMethodType::LocalBankTransfer
                | PaymentMethodType::Efecty
                | PaymentMethodType::Giropay
                | PaymentMethodType::PagoEfectivo
                | PaymentMethodType::PromptPay
                | PaymentMethodType::RedCompra
                | PaymentMethodType::RedPagos
                | PaymentMethodType::Sofort
                | PaymentMethodType::CryptoCurrency
                | PaymentMethodType::Evoucher
                | PaymentMethodType::Cashapp
                | PaymentMethodType::UpiCollect
                | PaymentMethodType::UpiIntent
                | PaymentMethodType::VietQr
                | PaymentMethodType::Mifinity
                | PaymentMethodType::LocalBankRedirect
                | PaymentMethodType::OpenBankingPIS => {
                    capture_method_not_supported!(connector, capture_method, payment_method_type)
                }
            },
            None => match capture_method {
                enums::CaptureMethod::Automatic
                | enums::CaptureMethod::SequentialAutomatic
                | enums::CaptureMethod::Manual
                | enums::CaptureMethod::ManualMultiple => Ok(()),
                enums::CaptureMethod::Scheduled => {
                    capture_method_not_supported!(connector, capture_method)
                }
            },
        }
    }
    fn validate_mandate_payment(
        &self,
        pm_type: Option<PaymentMethodType>,
        pm_data: domain::payments::PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
            PaymentMethodDataType::ApplePay,
            PaymentMethodDataType::GooglePay,
            PaymentMethodDataType::PaypalRedirect,
            PaymentMethodDataType::MomoRedirect,
            PaymentMethodDataType::KakaoPayRedirect,
            PaymentMethodDataType::GoPayRedirect,
            PaymentMethodDataType::GcashRedirect,
            PaymentMethodDataType::DanaRedirect,
            PaymentMethodDataType::TwintRedirect,
            PaymentMethodDataType::VippsRedirect,
            PaymentMethodDataType::KlarnaRedirect,
            PaymentMethodDataType::Ideal,
            PaymentMethodDataType::OpenBankingUk,
            PaymentMethodDataType::Trustly,
            PaymentMethodDataType::BancontactCard,
            PaymentMethodDataType::AchBankDebit,
            PaymentMethodDataType::SepaBankDebit,
            PaymentMethodDataType::BecsBankDebit,
        ]);
        is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }

    fn validate_psync_reference_id(
        &self,
        data: &hyperswitch_domain_models::router_request_types::PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        if data.encoded_data.is_some() {
            return Ok(());
        }
        Err(errors::ConnectorError::MissingRequiredField {
            field_name: "encoded_data",
        }
        .into())
    }
    fn is_webhook_source_verification_mandatory(&self) -> bool {
        true
    }
}

impl api::Payment for Adyen {}
impl api::PaymentAuthorize for Adyen {}
impl api::PaymentSync for Adyen {}
impl api::PaymentVoid for Adyen {}
impl api::PaymentCapture for Adyen {}
impl api::MandateSetup for Adyen {}
impl api::ConnectorAccessToken for Adyen {}
impl api::PaymentToken for Adyen {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Adyen
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Adyen
{
    // Not Implemented (R)
}

fn build_env_specific_endpoint(
    base_url: &str,
    test_mode: Option<bool>,
    connector_metadata: &Option<common_utils::pii::SecretSerdeValue>,
) -> CustomResult<String, errors::ConnectorError> {
    if test_mode.unwrap_or(true) {
        Ok(base_url.to_string())
    } else {
        let adyen_connector_metadata_object =
            transformers::AdyenConnectorMetadataObject::try_from(connector_metadata)?;
        let endpoint_prefix = adyen_connector_metadata_object.endpoint_prefix.ok_or(
            errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata.endpoint_prefix",
            },
        )?;
        Ok(base_url.replace("{{merchant_endpoint_prefix}}", &endpoint_prefix))
    }
}

impl
    services::ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::SetupMandateRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::SetupMandateType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!("{}{}/payments", endpoint, ADYEN_API_VERSION))
    }
    fn get_request_body(
        &self,
        req: &types::SetupMandateRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let authorize_req = types::PaymentsAuthorizeRouterData::foreign_from((
            req,
            types::PaymentsAuthorizeData::foreign_from(req),
        ));

        let amount = convert_amount(
            self.amount_converter,
            authorize_req.request.minor_amount,
            authorize_req.request.currency,
        )?;

        let connector_router_data = adyen::AdyenRouterData::try_from((amount, &authorize_req))?;
        let connector_req = adyen::AdyenPaymentRequest::try_from(&connector_router_data)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::SetupMandateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::SetupMandateType::get_headers(self, req, connectors)?)
                .set_body(types::SetupMandateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        errors::ConnectorError,
    >
    where
        api::SetupMandate: Clone,
        types::SetupMandateRequestData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: adyen::AdyenPaymentResponse = res
            .response
            .parse_struct("AdyenPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::foreign_try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            None,
            false,
            data.request.payment_method_type,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::PaymentSession for Adyen {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Adyen
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.as_str();

        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}{}/payments/{}/captures",
            endpoint, ADYEN_API_VERSION, id
        ))
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount_to_capture = convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;

        let connector_router_data = adyen::AdyenRouterData::try_from((amount_to_capture, req))?;
        let connector_req = adyen::AdyenCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: adyen::AdyenCaptureResponse = res
            .response
            .parse_struct("AdyenCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

/// Payment Sync can be useful only incase of Redirect flow.
/// For payments which doesn't involve redrection we have to rely on webhooks.
impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Adyen
{
    fn get_headers(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsSyncType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let encoded_data = req
            .request
            .encoded_data
            .clone()
            .get_required_value("encoded_data")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let adyen_redirection_type = serde_urlencoded::from_str::<
            transformers::AdyenRedirectRequestTypes,
        >(encoded_data.as_str())
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let connector_req = match adyen_redirection_type {
            adyen::AdyenRedirectRequestTypes::AdyenRedirection(req) => {
                adyen::AdyenRedirectRequest {
                    details: adyen::AdyenRedirectRequestTypes::AdyenRedirection(
                        adyen::AdyenRedirection {
                            redirect_result: req.redirect_result,
                            type_of_redirection_result: None,
                            result_code: None,
                        },
                    ),
                }
            }
            adyen::AdyenRedirectRequestTypes::AdyenThreeDS(req) => adyen::AdyenRedirectRequest {
                details: adyen::AdyenRedirectRequestTypes::AdyenThreeDS(adyen::AdyenThreeDS {
                    three_ds_result: req.three_ds_result,
                    type_of_redirection_result: None,
                    result_code: None,
                }),
            },
            adyen::AdyenRedirectRequestTypes::AdyenRefusal(req) => adyen::AdyenRedirectRequest {
                details: adyen::AdyenRedirectRequestTypes::AdyenRefusal(adyen::AdyenRefusal {
                    payload: req.payload,
                    type_of_redirection_result: None,
                    result_code: None,
                }),
            },
        };

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn get_url(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}{}/payments/details",
            endpoint, ADYEN_API_VERSION
        ))
    }

    fn build_request(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        // Adyen doesn't support PSync flow. We use PSync flow to fetch payment details,
        // specifically the redirect URL that takes the user to their Payment page. In non-redirection flows,
        // we rely on webhooks to obtain the payment status since there is no encoded data available.
        // encoded_data only includes the redirect URL and is only relevant in redirection flows.
        if req
            .request
            .encoded_data
            .clone()
            .get_required_value("encoded_data")
            .is_ok()
        {
            Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                    .set_body(types::PaymentsSyncType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            ))
        } else {
            Ok(None)
        }
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        logger::debug!(payment_sync_response=?res);
        let response: adyen::AdyenPaymentResponse = res
            .response
            .parse_struct("AdyenPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let is_multiple_capture_sync = match data.request.sync_type {
            types::SyncRequestType::MultipleCaptureSync(_) => true,
            types::SyncRequestType::SinglePaymentSync => false,
        };
        types::RouterData::foreign_try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            data.request.capture_method,
            is_multiple_capture_sync,
            data.request.payment_method_type,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<services::CaptureSyncMethod, errors::ConnectorError> {
        Ok(services::CaptureSyncMethod::Individual)
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError>
    where
        Self: services::ConnectorIntegration<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!("{}{}/payments", endpoint, ADYEN_API_VERSION))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_router_data = adyen::AdyenRouterData::try_from((amount, req))?;
        let connector_req = adyen::AdyenPaymentRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: adyen::AdyenPaymentResponse = res
            .response
            .parse_struct("AdyenPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::foreign_try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            data.request.capture_method,
            false,
            data.request.payment_method_type,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::PaymentsPreProcessing for Adyen {}

impl
    services::ConnectorIntegration<
        api::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsPreProcessingType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}{}/paymentMethods/balance",
            endpoint, ADYEN_API_VERSION
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenBalanceRequest::try_from(req)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsPreProcessingType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsPreProcessingType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsPreProcessingType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsPreProcessingRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsPreProcessingRouterData, errors::ConnectorError> {
        let response: adyen::AdyenBalanceResponse = res
            .response
            .parse_struct("AdyenBalanceResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let currency = match data.request.currency {
            Some(currency) => currency,
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "currency",
            })?,
        };
        let amount = match data.request.minor_amount {
            Some(amount) => amount,
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "amount",
            })?,
        };

        let amount = convert_amount(self.amount_converter, amount, currency)?;

        if response.balance.currency != currency || response.balance.value < amount {
            Ok(types::RouterData {
                response: Err(types::ErrorResponse {
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: consts::NO_ERROR_MESSAGE.to_string(),
                    reason: Some(consts::LOW_BALANCE_ERROR_MESSAGE.to_string()),
                    status_code: res.status_code,
                    attempt_status: Some(enums::AttemptStatus::Failure),
                    connector_transaction_id: Some(response.psp_reference),
                }),
                ..data.clone()
            })
        } else {
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.clone();

        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}{}/payments/{}/cancels",
            endpoint, ADYEN_API_VERSION, id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenCancelRequest::try_from(req)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
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
        data: &types::PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: adyen::AdyenCancelResponse = res
            .response
            .parse_struct("AdyenCancelResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::Payouts for Adyen {}
#[cfg(feature = "payouts")]
impl api::PayoutCancel for Adyen {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for Adyen {}
#[cfg(feature = "payouts")]
impl api::PayoutEligibility for Adyen {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Adyen {}

#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoCancel, types::PayoutsData, types::PayoutsResponseData>
    for Adyen
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.payout_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}pal/servlet/Payout/{}/declineThirdParty",
            endpoint, ADYEN_API_VERSION
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutCancelType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let auth = adyen::AdyenAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(
            headers::X_API_KEY.to_string(),
            auth.review_key.unwrap_or(auth.api_key).into_masked(),
        )];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenPayoutCancelRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
            .set_body(types::PayoutCancelType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoCancel>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoCancel>, errors::ConnectorError> {
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoCreate, types::PayoutsData, types::PayoutsResponseData>
    for Adyen
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.payout_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}pal/servlet/Payout/{}/storeDetailAndSubmitThirdParty",
            endpoint, ADYEN_API_VERSION
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutCreateType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.destination_currency,
        )?;
        let connector_router_data = adyen::AdyenRouterData::try_from((amount, req))?;
        let connector_req = adyen::AdyenPayoutCreateRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
            .set_body(types::PayoutCreateType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoCreate>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoCreate>, errors::ConnectorError> {
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl
    services::ConnectorIntegration<
        api::PoEligibility,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for Adyen
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoEligibility>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!("{}{}/payments", endpoint, ADYEN_API_VERSION))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoEligibility>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutEligibilityType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoEligibility>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.destination_currency,
        )?;

        let connector_router_data = adyen::AdyenRouterData::try_from((amount, req))?;
        let connector_req = adyen::AdyenPayoutEligibilityRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoEligibility>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutEligibilityType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::PayoutEligibilityType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PayoutEligibilityType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoEligibility>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoEligibility>, errors::ConnectorError> {
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for Adyen
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.payout_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        let payout_type = req.request.get_payout_type()?;
        Ok(format!(
            "{}pal/servlet/Payout/{}/{}",
            endpoint,
            ADYEN_API_VERSION,
            match payout_type {
                storage_enums::PayoutType::Bank | storage_enums::PayoutType::Wallet =>
                    "confirmThirdParty".to_string(),
                storage_enums::PayoutType::Card => "payout".to_string(),
            }
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutFulfillType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let auth = adyen::AdyenAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let payout_type = req
            .request
            .payout_type
            .to_owned()
            .get_required_value("payout_type")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "payout_type",
            })?;
        let mut api_key = vec![(
            headers::X_API_KEY.to_string(),
            match payout_type {
                storage_enums::PayoutType::Bank | storage_enums::PayoutType::Wallet => {
                    auth.review_key.unwrap_or(auth.api_key).into_masked()
                }
                storage_enums::PayoutType::Card => auth.api_key.into_masked(),
            },
        )];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.destination_currency,
        )?;

        let connector_router_data = adyen::AdyenRouterData::try_from((amount, req))?;
        let connector_req = adyen::AdyenPayoutFulfillRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
            .set_body(types::PayoutFulfillType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoFulfill>, errors::ConnectorError> {
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::Refund for Adyen {}
impl api::RefundExecute for Adyen {}
impl api::RefundSync for Adyen {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Adyen
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefundExecuteType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_header = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_header);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();

        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}{}/payments/{}/refunds",
            endpoint, ADYEN_API_VERSION, connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let connector_router_data = adyen::AdyenRouterData::try_from((refund_amount, req))?;
        let connector_req = adyen::AdyenRefundRequest::try_from(&connector_router_data)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::RefundExecuteType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: adyen::AdyenRefundResponse = res
            .response
            .parse_struct("AdyenRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Adyen
{
}

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<adyen::AdyenNotificationRequestItemWH, errors::ParsingError> {
    let mut webhook: adyen::AdyenIncomingWebhook = body.parse_struct("AdyenIncomingWebhook")?;

    let item_object = webhook
        .notification_items
        .drain(..)
        .next()
        // TODO: ParsingError doesn't seem to be an apt error for this case
        .ok_or(errors::ParsingError::UnknownError)?;

    Ok(item_object.notification_request_item)
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Adyen {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notif_item = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let base64_signature = notif_item.additional_data.hmac_signature.expose();
        Ok(base64_signature.as_bytes().to_vec())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let message = format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            notif.psp_reference,
            notif.original_reference.unwrap_or_default(),
            notif.merchant_account_code,
            notif.merchant_reference,
            notif.amount.value,
            notif.amount.currency,
            notif.event_code,
            notif.success
        );

        Ok(message.into_bytes())
    }

    async fn verify_webhook_source(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_label,
                connector_webhook_details,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let raw_key = hex::decode(connector_webhook_secrets.secret)
            .change_context(errors::ConnectorError::WebhookVerificationSecretInvalid)?;

        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &raw_key);
        let signed_messaged = hmac::sign(&signing_key, &message);
        let payload_sign = consts::BASE64_ENGINE.encode(signed_messaged.as_ref());
        Ok(payload_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        // for capture_event, original_reference field will have the authorized payment's PSP reference
        if adyen::is_capture_or_cancel_event(&notif.event_code) {
            return Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    notif
                        .original_reference
                        .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                ),
            ));
        }
        if adyen::is_transaction_event(&notif.event_code) {
            return Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(notif.merchant_reference),
            ));
        }
        if adyen::is_refund_event(&notif.event_code) {
            return Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::RefundId(notif.merchant_reference),
            ));
        }
        if adyen::is_chargeback_event(&notif.event_code) {
            return Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    notif
                        .original_reference
                        .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                ),
            ));
        }
        #[cfg(feature = "payouts")]
        if adyen::is_payout_event(&notif.event_code) {
            return Ok(api_models::webhooks::ObjectReferenceId::PayoutId(
                api_models::webhooks::PayoutIdType::PayoutAttemptId(notif.merchant_reference),
            ));
        }
        Err(report!(errors::ConnectorError::WebhookReferenceIdNotFound))
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(IncomingWebhookEvent::foreign_from((
            notif.event_code,
            notif.success,
            notif.additional_data.dispute_status,
        )))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        let response = adyen::AdyenWebhookResponse::from(notif);

        Ok(Box::new(response))
    }

    fn get_webhook_api_response(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
        _error_kind: Option<IncomingWebhookFlowError>,
    ) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        Ok(services::api::ApplicationResponse::TextPlain(
            "[accepted]".to_string(),
        ))
    }

    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(api::disputes::DisputePayload {
            amount: notif.amount.value.to_string(),
            currency: notif.amount.currency,
            dispute_stage: api_models::enums::DisputeStage::from(notif.event_code.clone()),
            connector_dispute_id: notif.psp_reference,
            connector_reason: notif.reason,
            connector_reason_code: notif.additional_data.chargeback_reason_code,
            challenge_required_by: notif.additional_data.defense_period_ends_at,
            connector_status: notif.event_code.to_string(),
            created_at: notif.event_date,
            updated_at: notif.event_date,
        })
    }

    fn get_mandate_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<hyperswitch_domain_models::router_flow_types::ConnectorMandateDetails>,
        errors::ConnectorError,
    > {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let mandate_reference =
            notif
                .additional_data
                .recurring_detail_reference
                .map(|mandate_id| {
                    hyperswitch_domain_models::router_flow_types::ConnectorMandateDetails {
                        connector_mandate_id: mandate_id.clone(),
                    }
                });
        Ok(mandate_reference)
    }

    fn get_network_txn_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<hyperswitch_domain_models::router_flow_types::ConnectorNetworkTxnId>,
        errors::ConnectorError,
    > {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let optional_network_txn_id =
            notif
                .additional_data
                .network_tx_reference
                .map(|network_txn_id| {
                    hyperswitch_domain_models::router_flow_types::ConnectorNetworkTxnId::new(
                        network_txn_id,
                    )
                });
        Ok(optional_network_txn_id)
    }
}

impl api::Dispute for Adyen {}
impl api::DefendDispute for Adyen {}
impl api::AcceptDispute for Adyen {}
impl api::SubmitEvidence for Adyen {}

impl
    services::ConnectorIntegration<
        api::Accept,
        types::AcceptDisputeRequestData,
        types::AcceptDisputeResponse,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::AcceptDisputeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::AcceptDisputeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::AcceptDisputeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.dispute_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}ca/services/DisputeService/v30/acceptDispute",
            endpoint
        ))
    }

    fn build_request(
        &self,
        req: &types::AcceptDisputeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::AcceptDisputeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::AcceptDisputeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::AcceptDisputeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
    fn get_request_body(
        &self,
        req: &types::AcceptDisputeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenAcceptDisputeRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &types::AcceptDisputeRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::AcceptDisputeRouterData, errors::ConnectorError> {
        let response: adyen::AdyenDisputeResponse = res
            .response
            .parse_struct("AdyenDisputeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::foreign_try_from((data, response))
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    services::ConnectorIntegration<
        api::Defend,
        types::DefendDisputeRequestData,
        types::DefendDisputeResponse,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::DefendDisputeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::DefendDisputeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::DefendDisputeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.dispute_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}ca/services/DisputeService/v30/defendDispute",
            endpoint
        ))
    }

    fn build_request(
        &self,
        req: &types::DefendDisputeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::DefendDisputeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::DefendDisputeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::DefendDisputeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::DefendDisputeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenDefendDisputeRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &types::DefendDisputeRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::DefendDisputeRouterData, errors::ConnectorError> {
        let response: adyen::AdyenDisputeResponse = res
            .response
            .parse_struct("AdyenDisputeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::foreign_try_from((data, response))
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    services::ConnectorIntegration<
        api::Evidence,
        types::SubmitEvidenceRequestData,
        types::SubmitEvidenceResponse,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::SubmitEvidenceRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::SubmitEvidenceType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::SubmitEvidenceRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.dispute_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{}ca/services/DisputeService/v30/supplyDefenseDocument",
            endpoint
        ))
    }

    fn get_request_body(
        &self,
        req: &types::SubmitEvidenceRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::Evidence::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::SubmitEvidenceRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::SubmitEvidenceType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::SubmitEvidenceType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::SubmitEvidenceType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::SubmitEvidenceRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::SubmitEvidenceRouterData, errors::ConnectorError> {
        let response: adyen::AdyenDisputeResponse = res
            .response
            .parse_struct("AdyenDisputeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::foreign_try_from((data, response))
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}
impl api::UploadFile for Adyen {}
impl api::RetrieveFile for Adyen {}
impl
    services::ConnectorIntegration<
        api::Retrieve,
        types::RetrieveFileRequestData,
        types::RetrieveFileResponse,
    > for Adyen
{
}
impl
    services::ConnectorIntegration<
        api::Upload,
        types::UploadFileRequestData,
        types::UploadFileResponse,
    > for Adyen
{
}
#[async_trait::async_trait]
impl api::FileUpload for Adyen {
    fn validate_file_upload(
        &self,
        purpose: api::FilePurpose,
        file_size: i32,
        file_type: mime::Mime,
    ) -> CustomResult<(), errors::ConnectorError> {
        match purpose {
            api::FilePurpose::DisputeEvidence => {
                let supported_file_types =
                    ["image/jpeg", "image/jpg", "image/png", "application/pdf"];
                if !supported_file_types.contains(&file_type.to_string().as_str()) {
                    Err(errors::ConnectorError::FileValidationFailed {
                        reason: "file_type does not match JPEG, JPG, PNG, or PDF format".to_owned(),
                    })?
                }
                //10 MB
                if (file_type.to_string().as_str() == "image/jpeg"
                    || file_type.to_string().as_str() == "image/jpg"
                    || file_type.to_string().as_str() == "image/png")
                    && file_size > 10000000
                {
                    Err(errors::ConnectorError::FileValidationFailed {
                        reason: "file_size exceeded the max file size of 10MB for Image formats"
                            .to_owned(),
                    })?
                }
                //2 MB
                if file_type.to_string().as_str() == "application/pdf" && file_size > 2000000 {
                    Err(errors::ConnectorError::FileValidationFailed {
                        reason: "file_size exceeded the max file size of 2MB for PDF formats"
                            .to_owned(),
                    })?
                }
            }
        }
        Ok(())
    }
}

impl ConnectorSpecifications for Adyen {}
