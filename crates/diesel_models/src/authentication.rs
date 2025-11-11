use std::str::FromStr;

use common_utils::{
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    pii,
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use error_stack::ResultExt;
use serde::{self, Deserialize, Serialize};

use crate::schema::authentication;

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Serialize, Deserialize,
)]
#[diesel(table_name = authentication,  primary_key(authentication_id), check_for_backend(diesel::pg::Pg))]
pub struct Authentication {
    pub authentication_id: common_utils::id_type::AuthenticationId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub authentication_connector: Option<String>,
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: common_enums::AuthenticationStatus,
    pub authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: time::PrimitiveDateTime,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub maximum_supported_version: Option<common_utils::types::SemanticVersion>,
    pub threeds_server_transaction_id: Option<String>,
    pub cavv: Option<String>,
    pub authentication_flow_type: Option<String>,
    pub message_version: Option<common_utils::types::SemanticVersion>,
    pub eci: Option<String>,
    pub trans_status: Option<common_enums::TransactionStatus>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub acs_url: Option<String>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
    pub profile_id: common_utils::id_type::ProfileId,
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub ds_trans_id: Option<String>,
    pub directory_server_id: Option<String>,
    pub acquirer_country_code: Option<String>,
    pub service_details: Option<serde_json::Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub authentication_client_secret: Option<String>,
    pub force_3ds_challenge: Option<bool>,
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    pub return_url: Option<String>,
    pub amount: Option<common_utils::types::MinorUnit>,
    pub currency: Option<common_enums::Currency>,
    pub billing_address: Option<Encryption>,
    pub shipping_address: Option<Encryption>,
    pub browser_info: Option<serde_json::Value>,
    pub email: Option<Encryption>,
    pub profile_acquirer_id: Option<common_utils::id_type::ProfileAcquirerId>,
    pub challenge_code: Option<String>,
    pub challenge_cancel: Option<String>,
    pub challenge_code_reason: Option<String>,
    pub message_extension: Option<pii::SecretSerdeValue>,
    pub challenge_request_key: Option<String>,
    pub customer_details: Option<Encryption>,
}

impl Authentication {
    pub fn is_separate_authn_required(&self) -> bool {
        self.maximum_supported_version
            .as_ref()
            .is_some_and(|version| version.get_major() == 2)
    }

    // get authentication_connector from authentication record and check if it is jwt flow
    pub fn is_jwt_flow(&self) -> CustomResult<bool, ValidationError> {
        Ok(self
            .authentication_connector
            .clone()
            .map(|connector| {
                common_enums::AuthenticationConnectors::from_str(&connector)
                    .change_context(ValidationError::InvalidValue {
                        message: "failed to parse authentication_connector".to_string(),
                    })
                    .map(|connector_enum| connector_enum.is_jwt_flow())
            })
            .transpose()?
            .unwrap_or(false))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Insertable)]
#[diesel(table_name = authentication)]
pub struct AuthenticationNew {
    pub authentication_id: common_utils::id_type::AuthenticationId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub authentication_connector: Option<String>,
    pub connector_authentication_id: Option<String>,
    // pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: common_enums::AuthenticationStatus,
    pub authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub maximum_supported_version: Option<common_utils::types::SemanticVersion>,
    pub threeds_server_transaction_id: Option<String>,
    pub cavv: Option<String>,
    pub authentication_flow_type: Option<String>,
    pub message_version: Option<common_utils::types::SemanticVersion>,
    pub eci: Option<String>,
    pub trans_status: Option<common_enums::TransactionStatus>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub acs_url: Option<String>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
    pub profile_id: common_utils::id_type::ProfileId,
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub ds_trans_id: Option<String>,
    pub directory_server_id: Option<String>,
    pub acquirer_country_code: Option<String>,
    pub service_details: Option<serde_json::Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub authentication_client_secret: Option<String>,
    pub force_3ds_challenge: Option<bool>,
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    pub return_url: Option<String>,
    pub amount: Option<common_utils::types::MinorUnit>,
    pub currency: Option<common_enums::Currency>,
    pub billing_address: Option<Encryption>,
    pub shipping_address: Option<Encryption>,
    pub browser_info: Option<serde_json::Value>,
    pub email: Option<Encryption>,
    pub profile_acquirer_id: Option<common_utils::id_type::ProfileAcquirerId>,
    pub challenge_code: Option<String>,
    pub challenge_cancel: Option<String>,
    pub challenge_code_reason: Option<String>,
    pub message_extension: Option<pii::SecretSerdeValue>,
    pub challenge_request_key: Option<String>,
    pub customer_details: Option<Encryption>,
}

