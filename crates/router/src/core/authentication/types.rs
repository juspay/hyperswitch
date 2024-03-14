use cards::CardNumber;
use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    core::{errors, payments},
    types::{authentication::AuthNFlowType, storage, transformers::ForeignTryFrom},
    utils::OptionExt,
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
        authentication: storage::Authentication,
        should_continue_confirm_transaction: &'a mut bool,
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
    pub trans_status: common_enums::TransactionStatus,
    pub acquirer_details: Option<AcquirerDetails>,
}

#[derive(Clone, Debug)]
pub struct PreAuthenticationData {
    pub threeds_server_transaction_id: String,
    pub message_version: String,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
}

impl ForeignTryFrom<&storage::Authentication> for PreAuthenticationData {
    type Error = Report<errors::ApiErrorResponse>;

    fn foreign_try_from(authentication: &storage::Authentication) -> Result<Self, Self::Error> {
        let error_message = errors::ApiErrorResponse::UnprocessableEntity { message: "Pre Authentication must be completed successfully before Authentication can be performed".to_string() };
        let threeds_server_transaction_id = authentication
            .threeds_server_transaction_id
            .clone()
            .get_required_value("threeds_server_transaction_id")
            .change_context(error_message)?;
        let message_version = authentication
            .message_version
            .as_ref()
            .get_required_value("threeds_server_transaction_id")?;
        Ok(Self {
            threeds_server_transaction_id,
            message_version: message_version.to_string(),
            acquirer_bin: authentication.acquirer_bin.clone(),
            acquirer_merchant_id: authentication.acquirer_merchant_id.clone(),
            connector_metadata: authentication.connector_metadata.clone(),
        })
    }
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
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct AcquirerDetails {
    pub acquirer_bin: String,
    pub acquirer_merchant_id: String,
}
