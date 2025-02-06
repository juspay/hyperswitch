// FIXME: Why were these data types grouped this way?
//
// Folder `types` is strange for Rust ecosystem, nevertheless it might be okay.
// But folder `enum` is even more strange I unlikely okay. Why should not we introduce folders `type`, `structs` and `traits`? :)
// Is it better to split data types according to business logic instead.
// For example, customers/address/dispute/mandate is "models".
// Separation of concerns instead of separation of forms.

pub mod api;
pub mod authentication;
pub mod domain;
#[cfg(feature = "frm")]
pub mod fraud_check;
pub mod payment_methods;
pub mod pm_auth;
use masking::Secret;
pub mod storage;
pub mod transformers;
use std::marker::PhantomData;

pub use api_models::{enums::Connector, mandates};
#[cfg(feature = "payouts")]
pub use api_models::{enums::PayoutConnectors, payouts as payout_types};
#[cfg(feature = "v2")]
use common_utils::errors::CustomResult;
pub use common_utils::{pii, pii::Email, request::RequestContent, types::MinorUnit};
#[cfg(feature = "v2")]
use error_stack::ResultExt;
#[cfg(feature = "frm")]
pub use hyperswitch_domain_models::router_data_v2::FrmFlowData;
use hyperswitch_domain_models::router_flow_types::{
    self,
    access_token_auth::AccessTokenAuth,
    dispute::{Accept, Defend, Evidence},
    files::{Retrieve, Upload},
    mandate_revoke::MandateRevoke,
    payments::{
        Approve, Authorize, AuthorizeSessionToken, Balance, CalculateTax, Capture,
        CompleteAuthorize, CreateConnectorCustomer, IncrementalAuthorization, InitPayment, PSync,
        PostProcessing, PostSessionTokens, PreProcessing, Reject, SdkSessionUpdate, Session,
        SetupMandate, Void,
    },
    refunds::{Execute, RSync},
    webhooks::VerifyWebhookSource,
};
pub use hyperswitch_domain_models::{
    payment_address::PaymentAddress,
    router_data::{
        AccessToken, AdditionalPaymentMethodConnectorResponse, ApplePayCryptogramData,
        ApplePayPredecryptData, ConnectorAuthType, ConnectorResponseData, ErrorResponse,
        GooglePayDecryptedData, GooglePayPaymentMethodDetails, PaymentMethodBalance,
        PaymentMethodToken, RecurringMandatePaymentData, RouterData,
    },
    router_data_v2::{
        AccessTokenFlowData, DisputesFlowData, ExternalAuthenticationFlowData, FilesFlowData,
        MandateRevokeFlowData, PaymentFlowData, RefundFlowData, RouterDataV2, UasFlowData,
        WebhookSourceVerifyData,
    },
    router_request_types::{
        unified_authentication_service::{
            UasAuthenticationRequestData, UasAuthenticationResponseData,
            UasPostAuthenticationRequestData, UasPreAuthenticationRequestData,
        },
        AcceptDisputeRequestData, AccessTokenRequestData, AuthorizeSessionTokenData,
        BrowserInformation, ChargeRefunds, ChargeRefundsOptions, CompleteAuthorizeData,
        CompleteAuthorizeRedirectResponse, ConnectorCustomerData, DefendDisputeRequestData,
        DestinationChargeRefund, DirectChargeRefund, MandateRevokeRequestData,
        MultipleCaptureRequestData, PaymentMethodTokenizationData, PaymentsApproveData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsIncrementalAuthorizationData, PaymentsPostProcessingData,
        PaymentsPostSessionTokensData, PaymentsPreProcessingData, PaymentsRejectData,
        PaymentsSessionData, PaymentsSyncData, PaymentsTaxCalculationData, RefundsData, ResponseId,
        RetrieveFileRequestData, SdkPaymentsSessionUpdateData, SetupMandateRequestData,
        SplitRefundsRequest, SubmitEvidenceRequestData, SyncRequestType, UploadFileRequestData,
        VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, CaptureSyncResponse, DefendDisputeResponse, MandateReference,
        MandateRevokeResponseData, PaymentsResponseData, PreprocessingResponseId,
        RefundsResponseData, RetrieveFileResponse, SubmitEvidenceResponse,
        TaxCalculationResponseData, UploadFileResponse, VerifyWebhookSourceResponseData,
        VerifyWebhookStatus,
    },
};
#[cfg(feature = "payouts")]
pub use hyperswitch_domain_models::{
    router_data_v2::PayoutFlowData, router_request_types::PayoutsData,
    router_response_types::PayoutsResponseData,
};
pub use hyperswitch_interfaces::types::{
    AcceptDisputeType, ConnectorCustomerType, DefendDisputeType, IncrementalAuthorizationType,
    MandateRevokeType, PaymentsAuthorizeType, PaymentsBalanceType, PaymentsCaptureType,
    PaymentsCompleteAuthorizeType, PaymentsInitType, PaymentsPostProcessingType,
    PaymentsPostSessionTokensType, PaymentsPreAuthorizeType, PaymentsPreProcessingType,
    PaymentsSessionType, PaymentsSyncType, PaymentsVoidType, RefreshTokenType, RefundExecuteType,
    RefundSyncType, Response, RetrieveFileType, SdkSessionUpdateType, SetupMandateType,
    SubmitEvidenceType, TokenizationType, UploadFileType, VerifyWebhookSourceType,
};
#[cfg(feature = "payouts")]
pub use hyperswitch_interfaces::types::{
    PayoutCancelType, PayoutCreateType, PayoutEligibilityType, PayoutFulfillType, PayoutQuoteType,
    PayoutRecipientAccountType, PayoutRecipientType, PayoutSyncType,
};

