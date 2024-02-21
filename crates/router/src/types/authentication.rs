use cards::CardNumber;
use serde::{Deserialize, Serialize};

use super::{api, RouterData};

#[derive(Debug, Clone)]
pub enum AuthenticationResponseData {
    PreAuthNResponse {
        threeds_server_transaction_id: String,
        maximum_supported_3ds_version: (i64, i64, i64),
        authentication_connector_id: String,
        three_ds_method_data: String,
        three_ds_method_url: Option<String>,
        message_version: String,
        connector_metadata: Option<serde_json::Value>,
    },
    AuthNResponse {
        authn_flow_type: AuthNFlowType,
        cavv: Option<String>,
        trans_status: api_models::payments::TransStatus,
    },
    PostAuthNResponse {
        trans_status: api_models::payments::TransStatus,
        authentication_value: Option<String>,
        eci: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChallengeParams {
    pub acs_url: Option<url::Url>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub three_dsserver_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthNFlowType {
    Challenge(Box<ChallengeParams>),
    Frictionless,
}

#[derive(Clone, Default, Debug)]
pub struct PreAuthNRequestData {
    // card number
    pub(crate) card_holder_account_number: CardNumber,
}

pub struct AuthNRequestData {}

pub struct PostAuthNRequestData {}

pub type PreAuthNRouterData =
    RouterData<api::PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>;

// pub type AuthNRouterData = RouterData<api::AuthN, AuthNRequestData, AuthenticationResponseData>;

// pub type PostAuthNRouterData =
//     RouterData<api::PostAuthN, PostAuthNRequestData, AuthenticationResponseData>;