#[derive(Debug)]
pub enum AuthenticationUpdate {
    PreAuthenticationVersionCallUpdate {
        maximum_supported_3ds_version: common_utils::types::SemanticVersion,
        message_version: common_utils::types::SemanticVersion,
    },
    PreAuthenticationThreeDsMethodCall {
        threeds_server_transaction_id: String,
        three_ds_method_data: Option<String>,
        three_ds_method_url: Option<String>,
        acquirer_bin: Option<String>,
        acquirer_merchant_id: Option<String>,
        connector_metadata: Option<serde_json::Value>,
    },
    PreAuthenticationUpdate {
        threeds_server_transaction_id: String,
        maximum_supported_3ds_version: common_utils::types::SemanticVersion,
        connector_authentication_id: String,
        three_ds_method_data: Option<String>,
        three_ds_method_url: Option<String>,
        message_version: common_utils::types::SemanticVersion,
        connector_metadata: Option<serde_json::Value>,
        authentication_status: common_enums::AuthenticationStatus,
        acquirer_bin: Option<String>,
        acquirer_merchant_id: Option<String>,
        directory_server_id: Option<String>,
        acquirer_country_code: Option<String>,
        billing_address: Option<Encryption>,
        shipping_address: Option<Encryption>,
        browser_info: Box<Option<serde_json::Value>>,
        email: Option<Encryption>,
    },
    AuthenticationUpdate {
        trans_status: common_enums::TransactionStatus,
        authentication_type: common_enums::DecoupledAuthenticationType,
        acs_url: Option<String>,
        challenge_request: Option<String>,
        acs_reference_number: Option<String>,
        acs_trans_id: Option<String>,
        acs_signed_content: Option<String>,
        connector_metadata: Option<serde_json::Value>,
        authentication_status: common_enums::AuthenticationStatus,
        ds_trans_id: Option<String>,
        eci: Option<String>,
        challenge_code: Option<String>,
        challenge_cancel: Option<String>,
        challenge_code_reason: Option<String>,
        message_extension: Option<pii::SecretSerdeValue>,
        challenge_request_key: Option<String>,
    },
    PostAuthenticationUpdate {
        trans_status: common_enums::TransactionStatus,
        eci: Option<String>,
        authentication_status: common_enums::AuthenticationStatus,
        challenge_cancel: Option<String>,
        challenge_code_reason: Option<String>,
    },
    ErrorUpdate {
        error_message: Option<String>,
        error_code: Option<String>,
        authentication_status: common_enums::AuthenticationStatus,
        connector_authentication_id: Option<String>,
    },
    PostAuthorizationUpdate {
        authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    },
    AuthenticationStatusUpdate {
        trans_status: common_enums::TransactionStatus,
        authentication_status: common_enums::AuthenticationStatus,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = authentication)]
pub struct AuthenticationUpdateInternal {
    pub connector_authentication_id: Option<String>,
    // pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: Option<String>,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: Option<common_enums::AuthenticationStatus>,
    pub authentication_lifecycle_status: Option<common_enums::AuthenticationLifecycleStatus>,
    pub modified_at: time::PrimitiveDateTime,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub maximum_supported_version: Option<common_utils::types::SemanticVersion>,
    pub threeds_server_transaction_id: Option<String>,
    pub authentication_flow_type: Option<String>,
    pub message_version: Option<common_utils::types::SemanticVersion>,
    pub eci: Option<String>,
    pub trans_status: Option<common_enums::TransactionStatus>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub acs_url: Option<String>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
    pub ds_trans_id: Option<String>,
    pub directory_server_id: Option<String>,
    pub acquirer_country_code: Option<String>,
    pub service_details: Option<serde_json::Value>,
    pub force_3ds_challenge: Option<bool>,
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    pub billing_address: Option<Encryption>,
    pub shipping_address: Option<Encryption>,
    pub browser_info: Option<serde_json::Value>,
    pub email: Option<Encryption>,
    pub profile_acquirer_id: Option<common_utils::id_type::ProfileAcquirerId>,
    pub challenge_code: Option<String>,
    pub challenge_cancel: Option<String>,
    pub challenge_code_reason: Option<String>,
    pub message_extension: Option<pii::SecretSerdeValue>,
    pub challenge_request_key: Option<String>,
    pub customer_details: Option<Encryption>,
}