pub use crate::core::payments::CustomerDetails;
#[cfg(feature = "payouts")]
use crate::{
    connector::utils::missing_field_err,
    core::utils::IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_PAYOUTS_FLOW,
};
use crate::{
    consts,
    core::{
        errors::{self},
        payments::{OperationSessionGetters, PaymentData},
    },
    services,
    types::transformers::{ForeignFrom, ForeignTryFrom},
};

pub type PaymentsAuthorizeRouterData =
    RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsPreProcessingRouterData =
    RouterData<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>;
pub type PaymentsPostProcessingRouterData =
    RouterData<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>;
pub type PaymentsAuthorizeSessionTokenRouterData =
    RouterData<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>;
pub type PaymentsCompleteAuthorizeRouterData =
    RouterData<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>;
pub type PaymentsInitRouterData =
    RouterData<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsBalanceRouterData =
    RouterData<Balance, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = RouterData<PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData = RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsIncrementalAuthorizationRouterData = RouterData<
    IncrementalAuthorization,
    PaymentsIncrementalAuthorizationData,
    PaymentsResponseData,
>;
pub type PaymentsTaxCalculationRouterData =
    RouterData<CalculateTax, PaymentsTaxCalculationData, TaxCalculationResponseData>;

pub type SdkSessionUpdateRouterData =
    RouterData<SdkSessionUpdate, SdkPaymentsSessionUpdateData, PaymentsResponseData>;

pub type PaymentsPostSessionTokensRouterData =
    RouterData<PostSessionTokens, PaymentsPostSessionTokensData, PaymentsResponseData>;

pub type PaymentsCancelRouterData = RouterData<Void, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsRejectRouterData = RouterData<Reject, PaymentsRejectData, PaymentsResponseData>;
pub type PaymentsApproveRouterData = RouterData<Approve, PaymentsApproveData, PaymentsResponseData>;
pub type PaymentsSessionRouterData = RouterData<Session, PaymentsSessionData, PaymentsResponseData>;
pub type RefundsRouterData<F> = RouterData<F, RefundsData, RefundsResponseData>;
pub type RefundExecuteRouterData = RouterData<Execute, RefundsData, RefundsResponseData>;
pub type RefundSyncRouterData = RouterData<RSync, RefundsData, RefundsResponseData>;
pub type TokenizationRouterData = RouterData<
    router_flow_types::PaymentMethodToken,
    PaymentMethodTokenizationData,
    PaymentsResponseData,
>;
pub type ConnectorCustomerRouterData =
    RouterData<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>;

pub type RefreshTokenRouterData = RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub type PaymentsResponseRouterData<R> =
    ResponseRouterData<Authorize, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<Void, R, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsBalanceResponseRouterData<R> =
    ResponseRouterData<Balance, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<PSync, R, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsSessionResponseRouterData<R> =
    ResponseRouterData<Session, R, PaymentsSessionData, PaymentsResponseData>;
pub type PaymentsInitResponseRouterData<R> =
    ResponseRouterData<InitPayment, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type SdkSessionUpdateResponseRouterData<R> =
    ResponseRouterData<SdkSessionUpdate, R, SdkPaymentsSessionUpdateData, PaymentsResponseData>;
pub type PaymentsCaptureResponseRouterData<R> =
    ResponseRouterData<Capture, R, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsPreprocessingResponseRouterData<R> =
    ResponseRouterData<PreProcessing, R, PaymentsPreProcessingData, PaymentsResponseData>;
