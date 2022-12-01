#[cfg(feature = "diesel")]
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[cfg(feature = "diesel")]
use crate::schema::connector_response;

#[derive(Clone, Debug, Deserialize, Serialize, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = connector_response))]
#[serde(deny_unknown_fields)]
pub struct ConnectorResponseNew {
    pub payment_id: String,
    pub merchant_id: String,
    pub txn_id: String,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub connector_name: String,
    pub connector_transaction_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "diesel", derive(Identifiable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = connector_response))]

pub struct ConnectorResponse {
    #[serde(skip_serializing)]
    pub id: i32,
    pub payment_id: String,
    pub merchant_id: String,
    pub txn_id: String,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub connector_name: String,
    pub connector_transaction_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = connector_response))]
pub struct ConnectorResponseUpdateInternal {
    pub connector_transaction_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub modified_at: PrimitiveDateTime,
    pub encoded_data: Option<String>,
}

#[derive(Debug)]
pub enum ConnectorResponseUpdate {
    ResponseUpdate {
        connector_transaction_id: Option<String>,
        authentication_data: Option<serde_json::Value>,
        encoded_data: Option<String>,
    },
}

impl ConnectorResponseUpdate {
    pub fn apply_changeset(self, source: ConnectorResponse) -> ConnectorResponse {
        let connector_response_update: ConnectorResponseUpdateInternal = self.into();
        ConnectorResponse {
            modified_at: connector_response_update.modified_at,
            connector_transaction_id: connector_response_update.connector_transaction_id,
            authentication_data: connector_response_update.authentication_data,
            encoded_data: connector_response_update.encoded_data,
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
            } => Self {
                connector_transaction_id,
                authentication_data,
                encoded_data,
                modified_at: common_utils::date_time::now(),
            },
        }
    }
}
