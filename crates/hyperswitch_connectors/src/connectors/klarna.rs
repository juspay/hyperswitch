pub mod transformers;

use api_models::webhooks::IncomingWebhookEvent;
use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts::BASE64_ENGINE,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{PayLaterData, PaymentMethodData},
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
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSessionRouterData, PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
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
    types::{self, Response},
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::{Mask, PeekInterface};
use router_env::logger;
use transformers as klarna;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{
        construct_not_supported_error_report, convert_amount, get_http_header,
        get_unimplemented_payment_method_error_message, missing_field_err, RefundsRequestData,
    },
};

#[derive(Clone)]
pub struct Klarna {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Klarna {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl ConnectorCommon for Klarna {
    fn id(&self) -> &'static str {
        "klarna"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.klarna.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = klarna::KlarnaAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let encoded_api_key =
            BASE64_ENGINE.encode(format!("{}:{}", auth.username.peek(), auth.password.peek()));
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Basic {encoded_api_key}").into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: klarna::KlarnaErrorResponse = res
            .response
            .parse_struct("KlarnaErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        // KlarnaErrorResponse will either have error_messages or error_message field Ref: https://docs.klarna.com/api/errors/
        let reason = response
            .error_messages
            .map(|messages| messages.join(" & "))
            .or(response.error_message.clone());
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response
                .error_message
                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
            reason,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Klarna {
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
                construct_not_supported_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl api::Payment for Klarna {}

impl api::PaymentAuthorize for Klarna {}
impl api::PaymentSync for Klarna {}
impl api::PaymentVoid for Klarna {}
impl api::PaymentCapture for Klarna {}
impl api::PaymentSession for Klarna {}
impl api::ConnectorAccessToken for Klarna {}
impl api::PaymentToken for Klarna {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Klarna
{
    // Not Implemented (R)
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Klarna {
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Klarna
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
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

fn build_region_specific_endpoint(
    base_url: &str,
    connector_metadata: &Option<common_utils::pii::SecretSerdeValue>,
) -> CustomResult<String, errors::ConnectorError> {
    let klarna_metadata_object =
        transformers::KlarnaConnectorMetadataObject::try_from(connector_metadata)?;
    let klarna_region = klarna_metadata_object
        .klarna_region
        .ok_or(errors::ConnectorError::InvalidConnectorConfig {
            config: "merchant_connector_account.metadata.klarna_region",
        })
        .map(String::from)?;

    Ok(base_url.replace("{{klarna_region}}", &klarna_region))
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Klarna {
    fn get_headers(
        &self,
        req: &PaymentsSessionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSessionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint =
            build_region_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        Ok(format!("{endpoint}payments/v1/sessions"))
    }

    fn get_request_body(
        &self,
        req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_router_data = klarna::KlarnaRouterData::from((amount, req));

        let connector_req = klarna::KlarnaSessionRequest::try_from(&connector_router_data)?;
        // encode only for for urlencoded things.
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsSessionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsSessionType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSessionType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsSessionType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSessionRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSessionRouterData, errors::ConnectorError> {
        let response: klarna::KlarnaSessionResponse = res
            .response
            .parse_struct("KlarnaSessionResponse")
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

impl api::MandateSetup for Klarna {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Klarna {
    // Not Implemented(R)
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Klarna".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Klarna {
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
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let order_id = req.request.connector_transaction_id.clone();
        let endpoint =
            build_region_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        Ok(format!(
            "{endpoint}ordermanagement/v1/orders/{order_id}/captures"
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
        let connector_router_data = klarna::KlarnaRouterData::from((amount, req));
        let connector_req = klarna::KlarnaCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsCaptureType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PaymentsCaptureType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        match res.headers {
            Some(headers) => {
                let capture_id = get_http_header("Capture-Id", &headers)
                    .attach_printable("Missing capture id in headers")
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
                let response = klarna::KlarnaCaptureResponse {
                    capture_id: Some(capture_id.to_owned()),
                };

                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
                .change_context(errors::ConnectorError::ResponseHandlingFailed)
            }
            None => Err(errors::ConnectorError::ResponseDeserializationFailed)
                .attach_printable("Expected headers, but received no headers in response")?,
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Klarna {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let order_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let endpoint =
            build_region_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        let payment_experience = req.request.payment_experience;

        match payment_experience {
            Some(common_enums::PaymentExperience::InvokeSdkClient) => {
                Ok(format!("{endpoint}ordermanagement/v1/orders/{order_id}"))
            }
            Some(common_enums::PaymentExperience::RedirectToUrl) => {
                Ok(format!("{endpoint}checkout/v3/orders/{order_id}"))
            }
            None => Err(error_stack::report!(errors::ConnectorError::NotSupported {
                message: "payment_experience not supported".to_string(),
                connector: "klarna",
            })),
            _ => Err(error_stack::report!(errors::ConnectorError::NotSupported {
                message: "payment_experience not supported".to_string(),
                connector: "klarna",
            })),
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
        let response: klarna::KlarnaPsyncResponse = res
            .response
            .parse_struct("klarna KlarnaPsyncResponse")
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

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Klarna {
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
        let payment_method_data = &req.request.payment_method_data;
        let payment_experience = req
            .request
            .payment_experience
            .as_ref()
            .ok_or_else(missing_field_err("payment_experience"))?;
        let payment_method_type = req
            .request
            .payment_method_type
            .as_ref()
            .ok_or_else(missing_field_err("payment_method_type"))?;
        let endpoint =
            build_region_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        match payment_method_data {
            PaymentMethodData::PayLater(PayLaterData::KlarnaSdk { token }) => {
                match (payment_experience, payment_method_type) {
                    (
                        common_enums::PaymentExperience::InvokeSdkClient,
                        common_enums::PaymentMethodType::Klarna,
                    ) => Ok(format!(
                        "{endpoint}payments/v1/authorizations/{token}/order",
                    )),
                    (
                        common_enums::PaymentExperience::DisplayQrCode
                        | common_enums::PaymentExperience::DisplayWaitScreen
                        | common_enums::PaymentExperience::InvokePaymentApp
                        | common_enums::PaymentExperience::InvokeSdkClient
                        | common_enums::PaymentExperience::LinkWallet
                        | common_enums::PaymentExperience::OneClick
                        | common_enums::PaymentExperience::RedirectToUrl
                        | common_enums::PaymentExperience::CollectOtp,
                        common_enums::PaymentMethodType::Ach
                        | common_enums::PaymentMethodType::Affirm
                        | common_enums::PaymentMethodType::AfterpayClearpay
                        | common_enums::PaymentMethodType::Alfamart
                        | common_enums::PaymentMethodType::AliPay
                        | common_enums::PaymentMethodType::AliPayHk
                        | common_enums::PaymentMethodType::Alma
                        | common_enums::PaymentMethodType::AmazonPay
                        | common_enums::PaymentMethodType::ApplePay
                        | common_enums::PaymentMethodType::Atome
                        | common_enums::PaymentMethodType::Bacs
                        | common_enums::PaymentMethodType::BancontactCard
                        | common_enums::PaymentMethodType::Becs
                        | common_enums::PaymentMethodType::Benefit
                        | common_enums::PaymentMethodType::Bizum
                        | common_enums::PaymentMethodType::Blik
                        | common_enums::PaymentMethodType::Boleto
                        | common_enums::PaymentMethodType::BcaBankTransfer
                        | common_enums::PaymentMethodType::BniVa
                        | common_enums::PaymentMethodType::BriVa
                        | common_enums::PaymentMethodType::CardRedirect
                        | common_enums::PaymentMethodType::CimbVa
                        | common_enums::PaymentMethodType::ClassicReward
                        | common_enums::PaymentMethodType::Credit
                        | common_enums::PaymentMethodType::CryptoCurrency
                        | common_enums::PaymentMethodType::Cashapp
                        | common_enums::PaymentMethodType::Dana
                        | common_enums::PaymentMethodType::DanamonVa
                        | common_enums::PaymentMethodType::Debit
                        | common_enums::PaymentMethodType::DirectCarrierBilling
                        | common_enums::PaymentMethodType::Efecty
                        | common_enums::PaymentMethodType::Eft
                        | common_enums::PaymentMethodType::Eps
                        | common_enums::PaymentMethodType::Evoucher
                        | common_enums::PaymentMethodType::Giropay
                        | common_enums::PaymentMethodType::Givex
                        | common_enums::PaymentMethodType::GooglePay
                        | common_enums::PaymentMethodType::GoPay
                        | common_enums::PaymentMethodType::Gcash
                        | common_enums::PaymentMethodType::Ideal
                        | common_enums::PaymentMethodType::Interac
                        | common_enums::PaymentMethodType::Indomaret
                        | common_enums::PaymentMethodType::Klarna
                        | common_enums::PaymentMethodType::KakaoPay
                        | common_enums::PaymentMethodType::MandiriVa
                        | common_enums::PaymentMethodType::Knet
                        | common_enums::PaymentMethodType::MbWay
                        | common_enums::PaymentMethodType::MobilePay
                        | common_enums::PaymentMethodType::Momo
                        | common_enums::PaymentMethodType::MomoAtm
                        | common_enums::PaymentMethodType::Multibanco
                        | common_enums::PaymentMethodType::LocalBankRedirect
                        | common_enums::PaymentMethodType::OnlineBankingThailand
                        | common_enums::PaymentMethodType::OnlineBankingCzechRepublic
                        | common_enums::PaymentMethodType::OnlineBankingFinland
                        | common_enums::PaymentMethodType::OnlineBankingFpx
                        | common_enums::PaymentMethodType::OnlineBankingPoland
                        | common_enums::PaymentMethodType::OnlineBankingSlovakia
                        | common_enums::PaymentMethodType::Oxxo
                        | common_enums::PaymentMethodType::PagoEfectivo
                        | common_enums::PaymentMethodType::PermataBankTransfer
                        | common_enums::PaymentMethodType::OpenBankingUk
                        | common_enums::PaymentMethodType::PayBright
                        | common_enums::PaymentMethodType::Paypal
                        | common_enums::PaymentMethodType::Paze
                        | common_enums::PaymentMethodType::Pix
                        | common_enums::PaymentMethodType::PaySafeCard
                        | common_enums::PaymentMethodType::Przelewy24
                        | common_enums::PaymentMethodType::Pse
                        | common_enums::PaymentMethodType::RedCompra
                        | common_enums::PaymentMethodType::RedPagos
                        | common_enums::PaymentMethodType::SamsungPay
                        | common_enums::PaymentMethodType::Sepa
                        | common_enums::PaymentMethodType::Sofort
                        | common_enums::PaymentMethodType::Swish
                        | common_enums::PaymentMethodType::TouchNGo
                        | common_enums::PaymentMethodType::Trustly
                        | common_enums::PaymentMethodType::Twint
                        | common_enums::PaymentMethodType::UpiCollect
                        | common_enums::PaymentMethodType::UpiIntent
                        | common_enums::PaymentMethodType::Venmo
                        | common_enums::PaymentMethodType::Vipps
                        | common_enums::PaymentMethodType::Walley
                        | common_enums::PaymentMethodType::WeChatPay
                        | common_enums::PaymentMethodType::SevenEleven
                        | common_enums::PaymentMethodType::Lawson
                        | common_enums::PaymentMethodType::LocalBankTransfer
                        | common_enums::PaymentMethodType::MiniStop
                        | common_enums::PaymentMethodType::FamilyMart
                        | common_enums::PaymentMethodType::Seicomart
                        | common_enums::PaymentMethodType::PayEasy
                        | common_enums::PaymentMethodType::Mifinity
                        | common_enums::PaymentMethodType::Fps
                        | common_enums::PaymentMethodType::DuitNow
                        | common_enums::PaymentMethodType::PromptPay
                        | common_enums::PaymentMethodType::VietQr
                        | common_enums::PaymentMethodType::OpenBankingPIS,
                    ) => Err(error_stack::report!(errors::ConnectorError::NotSupported {
                        message: payment_method_type.to_string(),
                        connector: "klarna",
                    })),
                }
            }
            PaymentMethodData::PayLater(PayLaterData::KlarnaRedirect {}) => {
                match (payment_experience, payment_method_type) {
                    (
                        common_enums::PaymentExperience::RedirectToUrl,
                        common_enums::PaymentMethodType::Klarna,
                    ) => Ok(format!("{endpoint}checkout/v3/orders",)),
                    (
                        common_enums::PaymentExperience::DisplayQrCode
                        | common_enums::PaymentExperience::DisplayWaitScreen
                        | common_enums::PaymentExperience::InvokePaymentApp
                        | common_enums::PaymentExperience::InvokeSdkClient
                        | common_enums::PaymentExperience::LinkWallet
                        | common_enums::PaymentExperience::OneClick
                        | common_enums::PaymentExperience::RedirectToUrl
                        | common_enums::PaymentExperience::CollectOtp,
                        common_enums::PaymentMethodType::Ach
                        | common_enums::PaymentMethodType::Affirm
                        | common_enums::PaymentMethodType::AfterpayClearpay
                        | common_enums::PaymentMethodType::Alfamart
                        | common_enums::PaymentMethodType::AliPay
                        | common_enums::PaymentMethodType::AliPayHk
                        | common_enums::PaymentMethodType::Alma
                        | common_enums::PaymentMethodType::AmazonPay
                        | common_enums::PaymentMethodType::ApplePay
                        | common_enums::PaymentMethodType::Atome
                        | common_enums::PaymentMethodType::Bacs
                        | common_enums::PaymentMethodType::BancontactCard
                        | common_enums::PaymentMethodType::Becs
                        | common_enums::PaymentMethodType::Benefit
                        | common_enums::PaymentMethodType::Bizum
                        | common_enums::PaymentMethodType::Blik
                        | common_enums::PaymentMethodType::Boleto
                        | common_enums::PaymentMethodType::BcaBankTransfer
                        | common_enums::PaymentMethodType::BniVa
                        | common_enums::PaymentMethodType::BriVa
                        | common_enums::PaymentMethodType::CardRedirect
                        | common_enums::PaymentMethodType::CimbVa
                        | common_enums::PaymentMethodType::ClassicReward
                        | common_enums::PaymentMethodType::Credit
                        | common_enums::PaymentMethodType::CryptoCurrency
                        | common_enums::PaymentMethodType::Cashapp
                        | common_enums::PaymentMethodType::Dana
                        | common_enums::PaymentMethodType::DanamonVa
                        | common_enums::PaymentMethodType::Debit
                        | common_enums::PaymentMethodType::DirectCarrierBilling
                        | common_enums::PaymentMethodType::Efecty
                        | common_enums::PaymentMethodType::Eft
                        | common_enums::PaymentMethodType::Eps
                        | common_enums::PaymentMethodType::Evoucher
                        | common_enums::PaymentMethodType::Giropay
                        | common_enums::PaymentMethodType::Givex
                        | common_enums::PaymentMethodType::GooglePay
                        | common_enums::PaymentMethodType::GoPay
                        | common_enums::PaymentMethodType::Gcash
                        | common_enums::PaymentMethodType::Ideal
                        | common_enums::PaymentMethodType::Interac
                        | common_enums::PaymentMethodType::Indomaret
                        | common_enums::PaymentMethodType::Klarna
                        | common_enums::PaymentMethodType::KakaoPay
                        | common_enums::PaymentMethodType::MandiriVa
                        | common_enums::PaymentMethodType::Knet
                        | common_enums::PaymentMethodType::MbWay
                        | common_enums::PaymentMethodType::MobilePay
                        | common_enums::PaymentMethodType::Momo
                        | common_enums::PaymentMethodType::MomoAtm
                        | common_enums::PaymentMethodType::Multibanco
                        | common_enums::PaymentMethodType::LocalBankRedirect
                        | common_enums::PaymentMethodType::OnlineBankingThailand
                        | common_enums::PaymentMethodType::OnlineBankingCzechRepublic
                        | common_enums::PaymentMethodType::OnlineBankingFinland
                        | common_enums::PaymentMethodType::OnlineBankingFpx
                        | common_enums::PaymentMethodType::OnlineBankingPoland
                        | common_enums::PaymentMethodType::OnlineBankingSlovakia
                        | common_enums::PaymentMethodType::Oxxo
                        | common_enums::PaymentMethodType::PagoEfectivo
                        | common_enums::PaymentMethodType::PermataBankTransfer
                        | common_enums::PaymentMethodType::OpenBankingUk
                        | common_enums::PaymentMethodType::PayBright
                        | common_enums::PaymentMethodType::Paypal
                        | common_enums::PaymentMethodType::Paze
                        | common_enums::PaymentMethodType::Pix
                        | common_enums::PaymentMethodType::PaySafeCard
                        | common_enums::PaymentMethodType::Przelewy24
                        | common_enums::PaymentMethodType::Pse
                        | common_enums::PaymentMethodType::RedCompra
                        | common_enums::PaymentMethodType::RedPagos
                        | common_enums::PaymentMethodType::SamsungPay
                        | common_enums::PaymentMethodType::Sepa
                        | common_enums::PaymentMethodType::Sofort
                        | common_enums::PaymentMethodType::Swish
                        | common_enums::PaymentMethodType::TouchNGo
                        | common_enums::PaymentMethodType::Trustly
                        | common_enums::PaymentMethodType::Twint
                        | common_enums::PaymentMethodType::UpiCollect
                        | common_enums::PaymentMethodType::UpiIntent
                        | common_enums::PaymentMethodType::Venmo
                        | common_enums::PaymentMethodType::Vipps
                        | common_enums::PaymentMethodType::Walley
                        | common_enums::PaymentMethodType::WeChatPay
                        | common_enums::PaymentMethodType::SevenEleven
                        | common_enums::PaymentMethodType::Lawson
                        | common_enums::PaymentMethodType::LocalBankTransfer
                        | common_enums::PaymentMethodType::MiniStop
                        | common_enums::PaymentMethodType::FamilyMart
                        | common_enums::PaymentMethodType::Seicomart
                        | common_enums::PaymentMethodType::PayEasy
                        | common_enums::PaymentMethodType::Mifinity
                        | common_enums::PaymentMethodType::Fps
                        | common_enums::PaymentMethodType::DuitNow
                        | common_enums::PaymentMethodType::PromptPay
                        | common_enums::PaymentMethodType::VietQr
                        | common_enums::PaymentMethodType::OpenBankingPIS,
                    ) => Err(error_stack::report!(errors::ConnectorError::NotSupported {
                        message: payment_method_type.to_string(),
                        connector: "klarna",
                    })),
                }
            }

            PaymentMethodData::Card(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(report!(errors::ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message(req.connector.as_str(),),
                )))
            }
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
        let connector_router_data = klarna::KlarnaRouterData::from((amount, req));
        let connector_req = klarna::KlarnaPaymentsRequest::try_from(&connector_router_data)?;

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
        let response: klarna::KlarnaAuthResponse = res
            .response
            .parse_struct("KlarnaPaymentsResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Klarna {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let order_id = req.request.connector_transaction_id.clone();
        let endpoint =
            build_region_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        Ok(format!(
            "{endpoint}ordermanagement/v1/orders/{order_id}/cancel"
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
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
        _event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        logger::debug!("Expected zero bytes response, skipped parsing of the response");

        let status = if res.status_code == 204 {
            enums::AttemptStatus::Voided
        } else {
            enums::AttemptStatus::VoidFailed
        };
        Ok(PaymentsCancelRouterData {
            status,
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

impl api::Refund for Klarna {}
impl api::RefundExecute for Klarna {}
impl api::RefundSync for Klarna {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Klarna {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        let order_id = req.request.connector_transaction_id.clone();
        let endpoint =
            build_region_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        Ok(format!(
            "{endpoint}ordermanagement/v1/orders/{order_id}/refunds",
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let connector_router_data = klarna::KlarnaRouterData::from((amount, req));
        let connector_req = klarna::KlarnaRefundRequest::try_from(&connector_router_data)?;
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
        match res.headers {
            Some(headers) => {
                let refund_id = get_http_header("Refund-Id", &headers)
                    .attach_printable("Missing refund id in headers")
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
                let response = klarna::KlarnaRefundResponse {
                    refund_id: refund_id.to_owned(),
                };

                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            None => Err(errors::ConnectorError::ResponseDeserializationFailed)
                .attach_printable("Expected headers, but received no headers in response")?,
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Klarna {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        let order_id = req.request.connector_transaction_id.clone();
        let refund_id = req.request.get_connector_refund_id()?;
        let endpoint =
            build_region_specific_endpoint(self.base_url(connectors), &req.connector_meta_data)?;

        Ok(format!(
            "{endpoint}ordermanagement/v1/orders/{order_id}/refunds/{refund_id}"
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
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: klarna::KlarnaRefundSyncResponse = res
            .response
            .parse_struct("klarna KlarnaRefundSyncResponse")
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
impl IncomingWebhook for Klarna {
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        Ok(IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

impl ConnectorSpecifications for Klarna {}