impl Default for AuthenticationUpdateInternal {
    fn default() -> Self {
        Self {
            connector_authentication_id: Default::default(),
            payment_method_id: Default::default(),
            authentication_type: Default::default(),
            authentication_status: Default::default(),
            authentication_lifecycle_status: Default::default(),
            modified_at: common_utils::date_time::now(),
            error_message: Default::default(),
            error_code: Default::default(),
            connector_metadata: Default::default(),
            maximum_supported_version: Default::default(),
            threeds_server_transaction_id: Default::default(),
            authentication_flow_type: Default::default(),
            message_version: Default::default(),
            eci: Default::default(),
            trans_status: Default::default(),
            acquirer_bin: Default::default(),
            acquirer_merchant_id: Default::default(),
            three_ds_method_data: Default::default(),
            three_ds_method_url: Default::default(),
            acs_url: Default::default(),
            challenge_request: Default::default(),
            acs_reference_number: Default::default(),
            acs_trans_id: Default::default(),
            acs_signed_content: Default::default(),
            ds_trans_id: Default::default(),
            directory_server_id: Default::default(),
            acquirer_country_code: Default::default(),
            service_details: Default::default(),
            force_3ds_challenge: Default::default(),
            psd2_sca_exemption_type: Default::default(),
            billing_address: Default::default(),
            shipping_address: Default::default(),
            browser_info: Default::default(),
            email: Default::default(),
            profile_acquirer_id: Default::default(),
            challenge_code: Default::default(),
            challenge_cancel: Default::default(),
            challenge_code_reason: Default::default(),
            message_extension: Default::default(),
            challenge_request_key: Default::default(),
            customer_details: Default::default(),
        }
    }
}

impl AuthenticationUpdateInternal {
    pub fn apply_changeset(self, source: Authentication) -> Authentication {
        let Self {
            connector_authentication_id,
            payment_method_id,
            authentication_type,
            authentication_status,
            authentication_lifecycle_status,
            modified_at: _,
            error_code,
            error_message,
            connector_metadata,
            maximum_supported_version,
            threeds_server_transaction_id,
            authentication_flow_type,
            message_version,
            eci,
            trans_status,
            acquirer_bin,
            acquirer_merchant_id,
            three_ds_method_data,
            three_ds_method_url,
            acs_url,
            challenge_request,
            acs_reference_number,
            acs_trans_id,
            acs_signed_content,
            ds_trans_id,
            directory_server_id,
            acquirer_country_code,
            service_details,
            force_3ds_challenge,
            psd2_sca_exemption_type,
            billing_address,
            shipping_address,
            browser_info,
            email,
            profile_acquirer_id,
            challenge_code,
            challenge_cancel,
            challenge_code_reason,
            message_extension,
            challenge_request_key,
            customer_details,
        } = self;
        Authentication {
            connector_authentication_id: connector_authentication_id
                .or(source.connector_authentication_id),
            payment_method_id: payment_method_id.unwrap_or(source.payment_method_id),
            authentication_type: authentication_type.or(source.authentication_type),
            authentication_status: authentication_status.unwrap_or(source.authentication_status),
            authentication_lifecycle_status: authentication_lifecycle_status
                .unwrap_or(source.authentication_lifecycle_status),
            modified_at: common_utils::date_time::now(),
            error_code: error_code.or(source.error_code),
            error_message: error_message.or(source.error_message),
            connector_metadata: connector_metadata.or(source.connector_metadata),
            maximum_supported_version: maximum_supported_version
                .or(source.maximum_supported_version),
            threeds_server_transaction_id: threeds_server_transaction_id
                .or(source.threeds_server_transaction_id),
            authentication_flow_type: authentication_flow_type.or(source.authentication_flow_type),
            message_version: message_version.or(source.message_version),
            eci: eci.or(source.eci),
            trans_status: trans_status.or(source.trans_status),
            acquirer_bin: acquirer_bin.or(source.acquirer_bin),
            acquirer_merchant_id: acquirer_merchant_id.or(source.acquirer_merchant_id),
            three_ds_method_data: three_ds_method_data.or(source.three_ds_method_data),
            three_ds_method_url: three_ds_method_url.or(source.three_ds_method_url),
            acs_url: acs_url.or(source.acs_url),
            challenge_request: challenge_request.or(source.challenge_request),
            acs_reference_number: acs_reference_number.or(source.acs_reference_number),
            acs_trans_id: acs_trans_id.or(source.acs_trans_id),
            acs_signed_content: acs_signed_content.or(source.acs_signed_content),
            ds_trans_id: ds_trans_id.or(source.ds_trans_id),
            directory_server_id: directory_server_id.or(source.directory_server_id),
            acquirer_country_code: acquirer_country_code.or(source.acquirer_country_code),
            service_details: service_details.or(source.service_details),
            force_3ds_challenge: force_3ds_challenge.or(source.force_3ds_challenge),
            psd2_sca_exemption_type: psd2_sca_exemption_type.or(source.psd2_sca_exemption_type),
            billing_address: billing_address.or(source.billing_address),
            shipping_address: shipping_address.or(source.shipping_address),
            browser_info: browser_info.or(source.browser_info),
            email: email.or(source.email),
            profile_acquirer_id: profile_acquirer_id.or(source.profile_acquirer_id),
            challenge_code: challenge_code.or(source.challenge_code),
            challenge_cancel: challenge_cancel.or(source.challenge_cancel),
            challenge_code_reason: challenge_code_reason.or(source.challenge_code_reason),
            message_extension: message_extension.or(source.message_extension),
            challenge_request_key: challenge_request_key.or(source.challenge_request_key),
            customer_details: customer_details.or(source.customer_details),
            ..source
        }
    }
}

