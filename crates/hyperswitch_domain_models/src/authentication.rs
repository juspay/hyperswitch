use std::str::FromStr;

use async_trait::async_trait;
use common_enums::MerchantCategoryCode;
use common_utils::{
    crypto::Encryptable,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    pii,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use rustc_hash::FxHashMap;
use serde_json::Value;

use super::behaviour;
use crate::type_encryption::{crypto_operation, AsyncLift, CryptoOperation};

// #[cfg(feature = "v1")]
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
}

// #[cfg(feature = "v1")]
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

#[async_trait]
impl behaviour::Conversion for Authentication {
    type DstType = diesel_models::authentication::Authentication;
    type NewDstType = diesel_models::authentication::AuthenticationNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(Self::DstType {
            authentication_id: self.authentication_id,
            merchant_id: self.merchant_id,
            authentication_connector: self.authentication_connector,
            connector_authentication_id: self.connector_authentication_id,
            authentication_data: self.authentication_data,
            payment_method_id: self.payment_method_id,
            authentication_type: self.authentication_type,
            authentication_status: self.authentication_status,
            authentication_lifecycle_status: self.authentication_lifecycle_status,
            created_at: self.created_at,
            modified_at: self.modified_at,
            error_message: self.error_message,
            error_code: self.error_code,
            connector_metadata: self.connector_metadata,
            maximum_supported_version: self.maximum_supported_version,
            threeds_server_transaction_id: self.threeds_server_transaction_id,
            cavv: self.cavv,
            authentication_flow_type: self.authentication_flow_type,
            message_version: self.message_version,
            eci: self.eci,
            trans_status: self.trans_status,
            acquirer_bin: self.acquirer_bin,
            acquirer_merchant_id: self.acquirer_merchant_id,
            three_ds_method_data: self.three_ds_method_data,
            three_ds_method_url: self.three_ds_method_url,
            acs_url: self.acs_url,
            challenge_request: self.challenge_request,
            acs_reference_number: self.acs_reference_number,
            acs_trans_id: self.acs_trans_id,
            acs_signed_content: self.acs_signed_content,
            profile_id: self.profile_id,
            payment_id: self.payment_id,
            merchant_connector_id: self.merchant_connector_id,
            ds_trans_id: self.ds_trans_id,
            directory_server_id: self.directory_server_id,
            acquirer_country_code: self.acquirer_country_code,
            service_details: self.service_details,
            organization_id: self.organization_id,
            authentication_client_secret: self.authentication_client_secret,
            force_3ds_challenge: self.force_3ds_challenge,
            psd2_sca_exemption_type: self.psd2_sca_exemption_type,
            return_url: self.return_url,
            amount: self.amount,
            currency: self.currency,
            billing_address: self.billing_address.map(Encryption::from),
            shipping_address: self.shipping_address.map(Encryption::from),
            browser_info: self.browser_info,
            email: self.email.map(|email| email.into()),
            profile_acquirer_id: self.profile_acquirer_id,
            challenge_code: self.challenge_code,
            challenge_cancel: self.challenge_cancel,
            challenge_code_reason: self.challenge_code_reason,
            message_extension: self.message_extension,
            challenge_request_key: self.challenge_request_key,
            customer_details: self.customer_details,
            earliest_supported_version: self.earliest_supported_version,
            latest_supported_version: self.latest_supported_version,
            mcc: self.mcc,
            platform: self.platform.map(|platform| platform.to_string()),
            device_type: self.device_type,
            device_brand: self.device_brand,
            device_os: self.device_os,
            device_display: self.device_display,
            browser_name: self.browser_name,
            browser_version: self.browser_version,
            scheme_name: self.scheme_name,
            exemption_requested: self.exemption_requested,
            exemption_accepted: self.exemption_accepted,
            issuer_id: self.issuer_id,
            issuer_country: self.issuer_country,
            merchant_country_code: self.merchant_country_code,
            billing_country: self.billing_country,
            shipping_country: self.shipping_country,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let encrypted_data = crypto_operation(
            state,
            common_utils::type_name!(Self),
            CryptoOperation::BatchDecrypt(EncryptedAuthentication::to_encryptable(
                EncryptedAuthentication {
                    billing_address: other.billing_address,
                    shipping_address: other.shipping_address,
                },
            )),
            Identifier::Merchant(other.merchant_id.clone()),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting authentication data".to_string(),
        })?;

        let decrypted_data = FromRequestEncryptableAuthentication::from_encryptable(encrypted_data)
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting authentication data".to_string(),
            })?;

        let email_decrypted = other
            .email
            .clone()
            .async_lift(|inner| async {
                crypto_operation::<String, pii::EmailStrategy>(
                    state,
                    common_utils::type_name!(Self),
                    CryptoOperation::DecryptOptional(inner),
                    Identifier::Merchant(other.merchant_id.clone()),
                    key.peek(),
                )
                .await
                .and_then(|val| val.try_into_optionaloperation())
            })
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting authentication email".to_string(),
            })?;

        Ok(Self {
            authentication_id: other.authentication_id,
            merchant_id: other.merchant_id,
            authentication_connector: other.authentication_connector,
            connector_authentication_id: other.connector_authentication_id,
            authentication_data: other.authentication_data,
            payment_method_id: other.payment_method_id,
            authentication_type: other.authentication_type,
            authentication_status: other.authentication_status,
            authentication_lifecycle_status: other.authentication_lifecycle_status,
            created_at: other.created_at,
            modified_at: other.modified_at,
            error_message: other.error_message,
            error_code: other.error_code,
            connector_metadata: other.connector_metadata,
            maximum_supported_version: other.maximum_supported_version,
            threeds_server_transaction_id: other.threeds_server_transaction_id,
            cavv: other.cavv,
            authentication_flow_type: other.authentication_flow_type,
            message_version: other.message_version,
            eci: other.eci,
            trans_status: other.trans_status,
            acquirer_bin: other.acquirer_bin,
            acquirer_merchant_id: other.acquirer_merchant_id,
            three_ds_method_data: other.three_ds_method_data,
            three_ds_method_url: other.three_ds_method_url,
            acs_url: other.acs_url,
            challenge_request: other.challenge_request,
            acs_reference_number: other.acs_reference_number,
            acs_trans_id: other.acs_trans_id,
            acs_signed_content: other.acs_signed_content,
            profile_id: other.profile_id,
            payment_id: other.payment_id,
            merchant_connector_id: other.merchant_connector_id,
            ds_trans_id: other.ds_trans_id,
            directory_server_id: other.directory_server_id,
            acquirer_country_code: other.acquirer_country_code,
            organization_id: other.organization_id,
            mcc: other.mcc,
            amount: other.amount,
            currency: other.currency,
            issuer_country: other.issuer_country,
            earliest_supported_version: other.earliest_supported_version,
            latest_supported_version: other.latest_supported_version,
            platform: other
                .platform
                .as_deref()
                .map(|s| {
                    api_models::payments::DeviceChannel::from_str(s).change_context(
                        ValidationError::InvalidValue {
                            message: "Invalid device channel".into(),
                        },
                    )
                })
                .transpose()?,
            device_type: other.device_type,
            device_brand: other.device_brand,
            device_os: other.device_os,
            device_display: other.device_display,
            browser_name: other.browser_name,
            browser_version: other.browser_version,
            issuer_id: other.issuer_id,
            scheme_name: other.scheme_name,
            exemption_requested: other.exemption_requested,
            exemption_accepted: other.exemption_accepted,
            service_details: other.service_details,
            authentication_client_secret: other.authentication_client_secret,
            force_3ds_challenge: other.force_3ds_challenge,
            psd2_sca_exemption_type: other.psd2_sca_exemption_type,
            return_url: other.return_url,
            billing_address: decrypted_data.billing_address,
            shipping_address: decrypted_data.shipping_address,
            browser_info: other.browser_info,
            email: email_decrypted,
            profile_acquirer_id: other.profile_acquirer_id,
            challenge_code: other.challenge_code,
            challenge_cancel: other.challenge_cancel,
            challenge_code_reason: other.challenge_code_reason,
            message_extension: other.message_extension,
            challenge_request_key: other.challenge_request_key,
            customer_details: other.customer_details,
            billing_country: other.billing_country,
            shipping_country: other.shipping_country,
            merchant_country_code: other.merchant_country_code,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(Self::NewDstType {
            authentication_id: __self.authentication_id,
            merchant_id: __self.merchant_id,
            authentication_connector: __self.authentication_connector,
            connector_authentication_id: __self.connector_authentication_id,
            payment_method_id: __self.payment_method_id,
            authentication_type: __self.authentication_type,
            authentication_status: __self.authentication_status,
            authentication_lifecycle_status: __self.authentication_lifecycle_status,
            error_message: __self.error_message,
            error_code: __self.error_code,
            connector_metadata: __self.connector_metadata,
            maximum_supported_version: __self.maximum_supported_version,
            threeds_server_transaction_id: __self.threeds_server_transaction_id,
            cavv: __self.cavv,
            authentication_flow_type: __self.authentication_flow_type,
            message_version: __self.message_version,
            eci: __self.eci,
            trans_status: __self.trans_status,
            acquirer_bin: __self.acquirer_bin,
            acquirer_merchant_id: __self.acquirer_merchant_id,
            three_ds_method_data: __self.three_ds_method_data,
            three_ds_method_url: __self.three_ds_method_url,
            acs_url: __self.acs_url,
            challenge_request: __self.challenge_request,
            acs_reference_number: __self.acs_reference_number,
            acs_trans_id: __self.acs_trans_id,
            acs_signed_content: __self.acs_signed_content,
            profile_id: __self.profile_id,
            payment_id: __self.payment_id,
            merchant_connector_id: __self.merchant_connector_id,
            ds_trans_id: __self.ds_trans_id,
            directory_server_id: __self.directory_server_id,
            acquirer_country_code: __self.acquirer_country_code,
            service_details: __self.service_details,
            organization_id: __self.organization_id,
            authentication_client_secret: __self.authentication_client_secret,
            force_3ds_challenge: __self.force_3ds_challenge,
            psd2_sca_exemption_type: __self.psd2_sca_exemption_type,
            return_url: __self.return_url,
            amount: __self.amount,
            currency: __self.currency,
            billing_address: __self.billing_address.map(Encryption::from),
            shipping_address: __self.shipping_address.map(Encryption::from),
            browser_info: __self.browser_info,
            email: __self.email.map(|email| email.into()),
            profile_acquirer_id: __self.profile_acquirer_id,
            challenge_code: __self.challenge_code,
            challenge_cancel: __self.challenge_cancel,
            challenge_code_reason: __self.challenge_code_reason,
            message_extension: __self.message_extension,
            challenge_request_key: __self.challenge_request_key,
            customer_details: __self.customer_details,
            earliest_supported_version: __self.earliest_supported_version,
            latest_supported_version: __self.latest_supported_version,
            mcc: __self.mcc,
            platform: __self.platform.map(|platform| platform.to_string()),
            device_type: __self.device_type,
            device_brand: __self.device_brand,
            device_os: __self.device_os,
            device_display: __self.device_display,
            browser_name: __self.browser_name,
            browser_version: __self.browser_version,
            scheme_name: __self.scheme_name,
            exemption_requested: __self.exemption_requested,
            exemption_accepted: __self.exemption_accepted,
            issuer_id: __self.issuer_id,
            issuer_country: __self.issuer_country,
            merchant_country_code: __self.merchant_country_code,
            created_at: __self.created_at,
            modified_at: __self.modified_at,
            authentication_data: __self.authentication_data,
            billing_country: __self.billing_country,
            shipping_country: __self.shipping_country,
        })
    }
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
