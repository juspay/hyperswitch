use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, PartialEq)]
pub struct MerchantInstanceRow {
    pub id: Option<uuid::Uuid>,
    pub id_merchant_table: uuid::Uuid,
    pub id_intermediate: Option<uuid::Uuid>,
    pub name: Option<String>,
    pub product: Option<String>,
    pub merchant_id: Option<String>,
    pub dimensions: Option<serde_json::Value>,
    pub auxiliary_dimensions: Option<serde_json::Value>,
    pub current_metric: Option<f64>,
    pub expected_metric: Option<f64>,
    pub attribution: Option<String>,
    pub max_duration: Option<i32>,
    pub start_time: Option<PrimitiveDateTime>,
    pub is_visible: Option<bool>,
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: Option<String>,
    pub ts_alert: Option<PrimitiveDateTime>,
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub last_updated_at: Option<PrimitiveDateTime>,
    pub slack_info: Option<serde_json::Value>,
    pub communication_info: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub priority: Option<String>,
    pub tenant_id: Option<String>,
}

impl MerchantInstanceRow {
    pub fn has_recovered(&self) -> bool {
        self.recovered_ts.is_some()
    }
}

macro_rules! merchant_instance {
    ($module:ident, $table:ident) => {
        pub mod $module {
            use super::*;
            use crate::observability::schema::$table;

            #[derive(
                Clone,
                Debug,
                PartialEq,
                Identifiable,
                Insertable,
                Queryable,
                Selectable,
                Deserialize,
                Serialize,
            )]
            #[diesel(table_name = $table, primary_key(id_merchant_table))]
            #[diesel(check_for_backend(diesel::pg::Pg))]
            pub struct MerchantInstance {
                pub id: Option<uuid::Uuid>,
                pub id_merchant_table: uuid::Uuid,
                pub id_intermediate: Option<uuid::Uuid>,
                pub name: Option<String>,
                pub product: Option<String>,
                pub merchant_id: Option<String>,
                pub dimensions: Option<serde_json::Value>,
                pub auxiliary_dimensions: Option<serde_json::Value>,
                pub current_metric: Option<f64>,
                pub expected_metric: Option<f64>,
                pub attribution: Option<String>,
                pub max_duration: Option<i32>,
                pub start_time: Option<PrimitiveDateTime>,
                pub is_visible: Option<bool>,
                pub recovered_ts: Option<PrimitiveDateTime>,
                pub ts_slack: Option<String>,
                pub ts_alert: Option<PrimitiveDateTime>,
                pub latest_ts_alert: Option<PrimitiveDateTime>,
                pub last_updated_at: Option<PrimitiveDateTime>,
                pub slack_info: Option<serde_json::Value>,
                pub communication_info: Option<serde_json::Value>,
                pub metadata: Option<serde_json::Value>,
                pub metadata_alert_details: Option<serde_json::Value>,
                pub priority: Option<String>,
                pub tenant_id: Option<String>,
            }

            impl From<MerchantInstance> for MerchantInstanceRow {
                fn from(instance: MerchantInstance) -> Self {
                    Self {
                        id: instance.id,
                        id_merchant_table: instance.id_merchant_table,
                        id_intermediate: instance.id_intermediate,
                        name: instance.name,
                        product: instance.product,
                        merchant_id: instance.merchant_id,
                        dimensions: instance.dimensions,
                        auxiliary_dimensions: instance.auxiliary_dimensions,
                        current_metric: instance.current_metric,
                        expected_metric: instance.expected_metric,
                        attribution: instance.attribution,
                        max_duration: instance.max_duration,
                        start_time: instance.start_time,
                        is_visible: instance.is_visible,
                        recovered_ts: instance.recovered_ts,
                        ts_slack: instance.ts_slack,
                        ts_alert: instance.ts_alert,
                        latest_ts_alert: instance.latest_ts_alert,
                        last_updated_at: instance.last_updated_at,
                        slack_info: instance.slack_info,
                        communication_info: instance.communication_info,
                        metadata: instance.metadata,
                        metadata_alert_details: instance.metadata_alert_details,
                        priority: instance.priority,
                        tenant_id: instance.tenant_id,
                    }
                }
            }

            impl From<MerchantInstanceRow> for MerchantInstance {
                fn from(row: MerchantInstanceRow) -> Self {
                    Self {
                        id: row.id,
                        id_merchant_table: row.id_merchant_table,
                        id_intermediate: row.id_intermediate,
                        name: row.name,
                        product: row.product,
                        merchant_id: row.merchant_id,
                        dimensions: row.dimensions,
                        auxiliary_dimensions: row.auxiliary_dimensions,
                        current_metric: row.current_metric,
                        expected_metric: row.expected_metric,
                        attribution: row.attribution,
                        max_duration: row.max_duration,
                        start_time: row.start_time,
                        is_visible: row.is_visible,
                        recovered_ts: row.recovered_ts,
                        ts_slack: row.ts_slack,
                        ts_alert: row.ts_alert,
                        latest_ts_alert: row.latest_ts_alert,
                        last_updated_at: row.last_updated_at,
                        slack_info: row.slack_info,
                        communication_info: row.communication_info,
                        metadata: row.metadata,
                        metadata_alert_details: row.metadata_alert_details,
                        priority: row.priority,
                        tenant_id: row.tenant_id,
                    }
                }
            }
        }
    };
}

merchant_instance!(slack, merchants_alert_external);
merchant_instance!(xyne, merchants_alert_external_xyne);
