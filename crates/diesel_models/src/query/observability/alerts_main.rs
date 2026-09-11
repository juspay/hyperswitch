use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::{report, ResultExt};

use crate::{errors, query::generics, DatabaseConnectionWithContext, StorageResult};

macro_rules! announcement_queries {
    ($module:ident, $table:ident) => {
        pub mod $module {
            use super::*;
            use crate::observability::{
                alerts_main::{$module::Announcement, AnnouncementRow},
                schema::$table::dsl,
            };

            impl Announcement {
                pub async fn insert(
                    conn: &DatabaseConnectionWithContext<'_>,
                    row: AnnouncementRow,
                ) -> StorageResult<AnnouncementRow> {
                    generics::generic_insert::<<Self as HasTable>::Table, Self, Self>(
                        conn,
                        Self::from(row),
                    )
                    .await
                    .map(AnnouncementRow::from)
                }

                pub async fn existing_ids(
                    conn: &DatabaseConnectionWithContext<'_>,
                    ids: Vec<uuid::Uuid>,
                ) -> StorageResult<Vec<uuid::Uuid>> {
                    if ids.is_empty() {
                        return Ok(Vec::new());
                    }

                    let query = <Self as HasTable>::table()
                        .select(dsl::id)
                        .filter(dsl::id.eq_any(ids));

                    generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
                        conn.request_id(),
                        conn.event_emitter(),
                        generics::db_metrics::DatabaseOperation::Filter,
                        query.load_async::<uuid::Uuid>(conn.raw_connection()),
                    )
                    .await
                    .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
                    .attach_printable("Error while checking which announcements exist")
                }
            }
        }
    };
}

announcement_queries!(slack, alerts_main);
announcement_queries!(xyne, alerts_main_xyne);
