use diesel::{associations::HasTable, ExpressionMethods, Table};

use super::generics;
use crate::{
    profile_acquirer::{ProfileAcquirer, ProfileAcquirerNew},
    schema::profile_acquirer::dsl,
    PgPooledConn, StorageResult,
};

impl ProfileAcquirerNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ProfileAcquirer> {
        generics::generic_insert(conn, self).await
    }
}

impl ProfileAcquirer {
    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::profile_id.eq(profile_id.to_owned()),
            None,
            None,
            None,
        )
        .await
    }
}
