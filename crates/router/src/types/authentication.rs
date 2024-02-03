use cards::CardNumber;
use serde::{Deserialize, Serialize};

use super::{api, RouterData};

#[derive(Debug, Clone)]
pub enum AuthenticationResponseData {
    PreAuthNResponse {
        threeds_server_transaction_id: String,
        maximum_supported_3ds_version: (i64, i64, i64),
        connector_authentication_id: String,
        three_ds_method_data: String,
        three_ds_method_url: Option<String>,
        message_version: String,
    },
    AuthNResponse {
        authn_flow_type: AuthNFlowType,
    },
    PostAuthNResponse {
        cavv: String,
    },
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum AuthNFlowType {
    #[default]
    NotApplicable,
    Challenge {
        challenge_url: String,
    },
    Frictionless {
        cavv: String,
    },
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
