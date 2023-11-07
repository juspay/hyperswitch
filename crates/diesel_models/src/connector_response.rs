use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::connector_response;

#[derive(Clone, Debug, Deserialize, Serialize, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = connector_response)]
#[serde(deny_unknown_fields)]
pub struct ConnectorResponseNew {
    pub payment_id: String,
    pub merchant_id: String,
    pub attempt_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub connector_name: Option<String>,
    pub connector_transaction_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub updated_by: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable)]
#[diesel(table_name = connector_response)]
pub struct ConnectorResponse {
    pub id: i32,
    pub payment_id: String,
    pub merchant_id: String,
    pub attempt_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub connector_name: Option<String>,
    pub connector_transaction_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub updated_by: String,
}

#[derive(Clone, Default, Debug, Deserialize, AsChangeset, Serialize)]
#[diesel(table_name = connector_response)]
pub struct ConnectorResponseUpdateInternal {
    pub connector_transaction_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub encoded_data: Option<String>,
    pub connector_name: Option<String>,
    pub updated_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConnectorResponseUpdate {
    ResponseUpdate {
        connector_transaction_id: Option<String>,
        authentication_data: Option<serde_json::Value>,
        encoded_data: Option<String>,
        connector_name: Option<String>,
        updated_by: String,
    },
    ErrorUpdate {
        connector_name: Option<String>,
        updated_by: String,
    },
}

impl ConnectorResponseUpdate {
    pub fn apply_changeset(self, source: ConnectorResponse) -> ConnectorResponse {
        let connector_response_update: ConnectorResponseUpdateInternal = self.into();
        ConnectorResponse {
            modified_at: connector_response_update
                .modified_at
                .unwrap_or_else(common_utils::date_time::now),
            connector_name: connector_response_update
                .connector_name
                .or(source.connector_name),
            connector_transaction_id: source
                .connector_transaction_id
                .or(connector_response_update.connector_transaction_id),
            authentication_data: connector_response_update
                .authentication_data
                .or(source.authentication_data),
            encoded_data: connector_response_update
                .encoded_data
                .or(source.encoded_data),
            updated_by: connector_response_update.updated_by,
            ..source
        }
    }
}

impl From<ConnectorResponseUpdate> for ConnectorResponseUpdateInternal {
    fn from(connector_response_update: ConnectorResponseUpdate) -> Self {
        match connector_response_update {
            ConnectorResponseUpdate::ResponseUpdate {
                connector_transaction_id,
                authentication_data,
                encoded_data,
                connector_name,
                updated_by,
            } => Self {
                connector_transaction_id,
                authentication_data,
                encoded_data,
                modified_at: Some(common_utils::date_time::now()),
                connector_name,
                updated_by,
            },
            ConnectorResponseUpdate::ErrorUpdate {
                connector_name,
                updated_by,
            } => Self {
                connector_name,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                ..Self::default()
            },
        }
    }
}
