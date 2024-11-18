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
        redirection_data: Box<Option<RedirectForm>>,
        mandate_reference: Box<Option<MandateReference>>,
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
    pub mandate_metadata: Option<common_utils::pii::SecretSerdeValue>,
    pub connector_mandate_request_reference_id: Option<String>,
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
    WorldpayDDCForm {
        endpoint: url::Url,
        method: Method,
        form_fields: HashMap<String, String>,
        collection_id: Option<String>,
    },
    KlarnaCheckout{
        html_snippet: String,
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

impl From<RedirectForm> for diesel_models::payment_attempt::RedirectForm {
    fn from(redirect_form: RedirectForm) -> Self {
        match redirect_form {
            RedirectForm::Form {
                endpoint,
                method,
                form_fields,
            } => Self::Form {
                endpoint,
                method,
                form_fields,
            },
            RedirectForm::Html { html_data } => Self::Html { html_data },
            RedirectForm::BlueSnap {
                payment_fields_token,
            } => Self::BlueSnap {
                payment_fields_token,
            },
            RedirectForm::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            } => Self::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            },
            RedirectForm::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            } => Self::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            },
            RedirectForm::Payme => Self::Payme,
            RedirectForm::Braintree {
                client_token,
                card_token,
                bin,
            } => Self::Braintree {
                client_token,
                card_token,
                bin,
            },
            RedirectForm::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            } => Self::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            },
            RedirectForm::Mifinity {
                initialization_token,
            } => Self::Mifinity {
                initialization_token,
            },
            RedirectForm::WorldpayDDCForm {
                endpoint,
                method,
                form_fields,
                collection_id,
            } => Self::WorldpayDDCForm {
                endpoint: common_utils::types::Url::wrap(endpoint),
                method,
                form_fields,
                collection_id,
            },
            RedirectForm::KlarnaCheckout { html_snippet } =>Self::KlarnaCheckout {
                html_snippet
            },
        }
    }
}

impl From<diesel_models::payment_attempt::RedirectForm> for RedirectForm {
    fn from(redirect_form: diesel_models::payment_attempt::RedirectForm) -> Self {
        match redirect_form {
            diesel_models::payment_attempt::RedirectForm::Form {
                endpoint,
                method,
                form_fields,
            } => Self::Form {
                endpoint,
                method,
                form_fields,
            },
            diesel_models::payment_attempt::RedirectForm::Html { html_data } => {
                Self::Html { html_data }
            }
            diesel_models::payment_attempt::RedirectForm::BlueSnap {
                payment_fields_token,
            } => Self::BlueSnap {
                payment_fields_token,
            },
            diesel_models::payment_attempt::RedirectForm::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            } => Self::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            },
            diesel_models::payment_attempt::RedirectForm::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            } => Self::CybersourceConsumerAuth {
                access_token,
                step_up_url,
            },
            diesel_models::payment_attempt::RedirectForm::Payme => Self::Payme,
            diesel_models::payment_attempt::RedirectForm::Braintree {
                client_token,
                card_token,
                bin,
            } => Self::Braintree {
                client_token,
                card_token,
                bin,
            },
            diesel_models::payment_attempt::RedirectForm::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            } => Self::Nmi {
                amount,
                currency,
                public_key,
                customer_vault_id,
                order_id,
            },
            diesel_models::payment_attempt::RedirectForm::Mifinity {
                initialization_token,
            } => Self::Mifinity {
                initialization_token,
            },
            diesel_models::payment_attempt::RedirectForm::WorldpayDDCForm {
                endpoint,
                method,
                form_fields,
                collection_id,
            } => Self::WorldpayDDCForm {
                endpoint: endpoint.into_inner(),
                method,
                form_fields,
                collection_id,
            },
            diesel_models::RedirectForm::KlarnaCheckout { html_snippet } => {
                Self::KlarnaCheckout { html_snippet}
            },

            diesel_models::RedirectForm::Form { endpoint, method, form_fields } => todo!(),
            diesel_models::RedirectForm::Html { html_data } => todo!(),
            diesel_models::RedirectForm::BlueSnap { payment_fields_token } => todo!(),
            diesel_models::RedirectForm::CybersourceAuthSetup { access_token, ddc_url, reference_id } => todo!(),
            diesel_models::RedirectForm::CybersourceConsumerAuth { access_token, step_up_url } => todo!(),
            diesel_models::RedirectForm::Payme => todo!(),
            diesel_models::RedirectForm::Braintree { client_token, card_token, bin } => todo!(),
            diesel_models::RedirectForm::Nmi { amount, currency, public_key, customer_vault_id, order_id } => todo!(),
            diesel_models::RedirectForm::Mifinity { initialization_token } => todo!(),
            diesel_models::RedirectForm::WorldpayDDCForm { endpoint, method, form_fields, collection_id } => todo!(),
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
