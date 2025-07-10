use common_utils::{
    crypto::Encryptable, encryption::Encryption, errors::CustomResult, pii,
    types::keymanager::ToEncryptable,
};
use masking::Secret;
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
    pub service_details: Option<Value>,
    pub organization_id: common_utils::id_type::OrganizationId,
    pub authentication_client_secret: Option<String>,
    pub force_3ds_challenge: Option<bool>,
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    pub return_url: Option<String>,
    pub amount: Option<common_utils::types::MinorUnit>,
    pub currency: Option<common_enums::Currency>,
    #[encrypt]
    pub billing_address: Option<Encryptable<Secret<Value>>>,
    #[encrypt]
    pub shipping_address: Option<Encryptable<Secret<Value>>>,
    pub browser_info: Option<Value>,
    pub email: Option<Encryptable<Secret<String, pii::EmailStrategy>>>,
}

// #[cfg(feature = "v1")]
// #[async_trait::async_trait]
// impl behaviour::Conversion for Authentication {
//     type DstType = diesel_models::authentication::Authentication;
//     type NewDstType = diesel_models::authentication::AuthenticationNew;

//     async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
//         Ok(diesel_models::authentication::Authentication {
//             authentication_id: self.authentication_id,
//             merchant_id: self.merchant_id,
//             authentication_connector: self.authentication_connector,
//             connector_authentication_id: self.connector_authentication_id,
//             authentication_data: self.authentication_data,
//             payment_method_id: self.payment_method_id,
//             authentication_type: self.authentication_type,
//             authentication_status: self.authentication_status,
//             authentication_lifecycle_status: self.authentication_lifecycle_status,
//             created_at: self.created_at,
//             modified_at: self.modified_at,
//             error_message: self.error_message,
//             error_code: self.error_code,
//             connector_metadata: self.connector_metadata,
//             maximum_supported_version: self.maximum_supported_version,
//             threeds_server_transaction_id: self.threeds_server_transaction_id,
//             cavv: self.cavv,
//             authentication_flow_type: self.authentication_flow_type,
//             message_version: self.message_version,
//             eci: self.eci,
//             trans_status: self.trans_status,
//             acquirer_bin: self.acquirer_bin,
//             acquirer_merchant_id: self.acquirer_merchant_id,
//             three_ds_method_data: self.three_ds_method_data,
//             three_ds_method_url: self.three_ds_method_url,
//             acs_url: self.acs_url,
//             challenge_request: self.challenge_request,
//             acs_reference_number: self.acs_reference_number,
//             acs_trans_id: self.acs_trans_id,
//             acs_signed_content: self.acs_signed_content,
//             profile_id: self.profile_id,
//             payment_id: self.payment_id,
//             merchant_connector_id: self.merchant_connector_id,
//             ds_trans_id: self.ds_trans_id,
//             directory_server_id: self.directory_server_id,
//             acquirer_country_code: self.acquirer_country_code,
//             service_details: self.service_details,
//             organization_id: self.organization_id,
//             authentication_client_secret: self.authentication_client_secret,
//             force_3ds_challenge: self.force_3ds_challenge,
//             psd2_sca_exemption_type: self.psd2_sca_exemption_type,
//             return_url: self.return_url,
//             amount: self.amount,
//             currency: self.currency,
//             billing_address: self.billing_address.map(Encryption::from),
//             shipping_address: self.shipping_address.map(Encryption::from),
//             browser_info: self.browser_info,
//             email: self.email.map(Encryption::from),
//         })
//     }

//     async fn convert_back(
//         state: &KeyManagerState,
//         item: Self::DstType,
//         key: &Secret<Vec<u8>>,
//         _key_store_ref_id: keymanager::Identifier,
//     ) -> CustomResult<Self, ValidationError>
//     where
//         Self: Sized,
//     {
//         let decrypted = types::crypto_operation(
//             state,
//             common_utils::type_name!(Self::DstType),
//             types::CryptoOperation::BatchDecrypt(EncryptedAuthentication::to_encryptable(
//                 EncryptedAuthentication {
//                     billing_address: item.billing_address.clone(),
//                     shipping_address: item.shipping_address.clone(),
//                     email: item.email.clone(),
//                 },
//             )),
//             keymanager::Identifier::Merchant(item.merchant_id.clone()),
//             key.peek(),
//         )
//         .await
//         .and_then(|val| val.try_into_batchoperation())
//         .change_context(ValidationError::InvalidValue {
//             message: "Failed while decrypting authentication data".to_string(),
//         })?;

//         let encryptable_authentication = EncryptedAuthentication::from_encryptable(decrypted).change_context(
//             ValidationError::InvalidValue {
//                 message: "Failed while decrypting authentication data".to_string(),
//             },
//         )?;

