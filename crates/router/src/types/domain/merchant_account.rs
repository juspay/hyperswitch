use common_utils::{
    crypto::{Encryptable, GcmAes256},
    ext_traits::AsyncExt,
    pii,
};
use error_stack::ResultExt;
use masking::Secret;
use storage_models::{
    encryption::Encryption, enums, merchant_account::MerchantAccountUpdateInternal,
};

use crate::{
    db::StorageInterface,
    errors::{CustomResult, ValidationError},
    types::domain::types::{get_key_and_algo, TypeEncryption},
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct MerchantAccount {
    pub id: Option<i32>,
    pub merchant_id: String,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub merchant_name: Option<Encryptable<Secret<String>>>,
    pub merchant_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub webhook_details: Option<serde_json::Value>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub publishable_key: Option<String>,
    pub storage_scheme: enums::MerchantStorageScheme,
    pub locker_id: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub api_key: Option<Encryptable<Secret<String>>>,
}

#[derive(Debug)]
pub enum MerchantAccountUpdate {
    Update {
        merchant_name: Option<Encryptable<Secret<String>>>,
        merchant_details: Option<Encryptable<Secret<serde_json::Value>>>,
        return_url: Option<String>,
        webhook_details: Option<serde_json::Value>,
        sub_merchants_enabled: Option<bool>,
        parent_merchant_id: Option<String>,
        enable_payment_response_hash: Option<bool>,
        payment_response_hash_key: Option<String>,
        redirect_to_merchant_with_http_post: Option<bool>,
        publishable_key: Option<String>,
        locker_id: Option<String>,
        metadata: Option<pii::SecretSerdeValue>,
        routing_algorithm: Option<serde_json::Value>,
    },
    StorageSchemeUpdate {
        storage_scheme: enums::MerchantStorageScheme,
    },
}

impl From<MerchantAccountUpdate> for MerchantAccountUpdateInternal {
    fn from(merchant_account_update: MerchantAccountUpdate) -> Self {
        match merchant_account_update {
            MerchantAccountUpdate::Update {
                merchant_name,
                merchant_details,
                return_url,
                webhook_details,
                routing_algorithm,
                sub_merchants_enabled,
                parent_merchant_id,
                enable_payment_response_hash,
                payment_response_hash_key,
                redirect_to_merchant_with_http_post,
                publishable_key,
                locker_id,
                metadata,
            } => Self {
                merchant_name: merchant_name.map(Encryption::from),
                merchant_details: merchant_details.map(Encryption::from),
                return_url,
                webhook_details,
                routing_algorithm,
                sub_merchants_enabled,
                parent_merchant_id,
                enable_payment_response_hash,
                payment_response_hash_key,
                redirect_to_merchant_with_http_post,
                publishable_key,
                locker_id,
                metadata,
                ..Default::default()
            },
            MerchantAccountUpdate::StorageSchemeUpdate { storage_scheme } => Self {
                storage_scheme: Some(storage_scheme),
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for MerchantAccount {
    type DstType = storage_models::merchant_account::MerchantAccount;
    type NewDstType = storage_models::merchant_account::MerchantAccountNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(storage_models::merchant_account::MerchantAccount {
            id: self.id.ok_or(ValidationError::MissingRequiredField {
                field_name: "id".to_string(),
            })?,
            merchant_id: self.merchant_id,
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            merchant_name: self.merchant_name.map(|name| name.into()),
            merchant_details: self.merchant_details.map(|details| details.into()),
            webhook_details: self.webhook_details,
            sub_merchants_enabled: self.sub_merchants_enabled,
            parent_merchant_id: self.parent_merchant_id,
            publishable_key: self.publishable_key,
            storage_scheme: self.storage_scheme,
            locker_id: self.locker_id,
            metadata: self.metadata,
            routing_algorithm: self.routing_algorithm,
            api_key: self.api_key.map(|api_key| api_key.into()),
        })
    }

    async fn convert_back(
        item: Self::DstType,
        db: &dyn StorageInterface,
        merchant_id: &str,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let key = get_key_and_algo(db, merchant_id.to_owned())
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed while getting key from key store".to_string(),
            })?;
        Ok(Self {
            id: Some(item.id),
            merchant_id: item.merchant_id,
            return_url: item.return_url,
            enable_payment_response_hash: item.enable_payment_response_hash,
            payment_response_hash_key: item.payment_response_hash_key,
            redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
            merchant_name: item
                .merchant_name
                .async_map(|value| Encryptable::decrypt(value, &key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting customer data".to_string(),
                })?,
            merchant_details: item
                .merchant_details
                .async_map(|value| Encryptable::decrypt(value, &key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting customer data".to_string(),
                })?,
            webhook_details: item.webhook_details,
            sub_merchants_enabled: item.sub_merchants_enabled,
            parent_merchant_id: item.parent_merchant_id,
            publishable_key: item.publishable_key,
            storage_scheme: item.storage_scheme,
            locker_id: item.locker_id,
            metadata: item.metadata,
            routing_algorithm: item.routing_algorithm,
            api_key: item
                .api_key
                .async_map(|value| Encryptable::decrypt(value, &key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting customer data".to_string(),
                })?,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(storage_models::merchant_account::MerchantAccountNew {
            merchant_id: self.merchant_id,
            merchant_name: self.merchant_name.map(Encryption::from),
            merchant_details: self.merchant_details.map(Encryption::from),
            return_url: self.return_url,
            webhook_details: self.webhook_details,
            sub_merchants_enabled: self.sub_merchants_enabled,
            parent_merchant_id: self.parent_merchant_id,
            enable_payment_response_hash: Some(self.enable_payment_response_hash),
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: Some(self.redirect_to_merchant_with_http_post),
            publishable_key: self.publishable_key,
            locker_id: self.locker_id,
            metadata: self.metadata,
            routing_algorithm: self.routing_algorithm,
            api_key: self.api_key.map(Encryption::from),
        })
    }
}