impl From<AuthenticationUpdate> for AuthenticationUpdateInternal {
    fn from(auth_update: AuthenticationUpdate) -> Self {
        match auth_update {
            AuthenticationUpdate::ErrorUpdate {
                error_message,
                error_code,
                authentication_status,
                connector_authentication_id,
            } => Self {
                error_code,
                error_message,
                authentication_status: Some(authentication_status),
                connector_authentication_id,
                authentication_type: None,
                authentication_lifecycle_status: None,
                modified_at: common_utils::date_time::now(),
                payment_method_id: None,
                connector_metadata: None,
                ..Default::default()
            },
            AuthenticationUpdate::PostAuthorizationUpdate {
                authentication_lifecycle_status,
            } => Self {
                connector_authentication_id: None,

                payment_method_id: None,
                authentication_type: None,
                authentication_status: None,
                authentication_lifecycle_status: Some(authentication_lifecycle_status),
                modified_at: common_utils::date_time::now(),
                error_message: None,
                error_code: None,
                connector_metadata: None,
                ..Default::default()
            },
            AuthenticationUpdate::PreAuthenticationUpdate {
                threeds_server_transaction_id,
                maximum_supported_3ds_version,
                connector_authentication_id,
                three_ds_method_data,
                three_ds_method_url,
                message_version,
                connector_metadata,
                authentication_status,
                acquirer_bin,
                acquirer_merchant_id,
                directory_server_id,
                acquirer_country_code,
                billing_address,
                shipping_address,
                browser_info,
                email,
            } => Self {
                threeds_server_transaction_id: Some(threeds_server_transaction_id),
                maximum_supported_version: Some(maximum_supported_3ds_version),
                connector_authentication_id: Some(connector_authentication_id),
                three_ds_method_data,
                three_ds_method_url,
                message_version: Some(message_version),
                connector_metadata,
                authentication_status: Some(authentication_status),
                acquirer_bin,
                acquirer_merchant_id,
                directory_server_id,
                acquirer_country_code,
                billing_address,
                shipping_address,
                browser_info: *browser_info,
                email,
                ..Default::default()
            },
            AuthenticationUpdate::AuthenticationUpdate {
                trans_status,
                authentication_type,
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                acs_signed_content,
                connector_metadata,
                authentication_status,
                ds_trans_id,
                eci,
                challenge_code,
                challenge_cancel,
                challenge_code_reason,
                message_extension,
                challenge_request_key,
            } => Self {
                trans_status: Some(trans_status),
                authentication_type: Some(authentication_type),
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                acs_signed_content,
                connector_metadata,
                authentication_status: Some(authentication_status),
                ds_trans_id,
                eci,
                challenge_code,
                challenge_cancel,
                challenge_code_reason,
                message_extension,
                challenge_request_key,
                ..Default::default()
            },
            AuthenticationUpdate::PostAuthenticationUpdate {
                trans_status,
                eci,
                authentication_status,
                challenge_cancel,
                challenge_code_reason,
            } => Self {
                trans_status: Some(trans_status),
                eci,
                authentication_status: Some(authentication_status),
                challenge_cancel,
                challenge_code_reason,
                ..Default::default()
            },
            AuthenticationUpdate::PreAuthenticationVersionCallUpdate {
                maximum_supported_3ds_version,
                message_version,
            } => Self {
                maximum_supported_version: Some(maximum_supported_3ds_version),
                message_version: Some(message_version),
                ..Default::default()
            },
            AuthenticationUpdate::PreAuthenticationThreeDsMethodCall {
                threeds_server_transaction_id,
                three_ds_method_data,
                three_ds_method_url,
                acquirer_bin,
                acquirer_merchant_id,
                connector_metadata,
            } => Self {
                threeds_server_transaction_id: Some(threeds_server_transaction_id),
                three_ds_method_data,
                three_ds_method_url,
                acquirer_bin,
                acquirer_merchant_id,
                connector_metadata,
                ..Default::default()
            },
            AuthenticationUpdate::AuthenticationStatusUpdate {
                trans_status,
                authentication_status,
            } => Self {
                trans_status: Some(trans_status),
                authentication_status: Some(authentication_status),
                ..Default::default()
            },
        }
    }
}
