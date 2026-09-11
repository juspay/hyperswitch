use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::{report, ResultExt};

use crate::{errors, query::generics, DatabaseConnectionWithContext, StorageResult};

// One set of helpers per channel, over the table the model is generated for.
macro_rules! merchant_instance_queries {
    ($module:ident, $table:ident) => {
        pub mod $module {
            use super::*;
            use crate::observability::{
                merchants_alert_external::{$module::MerchantInstance, MerchantInstanceRow},
                schema::$table::dsl,
            };

            impl MerchantInstance {
                pub async fn list_for_announcement(
                    conn: &DatabaseConnectionWithContext<'_>,
                    announcement: uuid::Uuid,
                ) -> StorageResult<Vec<MerchantInstanceRow>> {
                    generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
                        conn,
                        dsl::id.eq(announcement),
                        None,
                        None,
                        Some((dsl::merchant_id.asc(), dsl::id_merchant_table.asc())),
                    )
                    .await
                    .map(|rows| rows.into_iter().map(MerchantInstanceRow::from).collect())
                }

                pub async fn delete_for_announcement(
                    conn: &DatabaseConnectionWithContext<'_>,
                    announcement: uuid::Uuid,
                ) -> StorageResult<usize> {
                    let query = diesel::delete(
                        <Self as HasTable>::table().filter(dsl::id.eq(announcement)),
                    );

                    generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
                        conn.request_id(),
                        conn.event_emitter(),
                        generics::db_metrics::DatabaseOperation::Delete,
                        query.execute_async(conn.raw_connection()),
                    )
                    .await
                    .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
                    .attach_printable("Error while removing merchant alert instances")
                }

                // One statement for the whole batch:
                pub async fn insert_all(
                    conn: &DatabaseConnectionWithContext<'_>,
                    rows: Vec<MerchantInstanceRow>,
                ) -> StorageResult<usize> {
                    if rows.is_empty() {
                        return Ok(0);
                    }

                    let query = diesel::insert_into(<Self as HasTable>::table())
                        .values(rows.into_iter().map(Self::from).collect::<Vec<_>>());

                    generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
                        conn.request_id(),
                        conn.event_emitter(),
                        generics::db_metrics::DatabaseOperation::Insert,
                        query.execute_async(conn.raw_connection()),
                    )
                    .await
                    .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
                    .attach_printable("Error while saving merchant alert instances")
                }
            }
        }
    };
}

merchant_instance_queries!(slack, merchants_alert_external);
merchant_instance_queries!(xyne, merchants_alert_external_xyne);
