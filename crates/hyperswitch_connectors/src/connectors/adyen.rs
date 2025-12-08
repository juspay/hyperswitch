pub mod transformers;
use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums::{self, PaymentMethodType};
use common_utils::{
    consts,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, OptionExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{
        AmountConvertor, MinorUnit, MinorUnitForConnector, StringMinorUnit,
        StringMinorUnitForConnector,
    },
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, ExtendAuthorization, PSync, PaymentMethodToken, PreProcessing,
            Session, SetupMandate, Void,
        },
        refunds::{Execute, RSync},
        Accept, Defend, Evidence, GiftCardBalanceCheck, Retrieve, Upload,
    },
    router_request_types::{
        AcceptDisputeRequestData, AccessTokenRequestData, DefendDisputeRequestData,
        GiftCardBalanceCheckRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsExtendAuthorizationData,
        PaymentsPreProcessingData, PaymentsSessionData, PaymentsSyncData, RefundsData,
        RetrieveFileRequestData, SetupMandateRequestData, SubmitEvidenceRequestData,
        SyncRequestType, UploadFileRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, ConnectorInfo, DefendDisputeResponse,
        GiftCardBalanceCheckResponseData, PaymentMethodDetails, PaymentsResponseData,
        RefundsResponseData, RetrieveFileResponse, SubmitEvidenceResponse, SupportedPaymentMethods,
        SupportedPaymentMethodsExt, UploadFileResponse,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsExtendAuthorizationRouterData, PaymentsGiftCardBalanceCheckRouterData,
        PaymentsPreProcessingRouterData, PaymentsSyncRouterData, RefundsRouterData,
        SetupMandateRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::{PoCancel, PoCreate, PoEligibility, PoFulfill},
    router_response_types::PayoutsResponseData,
    types::{PayoutsData, PayoutsRouterData},
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types::{
    PayoutCancelType, PayoutCreateType, PayoutEligibilityType, PayoutFulfillType,
};
use hyperswitch_interfaces::{
    api::{
        self,
        disputes::{AcceptDispute, DefendDispute, Dispute, SubmitEvidence},
        files::{FilePurpose, FileUpload, RetrieveFile, UploadFile},
        CaptureSyncMethod, ConnectorCommon, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    disputes, errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        AcceptDisputeType, DefendDisputeType, ExtendedAuthorizationType, PaymentsAuthorizeType,
        PaymentsCaptureType, PaymentsGiftCardBalanceCheckType, PaymentsPreProcessingType,
        PaymentsSyncType, PaymentsVoidType, RefundExecuteType, Response, SetupMandateType,
        SubmitEvidenceType,
    },
    webhooks::{IncomingWebhook, IncomingWebhookFlowError, IncomingWebhookRequestDetails},
};
use masking::{ExposeInterface, Mask, Maskable, Secret};
use ring::hmac;
use router_env::{instrument, tracing};
use transformers as adyen;

#[cfg(feature = "payouts")]
use crate::utils::PayoutsData as UtilsPayoutData;
use crate::{
    capture_method_not_supported,
    constants::{self, headers},
    types::{
        AcceptDisputeRouterData, DefendDisputeRouterData, ResponseRouterData,
        SubmitEvidenceRouterData,
    },
    utils::{
        convert_amount, convert_payment_authorize_router_response,
        convert_setup_mandate_router_data_to_authorize_router_data, is_mandate_supported,
        ForeignTryFrom, PaymentMethodDataType,
    },
};
const ADYEN_API_VERSION: &str = "v68";

#[derive(Clone)]
pub struct Adyen {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
    amount_converter_webhooks: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Adyen {
    pub const fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
            amount_converter_webhooks: &StringMinorUnitForConnector,
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
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = adyen::AdyenAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::X_API_KEY.to_string(),
            auth.api_key.into_masked(),
        )])
    }
    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.adyen.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: adyen::AdyenErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.message.to_owned(),
            reason: Some(response.message),
            attempt_status: None,
            connector_transaction_id: response.psp_reference,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Adyen {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _payment_method: enums::PaymentMethod,
        pmt: Option<PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        let connector = self.id();
        match pmt {
            Some(payment_method_type) => match payment_method_type {
                #[cfg(feature = "v1")]
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
                #[cfg(feature = "v2")]
                PaymentMethodType::Affirm
                | PaymentMethodType::AfterpayClearpay
                | PaymentMethodType::ApplePay
                | PaymentMethodType::Credit
                | PaymentMethodType::Debit
                | PaymentMethodType::Card
                | PaymentMethodType::GooglePay
                | PaymentMethodType::MobilePay
                | PaymentMethodType::PayBright
                | PaymentMethodType::Sepa
                | PaymentMethodType::Vipps
                | PaymentMethodType::Venmo
                | PaymentMethodType::Skrill
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
                | PaymentMethodType::Walley
                | PaymentMethodType::Payjustnow => match capture_method {
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
                | PaymentMethodType::Breadpay
                | PaymentMethodType::Paysera
                | PaymentMethodType::Skrill
                | PaymentMethodType::CardRedirect
                | PaymentMethodType::DirectCarrierBilling
                | PaymentMethodType::Fps
                | PaymentMethodType::BhnCardNetwork
                | PaymentMethodType::DuitNow
                | PaymentMethodType::Interac
                | PaymentMethodType::Multibanco
                | PaymentMethodType::Przelewy24
                | PaymentMethodType::Becs
                | PaymentMethodType::Eft
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
                | PaymentMethodType::UpiQr
                | PaymentMethodType::VietQr
                | PaymentMethodType::Mifinity
                | PaymentMethodType::LocalBankRedirect
                | PaymentMethodType::OpenBankingPIS
                | PaymentMethodType::InstantBankTransfer
                | PaymentMethodType::InstantBankTransferFinland
                | PaymentMethodType::InstantBankTransferPoland
                | PaymentMethodType::IndonesianBankTransfer
                | PaymentMethodType::SepaBankTransfer
                | PaymentMethodType::Flexiti
                | PaymentMethodType::RevolutPay
                | PaymentMethodType::Bluecode
                | PaymentMethodType::SepaGuarenteedDebit => {
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
        pm_data: PaymentMethodData,
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
        data: &PaymentsSyncData,
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
impl api::PaymentsGiftCardBalanceCheck for Adyen {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Adyen
{
    // Not Implemented (R)
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Adyen {
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

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Adyen {
    fn get_headers(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            SetupMandateType::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!("{endpoint}{ADYEN_API_VERSION}/payments"))
    }
    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let authorize_req = convert_payment_authorize_router_response((
            req,
            convert_setup_mandate_router_data_to_authorize_router_data(req),
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
    ) -> CustomResult<
        RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        errors::ConnectorError,
    >
    where
        SetupMandate: Clone,
        SetupMandateRequestData: Clone,
        PaymentsResponseData: Clone,
    {
        let response: adyen::AdyenPaymentResponse = res
            .response
            .parse_struct("AdyenPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::foreign_try_from((
            ResponseRouterData {
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

impl api::PaymentSession for Adyen {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Adyen {
    // Not Implemented (R)
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Adyen {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
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
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.as_str();

        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}{ADYEN_API_VERSION}/payments/{id}/captures",
        ))
    }
    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
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
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: adyen::AdyenCaptureResponse = res
            .response
            .parse_struct("AdyenCaptureResponse")
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
    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

/// Payment Sync can be useful only incase of Redirect flow.
/// For payments which doesn't involve redrection we have to rely on webhooks.
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Adyen {
    fn get_headers(
        &self,
        req: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsSyncType::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
        _connectors: &Connectors,
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
        req: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!("{endpoint}{ADYEN_API_VERSION}/payments/details"))
    }

    fn build_request(
        &self,
        req: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
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
                RequestBuilder::new()
                    .method(Method::Post)
                    .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                    .set_body(PaymentsSyncType::get_request_body(self, req, connectors)?)
                    .build(),
            ))
        } else {
            Ok(None)
        }
    }

    fn handle_response(
        &self,
        data: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        router_env::logger::debug!(payment_sync_response=?res);
        let response: adyen::AdyenPaymentResponse = res
            .response
            .parse_struct("AdyenPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let is_multiple_capture_sync = match data.request.sync_type {
            SyncRequestType::MultipleCaptureSync(_) => true,
            SyncRequestType::SinglePaymentSync => false,
        };
        RouterData::foreign_try_from((
            ResponseRouterData {
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
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<CaptureSyncMethod, errors::ConnectorError> {
        Ok(CaptureSyncMethod::Individual)
    }
    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Adyen {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError>
    where
        Self: ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!("{endpoint}{ADYEN_API_VERSION}/payments"))
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
        let connector_router_data = adyen::AdyenRouterData::try_from((amount, req))?;
        let connector_req = adyen::AdyenPaymentRequest::try_from(&connector_router_data)?;
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
                .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                .set_body(PaymentsAuthorizeType::get_request_body(
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
        let response: adyen::AdyenPaymentResponse = res
            .response
            .parse_struct("AdyenPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::foreign_try_from((
            ResponseRouterData {
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

impl api::PaymentsPreProcessing for Adyen {}

impl ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
    for Adyen
{
    fn get_headers(
        &self,
        req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsPreProcessingType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}{ADYEN_API_VERSION}/paymentMethods/balance",
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenBalanceRequest::try_from(req)?;

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
            Ok(RouterData {
                response: Err(ErrorResponse {
                    code: NO_ERROR_CODE.to_string(),
                    message: NO_ERROR_MESSAGE.to_string(),
                    reason: Some(constants::LOW_BALANCE_ERROR_MESSAGE.to_string()),
                    status_code: res.status_code,
                    attempt_status: Some(enums::AttemptStatus::Failure),
                    connector_transaction_id: Some(response.psp_reference),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..data.clone()
            })
        } else {
            RouterData::try_from(ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Adyen {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.clone();

        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}{ADYEN_API_VERSION}/payments/{id}/cancels",
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenCancelRequest::try_from(req)?;

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
                .set_body(PaymentsVoidType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: adyen::AdyenCancelResponse = res
            .response
            .parse_struct("AdyenCancelResponse")
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
        GiftCardBalanceCheck,
        GiftCardBalanceCheckRequestData,
        GiftCardBalanceCheckResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &PaymentsGiftCardBalanceCheckRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsGiftCardBalanceCheckType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &PaymentsGiftCardBalanceCheckRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}{ADYEN_API_VERSION}/paymentMethods/balance",
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsGiftCardBalanceCheckRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenBalanceRequest::try_from(req)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsGiftCardBalanceCheckRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsGiftCardBalanceCheckType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(PaymentsGiftCardBalanceCheckType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(PaymentsGiftCardBalanceCheckType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsGiftCardBalanceCheckRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsGiftCardBalanceCheckRouterData, errors::ConnectorError> {
        let response: adyen::AdyenBalanceResponse = res
            .response
            .parse_struct("AdyenBalanceResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let currency = data
            .request
            .currency
            .get_required_value("currency")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "currency",
            })?;

        if response.balance.currency != currency {
            Ok(RouterData {
                response: Err(ErrorResponse {
                    code: NO_ERROR_CODE.to_string(),
                    message: NO_ERROR_MESSAGE.to_string(),
                    reason: Some(constants::MISMATCHED_CURRENCY.to_string()),
                    status_code: res.status_code,
                    attempt_status: Some(enums::AttemptStatus::Failure),
                    connector_transaction_id: Some(response.psp_reference),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..data.clone()
            })
        } else {
            RouterData::try_from(ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
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
impl ConnectorIntegration<PoCancel, PayoutsData, PayoutsResponseData> for Adyen {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.payout_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}pal/servlet/Payout/{ADYEN_API_VERSION}/declineThirdParty",
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PayoutCancelType::get_content_type(self).to_string().into(),
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
        req: &PayoutsRouterData<PoCancel>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenPayoutCancelRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutCancelType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutCancelType::get_headers(self, req, connectors)?)
            .set_body(PayoutCancelType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoCancel>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoCancel>, errors::ConnectorError> {
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
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
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData> for Adyen {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.payout_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}pal/servlet/Payout/{ADYEN_API_VERSION}/storeDetailAndSubmitThirdParty",
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PayoutCreateType::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        _connectors: &Connectors,
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
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutCreateType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutCreateType::get_headers(self, req, connectors)?)
            .set_body(PayoutCreateType::get_request_body(self, req, connectors)?)
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
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
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
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoEligibility, PayoutsData, PayoutsResponseData> for Adyen {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoEligibility>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!("{endpoint}{ADYEN_API_VERSION}/payments"))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoEligibility>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PayoutEligibilityType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoEligibility>,
        _connectors: &Connectors,
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
        req: &PayoutsRouterData<PoEligibility>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutEligibilityType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutEligibilityType::get_headers(self, req, connectors)?)
            .set_body(PayoutEligibilityType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoEligibility>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoEligibility>, errors::ConnectorError> {
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
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
        self.build_error_response(res, event_builder)
    }
}

impl api::PaymentExtendAuthorization for Adyen {}
impl
    ConnectorIntegration<ExtendAuthorization, PaymentsExtendAuthorizationData, PaymentsResponseData>
    for Adyen
{
    fn get_headers(
        &self,
        req: &PaymentsExtendAuthorizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsExtendAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.as_str();
        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}{ADYEN_API_VERSION}/payments/{id}/amountUpdates"
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsExtendAuthorizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = adyen::AdyenRouterData::try_from((amount, req))?;
        let connector_req =
            adyen::AdyenExtendAuthorizationRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsExtendAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&ExtendedAuthorizationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(ExtendedAuthorizationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(ExtendedAuthorizationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsExtendAuthorizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsExtendAuthorizationRouterData, errors::ConnectorError> {
        let response: adyen::AdyenExtendAuthorizationResponse = res
            .response
            .parse_struct("Adyen AdyenExtendAuthorizationResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Adyen {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.payout_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        let payout_type = req.request.get_payout_type()?;
        let path_segment = match payout_type {
            enums::PayoutType::Bank | enums::PayoutType::Wallet => "confirmThirdParty",
            enums::PayoutType::Card => "payout",
            enums::PayoutType::BankRedirect => {
                return Err(errors::ConnectorError::NotImplemented(
                    "bank redirect payouts not supoorted by adyen".to_string(),
                )
                .into())
            }
        };
        Ok(format!(
            "{}pal/servlet/Payout/{}/{}",
            endpoint, ADYEN_API_VERSION, path_segment
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PayoutFulfillType::get_content_type(self).to_string().into(),
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
                enums::PayoutType::Bank
                | enums::PayoutType::Wallet
                | enums::PayoutType::BankRedirect => {
                    auth.review_key.unwrap_or(auth.api_key).into_masked()
                }
                enums::PayoutType::Card => auth.api_key.into_masked(),
            },
        )];
        header.append(&mut api_key);
        Ok(header)
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

        let connector_router_data = adyen::AdyenRouterData::try_from((amount, req))?;
        let connector_req = adyen::AdyenPayoutFulfillRequest::try_from(&connector_router_data)?;
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
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
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
        self.build_error_response(res, event_builder)
    }
}

impl api::Refund for Adyen {}
impl api::RefundExecute for Adyen {}
impl api::RefundSync for Adyen {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Adyen {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            RefundExecuteType::get_content_type(self).to_string().into(),
        )];
        let mut api_header = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_header);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();

        let endpoint = build_env_specific_endpoint(
            self.base_url(connectors),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}{ADYEN_API_VERSION}/payments/{connector_payment_id}/refunds",
        ))
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
        let connector_router_data = adyen::AdyenRouterData::try_from((refund_amount, req))?;
        let connector_req = adyen::AdyenRefundRequest::try_from(&connector_router_data)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RefundExecuteType::get_headers(self, req, connectors)?)
                .set_body(RefundExecuteType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: adyen::AdyenRefundResponse = res
            .response
            .parse_struct("AdyenRefundResponse")
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
    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Adyen {}

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<adyen::AdyenNotificationRequestItemWH, common_utils::errors::ParsingError> {
    let mut webhook: adyen::AdyenIncomingWebhook = body.parse_struct("AdyenIncomingWebhook")?;

    let item_object = webhook
        .notification_items
        .drain(..)
        .next()
        // TODO: ParsingError doesn't seem to be an apt error for this case
        .ok_or(common_utils::errors::ParsingError::UnknownError)?;

    Ok(item_object.notification_request_item)
}

#[async_trait::async_trait]
impl IncomingWebhook for Adyen {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn common_utils::crypto::VerifySignature + Send>, errors::ConnectorError>
    {
        Ok(Box::new(common_utils::crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notif_item = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let base64_signature = notif_item.additional_data.hmac_signature.expose();
        Ok(base64_signature.as_bytes().to_vec())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
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
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: common_utils::crypto::Encryptable<Secret<serde_json::Value>>,
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
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        // for capture_event, original_reference field will have the authorized payment's PSP reference
        if adyen::is_capture_or_cancel_or_adjust_event(&notif.event_code) {
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
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(transformers::get_adyen_webhook_event(
            notif.event_code,
            notif.success,
            notif.additional_data.dispute_status,
        ))
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        let response = adyen::AdyenWebhookResponse::from(notif);

        Ok(Box::new(response))
    }

    fn get_webhook_api_response(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _error_kind: Option<IncomingWebhookFlowError>,
    ) -> CustomResult<ApplicationResponse<serde_json::Value>, errors::ConnectorError> {
        Ok(ApplicationResponse::TextPlain("[accepted]".to_string()))
    }

    fn get_dispute_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<disputes::DisputePayload, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let amount = convert_amount(
            self.amount_converter_webhooks,
            notif.amount.value,
            notif.amount.currency,
        )?;
        Ok(disputes::DisputePayload {
            amount,
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
        request: &IncomingWebhookRequestDetails<'_>,
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
        request: &IncomingWebhookRequestDetails<'_>,
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

    #[cfg(feature = "v1")]
    fn get_additional_payment_method_data(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<api_models::payment_methods::PaymentMethodUpdate>,
        errors::ConnectorError,
    > {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let expiry = notif
            .additional_data
            .expiry_date
            .map(|date| transformers::CardExpiry::parse(&date.expose()))
            .transpose()?
            .ok_or(errors::ConnectorError::ParsingFailed)?;

        let month_str = expiry.month();
        let year_str = expiry.year();

        Ok(Some(api_models::payment_methods::PaymentMethodUpdate {
            card: Some(api_models::payment_methods::CardDetailUpdate {
                card_exp_month: Some(month_str),
                card_exp_year: Some(year_str),
                card_holder_name: None,
                nick_name: None,
                issuer_country: notif.additional_data.card_issuing_country.clone(),
                card_issuer: notif.additional_data.card_issuing_bank.clone(),
                last4_digits: notif
                    .additional_data
                    .card_summary
                    .map(|last4| last4.expose().to_string()),
                card_network: adyen::from_payment_method_variant(
                    notif
                        .additional_data
                        .payment_method_variant
                        .map(|network| network.expose()),
                ),
            }),
            wallet: None,
            client_secret: None,
        }))
    }
}
impl Dispute for Adyen {}
impl DefendDispute for Adyen {}
impl AcceptDispute for Adyen {}
impl SubmitEvidence for Adyen {}

impl ConnectorIntegration<Accept, AcceptDisputeRequestData, AcceptDisputeResponse> for Adyen {
    fn get_headers(
        &self,
        req: &AcceptDisputeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            AcceptDisputeType::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &AcceptDisputeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.dispute_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}ca/services/DisputeService/v30/acceptDispute",
        ))
    }

    fn build_request(
        &self,
        req: &AcceptDisputeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&AcceptDisputeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(AcceptDisputeType::get_headers(self, req, connectors)?)
                .set_body(AcceptDisputeType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }
    fn get_request_body(
        &self,
        req: &AcceptDisputeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenAcceptDisputeRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &AcceptDisputeRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<AcceptDisputeRouterData, errors::ConnectorError> {
        let response: adyen::AdyenDisputeResponse = res
            .response
            .parse_struct("AdyenDisputeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        RouterData::foreign_try_from((data, response))
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

impl ConnectorIntegration<Defend, DefendDisputeRequestData, DefendDisputeResponse> for Adyen {
    fn get_headers(
        &self,
        req: &DefendDisputeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            DefendDisputeType::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &DefendDisputeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.dispute_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}ca/services/DisputeService/v30/defendDispute",
        ))
    }

    fn build_request(
        &self,
        req: &DefendDisputeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&DefendDisputeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(DefendDisputeType::get_headers(self, req, connectors)?)
                .set_body(DefendDisputeType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn get_request_body(
        &self,
        req: &DefendDisputeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenDefendDisputeRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &DefendDisputeRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<DefendDisputeRouterData, errors::ConnectorError> {
        let response: adyen::AdyenDisputeResponse = res
            .response
            .parse_struct("AdyenDisputeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        RouterData::foreign_try_from((data, response))
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

impl ConnectorIntegration<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse> for Adyen {
    fn get_headers(
        &self,
        req: &SubmitEvidenceRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            SubmitEvidenceType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &SubmitEvidenceRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = build_env_specific_endpoint(
            connectors.adyen.dispute_base_url.as_str(),
            req.test_mode,
            &req.connector_meta_data,
        )?;
        Ok(format!(
            "{endpoint}ca/services/DisputeService/v30/supplyDefenseDocument",
        ))
    }

    fn get_request_body(
        &self,
        req: &SubmitEvidenceRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::Evidence::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &SubmitEvidenceRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&SubmitEvidenceType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(SubmitEvidenceType::get_headers(self, req, connectors)?)
            .set_body(SubmitEvidenceType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &SubmitEvidenceRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SubmitEvidenceRouterData, errors::ConnectorError> {
        let response: adyen::AdyenDisputeResponse = res
            .response
            .parse_struct("AdyenDisputeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        RouterData::foreign_try_from((data, response))
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
impl UploadFile for Adyen {}
impl RetrieveFile for Adyen {}
impl ConnectorIntegration<Retrieve, RetrieveFileRequestData, RetrieveFileResponse> for Adyen {}
impl ConnectorIntegration<Upload, UploadFileRequestData, UploadFileResponse> for Adyen {}
#[async_trait::async_trait]
impl FileUpload for Adyen {
    fn validate_file_upload(
        &self,
        purpose: FilePurpose,
        file_size: i32,
        file_type: mime::Mime,
    ) -> CustomResult<(), errors::ConnectorError> {
        match purpose {
            FilePurpose::DisputeEvidence => {
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

static ADYEN_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods1 = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::Manual,
        enums::CaptureMethod::SequentialAutomatic,
        enums::CaptureMethod::ManualMultiple,
    ];

    let supported_capture_methods2 = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::Manual,
        enums::CaptureMethod::SequentialAutomatic,
    ];

    let supported_capture_methods3 = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::SequentialAutomatic,
    ];

    let supported_card_network = vec![
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::CartesBancaires,
        common_enums::CardNetwork::UnionPay,
        common_enums::CardNetwork::DinersClub,
        common_enums::CardNetwork::Discover,
        common_enums::CardNetwork::Interac,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::Maestro,
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::Visa,
    ];

    let mut adyen_supported_payment_methods = SupportedPaymentMethods::new();

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
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

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        PaymentMethodType::Debit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
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

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::GooglePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::ApplePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::Paypal,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::AliPay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::AliPayHk,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::GoPay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::KakaoPay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::Gcash,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::Momo,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::TouchNGo,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::MbWay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::MobilePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::WeChatPay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::SamsungPay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods2.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::Paze,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods2.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::Twint,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods2.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::Vipps,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::Dana,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        PaymentMethodType::Swish,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        PaymentMethodType::Klarna,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods2.clone(),
            specific_features: None,
        },
    );
    adyen_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        PaymentMethodType::Affirm,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
            specific_features: None,
        },
    );
    adyen_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        PaymentMethodType::AfterpayClearpay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
            specific_features: None,
        },
    );
    adyen_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        PaymentMethodType::PayBright,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
            specific_features: None,
        },
    );
    adyen_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        PaymentMethodType::Walley,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods2.clone(),
            specific_features: None,
        },
    );
    adyen_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        PaymentMethodType::Alma,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods2.clone(),
            specific_features: None,
        },
    );
    adyen_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        PaymentMethodType::Atome,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::BancontactCard,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Bizum,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Blik,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Eps,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Ideal,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::OnlineBankingCzechRepublic,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::OnlineBankingFinland,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::OnlineBankingPoland,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::OnlineBankingSlovakia,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::OnlineBankingFpx,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::OnlineBankingThailand,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::OpenBankingUk,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Trustly,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankDebit,
        PaymentMethodType::Ach,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods2.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankDebit,
        PaymentMethodType::Sepa,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods1.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankDebit,
        PaymentMethodType::Bacs,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods2.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankTransfer,
        PaymentMethodType::PermataBankTransfer,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankTransfer,
        PaymentMethodType::BcaBankTransfer,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankTransfer,
        PaymentMethodType::BniVa,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankTransfer,
        PaymentMethodType::BriVa,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankTransfer,
        PaymentMethodType::CimbVa,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankTransfer,
        PaymentMethodType::DanamonVa,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankTransfer,
        PaymentMethodType::MandiriVa,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::BankTransfer,
        PaymentMethodType::Pix,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::CardRedirect,
        PaymentMethodType::Knet,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::CardRedirect,
        PaymentMethodType::Benefit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::CardRedirect,
        PaymentMethodType::MomoAtm,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::Boleto,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::Alfamart,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::Indomaret,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::Oxxo,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::SevenEleven,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::Lawson,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::MiniStop,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::FamilyMart,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::Seicomart,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::Voucher,
        PaymentMethodType::PayEasy,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::GiftCard,
        PaymentMethodType::PaySafeCard,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods3.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods.add(
        enums::PaymentMethod::GiftCard,
        PaymentMethodType::Givex,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods2.clone(),
            specific_features: None,
        },
    );

    adyen_supported_payment_methods
});

static ADYEN_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name: "Adyen",
        description: "Adyen is a Dutch payment company with the status of an acquiring bank that allows businesses to accept e-commerce, mobile, and point-of-sale payments. It is listed on the stock exchange Euronext Amsterdam",
        connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
        integration_status: enums::ConnectorIntegrationStatus::Live,
    };

static ADYEN_SUPPORTED_WEBHOOK_FLOWS: &[enums::EventClass] = &[
    enums::EventClass::Payments,
    enums::EventClass::Refunds,
    enums::EventClass::Disputes,
    #[cfg(feature = "payouts")]
    enums::EventClass::Payouts,
    enums::EventClass::Mandates,
];

impl ConnectorSpecifications for Adyen {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&ADYEN_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*ADYEN_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(ADYEN_SUPPORTED_WEBHOOK_FLOWS)
    }

    #[cfg(feature = "v1")]
    fn generate_connector_customer_id(
        &self,
        customer_id: &Option<common_utils::id_type::CustomerId>,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> Option<String> {
        customer_id.as_ref().map(|cid| {
            format!(
                "{}_{}",
                merchant_id.get_string_repr(),
                cid.get_string_repr()
            )
        })
    }
}
