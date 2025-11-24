use crate::type_encryption::{crypto_operation, AsyncLift, CryptoOperation};

use super::behaviour;
use async_trait::async_trait;
#[cfg(feature = "v1")]
use common_enums::MerchantCategoryCode;
use common_utils::{
    crypto::Encryptable,
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    pii,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use error_stack::ResultExt;
use masking::Secret;
use masking::PeekInterface;
use rustc_hash::FxHashMap;
use serde_json::Value;

#[cfg(feature = "v1")]
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
    pub amount: Option<common_utils::types::MinorUnit>,
    pub currency: Option<common_enums::Currency>,
    pub merchant_country: Option<String>,
    pub billing_country: Option<String>,
    pub shipping_country: Option<String>,
    pub issuer_country: Option<String>,
    pub earliest_supported_version: Option<common_utils::types::SemanticVersion>,
    pub latest_supported_version: Option<common_utils::types::SemanticVersion>,
    pub whitelist_decision: Option<bool>,
    pub device_manufacturer: Option<String>,
    pub platform: Option<String>,
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
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let encrypted_data = crypto_operation(
            &state,
            common_utils::type_name!(Authentication),
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
                    &state,
                    common_utils::type_name!(Authentication),
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
            mcc: None,
            amount: other.amount,
            currency: other.currency,
            merchant_country: None,
            billing_country: None,
            shipping_country: None,
            issuer_country: None,
            earliest_supported_version: None,
            latest_supported_version: None,
            whitelist_decision: None,
            device_manufacturer: None,
            platform: None,
            device_type: None,
            device_brand: None,
            device_os: None,
            device_display: None,
            browser_name: None,
            browser_version: None,
            issuer_id: None,
            scheme_name: None,
            exemption_requested: None,
            exemption_accepted: None,
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
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(Self::NewDstType {
            authentication_id: self.authentication_id,
            merchant_id: self.merchant_id,
            authentication_connector: self.authentication_connector,
            connector_authentication_id: self.connector_authentication_id,
            payment_method_id: self.payment_method_id,
            authentication_type: self.authentication_type,
            authentication_status: self.authentication_status,
            authentication_lifecycle_status: self.authentication_lifecycle_status,
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
        })
    }
}