//         Ok(Self {
//             authentication_id: item.authentication_id,
//             merchant_id: item.merchant_id,
//             authentication_connector: item.authentication_connector,
//             connector_authentication_id: item.connector_authentication_id,
//             authentication_data: item.authentication_data,
//             payment_method_id: item.payment_method_id,
//             authentication_type: item.authentication_type,
//             authentication_status: item.authentication_status,
//             authentication_lifecycle_status: item.authentication_lifecycle_status,
//             created_at: item.created_at,
//             modified_at: item.modified_at,
//             error_message: item.error_message,
//             error_code: item.error_code,
//             connector_metadata: item.connector_metadata,
//             maximum_supported_version: item.maximum_supported_version,
//             threeds_server_transaction_id: item.threeds_server_transaction_id,
//             cavv: item.cavv,
//             authentication_flow_type: item.authentication_flow_type,
//             message_version: item.message_version,
//             eci: item.eci,
//             trans_status: item.trans_status,
//             acquirer_bin: item.acquirer_bin,
//             acquirer_merchant_id: item.acquirer_merchant_id,
//             three_ds_method_data: item.three_ds_method_data,
//             three_ds_method_url: item.three_ds_method_url,
//             acs_url: item.acs_url,
//             challenge_request: item.challenge_request,
//             acs_reference_number: item.acs_reference_number,
//             acs_trans_id: item.acs_trans_id,
//             acs_signed_content: item.acs_signed_content,
//             profile_id: item.profile_id,
//             payment_id: item.payment_id,
//             merchant_connector_id: item.merchant_connector_id,
//             ds_trans_id: item.ds_trans_id,
//             directory_server_id: item.directory_server_id,
//             acquirer_country_code: item.acquirer_country_code,
//             service_details: item.service_details,
//             organization_id: item.organization_id,
//             authentication_client_secret: item.authentication_client_secret,
//             force_3ds_challenge: item.force_3ds_challenge,
//             psd2_sca_exemption_type: item.psd2_sca_exemption_type,
//             return_url: item.return_url,
//             amount: item.amount,
//             currency: item.currency,
//             billing_address: encryptable_authentication.billing_address,
//             shipping_address: encryptable_authentication.shipping_address,
//             browser_info: item.browser_info,
//             email: encryptable_authentication.email.map(|email| {
//                 let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> = Encryptable::new(
//                     email.clone().into_inner().switch_strategy(),
//                     email.into_encrypted(),
//                 );
//                 encryptable
//             }),
//         })
//     }

//     async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
//         let now = date_time::now();
//         Ok(diesel_models::authentication::AuthenticationNew {
//             authentication_id: self.authentication_id,
//             merchant_id: self.merchant_id,
//             authentication_connector: self.authentication_connector,
//             connector_authentication_id: self.connector_authentication_id,
//             payment_method_id: self.payment_method_id,
//             authentication_type: self.authentication_type,
//             authentication_status: self.authentication_status,
//             authentication_lifecycle_status: self.authentication_lifecycle_status,
//             error_message: self.error_message,
//             error_code: self.error_code,
//             connector_metadata: self.connector_metadata,
//             maximum_supported_version: self.maximum_supported_version,
//             threeds_server_transaction_id: self.threeds_server_transaction_id,
//             cavv: self.cavv,
//             authentication_flow_type: self.authentication_flow_type,
//             message_version: self.message_version,
//             eci: self.eci,
//             trans_status: self.trans_status,
//             acquirer_bin: self.acquirer_bin,
//             acquirer_merchant_id: self.acquirer_merchant_id,
//             three_ds_method_data: self.three_ds_method_data,
//             three_ds_method_url: self.three_ds_method_url,
//             acs_url: self.acs_url,
//             challenge_request: self.challenge_request,
//             acs_reference_number: self.acs_reference_number,
//             acs_trans_id: self.acs_trans_id,
//             acs_signed_content: self.acs_signed_content,
//             profile_id: self.profile_id,
//             payment_id: self.payment_id,
//             merchant_connector_id: self.merchant_connector_id,
//             ds_trans_id: self.ds_trans_id,
//             directory_server_id: self.directory_server_id,
//             acquirer_country_code: self.acquirer_country_code,
//             service_details: self.service_details,
//             organization_id: self.organization_id,
//             authentication_client_secret: self.authentication_client_secret,
//             force_3ds_challenge: self.force_3ds_challenge,
//             psd2_sca_exemption_type: self.psd2_sca_exemption_type,
//             return_url: self.return_url,
//             amount: self.amount,
//             currency: self.currency,
//             billing_address: self.billing_address.map(Encryption::from),
//             shipping_address: self.shipping_address.map(Encryption::from),
//             browser_info: self.browser_info,
//             email: self.email.map(Encryption::from),
//         })
//     }
// }
