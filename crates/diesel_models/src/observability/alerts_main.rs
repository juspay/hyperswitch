use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::raw_json::RawJson;

/// One announcement, independent of the channel whose table it came from.
#[derive(Clone, Debug)]
pub struct AnnouncementRow {
    pub id: uuid::Uuid,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimensions: Option<RawJson>,
    pub ts_slack: Option<String>,
    pub ts_alert: Option<PrimitiveDateTime>,
    pub duration: Option<i32>,
    pub sent: Option<bool>,
    pub critical: Option<bool>,
    pub rca_metadata: Option<serde_json::Value>,
    pub metadata: Option<RawJson>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

impl AnnouncementRow {
    pub fn was_delivered(&self) -> bool {
        self.sent.unwrap_or(false)
    }
}

// `alerts_main` and `alerts_main_xyne` are the same table per delivery channel.
macro_rules! announcement {
    ($module:ident, $table:ident) => {
        pub mod $module {
            use super::*;
            use crate::observability::schema::$table;

            // Serialize/Deserialize satisfy `DejaQueryResult`, which the query helpers require under `deja`.
            #[derive(
                Clone,
                Debug,
                Identifiable,
                Insertable,
                Queryable,
                Selectable,
                Deserialize,
                Serialize,
            )]
            #[diesel(table_name = $table, primary_key(id), check_for_backend(diesel::pg::Pg))]
            pub struct Announcement {
                pub id: uuid::Uuid,
                pub name: Option<String>,
                pub product: Option<String>,
                pub dimensions: Option<RawJson>,
                pub ts_slack: Option<String>,
                pub ts_alert: Option<PrimitiveDateTime>,
                pub duration: Option<i32>,
                pub sent: Option<bool>,
                pub critical: Option<bool>,
                pub rca_metadata: Option<serde_json::Value>,
                pub metadata: Option<RawJson>,
                pub last_updated_at: Option<PrimitiveDateTime>,
            }

            impl From<Announcement> for AnnouncementRow {
                fn from(announcement: Announcement) -> Self {
                    Self {
                        id: announcement.id,
                        name: announcement.name,
                        product: announcement.product,
                        dimensions: announcement.dimensions,
                        ts_slack: announcement.ts_slack,
                        ts_alert: announcement.ts_alert,
                        duration: announcement.duration,
                        sent: announcement.sent,
                        critical: announcement.critical,
                        rca_metadata: announcement.rca_metadata,
                        metadata: announcement.metadata,
                        last_updated_at: announcement.last_updated_at,
                    }
                }
            }

            impl From<AnnouncementRow> for Announcement {
                fn from(row: AnnouncementRow) -> Self {
                    Self {
                        id: row.id,
                        name: row.name,
                        product: row.product,
                        dimensions: row.dimensions,
                        ts_slack: row.ts_slack,
                        ts_alert: row.ts_alert,
                        duration: row.duration,
                        sent: row.sent,
                        critical: row.critical,
                        rca_metadata: row.rca_metadata,
                        metadata: row.metadata,
                        last_updated_at: row.last_updated_at,
                    }
                }
            }
        }
    };
}

announcement!(slack, alerts_main);
announcement!(xyne, alerts_main_xyne);
