pub mod transformers;

use std::{collections::HashMap, sync::LazyLock};

use api_models::webhooks::IncomingWebhookEvent;
use common_enums::{
    CallConnectorAction, CaptureMethod, PaymentAction, PaymentChargeType, PaymentMethodType,
    PaymentResourceUpdateStatus, StripeChargeType,
};
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::{ByteSliceExt as _, BytesExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{
        AmountConvertor, MinorUnit, MinorUnitForConnector, StringMinorUnit,
        StringMinorUnitForConnector,
    },
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        AccessTokenAuth, Authorize, Capture, CreateConnectorCustomer, Evidence, Execute,
        IncrementalAuthorization, PSync, PaymentMethodToken, RSync, Retrieve, Session,
        SetupMandate, UpdateMetadata, Upload, Void,
    },
    router_request_types::{
        AccessTokenRequestData, ConnectorCustomerData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsIncrementalAuthorizationData, PaymentsSessionData, PaymentsSyncData,
        PaymentsUpdateMetadataData, RefundsData, RetrieveFileRequestData, SetupMandateRequestData,
        SplitRefundsRequest, SubmitEvidenceRequestData, UploadFileRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        RetrieveFileResponse, SubmitEvidenceResponse, SupportedPaymentMethods,
        SupportedPaymentMethodsExt, UploadFileResponse,
    },
    types::{
        ConnectorCustomerRouterData, PaymentsAuthorizeRouterData, PaymentsCancelRouterData,
        PaymentsCaptureRouterData, PaymentsIncrementalAuthorizationRouterData,
        PaymentsSyncRouterData, PaymentsUpdateMetadataRouterData, RefundsRouterData,
        TokenizationRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::{PoCancel, PoCreate, PoFulfill, PoRecipient, PoRecipientAccount},
    types::{PayoutsData, PayoutsResponseData, PayoutsRouterData},
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types::{
    PayoutCancelType, PayoutCreateType, PayoutFulfillType, PayoutRecipientAccountType,
    PayoutRecipientType,
};
use hyperswitch_interfaces::{
    api::{
        self,
        disputes::SubmitEvidence,
        files::{FilePurpose, FileUpload, RetrieveFile, UploadFile},
        ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorRedirectResponse,
        ConnectorSpecifications, ConnectorValidation, PaymentIncrementalAuthorization,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    disputes::DisputePayload,
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::{
        ConnectorCustomerType, IncrementalAuthorizationType, PaymentsAuthorizeType,
        PaymentsCaptureType, PaymentsSyncType, PaymentsUpdateMetadataType, PaymentsVoidType,
        RefundExecuteType, RefundSyncType, Response, RetrieveFileType, SubmitEvidenceType,
        TokenizationType, UploadFileType,
    },
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::{Mask as _, Maskable, PeekInterface};
use router_env::{instrument, tracing};
use stripe::auth_headers;

use self::transformers as stripe;
#[cfg(feature = "payouts")]
use crate::utils::{PayoutsData as OtherPayoutsData, RouterData as OtherRouterData};
use crate::{
    connectors::stripe::transformers::get_stripe_compatible_connect_account_header,
    constants::headers::{AUTHORIZATION, CONTENT_TYPE, STRIPE_COMPATIBLE_CONNECT_ACCOUNT},
    types::{
        ResponseRouterData, RetrieveFileRouterData, SubmitEvidenceRouterData, UploadFileRouterData,
    },
    utils::{
        self, get_authorise_integrity_object, get_capture_integrity_object,
        get_refund_integrity_object, get_sync_integrity_object, PaymentMethodDataType,
        RefundsRequestData as OtherRefundsRequestData,
    },
};
#[derive(Clone)]
pub struct Stripe {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
    amount_converter_webhooks: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Stripe {
    pub const fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
            amount_converter_webhooks: &StringMinorUnitForConnector,
        }
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Stripe
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            Self::common_get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Stripe {
    fn id(&self) -> &'static str {
        "stripe"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        // &self.base_url
        connectors.stripe.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = stripe::StripeAuthType::try_from(auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            (
                AUTHORIZATION.to_string(),
                format!("Bearer {}", auth.api_key.peek()).into_masked(),
            ),
            (
                auth_headers::STRIPE_API_VERSION.to_string(),
                auth_headers::STRIPE_VERSION.to_string().into_masked(),
            ),
        ])
    }

    #[cfg(feature = "payouts")]
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        use hyperswitch_interfaces::consts::NO_ERROR_CODE;

        let response: stripe::StripeConnectErrorResponse = res
            .response
            .parse_struct("StripeConnectErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message,
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Stripe {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<CaptureMethod>,
        _payment_method: common_enums::PaymentMethod,
        _pmt: Option<PaymentMethodType>,
    ) -> CustomResult<(), ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            CaptureMethod::SequentialAutomatic
            | CaptureMethod::Automatic
            | CaptureMethod::Manual => Ok(()),
            CaptureMethod::ManualMultiple | CaptureMethod::Scheduled => Err(
                utils::construct_not_supported_error_report(capture_method, self.id()),
            ),
        }
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
            PaymentMethodDataType::ApplePay,
            PaymentMethodDataType::GooglePay,
            PaymentMethodDataType::AchBankDebit,
            PaymentMethodDataType::BacsBankDebit,
            PaymentMethodDataType::BecsBankDebit,
            PaymentMethodDataType::SepaBankDebit,
            PaymentMethodDataType::Sofort,
            PaymentMethodDataType::Ideal,
            PaymentMethodDataType::BancontactCard,
        ]);
        utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl api::Payment for Stripe {}

impl api::PaymentAuthorize for Stripe {}
impl api::PaymentUpdateMetadata for Stripe {}
impl api::PaymentSync for Stripe {}
impl api::PaymentVoid for Stripe {}
impl api::PaymentCapture for Stripe {}
impl api::PaymentSession for Stripe {}
impl api::ConnectorAccessToken for Stripe {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Stripe {
    // Not Implemented (R)
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Stripe {
    // Not Implemented (R)
}

impl api::ConnectorCustomer for Stripe {}

impl ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>
    for Stripe
{
    fn get_headers(
        &self,
        req: &ConnectorCustomerRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            ConnectorCustomerType::get_content_type(self)
                .to_string()
                .into(),
        )];
        if let Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(
            stripe_split_payment,
        )) = &req.request.split_payments
        {
            if stripe_split_payment.charge_type
                == PaymentChargeType::Stripe(StripeChargeType::Direct)
            {
                let mut customer_account_header = vec![(
                    STRIPE_COMPATIBLE_CONNECT_ACCOUNT.to_string(),
                    stripe_split_payment
                        .transfer_account_id
                        .clone()
                        .into_masked(),
                )];
                header.append(&mut customer_account_header);
            }
        }
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v1/customers"))
    }

    fn get_request_body(
        &self,
        req: &ConnectorCustomerRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::CustomerRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&ConnectorCustomerType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(ConnectorCustomerType::get_headers(self, req, connectors)?)
                .set_body(ConnectorCustomerType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &ConnectorCustomerRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<ConnectorCustomerRouterData, ConnectorError>
    where
        PaymentsResponseData: Clone,
    {
        let response: stripe::StripeCustomerResponse = res
            .response
            .parse_struct("StripeCustomerResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl api::PaymentToken for Stripe {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Stripe
{
    fn get_headers(
        &self,
        req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            TokenizationType::get_content_type(self).to_string().into(),
        )];
        if let Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(
            stripe_split_payment,
        )) = &req.request.split_payments
        {
            if stripe_split_payment.charge_type
                == PaymentChargeType::Stripe(StripeChargeType::Direct)
            {
                let mut customer_account_header = vec![(
                    STRIPE_COMPATIBLE_CONNECT_ACCOUNT.to_string(),
                    stripe_split_payment
                        .transfer_account_id
                        .clone()
                        .into_masked(),
                )];
                header.append(&mut customer_account_header);
            }
        }
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        if matches!(
            req.request.split_payments,
            Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(_))
        ) {
            return Ok(format!(
                "{}{}",
                self.base_url(connectors),
                "v1/payment_methods"
            ));
        }
        Ok(format!("{}{}", self.base_url(connectors), "v1/tokens"))
    }

    fn get_request_body(
        &self,
        req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::TokenRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&TokenizationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(TokenizationType::get_headers(self, req, connectors)?)
                .set_body(TokenizationType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &TokenizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<TokenizationRouterData, ConnectorError>
    where
        PaymentsResponseData: Clone,
    {
        let response: stripe::StripeTokenResponse = res
            .response
            .parse_struct("StripeTokenResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl api::MandateSetup for Stripe {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Stripe {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            Self::common_get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;

        if let Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(
            stripe_split_payment,
        )) = &req.request.split_payments
        {
            transformers::transform_headers_for_connect_platform(
                stripe_split_payment.charge_type.clone(),
                stripe_split_payment.transfer_account_id.clone(),
                &mut header,
            );
        }

        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let id = req.request.connector_transaction_id.as_str();
        Ok(format!(
            "{}{}/{}/capture",
            self.base_url(connectors),
            "v1/payment_intents",
            id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let connector_req = stripe::CaptureRequest::try_from(amount)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
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
    ) -> CustomResult<PaymentsCaptureRouterData, ConnectorError>
    where
        PaymentsCaptureData: Clone,
        PaymentsResponseData: Clone,
    {
        let response: stripe::PaymentIntentResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        let response_integrity_object = get_capture_integrity_object(
            self.amount_converter,
            response.amount_received,
            response.currency.clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(ConnectorError::ResponseHandlingFailed);

        new_router_data.map(|mut router_data| {
            router_data.request.integrity_object = Some(response_integrity_object);
            router_data
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Stripe {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            PaymentsSyncType::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);

        if let Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(
            stripe_split_payment,
        )) = &req.request.split_payments
        {
            transformers::transform_headers_for_connect_platform(
                stripe_split_payment.charge_type.clone(),
                stripe_split_payment.transfer_account_id.clone(),
                &mut header,
            );
        }
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let id = req.request.connector_transaction_id.clone();

        match id.get_connector_transaction_id() {
            Ok(x) if x.starts_with("set") => Ok(format!(
                "{}{}/{}?expand[0]=latest_attempt", // expand latest attempt to extract payment checks and three_d_secure data
                self.base_url(connectors),
                "v1/setup_intents",
                x,
            )),
            Ok(x) => Ok(format!(
                "{}{}/{}{}",
                self.base_url(connectors),
                "v1/payment_intents",
                x,
                "?expand[0]=latest_charge" //updated payment_id(if present) reside inside latest_charge field
            )),
            x => x.change_context(ConnectorError::MissingConnectorTransactionID),
        }
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
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
    ) -> CustomResult<PaymentsSyncRouterData, ConnectorError>
    where
        PaymentsResponseData: Clone,
    {
        let id = data.request.connector_transaction_id.clone();
        match id.get_connector_transaction_id() {
            Ok(x) if x.starts_with("set") => {
                let response: stripe::SetupIntentResponse = res
                    .response
                    .parse_struct("SetupIntentSyncResponse")
                    .change_context(ConnectorError::ResponseDeserializationFailed)?;

                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);

                RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            Ok(_) => {
                let response: stripe::PaymentIntentSyncResponse = res
                    .response
                    .parse_struct("PaymentIntentSyncResponse")
                    .change_context(ConnectorError::ResponseDeserializationFailed)?;

                let response_integrity_object = get_sync_integrity_object(
                    self.amount_converter,
                    response.amount,
                    response.currency.clone(),
                )?;

                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);

                let new_router_data = RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                });
                new_router_data.map(|mut router_data| {
                    router_data.request.integrity_object = Some(response_integrity_object);
                    router_data
                })
            }
            Err(err) => Err(err).change_context(ConnectorError::MissingConnectorTransactionID),
        }
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

#[async_trait::async_trait]
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Stripe {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];

        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);

        if let Some(id) = get_stripe_compatible_connect_account_header(req)? {
            let mut customer_account_header = vec![(
                STRIPE_COMPATIBLE_CONNECT_ACCOUNT.to_string(),
                id.into_masked(),
            )];
            header.append(&mut customer_account_header);
        }
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "v1/payment_intents"
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_req = stripe::PaymentIntentRequest::try_from((req, amount))?;

        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
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
    ) -> CustomResult<PaymentsAuthorizeRouterData, ConnectorError> {
        let response: stripe::PaymentIntentResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        let response_integrity_object = get_authorise_integrity_object(
            self.amount_converter,
            response.amount,
            response.currency.clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(ConnectorError::ResponseHandlingFailed);

        new_router_data.map(|mut router_data| {
            router_data.request.integrity_object = Some(response_integrity_object);
            router_data
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl PaymentIncrementalAuthorization for Stripe {}

impl
    ConnectorIntegration<
        IncrementalAuthorization,
        PaymentsIncrementalAuthorizationData,
        PaymentsResponseData,
    > for Stripe
{
    fn get_headers(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_http_method(&self) -> Method {
        Method::Post
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}v1/payment_intents/{}/increment_authorization",
            self.base_url(connectors),
            req.request.connector_transaction_id,
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            MinorUnit::new(req.request.total_amount),
            req.request.currency,
        )?;
        let connector_req = stripe::StripeIncrementalAuthRequest { amount }; // Incremental authorization can be done a maximum of 10 times in Stripe

        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsIncrementalAuthorizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
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
        ConnectorError,
    > {
        let response: stripe::PaymentIntentResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>
    for Stripe
{
    fn get_headers(
        &self,
        req: &RouterData<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        req: &PaymentsUpdateMetadataRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let payment_id = &req.request.connector_transaction_id;
        Ok(format!(
            "{}v1/payment_intents/{}",
            self.base_url(connectors),
            payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsUpdateMetadataRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::UpdateMetadataRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsUpdateMetadataRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PaymentsUpdateMetadataType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PaymentsUpdateMetadataType::get_headers(
                self, req, connectors,
            )?)
            .set_body(PaymentsUpdateMetadataType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsUpdateMetadataRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsUpdateMetadataRouterData, ConnectorError> {
        router_env::logger::debug!("skipped parsing of the response");
        // If 200 status code, then metadata was updated successfully.
        let status = if res.status_code == 200 {
            PaymentResourceUpdateStatus::Success
        } else {
            PaymentResourceUpdateStatus::Failure
        };
        Ok(PaymentsUpdateMetadataRouterData {
            response: Ok(PaymentsResponseData::PaymentResourceUpdateResponse { status }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Stripe {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            PaymentsVoidType::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;

        if let Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(
            stripe_split_payment,
        )) = &req.request.split_payments
        {
            transformers::transform_headers_for_connect_platform(
                stripe_split_payment.charge_type.clone(),
                stripe_split_payment.transfer_account_id.clone(),
                &mut header,
            );
        }

        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let payment_id = &req.request.connector_transaction_id;
        Ok(format!(
            "{}v1/payment_intents/{}/cancel",
            self.base_url(connectors),
            payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::CancelRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PaymentsVoidType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
            .set_body(PaymentsVoidType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, ConnectorError> {
        let response: stripe::PaymentIntentResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

type Verify = dyn ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Stripe {
    fn get_headers(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            Verify::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;

        if let Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(
            stripe_split_payment,
        )) = &req.request.split_payments
        {
            transformers::transform_headers_for_connect_platform(
                stripe_split_payment.charge_type.clone(),
                stripe_split_payment.transfer_account_id.clone(),
                &mut header,
            );
        }
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "v1/setup_intents"
        ))
    }

    fn get_request_body(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::SetupIntentRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&Verify::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(Verify::get_headers(self, req, connectors)?)
                .set_body(Verify::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        ConnectorError,
    >
    where
        SetupMandate: Clone,
        SetupMandateRequestData: Clone,
        PaymentsResponseData: Clone,
    {
        let response: stripe::SetupIntentResponse = res
            .response
            .parse_struct("SetupIntentResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl api::Refund for Stripe {}
impl api::RefundExecute for Stripe {}
impl api::RefundSync for Stripe {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Stripe {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            RefundExecuteType::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);

        if let Some(SplitRefundsRequest::StripeSplitRefund(ref stripe_split_refund)) =
            req.request.split_refunds.as_ref()
        {
            match &stripe_split_refund.charge_type {
                PaymentChargeType::Stripe(stripe_charge) => {
                    if stripe_charge == &StripeChargeType::Direct {
                        let mut customer_account_header = vec![(
                            STRIPE_COMPATIBLE_CONNECT_ACCOUNT.to_string(),
                            stripe_split_refund
                                .transfer_account_id
                                .clone()
                                .into_masked(),
                        )];
                        header.append(&mut customer_account_header);
                    }
                }
            }
        }
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v1/refunds"))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let request_body = match req.request.split_refunds.as_ref() {
            Some(SplitRefundsRequest::StripeSplitRefund(_)) => RequestContent::FormUrlEncoded(
                Box::new(stripe::ChargeRefundRequest::try_from(req)?),
            ),
            _ => RequestContent::FormUrlEncoded(Box::new(stripe::RefundRequest::try_from((
                req,
                refund_amount,
            ))?)),
        };
        Ok(request_body)
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(RefundExecuteType::get_headers(self, req, connectors)?)
            .set_body(RefundExecuteType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, ConnectorError> {
        let response: stripe::RefundResponse =
            res.response
                .parse_struct("Stripe RefundResponse")
                .change_context(ConnectorError::ResponseDeserializationFailed)?;

        let response_integrity_object = get_refund_integrity_object(
            self.amount_converter,
            response.amount,
            response.currency.clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
            })
            .change_context(ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Stripe {
    fn get_headers(
        &self,
        req: &RouterData<RSync, RefundsData, RefundsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            RefundSyncType::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);

        if let Some(SplitRefundsRequest::StripeSplitRefund(ref stripe_refund)) =
            req.request.split_refunds.as_ref()
        {
            transformers::transform_headers_for_connect_platform(
                stripe_refund.charge_type.clone(),
                stripe_refund.transfer_account_id.clone(),
                &mut header,
            );
        }
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<RSync>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let id = req.request.get_connector_refund_id()?;
        Ok(format!("{}v1/refunds/{}", self.base_url(connectors), id))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<RSync>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &RefundsRouterData<RSync>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<RSync, RefundsData, RefundsResponseData>, ConnectorError> {
        let response: stripe::RefundResponse =
            res.response
                .parse_struct("Stripe RefundResponse")
                .change_context(ConnectorError::ResponseDeserializationFailed)?;

        let response_integrity_object = get_refund_integrity_object(
            self.amount_converter,
            response.amount,
            response.currency.clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
            })
            .change_context(ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl UploadFile for Stripe {}

#[async_trait::async_trait]
impl FileUpload for Stripe {
    fn validate_file_upload(
        &self,
        purpose: FilePurpose,
        file_size: i32,
        file_type: mime::Mime,
    ) -> CustomResult<(), ConnectorError> {
        match purpose {
            FilePurpose::DisputeEvidence => {
                let supported_file_types = ["image/jpeg", "image/png", "application/pdf"];
                // 5 Megabytes (MB)
                if file_size > 5000000 {
                    Err(ConnectorError::FileValidationFailed {
                        reason: "file_size exceeded the max file size of 5MB".to_owned(),
                    })?
                }
                if !supported_file_types.contains(&file_type.to_string().as_str()) {
                    Err(ConnectorError::FileValidationFailed {
                        reason: "file_type does not match JPEG, JPG, PNG, or PDF format".to_owned(),
                    })?
                }
            }
        }
        Ok(())
    }
}

impl ConnectorIntegration<Upload, UploadFileRequestData, UploadFileResponse> for Stripe {
    fn get_headers(
        &self,
        req: &RouterData<Upload, UploadFileRequestData, UploadFileResponse>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_content_type(&self) -> &'static str {
        "multipart/form-data"
    }

    fn get_url(
        &self,
        _req: &UploadFileRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}{}",
            connectors.stripe.base_url_file_upload, "v1/files"
        ))
    }

    fn get_request_body(
        &self,
        req: &UploadFileRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        transformers::construct_file_upload_request(req.clone())
    }

    fn build_request(
        &self,
        req: &UploadFileRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&UploadFileType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(UploadFileType::get_headers(self, req, connectors)?)
                .set_body(UploadFileType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &UploadFileRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<Upload, UploadFileRequestData, UploadFileResponse>, ConnectorError>
    {
        let response: stripe::FileUploadResponse = res
            .response
            .parse_struct("Stripe FileUploadResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        Ok(UploadFileRouterData {
            response: Ok(UploadFileResponse {
                provider_file_id: response.file_id,
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl RetrieveFile for Stripe {}

impl ConnectorIntegration<Retrieve, RetrieveFileRequestData, RetrieveFileResponse> for Stripe {
    fn get_headers(
        &self,
        req: &RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_url(
        &self,
        req: &RetrieveFileRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}v1/files/{}/contents",
            connectors.stripe.base_url_file_upload, req.request.provider_file_id
        ))
    }

    fn build_request(
        &self,
        req: &RetrieveFileRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&RetrieveFileType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RetrieveFileType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &RetrieveFileRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RetrieveFileRouterData, ConnectorError> {
        let response = res.response;

        event_builder.map(|event| event.set_response_body(&serde_json::json!({"connector_response_type": "file", "status_code": res.status_code})));
        router_env::logger::info!(connector_response_type=?"file");

        Ok(RetrieveFileRouterData {
            response: Ok(RetrieveFileResponse {
                file_data: response.to_vec(),
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

impl SubmitEvidence for Stripe {}

impl ConnectorIntegration<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse> for Stripe {
    fn get_headers(
        &self,
        req: &SubmitEvidenceRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            CONTENT_TYPE.to_string(),
            SubmitEvidenceType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        req: &SubmitEvidenceRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "v1/disputes/",
            req.request.connector_dispute_id
        ))
    }

    fn get_request_body(
        &self,
        req: &SubmitEvidenceRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::Evidence::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &SubmitEvidenceRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&SubmitEvidenceType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(SubmitEvidenceType::get_headers(self, req, connectors)?)
            .set_body(SubmitEvidenceType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &SubmitEvidenceRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SubmitEvidenceRouterData, ConnectorError> {
        let response: stripe::DisputeObj = res
            .response
            .parse_struct("Stripe DisputeObj")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        Ok(SubmitEvidenceRouterData {
            response: Ok(SubmitEvidenceResponse {
                dispute_status: api_models::enums::DisputeStatus::DisputeChallenged,
                connector_status: Some(response.status),
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .clone()
                    .map(|decline_code| {
                        format!("message - {message}, decline_code - {decline_code}")
                    })
                    .unwrap_or(message)
            }),
            attempt_status: None,
            connector_transaction_id: response.error.payment_intent.map(|pi| pi.id),
            network_advice_code: response.error.network_advice_code,
            network_decline_code: response.error.network_decline_code,
            network_error_message: response.error.decline_code.or(response.error.advice_code),
            connector_metadata: None,
        })
    }
}

fn get_signature_elements_from_header(
    headers: &actix_web::http::header::HeaderMap,
) -> CustomResult<HashMap<String, Vec<u8>>, ConnectorError> {
    let security_header = headers
        .get("Stripe-Signature")
        .map(|header_value| {
            header_value
                .to_str()
                .map(String::from)
                .map_err(|_| ConnectorError::WebhookSignatureNotFound)
        })
        .ok_or(ConnectorError::WebhookSignatureNotFound)??;

    let props = security_header.split(',').collect::<Vec<&str>>();
    let mut security_header_kvs: HashMap<String, Vec<u8>> = HashMap::with_capacity(props.len());

    for prop_str in &props {
        let (prop_key, prop_value) = prop_str
            .split_once('=')
            .ok_or(ConnectorError::WebhookSourceVerificationFailed)?;

        security_header_kvs.insert(prop_key.to_string(), prop_value.bytes().collect());
    }

    Ok(security_header_kvs)
}

#[async_trait::async_trait]
impl IncomingWebhook for Stripe {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        let mut security_header_kvs = get_signature_elements_from_header(request.headers)?;

        let signature = security_header_kvs
            .remove("v1")
            .ok_or(ConnectorError::WebhookSignatureNotFound)?;

        hex::decode(signature).change_context(ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        let mut security_header_kvs = get_signature_elements_from_header(request.headers)?;

        let timestamp = security_header_kvs
            .remove("t")
            .ok_or(ConnectorError::WebhookSignatureNotFound)?;

        Ok(format!(
            "{}.{}",
            String::from_utf8_lossy(&timestamp),
            String::from_utf8_lossy(request.body)
        )
        .into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        let details: stripe::WebhookEvent = request
            .body
            .parse_struct("WebhookEvent")
            .change_context(ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(match details.event_data.event_object.object {
            stripe::WebhookEventObjectType::PaymentIntent => {
                match details
                    .event_data
                    .event_object
                    .metadata
                    .and_then(|meta_data| meta_data.order_id)
                {
                    // if order_id is present
                    Some(order_id) => api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::PaymentAttemptId(order_id),
                    ),
                    // else used connector_transaction_id
                    None => api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::ConnectorTransactionId(
                            details.event_data.event_object.id,
                        ),
                    ),
                }
            }
            stripe::WebhookEventObjectType::Charge => {
                match details
                    .event_data
                    .event_object
                    .metadata
                    .and_then(|meta_data| meta_data.order_id)
                {
                    // if order_id is present
                    Some(order_id) => api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::PaymentAttemptId(order_id),
                    ),
                    // else used connector_transaction_id
                    None => api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::ConnectorTransactionId(
                            details
                                .event_data
                                .event_object
                                .payment_intent
                                .ok_or(ConnectorError::WebhookReferenceIdNotFound)?,
                        ),
                    ),
                }
            }
            stripe::WebhookEventObjectType::Dispute => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        details
                            .event_data
                            .event_object
                            .payment_intent
                            .ok_or(ConnectorError::WebhookReferenceIdNotFound)?,
                    ),
                )
            }
            stripe::WebhookEventObjectType::Source => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::PreprocessingId(
                        details.event_data.event_object.id,
                    ),
                )
            }
            stripe::WebhookEventObjectType::Refund => {
                match details
                    .event_data
                    .event_object
                    .metadata
                    .clone()
                    .and_then(|meta_data| meta_data.order_id)
                {
                    // if meta_data is present
                    Some(order_id) => {
                        // Issue: 2076
                        match details
                            .event_data
                            .event_object
                            .metadata
                            .and_then(|meta_data| meta_data.is_refund_id_as_reference)
                        {
                            // if the order_id is refund_id
                            Some(_) => api_models::webhooks::ObjectReferenceId::RefundId(
                                api_models::webhooks::RefundIdType::RefundId(order_id),
                            ),
                            // if the order_id is payment_id
                            // since payment_id was being passed before the deployment of this pr
                            _ => api_models::webhooks::ObjectReferenceId::RefundId(
                                api_models::webhooks::RefundIdType::ConnectorRefundId(
                                    details.event_data.event_object.id,
                                ),
                            ),
                        }
                    }
                    // else use connector_transaction_id
                    None => api_models::webhooks::ObjectReferenceId::RefundId(
                        api_models::webhooks::RefundIdType::ConnectorRefundId(
                            details.event_data.event_object.id,
                        ),
                    ),
                }
            }
        })
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        let details: stripe::WebhookEventTypeBody = request
            .body
            .parse_struct("WebhookEventTypeBody")
            .change_context(ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(match details.event_type {
            stripe::WebhookEventType::PaymentIntentFailed => {
                IncomingWebhookEvent::PaymentIntentFailure
            }
            stripe::WebhookEventType::PaymentIntentSucceed => {
                IncomingWebhookEvent::PaymentIntentSuccess
            }
            stripe::WebhookEventType::PaymentIntentCanceled => {
                IncomingWebhookEvent::PaymentIntentCancelled
            }
            stripe::WebhookEventType::PaymentIntentAmountCapturableUpdated => {
                IncomingWebhookEvent::PaymentIntentAuthorizationSuccess
            }
            stripe::WebhookEventType::ChargeSucceeded => {
                if let Some(stripe::WebhookPaymentMethodDetails {
                    payment_method:
                        stripe::WebhookPaymentMethodType::AchCreditTransfer
                        | stripe::WebhookPaymentMethodType::MultibancoBankTransfers,
                }) = details.event_data.event_object.payment_method_details
                {
                    IncomingWebhookEvent::PaymentIntentSuccess
                } else {
                    IncomingWebhookEvent::EventNotSupported
                }
            }
            stripe::WebhookEventType::ChargeRefundUpdated => details
                .event_data
                .event_object
                .status
                .map(|status| match status {
                    stripe::WebhookEventStatus::Succeeded => IncomingWebhookEvent::RefundSuccess,
                    stripe::WebhookEventStatus::Failed => IncomingWebhookEvent::RefundFailure,
                    _ => IncomingWebhookEvent::EventNotSupported,
                })
                .unwrap_or(IncomingWebhookEvent::EventNotSupported),
            stripe::WebhookEventType::SourceChargeable => IncomingWebhookEvent::SourceChargeable,
            stripe::WebhookEventType::DisputeCreated => IncomingWebhookEvent::DisputeOpened,
            stripe::WebhookEventType::DisputeClosed => IncomingWebhookEvent::DisputeCancelled,
            stripe::WebhookEventType::DisputeUpdated => details
                .event_data
                .event_object
                .status
                .map(Into::into)
                .unwrap_or(IncomingWebhookEvent::EventNotSupported),
            stripe::WebhookEventType::PaymentIntentPartiallyFunded => {
                IncomingWebhookEvent::PaymentIntentPartiallyFunded
            }
            stripe::WebhookEventType::PaymentIntentRequiresAction => {
                IncomingWebhookEvent::PaymentActionRequired
            }
            stripe::WebhookEventType::ChargeDisputeFundsWithdrawn => {
                IncomingWebhookEvent::DisputeLost
            }
            stripe::WebhookEventType::ChargeDisputeFundsReinstated => {
                IncomingWebhookEvent::DisputeWon
            }
            stripe::WebhookEventType::Unknown
            | stripe::WebhookEventType::ChargeCaptured
            | stripe::WebhookEventType::ChargeExpired
            | stripe::WebhookEventType::ChargeFailed
            | stripe::WebhookEventType::ChargePending
            | stripe::WebhookEventType::ChargeUpdated
            | stripe::WebhookEventType::ChargeRefunded
            | stripe::WebhookEventType::PaymentIntentCreated
            | stripe::WebhookEventType::PaymentIntentProcessing
            | stripe::WebhookEventType::SourceTransactionCreated => {
                IncomingWebhookEvent::EventNotSupported
            }
        })
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        let details: stripe::WebhookEvent = request
            .body
            .parse_struct("WebhookEvent")
            .change_context(ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(Box::new(details.event_data.event_object))
    }
    fn get_dispute_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<DisputePayload, ConnectorError> {
        let details: stripe::WebhookEvent = request
            .body
            .parse_struct("WebhookEvent")
            .change_context(ConnectorError::WebhookBodyDecodingFailed)?;
        let amt = details.event_data.event_object.amount.ok_or_else(|| {
            ConnectorError::MissingRequiredField {
                field_name: "amount",
            }
        })?;

        Ok(DisputePayload {
            amount: utils::convert_amount(
                self.amount_converter_webhooks,
                amt,
                details.event_data.event_object.currency,
            )?,
            currency: details.event_data.event_object.currency,
            dispute_stage: api_models::enums::DisputeStage::Dispute,
            connector_dispute_id: details.event_data.event_object.id,
            connector_reason: details.event_data.event_object.reason,
            connector_reason_code: None,
            challenge_required_by: details
                .event_data
                .event_object
                .evidence_details
                .map(|payload| payload.due_by),
            connector_status: details
                .event_data
                .event_object
                .status
                .ok_or(ConnectorError::WebhookResourceObjectNotFound)?
                .to_string(),
            created_at: Some(details.event_data.event_object.created),
            updated_at: None,
        })
    }
}

impl ConnectorRedirectResponse for Stripe {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: PaymentAction,
    ) -> CustomResult<CallConnectorAction, ConnectorError> {
        match action {
            PaymentAction::PSync
            | PaymentAction::CompleteAuthorize
            | PaymentAction::PaymentAuthenticateCompleteAuthorize => {
                Ok(CallConnectorAction::Trigger)
            }
        }
    }
}

impl api::Payouts for Stripe {}
#[cfg(feature = "payouts")]
impl api::PayoutCancel for Stripe {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for Stripe {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Stripe {}
#[cfg(feature = "payouts")]
impl api::PayoutRecipient for Stripe {}
#[cfg(feature = "payouts")]
impl api::PayoutRecipientAccount for Stripe {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCancel, PayoutsData, PayoutsResponseData> for Stripe {
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let transfer_id = req.request.get_transfer_id()?;
        Ok(format!(
            "{}v1/transfers/{}/reversals",
            connectors.stripe.base_url, transfer_id
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, _connectors)
    }

    fn get_request_body(
        &self,
        req: &RouterData<PoCancel, PayoutsData, PayoutsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::StripeConnectReversalRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutCancelType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutCancelType::get_headers(self, req, connectors)?)
            .set_body(PayoutCancelType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoCancel>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoCancel>, ConnectorError> {
        let response: stripe::StripeConnectReversalResponse = res
            .response
            .parse_struct("StripeConnectReversalResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData> for Stripe {
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}v1/transfers", connectors.stripe.base_url))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::StripeConnectPayoutCreateRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutCreateType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutCreateType::get_headers(self, req, connectors)?)
            .set_body(PayoutCreateType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoCreate>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoCreate>, ConnectorError> {
        let response: stripe::StripeConnectPayoutCreateResponse = res
            .response
            .parse_struct("StripeConnectPayoutCreateResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Stripe {
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}v1/payouts", connectors.stripe.base_url,))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut headers = self.build_headers(req, connectors)?;
        let customer_account = req.get_connector_customer_id()?;
        let mut customer_account_header = vec![(
            STRIPE_COMPATIBLE_CONNECT_ACCOUNT.to_string(),
            customer_account.into_masked(),
        )];
        headers.append(&mut customer_account_header);
        Ok(headers)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::StripeConnectPayoutFulfillRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
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
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, ConnectorError> {
        let response: stripe::StripeConnectPayoutFulfillResponse = res
            .response
            .parse_struct("StripeConnectPayoutFulfillResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoRecipient, PayoutsData, PayoutsResponseData> for Stripe {
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}v1/accounts", connectors.stripe.base_url))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::StripeConnectRecipientCreateRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoRecipient>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutRecipientType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutRecipientType::get_headers(self, req, connectors)?)
            .set_body(PayoutRecipientType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoRecipient>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoRecipient>, ConnectorError> {
        let response: stripe::StripeConnectRecipientCreateResponse = res
            .response
            .parse_struct("StripeConnectRecipientCreateResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoRecipientAccount, PayoutsData, PayoutsResponseData> for Stripe {
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let connector_customer_id = req.get_connector_customer_id()?;
        Ok(format!(
            "{}v1/accounts/{}/external_accounts",
            connectors.stripe.base_url, connector_customer_id
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = stripe::StripeConnectRecipientAccountCreateRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoRecipientAccount>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutRecipientAccountType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutRecipientAccountType::get_headers(
                self, req, connectors,
            )?)
            .set_body(PayoutRecipientAccountType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoRecipientAccount>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoRecipientAccount>, ConnectorError> {
        let response: stripe::StripeConnectRecipientAccountCreateResponse = res
            .response
            .parse_struct("StripeConnectRecipientAccountCreateResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

static STRIPE_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let default_capture_methods = vec![
        CaptureMethod::Automatic,
        CaptureMethod::Manual,
        CaptureMethod::SequentialAutomatic,
    ];

    let automatic_capture_supported =
        vec![CaptureMethod::Automatic, CaptureMethod::SequentialAutomatic];

    let supported_card_network = vec![
        common_enums::CardNetwork::Visa,
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::Discover,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::DinersClub,
        common_enums::CardNetwork::UnionPay,
    ];

    let mut stripe_supported_payment_methods = SupportedPaymentMethods::new();

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::Card,
        PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    },
                ),
            ),
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::Card,
        PaymentMethodType::Debit,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    },
                ),
            ),
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::PayLater,
        PaymentMethodType::Klarna,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::PayLater,
        PaymentMethodType::Affirm,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::PayLater,
        PaymentMethodType::AfterpayClearpay,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::Wallet,
        PaymentMethodType::AliPay,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::Wallet,
        PaymentMethodType::AmazonPay,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::Wallet,
        PaymentMethodType::ApplePay,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::Wallet,
        PaymentMethodType::GooglePay,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::Wallet,
        PaymentMethodType::WeChatPay,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::Wallet,
        PaymentMethodType::Cashapp,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::Wallet,
        PaymentMethodType::RevolutPay,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankDebit,
        PaymentMethodType::Becs,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankDebit,
        PaymentMethodType::Ach,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankDebit,
        PaymentMethodType::Sepa,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankDebit,
        PaymentMethodType::Bacs,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankRedirect,
        PaymentMethodType::BancontactCard,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Blik,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankTransfer,
        PaymentMethodType::Ach,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankTransfer,
        PaymentMethodType::SepaBankTransfer,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankTransfer,
        PaymentMethodType::Bacs,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankTransfer,
        PaymentMethodType::Multibanco,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Giropay,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::NotSupported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Ideal,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Przelewy24,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Eps,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankRedirect,
        PaymentMethodType::OnlineBankingFpx,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods.add(
        common_enums::PaymentMethod::BankRedirect,
        PaymentMethodType::Sofort,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: automatic_capture_supported.clone(),
            specific_features: None,
        },
    );

    stripe_supported_payment_methods
});

static STRIPE_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Stripe",
    description: "Stripe is a payment processing platform that provides businesses with tools and APIs to accept payments online and manage their financial infrastructure",
    connector_type: common_enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: common_enums::ConnectorIntegrationStatus::Live,
};

static STRIPE_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 3] = [
    common_enums::EventClass::Payments,
    common_enums::EventClass::Refunds,
    common_enums::EventClass::Disputes,
];

impl ConnectorSpecifications for Stripe {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&STRIPE_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*STRIPE_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&STRIPE_SUPPORTED_WEBHOOK_FLOWS)
    }

    fn should_call_connector_customer(
        &self,
        _payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> bool {
        true
    }
}
