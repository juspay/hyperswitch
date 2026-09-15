use crate::{
    observability::alerts_info::{AlertsInfo, AlertsInfoNew},
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
