use cards::CardNumber;
use serde::{Deserialize, Serialize};

use crate::{
    core::payments,
    types::{authentication::AuthNFlowType, storage},
};
pub enum PreAuthenthenticationFlowInput<'a, F: Clone> {
    PaymentAuthNFlow {
        payment_data: &'a mut payments::PaymentData<F>,
        should_continue_confirm_transaction: &'a mut bool,
        card_number: CardNumber,
    },
    PaymentMethodAuthNFlow {
        card_number: CardNumber,
        other_fields: String, //should be expanded when implementation begins
    },
}

pub enum PostAuthenthenticationFlowInput<'a, F: Clone> {
    PaymentAuthNFlow {
        payment_data: &'a mut payments::PaymentData<F>,
        authentication_data: (storage::Authentication, AuthenticationData),
    },
    PaymentMethodAuthNFlow {
        other_fields: String, //should be expanded when implementation begins
    },
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct AuthenticationData {
    pub maximum_supported_version: (i64, i64, i64),
    pub threeds_server_transaction_id: String,
    pub cavv: Option<String>,
    pub authn_flow_type: Option<AuthNFlowType>,
    pub three_ds_method_data: ThreeDsMethodData,
    pub message_version: String,
    pub eci: Option<String>,
    pub trans_status: TransStatus,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum TransStatus {
    /// Authentication/ Account Verification Successful
    Y,
    /// Not Authenticated /Account Not Verified; Transaction denied
    #[default]
    N,
    /// Authentication/ Account Verification Could Not Be Performed; Technical or other problem, as indicated in ARes or RReq
    U,
    /// Attempts Processing Performed; Not Authenticated/Verified , but a proof of attempted authentication/verification is provided
    A,
    /// Authentication/ Account Verification Rejected; Issuer is rejecting authentication/verification and request that authorisation not be attempted.
    R,
}

impl AuthenticationData {
    pub fn is_separate_authn_required(&self) -> bool {
        self.maximum_supported_version.0 == 2
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ThreeDsMethodData {
    pub three_ds_method_data_submission: bool,
    pub three_ds_method_data: String,
    pub three_ds_method_url: Option<String>,
}