pub type TokenizationResponseRouterData<R> =
    ResponseRouterData<PaymentMethodToken, R, PaymentMethodTokenizationData, PaymentsResponseData>;
pub type ConnectorCustomerResponseRouterData<R> =
    ResponseRouterData<CreateConnectorCustomer, R, ConnectorCustomerData, PaymentsResponseData>;

pub type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;

pub type SetupMandateRouterData =
    RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;

pub type AcceptDisputeRouterData =
    RouterData<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>;

pub type VerifyWebhookSourceRouterData = RouterData<
    VerifyWebhookSource,
    VerifyWebhookSourceRequestData,
    VerifyWebhookSourceResponseData,
>;

pub type SubmitEvidenceRouterData =
    RouterData<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>;

pub type UploadFileRouterData = RouterData<Upload, UploadFileRequestData, UploadFileResponse>;

pub type RetrieveFileRouterData =
    RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>;

pub type DefendDisputeRouterData =
    RouterData<Defend, DefendDisputeRequestData, DefendDisputeResponse>;

pub type MandateRevokeRouterData =
    RouterData<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>;

#[cfg(feature = "payouts")]
pub type PayoutsRouterData<F> = RouterData<F, PayoutsData, PayoutsResponseData>;

#[cfg(feature = "payouts")]
pub type PayoutsResponseRouterData<F, R> =
    ResponseRouterData<F, R, PayoutsData, PayoutsResponseData>;

#[cfg(feature = "payouts")]
pub type PayoutActionData = Vec<(
    storage::Payouts,
    storage::PayoutAttempt,
    Option<domain::Customer>,
    Option<api_models::payments::Address>,
)>;

#[cfg(feature = "payouts")]
pub trait PayoutIndividualDetailsExt {
    type Error;
    fn get_external_account_account_holder_type(&self) -> Result<String, Self::Error>;
}

#[cfg(feature = "payouts")]
impl PayoutIndividualDetailsExt for api_models::payouts::PayoutIndividualDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn get_external_account_account_holder_type(&self) -> Result<String, Self::Error> {
        self.external_account_account_holder_type
            .clone()
            .ok_or_else(missing_field_err("external_account_account_holder_type"))
    }
}

pub trait Capturable {
    fn get_captured_amount<F>(&self, _payment_data: &PaymentData<F>) -> Option<i64>
    where
        F: Clone,
    {
        None
    }
    fn get_amount_capturable<F>(
        &self,
        _payment_data: &PaymentData<F>,
        _attempt_status: common_enums::AttemptStatus,
    ) -> Option<i64>
    where
        F: Clone,
    {
        None
    }
}

#[cfg(feature = "v1")]
impl Capturable for PaymentsAuthorizeData {
    fn get_captured_amount<F>(&self, payment_data: &PaymentData<F>) -> Option<i64>
    where
        F: Clone,
    {
        Some(
            payment_data
                .payment_attempt
                .get_total_amount()
                .get_amount_as_i64(),
        )
    }

    fn get_amount_capturable<F>(
        &self,
        payment_data: &PaymentData<F>,
        attempt_status: common_enums::AttemptStatus,
    ) -> Option<i64>
    where
        F: Clone,
    {
        match payment_data.get_capture_method().unwrap_or_default()
        {
            common_enums::CaptureMethod::Automatic|common_enums::CaptureMethod::SequentialAutomatic  => {
                let intent_status = common_enums::IntentStatus::foreign_from(attempt_status);
                match intent_status {
                    common_enums::IntentStatus::Succeeded
                    | common_enums::IntentStatus::Failed
                    | common_enums::IntentStatus::Processing => Some(0),
                    common_enums::IntentStatus::Cancelled
                    | common_enums::IntentStatus::PartiallyCaptured
                    | common_enums::IntentStatus::RequiresCustomerAction
                    | common_enums::IntentStatus::RequiresMerchantAction
                    | common_enums::IntentStatus::RequiresPaymentMethod
                    | common_enums::IntentStatus::RequiresConfirmation
                    | common_enums::IntentStatus::RequiresCapture
                    | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
                }
            },
            common_enums::CaptureMethod::Manual => Some(payment_data.payment_attempt.get_total_amount().get_amount_as_i64()),
            // In case of manual multiple, amount capturable must be inferred from all captures.
            common_enums::CaptureMethod::ManualMultiple |
            // Scheduled capture is not supported as of now
            common_enums::CaptureMethod::Scheduled => None,
        }
    }
}

