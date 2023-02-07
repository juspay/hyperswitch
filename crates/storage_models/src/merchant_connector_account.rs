use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;

use crate::{enums as storage_enums, schema::merchant_connector_account};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccount {
    pub id: i32,
    pub merchant_id: String,
    pub connector_name: String,
    pub connector_account_details: serde_json::Value,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: String,
    #[diesel(deserialize_as = super::OptionalDieselArray<serde_json::Value>)]
    pub payment_methods_enabled: Option<Vec<serde_json::Value>>,
    pub connector_type: storage_enums::ConnectorType,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountNew {
    pub merchant_id: Option<String>,
    pub connector_type: Option<storage_enums::ConnectorType>,
    pub connector_name: Option<String>,
    pub connector_account_details: Option<Secret<serde_json::Value>>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: String,
    pub payment_methods_enabled: Option<Vec<serde_json::Value>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum MerchantConnectorAccountUpdate {
    Update {
        merchant_id: Option<String>,
        connector_type: Option<storage_enums::ConnectorType>,
        connector_name: Option<String>,
        connector_account_details: Option<Secret<serde_json::Value>>,
        test_mode: Option<bool>,
        disabled: Option<bool>,
        merchant_connector_id: Option<String>,
        payment_methods_enabled: Option<Vec<serde_json::Value>>,
        metadata: Option<serde_json::Value>,
    },
}
#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountUpdateInternal {
    merchant_id: Option<String>,
    connector_type: Option<storage_enums::ConnectorType>,
    connector_name: Option<String>,
    connector_account_details: Option<Secret<serde_json::Value>>,
    test_mode: Option<bool>,
    disabled: Option<bool>,
    merchant_connector_id: Option<String>,
    payment_methods_enabled: Option<Vec<serde_json::Value>>,
    metadata: Option<serde_json::Value>,
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
                connector_account_details,
                test_mode,
                disabled,
                merchant_connector_id,
                payment_methods_enabled,
                metadata,
            },
        }
    }
}
