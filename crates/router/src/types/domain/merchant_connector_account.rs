use common_utils::{
    crypto::{Encryptable, GcmAes256},
    errors::{CustomResult, ValidationError},
    pii,
};
use error_stack::ResultExt;
use masking::Secret;
use storage_models::{
    encryption::Encryption, enums,
    merchant_connector_account::MerchantConnectorAccountUpdateInternal,
};

use super::{
    behaviour,
    types::{self, TypeEncryption},
};
use crate::db::StorageInterface;

#[derive(Clone, Debug)]
pub struct MerchantConnectorAccount {
    pub id: Option<i32>,
    pub merchant_id: String,
    pub connector_name: String,
    pub connector_account_details: Encryptable<Secret<serde_json::Value>>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: String,
    pub payment_methods_enabled: Option<Vec<serde_json::Value>>,
    pub connector_type: enums::ConnectorType,
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug)]
pub enum MerchantConnectorAccountUpdate {
    Update {
        merchant_id: Option<String>,
        connector_type: Option<enums::ConnectorType>,
        connector_name: Option<String>,
        connector_account_details: Option<Encryptable<Secret<serde_json::Value>>>,
        test_mode: Option<bool>,
        disabled: Option<bool>,
        merchant_connector_id: Option<String>,
        payment_methods_enabled: Option<Vec<serde_json::Value>>,
        metadata: Option<pii::SecretSerdeValue>,
    },
}

#[async_trait::async_trait]
impl behaviour::Conversion for MerchantConnectorAccount {
    type DstType = storage_models::merchant_connector_account::MerchantConnectorAccount;
    type NewDstType = storage_models::merchant_connector_account::MerchantConnectorAccountNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(
            storage_models::merchant_connector_account::MerchantConnectorAccount {
                id: self.id.ok_or(ValidationError::MissingRequiredField {
                    field_name: "id".to_string(),
                })?,
                merchant_id: self.merchant_id,
                connector_name: self.connector_name,
                connector_account_details: self.connector_account_details.into(),
                test_mode: self.test_mode,
                disabled: self.disabled,
                merchant_connector_id: self.merchant_connector_id,
                payment_methods_enabled: self.payment_methods_enabled,
                connector_type: self.connector_type,
                metadata: self.metadata,
            },
        )
    }

    async fn convert_back(
        other: Self::DstType,
        db: &dyn StorageInterface,
        merchant_id: &str,
    ) -> CustomResult<Self, ValidationError> {
        let key = types::get_merchant_enc_key(db, merchant_id).await.change_context(
            ValidationError::InvalidValue {
                message: "Error while getting key from keystore".to_string(),
            },
        )?;
        Ok(Self {
            id: Some(other.id),
            merchant_id: other.merchant_id,
            connector_name: other.connector_name,
            connector_account_details: Encryptable::decrypt(
                other.connector_account_details,
                &key,
                GcmAes256 {},
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting connector account details".to_string(),
            })?,
            test_mode: other.test_mode,
            disabled: other.disabled,
            merchant_connector_id: other.merchant_connector_id,
            payment_methods_enabled: other.payment_methods_enabled,
            connector_type: other.connector_type,
            metadata: other.metadata,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(Self::NewDstType {
            merchant_id: Some(self.merchant_id),
            connector_name: Some(self.connector_name),
            connector_account_details: Some(self.connector_account_details.into()),
            test_mode: self.test_mode,
            disabled: self.disabled,
            merchant_connector_id: self.merchant_connector_id,
            payment_methods_enabled: self.payment_methods_enabled,
            connector_type: Some(self.connector_type),
            metadata: self.metadata,
        })
    }
}

impl From<MerchantConnectorAccountUpdate> for MerchantConnectorAccountUpdateInternal {
    fn from(merchant_connector_account_update: MerchantConnectorAccountUpdate) -> Self {
        match merchant_connector_account_update {
            MerchantConnectorAccountUpdate::Update {
                merchant_id,
                connector_type,
                connector_name,
                connector_account_details,
                test_mode,
                disabled,
                merchant_connector_id,
                payment_methods_enabled,
                metadata,
            } => Self {
                merchant_id,
                connector_type,
                connector_name,
                connector_account_details: connector_account_details.map(Encryption::from),
                test_mode,
                disabled,
                merchant_connector_id,
                payment_methods_enabled,
                metadata,
            },
        }
    }
}