#[cfg(feature = "v1")]
impl Capturable for PaymentsCaptureData {
    fn get_captured_amount<F>(&self, _payment_data: &PaymentData<F>) -> Option<i64>
    where
        F: Clone,
    {
        Some(self.amount_to_capture)
    }
    fn get_amount_capturable<F>(
        &self,
        _payment_data: &PaymentData<F>,
        attempt_status: common_enums::AttemptStatus,
    ) -> Option<i64>
    where
        F: Clone,
    {
        let intent_status = common_enums::IntentStatus::foreign_from(attempt_status);
        match intent_status {
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::PartiallyCaptured => Some(0),
            common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        }
    }
}

#[cfg(feature = "v1")]
impl Capturable for CompleteAuthorizeData {
    fn get_captured_amount<F>(&self, payment_data: &PaymentData<F>) -> Option<i64>
    where
        F: Clone,
    {
        Some(
            payment_data
                .payment_attempt
                .get_total_amount()
                .get_amount_as_i64(),
        )
    }
    fn get_amount_capturable<F>(
        &self,
        payment_data: &PaymentData<F>,
        attempt_status: common_enums::AttemptStatus,
    ) -> Option<i64>
    where
        F: Clone,
    {
        match payment_data
            .get_capture_method()
            .unwrap_or_default()
        {
            common_enums::CaptureMethod::Automatic | common_enums::CaptureMethod::SequentialAutomatic => {
                let intent_status = common_enums::IntentStatus::foreign_from(attempt_status);
                match intent_status {
                    common_enums::IntentStatus::Succeeded|
                    common_enums::IntentStatus::Failed|
                    common_enums::IntentStatus::Processing => Some(0),
                    common_enums::IntentStatus::Cancelled
                    | common_enums::IntentStatus::PartiallyCaptured
                    | common_enums::IntentStatus::RequiresCustomerAction
                    | common_enums::IntentStatus::RequiresMerchantAction
                    | common_enums::IntentStatus::RequiresPaymentMethod
                    | common_enums::IntentStatus::RequiresConfirmation
                    | common_enums::IntentStatus::RequiresCapture
                    | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
                }
            },
            common_enums::CaptureMethod::Manual => Some(payment_data.payment_attempt.get_total_amount().get_amount_as_i64()),
            // In case of manual multiple, amount capturable must be inferred from all captures.
            common_enums::CaptureMethod::ManualMultiple |
            // Scheduled capture is not supported as of now
            common_enums::CaptureMethod::Scheduled => None,
        }
    }
}

impl Capturable for SetupMandateRequestData {}
impl Capturable for PaymentsTaxCalculationData {}
impl Capturable for SdkPaymentsSessionUpdateData {}
impl Capturable for PaymentsPostSessionTokensData {}
impl Capturable for PaymentsCancelData {
    fn get_captured_amount<F>(&self, payment_data: &PaymentData<F>) -> Option<i64>
    where
        F: Clone,
    {
        // return previously captured amount
        payment_data
            .payment_intent
            .amount_captured
            .map(|amt| amt.get_amount_as_i64())
    }
    fn get_amount_capturable<F>(
        &self,
        _payment_data: &PaymentData<F>,
        attempt_status: common_enums::AttemptStatus,
    ) -> Option<i64>
    where
        F: Clone,
    {
        let intent_status = common_enums::IntentStatus::foreign_from(attempt_status);
        match intent_status {
            common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::PartiallyCaptured => Some(0),
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        }
    }
}
impl Capturable for PaymentsApproveData {}
impl Capturable for PaymentsRejectData {}
impl Capturable for PaymentsSessionData {}
impl Capturable for PaymentsIncrementalAuthorizationData {
    fn get_amount_capturable<F>(
        &self,
        _payment_data: &PaymentData<F>,
        _attempt_status: common_enums::AttemptStatus,
    ) -> Option<i64>
    where
        F: Clone,
    {
        Some(self.total_amount)
    }
}
impl Capturable for PaymentsSyncData {
    #[cfg(feature = "v1")]
    fn get_captured_amount<F>(&self, payment_data: &PaymentData<F>) -> Option<i64>
    where
        F: Clone,
    {
        payment_data
            .payment_attempt
            .amount_to_capture
            .or_else(|| Some(payment_data.payment_attempt.get_total_amount()))
            .map(|amt| amt.get_amount_as_i64())
    }

    #[cfg(feature = "v2")]
    fn get_captured_amount<F>(&self, payment_data: &PaymentData<F>) -> Option<i64>
    where
        F: Clone,
    {
        // TODO: add a getter for this
        payment_data
            .payment_attempt
            .amount_details
            .get_amount_to_capture()
            .or_else(|| Some(payment_data.payment_attempt.get_total_amount()))
            .map(|amt| amt.get_amount_as_i64())
    }

