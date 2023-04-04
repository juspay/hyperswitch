use common_utils::{
    errors::{CustomResult, ValidationError},
    pii,
};
use masking::{ExposeInterface, Secret};
use storage_models::enums;

use super::behaviour;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MerchantConnectorAccount {
    pub id: Option<i32>,
    pub merchant_id: String,
    pub connector_name: String,
    pub connector_account_details: Secret<serde_json::Value>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: String,
    pub payment_methods_enabled: Option<Vec<serde_json::Value>>,
    pub connector_type: enums::ConnectorType,
    pub metadata: Option<pii::SecretSerdeValue>,
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
                connector_account_details: self.connector_account_details.expose(),
                test_mode: self.test_mode,
                disabled: self.disabled,
                merchant_connector_id: self.merchant_connector_id,
                payment_methods_enabled: self.payment_methods_enabled,
                connector_type: self.connector_type,
                metadata: self.metadata,
            },
        )
    }

    async fn convert_back(other: Self::DstType) -> CustomResult<Self, ValidationError> {
        Ok(Self {
            id: Some(other.id),
            merchant_id: other.merchant_id,
            connector_name: other.connector_name,
            connector_account_details: other.connector_account_details.into(),
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
            connector_account_details: Some(self.connector_account_details),
            test_mode: self.test_mode,
            disabled: self.disabled,
            merchant_connector_id: self.merchant_connector_id,
            payment_methods_enabled: self.payment_methods_enabled,
            connector_type: Some(self.connector_type),
            metadata: self.metadata,
        })
    }
}
