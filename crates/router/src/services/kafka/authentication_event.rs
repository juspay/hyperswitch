use diesel_models::{authentication::Authentication, enums as storage_enums};
use time::OffsetDateTime;

#[serde_with::skip_serializing_none]
#[derive(serde::Serialize, Debug)]
pub struct KafkaAuthenticationEvent<'a> {
    pub authentication_id: &'a String,
    pub merchant_id: &'a common_utils::id_type::MerchantId,
    pub authentication_connector: &'a String,
    pub connector_authentication_id: Option<&'a String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: &'a String,
    pub authentication_type: Option<storage_enums::DecoupledAuthenticationType>,
    pub authentication_status: storage_enums::AuthenticationStatus,
    pub authentication_lifecycle_status: storage_enums::AuthenticationLifecycleStatus,
    #[serde(default, with = "time::serde::timestamp::milliseconds")]
    pub created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp::milliseconds")]
    pub modified_at: OffsetDateTime,
    pub error_message: Option<&'a String>,
    pub error_code: Option<&'a String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub maximum_supported_version: Option<common_utils::types::SemanticVersion>,
    pub threeds_server_transaction_id: Option<&'a String>,
    pub cavv: Option<&'a String>,
    pub authentication_flow_type: Option<&'a String>,
    pub message_version: Option<common_utils::types::SemanticVersion>,
    pub eci: Option<&'a String>,
    pub trans_status: Option<storage_enums::TransactionStatus>,
    pub acquirer_bin: Option<&'a String>,
    pub acquirer_merchant_id: Option<&'a String>,
    pub three_ds_method_data: Option<&'a String>,
    pub three_ds_method_url: Option<&'a String>,
    pub acs_url: Option<&'a String>,
    pub challenge_request: Option<&'a String>,
    pub acs_reference_number: Option<&'a String>,
    pub acs_trans_id: Option<&'a String>,
    pub acs_signed_content: Option<&'a String>,
    pub profile_id: &'a common_utils::id_type::ProfileId,
    pub payment_id: Option<&'a common_utils::id_type::PaymentId>,
    pub merchant_connector_id: &'a common_utils::id_type::MerchantConnectorAccountId,
    pub ds_trans_id: Option<&'a String>,
    pub directory_server_id: Option<&'a String>,
    pub acquirer_country_code: Option<&'a String>,
    pub organization_id: &'a common_utils::id_type::OrganizationId,
}

impl<'a> KafkaAuthenticationEvent<'a> {
    pub fn from_storage(authentication: &'a Authentication) -> Self {
        Self {
            created_at: authentication.created_at.assume_utc(),
            modified_at: authentication.modified_at.assume_utc(),
            authentication_id: &authentication.authentication_id,
            merchant_id: &authentication.merchant_id,
            authentication_status: authentication.authentication_status,
            authentication_connector: &authentication.authentication_connector,
            connector_authentication_id: authentication.connector_authentication_id.as_ref(),
            authentication_data: authentication.authentication_data.clone(),
            payment_method_id: &authentication.payment_method_id,
            authentication_type: authentication.authentication_type,
            authentication_lifecycle_status: authentication.authentication_lifecycle_status,
            error_code: authentication.error_code.as_ref(),
            error_message: authentication.error_message.as_ref(),
            connector_metadata: authentication.connector_metadata.clone(),
            maximum_supported_version: authentication.maximum_supported_version.clone(),
            threeds_server_transaction_id: authentication.threeds_server_transaction_id.as_ref(),
            cavv: authentication.cavv.as_ref(),
            authentication_flow_type: authentication.authentication_flow_type.as_ref(),
            message_version: authentication.message_version.clone(),
            eci: authentication.eci.as_ref(),
            trans_status: authentication.trans_status.clone(),
            acquirer_bin: authentication.acquirer_bin.as_ref(),
            acquirer_merchant_id: authentication.acquirer_merchant_id.as_ref(),
            three_ds_method_data: authentication.three_ds_method_data.as_ref(),
            three_ds_method_url: authentication.three_ds_method_url.as_ref(),
            acs_url: authentication.acs_url.as_ref(),
            challenge_request: authentication.challenge_request.as_ref(),
            acs_reference_number: authentication.acs_reference_number.as_ref(),
            acs_trans_id: authentication.acs_trans_id.as_ref(),
            acs_signed_content: authentication.acs_signed_content.as_ref(),
            profile_id: &authentication.profile_id,
            payment_id: authentication.payment_id.as_ref(),
            merchant_connector_id: &authentication.merchant_connector_id,
            ds_trans_id: authentication.ds_trans_id.as_ref(),
            directory_server_id: authentication.directory_server_id.as_ref(),
            acquirer_country_code: authentication.acquirer_country_code.as_ref(),
            organization_id: &authentication.organization_id,
        }
    }
}

impl super::KafkaMessage for KafkaAuthenticationEvent<'_> {
    fn key(&self) -> String {
        format!(
            "{}_{}",
            self.merchant_id.get_string_repr(),
            self.authentication_id
        )
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::Authentication
    }
}
