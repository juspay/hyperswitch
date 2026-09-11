use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// One lifecycle row, independent of the channel whose table it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct AlertStateRow {
    pub id_intermediate: uuid::Uuid,
    pub id: Option<uuid::Uuid>,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimensions: Option<serde_json::Value>,
    pub ts_slack: Option<String>,
    pub ts_alert: Option<PrimitiveDateTime>,
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub max_duration: Option<i32>,
    pub other_metrics: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub rca_metadata: Option<serde_json::Value>,
    pub group_id: Option<String>,
    pub priority: Option<String>,
    pub last_updated_at: Option<PrimitiveDateTime>,
    pub recovered_ts: Option<PrimitiveDateTime>,
}

impl AlertStateRow {
    pub fn has_recovered(&self) -> bool {
        self.recovered_ts.is_some()
    }
}

// One table per delivery channel, paired with the `alerts_main` twin of the same channel through `id`, which references it and cascades on delete.
macro_rules! alert_state {
    ($module:ident, $table:ident) => {
        pub mod $module {
            use super::*;
            use crate::observability::schema::$table;

            // Serialize/Deserialize satisfy `DejaQueryResult`, which the query helpers require under `deja`.
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
            #[diesel(table_name = $table, primary_key(id_intermediate))]
            #[diesel(check_for_backend(diesel::pg::Pg))]
            pub struct AlertState {
                pub id_intermediate: uuid::Uuid,
                pub id: Option<uuid::Uuid>,
                pub name: Option<String>,
                pub product: Option<String>,
                pub dimensions: Option<serde_json::Value>,
                pub ts_slack: Option<String>,
                pub ts_alert: Option<PrimitiveDateTime>,
                pub latest_ts_alert: Option<PrimitiveDateTime>,
                pub max_duration: Option<i32>,
                pub other_metrics: Option<serde_json::Value>,
                pub metadata: Option<serde_json::Value>,
                pub metadata_alert_details: Option<serde_json::Value>,
                pub rca_metadata: Option<serde_json::Value>,
                pub group_id: Option<String>,
                pub priority: Option<String>,
                pub last_updated_at: Option<PrimitiveDateTime>,
                pub recovered_ts: Option<PrimitiveDateTime>,
            }

            impl From<AlertState> for AlertStateRow {
                fn from(state: AlertState) -> Self {
                    Self {
                        id_intermediate: state.id_intermediate,
                        id: state.id,
                        name: state.name,
                        product: state.product,
                        dimensions: state.dimensions,
                        ts_slack: state.ts_slack,
                        ts_alert: state.ts_alert,
                        latest_ts_alert: state.latest_ts_alert,
                        max_duration: state.max_duration,
                        other_metrics: state.other_metrics,
                        metadata: state.metadata,
                        metadata_alert_details: state.metadata_alert_details,
                        rca_metadata: state.rca_metadata,
                        group_id: state.group_id,
                        priority: state.priority,
                        last_updated_at: state.last_updated_at,
                        recovered_ts: state.recovered_ts,
                    }
                }
            }

            impl From<AlertStateRow> for AlertState {
                fn from(row: AlertStateRow) -> Self {
                    Self {
                        id_intermediate: row.id_intermediate,
                        id: row.id,
                        name: row.name,
                        product: row.product,
                        dimensions: row.dimensions,
                        ts_slack: row.ts_slack,
                        ts_alert: row.ts_alert,
                        latest_ts_alert: row.latest_ts_alert,
                        max_duration: row.max_duration,
                        other_metrics: row.other_metrics,
                        metadata: row.metadata,
                        metadata_alert_details: row.metadata_alert_details,
                        rca_metadata: row.rca_metadata,
                        group_id: row.group_id,
                        priority: row.priority,
                        last_updated_at: row.last_updated_at,
                        recovered_ts: row.recovered_ts,
                    }
                }
            }
        }
    };
}

alert_state!(slack, alerts_intermediate);
alert_state!(xyne, alerts_intermediate_xyne);
