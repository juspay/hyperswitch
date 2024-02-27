use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;
use router_env::{instrument, tracing};

use crate::{
    errors, gsm::*, query::generics, schema::gateway_status_map::dsl, PgPooledConn, StorageResult,
};

impl GatewayStatusMappingNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<GatewayStatusMap> {
        generics::generic_insert(conn, self).await
    }
}

impl GatewayStatusMap {
    pub async fn find(
        conn: &PgPooledConn,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::connector
                .eq(connector)
                .and(dsl::flow.eq(flow))
                .and(dsl::sub_flow.eq(sub_flow))
                .and(dsl::code.eq(code))
                .and(dsl::message.eq(message)),
        )
        .await
    }

    pub async fn retrieve_decision(
        conn: &PgPooledConn,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> StorageResult<String> {
        Self::find(conn, connector, flow, sub_flow, code, message)
            .await
            .map(|item| item.decision)
    }

    pub async fn update(
        conn: &PgPooledConn,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
        gsm: GatewayStatusMappingUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<
            <Self as HasTable>::Table,
            GatewayStatusMapperUpdateInternal,
            _,
            _,
        >(
            conn,
            dsl::connector
                .eq(connector)
                .and(dsl::flow.eq(flow))
                .and(dsl::sub_flow.eq(sub_flow))
                .and(dsl::code.eq(code))
                .and(dsl::message.eq(message)),
            gsm.into(),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound)
                .attach_printable("Error while updating gsm entry")
        })
    }

    pub async fn delete(
        conn: &PgPooledConn,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::connector
                .eq(connector)
                .and(dsl::flow.eq(flow))
                .and(dsl::sub_flow.eq(sub_flow))
                .and(dsl::code.eq(code))
                .and(dsl::message.eq(message)),
        )
        .await
    }
}
