use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    dispute::{Dispute, DisputeNew, DisputeUpdate, DisputeUpdateInternal},
    errors,
    schema::dispute::dsl,
    PgPooledConn, StorageResult,
};

impl DisputeNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Dispute> {
        generics::generic_insert(conn, self).await
    }
}

impl Dispute {
    #[instrument(skip(conn))]
    pub async fn find_by_attempt_id_connector_dispute_id(
        conn: &PgPooledConn,
        attempt_id: &str,
        connector_dispute_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::attempt_id
                .eq(attempt_id.to_owned())
                .and(dsl::connector_dispute_id.eq(connector_dispute_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn update(self, conn: &PgPooledConn, dispute: DisputeUpdate) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id,
            DisputeUpdateInternal::from(dispute),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }
}
