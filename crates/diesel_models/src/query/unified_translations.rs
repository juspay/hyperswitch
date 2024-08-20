use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;

use crate::{
    errors,
    query::generics,
    schema::unified_translations::dsl,
    unified_translations::{UnifiedTranslationsUpdateInternal, *},
    PgPooledConn, StorageResult,
};

impl UnifiedTranslationsNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<UnifiedTranslations> {
        generics::generic_insert(conn, self).await
    }
}

impl UnifiedTranslations {
    pub async fn find_by_unified_code_unified_message_locale(
        conn: &PgPooledConn,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::unified_code
                .eq(unified_code)
                .and(dsl::unified_message.eq(unified_message))
                .and(dsl::locale.eq(locale)),
        )
        .await
    }

    pub async fn update_by_unified_code_unified_message_locale(
        conn: &PgPooledConn,
        unified_code: String,
        unified_message: String,
        locale: String,
        data: UnifiedTranslationsUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<
            <Self as HasTable>::Table,
            UnifiedTranslationsUpdateInternal,
            _,
            _,
        >(
            conn,
            dsl::unified_code
                .eq(unified_code)
                .and(dsl::unified_message.eq(unified_message))
                .and(dsl::locale.eq(locale)),
            data.into(),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound)
                .attach_printable("Error while updating unified_translations entry")
        })
    }

    pub async fn delete_by_unified_code_unified_message_locale(
        conn: &PgPooledConn,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::unified_code
                .eq(unified_code)
                .and(dsl::unified_message.eq(unified_message))
                .and(dsl::locale.eq(locale)),
        )
        .await
    }
}
