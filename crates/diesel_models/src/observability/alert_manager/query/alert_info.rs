use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::ResultExt;

use crate::{
    observability::alert_manager::alert_info::{AlertsInfo, AlertsInfoNew},
    observability::{alert_manager::alert_info::AlertsInfoUpdate, schema::alerts_info::dsl},
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl AlertsInfoNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertsInfo> {
        generics::generic_insert(conn, self).await
    }
}

impl AlertsInfo {
    pub async fn find_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, id).await
    }

    pub async fn list_by_filter(
        conn: &DatabaseConnectionWithContext<'_>,
        name: Option<String>,
        product: Option<String>,
        is_enabled: Option<bool>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = crate::list::into_boxed_list(Self::table());

        if let Some(name) = name {
            query = query.filter(dsl::name.eq(name));
        }
        if let Some(product) = product {
            query = query.filter(dsl::product.eq(product));
        }
        if let Some(is_enabled) = is_enabled {
            query = query.filter(dsl::is_enabled.eq(is_enabled));
        }

        query
            .get_results_async(conn.raw_connection())
            .await
            .map_err(|error| {
                error_stack::report!(crate::errors::DatabaseError::Others)
                    .attach_printable(error.to_string())
            })
            .attach_printable("Error filtering alerts_info")
    }

    pub async fn update_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: String,
        update: AlertsInfoUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(conn, id, update).await
    }

    pub async fn delete_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(conn, dsl::id.eq(id)).await
    }
}
