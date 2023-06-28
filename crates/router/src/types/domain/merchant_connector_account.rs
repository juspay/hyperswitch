use common_utils::{
    crypto::{Encryptable, GcmAes256},
    date_time,
    errors::{CustomResult, ValidationError},
    pii,
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use storage_models::{
    encryption::Encryption, enums,
    merchant_connector_account::MerchantConnectorAccountUpdateInternal,
};

use super::{behaviour, types::TypeEncryption};
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
    pub frm_configs: Option<Secret<serde_json::Value>>, //Option<FrmConfigs>
    pub connector_label: String,
    pub business_country: enums::CountryAlpha2,
    pub business_label: String,
    pub business_sub_label: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
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
        frm_configs: Option<Secret<serde_json::Value>>,
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
                frm_configs: self.frm_configs,
                business_country: self.business_country,
                business_label: self.business_label,
                connector_label: self.connector_label,
                business_sub_label: self.business_sub_label,
                created_at: self.created_at,
                modified_at: self.modified_at,
            },
        )
    }

    async fn convert_back(
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError> {
        Ok(Self {
            id: Some(other.id),
            merchant_id: other.merchant_id,
            connector_name: other.connector_name,
            connector_account_details: Encryptable::decrypt(
                other.connector_account_details,
                key.peek(),
                GcmAes256,
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

            frm_configs: other.frm_configs,
            business_country: other.business_country,
            business_label: other.business_label,
            connector_label: other.connector_label,
            business_sub_label: other.business_sub_label,
            created_at: other.created_at,
            modified_at: other.modified_at,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
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
            frm_configs: self.frm_configs,
            business_country: self.business_country,
            business_label: self.business_label,
            connector_label: self.connector_label,
            business_sub_label: self.business_sub_label,
            created_at: now,
            modified_at: now,
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
                frm_configs,
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
                frm_configs,
                modified_at: Some(common_utils::date_time::now()),
            },
        }
    }
}
