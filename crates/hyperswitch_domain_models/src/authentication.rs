use std::str::FromStr;

use common_enums::MerchantCategoryCode;
use common_utils::{
    crypto::Encryptable,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    pii,
    types::keymanager::ToEncryptable,
};
use error_stack::ResultExt;
use hyperswitch_masking::Secret;
use rustc_hash::FxHashMap;
use serde_json::Value;

use crate::type_encryption;

#[derive(Clone, Debug, router_derive::ToEncryption, serde::Serialize)]
pub struct Authentication {
    pub authentication_id: common_utils::id_type::AuthenticationId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub authentication_connector: Option<String>,
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<Value>,
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
    pub connector_metadata: Option<Value>,
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
    pub organization_id: common_utils::id_type::OrganizationId,
    pub mcc: Option<MerchantCategoryCode>,
    pub currency: Option<common_enums::Currency>,
    pub billing_country: Option<String>,
    pub shipping_country: Option<String>,
    pub issuer_country: Option<String>,
    pub earliest_supported_version: Option<common_utils::types::SemanticVersion>,
    pub latest_supported_version: Option<common_utils::types::SemanticVersion>,
    pub platform: Option<api_models::payments::DeviceChannel>,
    pub device_type: Option<String>,
    pub device_brand: Option<String>,
    pub device_os: Option<String>,
    pub device_display: Option<String>,
    pub browser_name: Option<String>,
    pub browser_version: Option<String>,
    pub issuer_id: Option<String>,
    pub scheme_name: Option<String>,
    pub exemption_requested: Option<bool>,
    pub exemption_accepted: Option<bool>,
    pub service_details: Option<Value>,
    pub authentication_client_secret: Option<String>,
    pub force_3ds_challenge: Option<bool>,
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    pub return_url: Option<String>,
    #[encrypt(ty = Value)]
    pub billing_address: Option<Encryptable<Secret<Value>>>,
    #[encrypt(ty = Value)]
    pub shipping_address: Option<Encryptable<Secret<Value>>>,
    pub browser_info: Option<Value>,
    pub email: Option<Encryptable<Secret<String, pii::EmailStrategy>>>,
    pub profile_acquirer_id: Option<common_utils::id_type::ProfileAcquirerId>,
    pub challenge_code: Option<String>,
    pub challenge_cancel: Option<String>,
    pub challenge_code_reason: Option<String>,
    pub message_extension: Option<pii::SecretSerdeValue>,
    pub challenge_request_key: Option<String>,
    pub customer_details: Option<Encryption>,
    pub amount: Option<common_utils::types::MinorUnit>,
    pub merchant_country_code: Option<String>,
    pub processor_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub created_by: Option<common_utils::types::CreatedBy>,
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct PgRedirectResponseForAuthentication {
    pub authentication_id: common_utils::id_type::AuthenticationId,
    pub status: common_enums::TransactionStatus,
    pub gateway_id: String,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub amount: Option<common_utils::types::MinorUnit>,
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
        connector_metadata: Option<Value>,
    },
    PreAuthenticationUpdate {
        threeds_server_transaction_id: String,
        maximum_supported_3ds_version: common_utils::types::SemanticVersion,
        connector_authentication_id: String,
        three_ds_method_data: Option<String>,
        three_ds_method_url: Option<String>,
        message_version: common_utils::types::SemanticVersion,
        connector_metadata: Option<Value>,
        authentication_status: common_enums::AuthenticationStatus,
        acquirer_bin: Option<String>,
        acquirer_merchant_id: Option<String>,
        directory_server_id: Option<String>,
        acquirer_country_code: Option<String>,
        billing_address: Box<Option<Encryptable<Secret<Value>>>>,
        shipping_address: Box<Option<Encryptable<Secret<Value>>>>,
        browser_info: Box<Option<Value>>,
        email: Option<Encryptable<Secret<String, pii::EmailStrategy>>>,
        scheme_id: Option<String>,
        merchant_category_code: Option<MerchantCategoryCode>,
        merchant_country_code: Option<String>,
        billing_country: Option<String>,
        shipping_country: Option<String>,
        earliest_supported_version: Option<common_utils::types::SemanticVersion>,
        latest_supported_version: Option<common_utils::types::SemanticVersion>,
    },
    AuthenticationUpdate {
        trans_status: common_enums::TransactionStatus,
        authentication_type: common_enums::DecoupledAuthenticationType,
        acs_url: Option<String>,
        challenge_request: Option<String>,
        acs_reference_number: Option<String>,
        acs_trans_id: Option<String>,
        acs_signed_content: Option<String>,
        connector_metadata: Option<Value>,
        authentication_status: common_enums::AuthenticationStatus,
        ds_trans_id: Option<String>,
        eci: Option<String>,
        challenge_code: Option<String>,
        challenge_cancel: Option<String>,
        challenge_code_reason: Option<String>,
        message_extension: Option<pii::SecretSerdeValue>,
        challenge_request_key: Option<String>,
        device_type: Option<String>,
        device_brand: Option<String>,
        device_os: Option<String>,
        device_display: Option<String>,
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

impl From<AuthenticationUpdate> for diesel_models::authentication::AuthenticationUpdate {
    fn from(authentication_update: AuthenticationUpdate) -> Self {
        match authentication_update {
            AuthenticationUpdate::PreAuthenticationVersionCallUpdate {
                maximum_supported_3ds_version,
                message_version,
            } => Self::PreAuthenticationVersionCallUpdate {
                maximum_supported_3ds_version,
                message_version,
            },
            AuthenticationUpdate::PreAuthenticationThreeDsMethodCall {
                threeds_server_transaction_id,
                three_ds_method_data,
                three_ds_method_url,
                acquirer_bin,
                acquirer_merchant_id,
                connector_metadata,
            } => Self::PreAuthenticationThreeDsMethodCall {
                threeds_server_transaction_id,
                three_ds_method_data,
                three_ds_method_url,
                acquirer_bin,
                acquirer_merchant_id,
                connector_metadata,
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
                scheme_id,
                merchant_category_code,
                merchant_country_code,
                billing_country,
                shipping_country,
                earliest_supported_version,
                latest_supported_version,
            } => Self::PreAuthenticationUpdate {
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
                billing_address: billing_address.map(|billing_address| billing_address.into()),
                shipping_address: shipping_address.map(|shipping_address| shipping_address.into()),
                browser_info,
                email: email.map(|email| email.into()),
                scheme_id,
                merchant_category_code,
                merchant_country_code,
                billing_country,
                shipping_country,
                earliest_supported_version,
                latest_supported_version,
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
                device_type,
                device_brand,
                device_os,
                device_display,
            } => Self::AuthenticationUpdate {
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
                device_type,
                device_brand,
                device_os,
                device_display,
            },
            AuthenticationUpdate::PostAuthenticationUpdate {
                trans_status,
                eci,
                authentication_status,
                challenge_cancel,
                challenge_code_reason,
            } => Self::PostAuthenticationUpdate {
                trans_status,
                eci,
                authentication_status,
                challenge_cancel,
                challenge_code_reason,
            },
            AuthenticationUpdate::ErrorUpdate {
                error_message,
                error_code,
                authentication_status,
                connector_authentication_id,
            } => Self::ErrorUpdate {
                error_message,
                error_code,
                authentication_status,
                connector_authentication_id,
            },
            AuthenticationUpdate::PostAuthorizationUpdate {
                authentication_lifecycle_status,
            } => Self::PostAuthorizationUpdate {
                authentication_lifecycle_status,
            },
            AuthenticationUpdate::AuthenticationStatusUpdate {
                trans_status,
                authentication_status,
            } => Self::AuthenticationStatusUpdate {
                trans_status,
                authentication_status,
            },
        }
    }
}