    fn get_amount_capturable<F>(
        &self,
        _payment_data: &PaymentData<F>,
        attempt_status: common_enums::AttemptStatus,
    ) -> Option<i64>
    where
        F: Clone,
    {
        if attempt_status.is_terminal_status() {
            Some(0)
        } else {
            None
        }
    }
}

pub struct AddAccessTokenResult {
    pub access_token_result: Result<Option<AccessToken>, ErrorResponse>,
    pub connector_supports_access_token: bool,
}

pub struct PaymentMethodTokenResult {
    pub payment_method_token_result: Result<Option<String>, ErrorResponse>,
    pub is_payment_method_tokenization_performed: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Redirection {
    Redirect,
    NoRedirect,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PollConfig {
    pub delay_in_secs: i8,
    pub frequency: i8,
}

impl PollConfig {
    pub fn get_poll_config_key(connector: String) -> String {
        format!("poll_config_external_three_ds_{connector}")
    }
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            delay_in_secs: consts::DEFAULT_POLL_DELAY_IN_SECS,
            frequency: consts::DEFAULT_POLL_FREQUENCY,
        }
    }
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub struct RedirectPaymentFlowResponse {
    pub payments_response: api_models::payments::PaymentsResponse,
    pub business_profile: domain::Profile,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct RedirectPaymentFlowResponse<D> {
    pub payment_data: D,
    pub profile: domain::Profile,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub struct AuthenticatePaymentFlowResponse {
    pub payments_response: api_models::payments::PaymentsResponse,
    pub poll_config: PollConfig,
    pub business_profile: domain::Profile,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct ConnectorResponse {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub connector: String,
    pub payment_id: common_utils::id_type::PaymentId,
    pub amount: i64,
    pub connector_transaction_id: String,
    pub return_url: Option<String>,
    pub three_ds_form: Option<services::RedirectForm>,
}

pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: RouterData<Flow, Request, Response>,
    pub http_code: u16,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum RecipientIdType {
    ConnectorId(Secret<String>),
    LockerId(Secret<String>),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantAccountData {
    Iban {
        iban: Secret<String>,
        name: String,
        connector_recipient_id: Option<RecipientIdType>,
    },
    Bacs {
        account_number: Secret<String>,
        sort_code: Secret<String>,
        name: String,
        connector_recipient_id: Option<RecipientIdType>,
    },
}

impl ForeignFrom<MerchantAccountData> for api_models::admin::MerchantAccountData {
    fn foreign_from(from: MerchantAccountData) -> Self {
        match from {
            MerchantAccountData::Iban {
                iban,
                name,
                connector_recipient_id,
            } => Self::Iban {
                iban,
                name,
                connector_recipient_id: match connector_recipient_id {
                    Some(RecipientIdType::ConnectorId(id)) => Some(id.clone()),
                    _ => None,
                },
            },
            MerchantAccountData::Bacs {
                account_number,
                sort_code,
                name,
                connector_recipient_id,
            } => Self::Bacs {
                account_number,
                sort_code,
                name,
                connector_recipient_id: match connector_recipient_id {
                    Some(RecipientIdType::ConnectorId(id)) => Some(id.clone()),
                    _ => None,
                },
            },
        }
    }
}

impl From<api_models::admin::MerchantAccountData> for MerchantAccountData {
    fn from(from: api_models::admin::MerchantAccountData) -> Self {
        match from {
            api_models::admin::MerchantAccountData::Iban {
                iban,
                name,
                connector_recipient_id,
            } => Self::Iban {
                iban,
                name,
                connector_recipient_id: connector_recipient_id.map(RecipientIdType::ConnectorId),
            },
            api_models::admin::MerchantAccountData::Bacs {
                account_number,
                sort_code,
                name,
                connector_recipient_id,
            } => Self::Bacs {
                account_number,
                sort_code,
                name,
                connector_recipient_id: connector_recipient_id.map(RecipientIdType::ConnectorId),
            },
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantRecipientData {
    ConnectorRecipientId(Secret<String>),
    WalletId(Secret<String>),
    AccountData(MerchantAccountData),
}

impl ForeignFrom<MerchantRecipientData> for api_models::admin::MerchantRecipientData {
    fn foreign_from(value: MerchantRecipientData) -> Self {
        match value {
            MerchantRecipientData::ConnectorRecipientId(id) => Self::ConnectorRecipientId(id),
            MerchantRecipientData::WalletId(id) => Self::WalletId(id),
            MerchantRecipientData::AccountData(data) => {
                Self::AccountData(api_models::admin::MerchantAccountData::foreign_from(data))
            }
        }
    }
}

impl From<api_models::admin::MerchantRecipientData> for MerchantRecipientData {
    fn from(value: api_models::admin::MerchantRecipientData) -> Self {
        match value {
            api_models::admin::MerchantRecipientData::ConnectorRecipientId(id) => {
                Self::ConnectorRecipientId(id)
            }
            api_models::admin::MerchantRecipientData::WalletId(id) => Self::WalletId(id),
            api_models::admin::MerchantRecipientData::AccountData(data) => {
                Self::AccountData(data.into())
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdditionalMerchantData {
    OpenBankingRecipientData(MerchantRecipientData),
}

impl ForeignFrom<api_models::admin::AdditionalMerchantData> for AdditionalMerchantData {
    fn foreign_from(value: api_models::admin::AdditionalMerchantData) -> Self {
        match value {
            api_models::admin::AdditionalMerchantData::OpenBankingRecipientData(data) => {
                Self::OpenBankingRecipientData(MerchantRecipientData::from(data))
            }
        }
    }
}

impl ForeignFrom<AdditionalMerchantData> for api_models::admin::AdditionalMerchantData {
    fn foreign_from(value: AdditionalMerchantData) -> Self {
        match value {
            AdditionalMerchantData::OpenBankingRecipientData(data) => {
                Self::OpenBankingRecipientData(
                    api_models::admin::MerchantRecipientData::foreign_from(data),
                )
            }
        }
    }
}

impl ForeignFrom<api_models::admin::ConnectorAuthType> for ConnectorAuthType {
    fn foreign_from(value: api_models::admin::ConnectorAuthType) -> Self {
        match value {
            api_models::admin::ConnectorAuthType::TemporaryAuth => Self::TemporaryAuth,
            api_models::admin::ConnectorAuthType::HeaderKey { api_key } => {
                Self::HeaderKey { api_key }
            }
            api_models::admin::ConnectorAuthType::BodyKey { api_key, key1 } => {
                Self::BodyKey { api_key, key1 }
            }
            api_models::admin::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Self::SignatureKey {
                api_key,
                key1,
                api_secret,
            },
            api_models::admin::ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Self::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            },
            api_models::admin::ConnectorAuthType::CurrencyAuthKey { auth_key_map } => {
                Self::CurrencyAuthKey { auth_key_map }
            }
            api_models::admin::ConnectorAuthType::NoKey => Self::NoKey,
            api_models::admin::ConnectorAuthType::CertificateAuth {
                certificate,
                private_key,
            } => Self::CertificateAuth {
                certificate,
                private_key,
            },
        }
    }
}

impl ForeignFrom<ConnectorAuthType> for api_models::admin::ConnectorAuthType {
    fn foreign_from(from: ConnectorAuthType) -> Self {
        match from {
            ConnectorAuthType::TemporaryAuth => Self::TemporaryAuth,
            ConnectorAuthType::HeaderKey { api_key } => Self::HeaderKey { api_key },
            ConnectorAuthType::BodyKey { api_key, key1 } => Self::BodyKey { api_key, key1 },
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Self::SignatureKey {
                api_key,
                key1,
                api_secret,
            },
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Self::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            },
            ConnectorAuthType::CurrencyAuthKey { auth_key_map } => {
                Self::CurrencyAuthKey { auth_key_map }
            }
            ConnectorAuthType::NoKey => Self::NoKey,
            ConnectorAuthType::CertificateAuth {
                certificate,
                private_key,
            } => Self::CertificateAuth {
                certificate,
                private_key,
            },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorsList {
    pub connectors: Vec<String>,
}

impl ForeignTryFrom<ConnectorAuthType> for AccessTokenRequestData {
    type Error = errors::ApiErrorResponse;
    fn foreign_try_from(connector_auth: ConnectorAuthType) -> Result<Self, Self::Error> {
        match connector_auth {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                app_id: api_key,
                id: None,
            }),
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
            }),
            ConnectorAuthType::SignatureKey { api_key, key1, .. } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
            }),
            ConnectorAuthType::MultiAuthKey { api_key, key1, .. } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
            }),

            _ => Err(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector_account_details",
            }),
        }
    }
}

impl ForeignFrom<&PaymentsAuthorizeRouterData> for AuthorizeSessionTokenData {
    fn foreign_from(data: &PaymentsAuthorizeRouterData) -> Self {
        Self {
            amount_to_capture: data.amount_captured,
            currency: data.request.currency,
            connector_transaction_id: data.payment_id.clone(),
            amount: Some(data.request.amount),
        }
    }
}

pub trait Tokenizable {
    fn set_session_token(&mut self, token: Option<String>);
}

impl Tokenizable for SetupMandateRequestData {
    fn set_session_token(&mut self, _token: Option<String>) {}
}

impl Tokenizable for PaymentsAuthorizeData {
    fn set_session_token(&mut self, token: Option<String>) {
        self.session_token = token;
    }
}

impl Tokenizable for CompleteAuthorizeData {
    fn set_session_token(&mut self, _token: Option<String>) {}
}

impl ForeignFrom<&SetupMandateRouterData> for PaymentsAuthorizeData {
    fn foreign_from(data: &SetupMandateRouterData) -> Self {
        Self {
            currency: data.request.currency,
            payment_method_data: data.request.payment_method_data.clone(),
            confirm: data.request.confirm,
            statement_descriptor_suffix: data.request.statement_descriptor_suffix.clone(),
            mandate_id: data.request.mandate_id.clone(),
            setup_future_usage: data.request.setup_future_usage,
            off_session: data.request.off_session,
            setup_mandate_details: data.request.setup_mandate_details.clone(),
            router_return_url: data.request.router_return_url.clone(),
            email: data.request.email.clone(),
            customer_name: data.request.customer_name.clone(),
            amount: 0,
            order_tax_amount: Some(MinorUnit::zero()),
            minor_amount: MinorUnit::new(0),
            statement_descriptor: None,
            capture_method: None,
            webhook_url: None,
            complete_authorize_url: None,
            browser_info: data.request.browser_info.clone(),
            order_details: None,
            order_category: None,
            session_token: None,
            enrolled_for_3ds: true,
            related_transaction_id: None,
            payment_experience: None,
            payment_method_type: None,
            customer_id: None,
            surcharge_details: None,
            request_incremental_authorization: data.request.request_incremental_authorization,
            metadata: None,
            authentication_data: None,
            customer_acceptance: data.request.customer_acceptance.clone(),
            split_payments: None, // TODO: allow charges on mandates?
            merchant_order_reference_id: None,
            integrity_object: None,
            additional_payment_method_data: None,
            shipping_cost: data.request.shipping_cost,
        }
    }
}

impl<F1, F2, T1, T2> ForeignFrom<(&RouterData<F1, T1, PaymentsResponseData>, T2)>
    for RouterData<F2, T2, PaymentsResponseData>
{
    fn foreign_from(item: (&RouterData<F1, T1, PaymentsResponseData>, T2)) -> Self {
        let data = item.0;
        let request = item.1;
        Self {
            flow: PhantomData,
            request,
            merchant_id: data.merchant_id.clone(),
            connector: data.connector.clone(),
            attempt_id: data.attempt_id.clone(),
            tenant_id: data.tenant_id.clone(),
            status: data.status,
            payment_method: data.payment_method,
            connector_auth_type: data.connector_auth_type.clone(),
            description: data.description.clone(),
            address: data.address.clone(),
            auth_type: data.auth_type,
            connector_meta_data: data.connector_meta_data.clone(),
            connector_wallets_details: data.connector_wallets_details.clone(),
            amount_captured: data.amount_captured,
            minor_amount_captured: data.minor_amount_captured,
            access_token: data.access_token.clone(),
            response: data.response.clone(),
            payment_id: data.payment_id.clone(),
            session_token: data.session_token.clone(),
            reference_id: data.reference_id.clone(),
            customer_id: data.customer_id.clone(),
            payment_method_token: None,
            preprocessing_id: None,
            connector_customer: data.connector_customer.clone(),
            recurring_mandate_payment_data: data.recurring_mandate_payment_data.clone(),
            connector_request_reference_id: data.connector_request_reference_id.clone(),
            #[cfg(feature = "payouts")]
            payout_method_data: data.payout_method_data.clone(),
            #[cfg(feature = "payouts")]
            quote_id: data.quote_id.clone(),
            test_mode: data.test_mode,
            payment_method_status: None,
            payment_method_balance: data.payment_method_balance.clone(),
            connector_api_version: data.connector_api_version.clone(),
            connector_http_status_code: data.connector_http_status_code,
            external_latency: data.external_latency,
            apple_pay_flow: data.apple_pay_flow.clone(),
            frm_metadata: data.frm_metadata.clone(),
            dispute_id: data.dispute_id.clone(),
            refund_id: data.refund_id.clone(),
            connector_response: data.connector_response.clone(),
            integrity_check: Ok(()),
            additional_merchant_data: data.additional_merchant_data.clone(),
            header_payload: data.header_payload.clone(),
            connector_mandate_request_reference_id: data
                .connector_mandate_request_reference_id
                .clone(),
            authentication_id: data.authentication_id.clone(),
            psd2_sca_exemption_type: data.psd2_sca_exemption_type,
        }
    }
}

#[cfg(feature = "payouts")]
impl<F1, F2>
    ForeignFrom<(
        &RouterData<F1, PayoutsData, PayoutsResponseData>,
        PayoutsData,
    )> for RouterData<F2, PayoutsData, PayoutsResponseData>
{
    fn foreign_from(
        item: (
            &RouterData<F1, PayoutsData, PayoutsResponseData>,
            PayoutsData,
        ),
    ) -> Self {
        let data = item.0;
        let request = item.1;
        Self {
            flow: PhantomData,
            request,
            merchant_id: data.merchant_id.clone(),
            connector: data.connector.clone(),
            attempt_id: data.attempt_id.clone(),
            tenant_id: data.tenant_id.clone(),
            status: data.status,
            payment_method: data.payment_method,
            connector_auth_type: data.connector_auth_type.clone(),
            description: data.description.clone(),
            address: data.address.clone(),
            auth_type: data.auth_type,
            connector_meta_data: data.connector_meta_data.clone(),
            connector_wallets_details: data.connector_wallets_details.clone(),
            amount_captured: data.amount_captured,
            minor_amount_captured: data.minor_amount_captured,
            access_token: data.access_token.clone(),
            response: data.response.clone(),
            payment_id: data.payment_id.clone(),
            session_token: data.session_token.clone(),
            reference_id: data.reference_id.clone(),
            customer_id: data.customer_id.clone(),
            payment_method_token: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            connector_customer: data.connector_customer.clone(),
            connector_request_reference_id:
                IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_PAYOUTS_FLOW.to_string(),
            payout_method_data: data.payout_method_data.clone(),
            quote_id: data.quote_id.clone(),
            test_mode: data.test_mode,
            payment_method_balance: None,
            payment_method_status: None,
            connector_api_version: None,
            connector_http_status_code: data.connector_http_status_code,
            external_latency: data.external_latency,
            apple_pay_flow: None,
            frm_metadata: None,
            refund_id: None,
            dispute_id: None,
            connector_response: data.connector_response.clone(),
            integrity_check: Ok(()),
            header_payload: data.header_payload.clone(),
            authentication_id: None,
            psd2_sca_exemption_type: None,
            additional_merchant_data: data.additional_merchant_data.clone(),
            connector_mandate_request_reference_id: None,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<&domain::MerchantConnectorAccountFeatureMetadata>
    for api_models::admin::MerchantConnectorAccountFeatureMetadata
{
    fn foreign_from(item: &domain::MerchantConnectorAccountFeatureMetadata) -> Self {
        let revenue_recovery = item
            .revenue_recovery
            .as_ref()
            .map(
                |revenue_recovery_metadata| api_models::admin::RevenueRecoveryMetadata {
                    max_retry_count: revenue_recovery_metadata.max_retry_count,
                    billing_connector_retry_threshold: revenue_recovery_metadata
                        .billing_connector_retry_threshold,
                    billing_account_reference: revenue_recovery_metadata
                        .mca_reference
                        .recovery_to_billing
                        .clone(),
                },
            );
        Self { revenue_recovery }
    }
}

#[cfg(feature = "v2")]
impl ForeignTryFrom<&api_models::admin::MerchantConnectorAccountFeatureMetadata>
    for domain::MerchantConnectorAccountFeatureMetadata
{
    type Error = errors::ApiErrorResponse;
    fn foreign_try_from(
        feature_metadata: &api_models::admin::MerchantConnectorAccountFeatureMetadata,
    ) -> Result<Self, Self::Error> {
        let revenue_recovery = feature_metadata
            .revenue_recovery
            .as_ref()
            .map(|revenue_recovery_metadata| {
                domain::AccountReferenceMap::new(
                    revenue_recovery_metadata.billing_account_reference.clone(),
                )
                .map(|mca_reference| domain::RevenueRecoveryMetadata {
                    max_retry_count: revenue_recovery_metadata.max_retry_count,
                    billing_connector_retry_threshold: revenue_recovery_metadata
                        .billing_connector_retry_threshold,
                    mca_reference,
                })
            })
            .transpose()?;

        Ok(Self { revenue_recovery })
    }
}
