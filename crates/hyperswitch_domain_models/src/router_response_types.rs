pub mod disputes;
pub mod fraud_check;
use std::collections::HashMap;

use common_utils::{request::Method, types as common_types, types::MinorUnit};
pub use disputes::{AcceptDisputeResponse, DefendDisputeResponse, SubmitEvidenceResponse};

use crate::router_request_types::{authentication::AuthNFlowType, ResponseId};
#[derive(Debug, Clone)]
pub struct RefundsResponseData {
    pub connector_refund_id: String,
    pub refund_status: common_enums::RefundStatus,
    // pub amount_received: Option<i32>, // Calculation for amount received not in place yet
}

#[derive(Debug, Clone)]
pub enum PaymentsResponseData {
    TransactionResponse {
        resource_id: ResponseId,
        redirection_data: Option<RedirectForm>,
        mandate_reference:  Box<Option<MandateReference>>,
        connector_metadata: Option<serde_json::Value>,
        network_txn_id: Option<String>,
        connector_response_reference_id: Option<String>,
        incremental_authorization_allowed: Option<bool>,
        charge_id: Option<String>,
    },
    MultipleCaptureResponse {
        // pending_capture_id_list: Vec<String>,
        capture_sync_response_list: HashMap<String, CaptureSyncResponse>,
    },
    SessionResponse {
        session_token: api_models::payments::SessionToken,
    },
    SessionTokenResponse {
        session_token: String,
    },
    TransactionUnresolvedResponse {
        resource_id: ResponseId,
        //to add more info on cypto response, like `unresolved` reason(overpaid, underpaid, delayed)
        reason: Option<api_models::enums::UnresolvedResponseReason>,
        connector_response_reference_id: Option<String>,
    },
    TokenizationResponse {
        token: String,
    },

    ConnectorCustomerResponse {
        connector_customer_id: String,
    },

    ThreeDSEnrollmentResponse {
        enrolled_v2: bool,
        related_transaction_id: Option<String>,
    },
    PreProcessingResponse {
        pre_processing_id: PreprocessingResponseId,
        connector_metadata: Option<serde_json::Value>,
        session_token: Option<api_models::payments::SessionToken>,
        connector_response_reference_id: Option<String>,
    },
    IncrementalAuthorizationResponse {
        status: common_enums::AuthorizationStatus,
        connector_authorization_id: Option<String>,
        error_code: Option<String>,
        error_message: Option<String>,
    },
    PostProcessingResponse {
        session_token: Option<api_models::payments::OpenBankingSessionToken>,
    },
    SessionUpdateResponse {
        status: common_enums::SessionUpdateStatus,
    },
}

#[derive(Debug, Clone)]
pub struct TaxCalculationResponseData {
    pub order_tax_amount: MinorUnit,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct MandateReference {
    pub connector_mandate_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub mandate_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum CaptureSyncResponse {
    Success {
        resource_id: ResponseId,
        status: common_enums::AttemptStatus,
        connector_response_reference_id: Option<String>,
        amount: Option<MinorUnit>,
    },
    Error {
        code: String,
        message: String,
        reason: Option<String>,
        status_code: u16,
        amount: Option<MinorUnit>,
    },
}

impl CaptureSyncResponse {
    pub fn get_amount_captured(&self) -> Option<MinorUnit> {
        match self {
            Self::Success { amount, .. } | Self::Error { amount, .. } => *amount,
        }
    }
    pub fn get_connector_response_reference_id(&self) -> Option<String> {
        match self {
            Self::Success {
                connector_response_reference_id,
                ..
            } => connector_response_reference_id.clone(),
            Self::Error { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PreprocessingResponseId {
    PreProcessingId(String),
    ConnectorTransactionId(String),
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum RedirectForm {
    Form {
        endpoint: String,
        method: Method,
        form_fields: HashMap<String, String>,
    },
    Html {
        html_data: String,
    },
    BlueSnap {
        payment_fields_token: String, // payment-field-token
    },
    CybersourceAuthSetup {
        access_token: String,
        ddc_url: String,
        reference_id: String,
    },
    CybersourceConsumerAuth {
        access_token: String,
        step_up_url: String,
    },
    Payme,
    Braintree {
        client_token: String,
        card_token: String,
        bin: String,
    },
    Nmi {
        amount: String,
        currency: common_enums::Currency,
        public_key: masking::Secret<String>,
        customer_vault_id: String,
        order_id: String,
    },
    Mifinity {
        initialization_token: String,
    },
}

impl From<(url::Url, Method)> for RedirectForm {
    fn from((mut redirect_url, method): (url::Url, Method)) -> Self {
        let form_fields = HashMap::from_iter(
            redirect_url
                .query_pairs()
                .map(|(key, value)| (key.to_string(), value.to_string())),
        );

        // Do not include query params in the endpoint
        redirect_url.set_query(None);

        Self::Form {
            endpoint: redirect_url.to_string(),
            method,
            form_fields,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct UploadFileResponse {
    pub provider_file_id: String,
}
#[derive(Clone, Debug)]
pub struct RetrieveFileResponse {
    pub file_data: Vec<u8>,
}

#[cfg(feature = "payouts")]
#[derive(Clone, Debug, Default)]
pub struct PayoutsResponseData {
    pub status: Option<common_enums::PayoutStatus>,
    pub connector_payout_id: Option<String>,
    pub payout_eligible: Option<bool>,
    pub should_add_next_step_to_process_tracker: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerifyWebhookSourceResponseData {
    pub verify_webhook_status: VerifyWebhookStatus,
}

#[derive(Debug, Clone)]
pub enum VerifyWebhookStatus {
    SourceVerified,
    SourceNotVerified,
}

#[derive(Debug, Clone)]
pub struct MandateRevokeResponseData {
    pub mandate_status: common_enums::MandateStatus,
}

#[derive(Debug, Clone)]
pub enum AuthenticationResponseData {
    PreAuthVersionCallResponse {
        maximum_supported_3ds_version: common_types::SemanticVersion,
    },
    PreAuthThreeDsMethodCallResponse {
        threeds_server_transaction_id: String,
        three_ds_method_data: Option<String>,
        three_ds_method_url: Option<String>,
        connector_metadata: Option<serde_json::Value>,
    },
    PreAuthNResponse {
        threeds_server_transaction_id: String,
        maximum_supported_3ds_version: common_utils::types::SemanticVersion,
        connector_authentication_id: String,
        three_ds_method_data: Option<String>,
        three_ds_method_url: Option<String>,
        message_version: common_utils::types::SemanticVersion,
        connector_metadata: Option<serde_json::Value>,
        directory_server_id: Option<String>,
    },
    AuthNResponse {
        authn_flow_type: AuthNFlowType,
        authentication_value: Option<String>,
        trans_status: common_enums::TransactionStatus,
        connector_metadata: Option<serde_json::Value>,
        ds_trans_id: Option<String>,
    },
    PostAuthNResponse {
        trans_status: common_enums::TransactionStatus,
        authentication_value: Option<String>,
        eci: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct CompleteAuthorizeRedirectResponse {
    pub params: Option<masking::Secret<String>>,
    pub payload: Option<common_utils::pii::SecretSerdeValue>,
}
