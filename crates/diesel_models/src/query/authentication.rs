use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    authentication::{Authentication, AuthenticationNew, AuthenticationUpdateInternal},
    errors, kv,
    schema::authentication::dsl,
    PgPooledConn, StorageResult,
};

impl AuthenticationNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Authentication> {
        Box::pin(generics::generic_insert(conn, self)).await
    }

    pub async fn generate_drainer_insert_query(
        self,
        conn: &mut PgPooledConn,
    ) -> StorageResult<kv::SerializableQuery> {
        kv::generate_insert_query(conn, self).await
    }
}

impl Authentication {
    pub async fn update_by_processor_merchant_id_authentication_id(
        conn: &PgPooledConn,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        authentication_update: AuthenticationUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::authentication_id.eq(authentication_id.to_owned())),
            authentication_update,
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => Err(error.attach_printable(
                    "Authentication with the given Authentication ID does not exist",
                )),
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
                        conn,
                        dsl::processor_merchant_id
                            .eq(processor_merchant_id.to_owned())
                            .and(dsl::authentication_id.eq(authentication_id.to_owned())),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn update_by_merchant_id_authentication_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        authentication_update: AuthenticationUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::authentication_id.eq(authentication_id.to_owned())),
            authentication_update,
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => Err(error.attach_printable(
                    "Authentication with the given Authentication ID does not exist",
                )),
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
                        conn,
                        dsl::merchant_id
                            .eq(merchant_id.to_owned())
                            .and(dsl::authentication_id.eq(authentication_id.to_owned())),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn find_by_processor_merchant_id_authentication_id(
        conn: &PgPooledConn,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::authentication_id.eq(authentication_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id_authentication_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::authentication_id.eq(authentication_id.to_owned())),
        )
        .await
    }

    pub async fn find_authentication_by_processor_merchant_id_connector_authentication_id(
        conn: &PgPooledConn,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        connector_authentication_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::connector_authentication_id.eq(connector_authentication_id.to_owned())),
        )
        .await
    }

    pub async fn find_authentication_by_merchant_id_connector_authentication_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_authentication_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_authentication_id.eq(connector_authentication_id.to_owned())),
        )
        .await
    }
}

impl AuthenticationUpdateInternal {
    pub async fn generate_drainer_update_query(
        self,
        conn: &mut PgPooledConn,
        processor_merchant_id: common_utils::id_type::MerchantId,
        authentication_id: common_utils::id_type::AuthenticationId,
    ) -> StorageResult<kv::SerializableQuery> {
        // Stagger release: match on processor_merchant_id, falling back to merchant_id for legacy
        // rows where processor_merchant_id is NULL (the value passed is unwrap_or(merchant_id)).
        kv::generate_update_query_with_predicate::<<Authentication as HasTable>::Table, _, _>(
            conn,
            dsl::authentication_id.eq(authentication_id).and(
                dsl::processor_merchant_id
                    .eq(processor_merchant_id.clone())
                    .or(dsl::merchant_id.eq(processor_merchant_id)),
            ),
            self,
        )
        .await
    }
}
