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
    pub async fn find(
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
    pub async fn retrieve_translation(
        conn: &PgPooledConn,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> StorageResult<String> {
        Self::find(conn, unified_code, unified_message, locale)
            .await
            .map(|item| item.translation)
    }

    pub async fn update(
        conn: &PgPooledConn,
        unified_code: String,
        unified_message: String,
        locale: String,
        translation: String,
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
                .and(dsl::locale.eq(locale))
                .and(dsl::translation.eq(translation)),
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

    pub async fn delete(
        conn: &PgPooledConn,
        unified_code: String,
        unified_message: String,
        locale: String,
        translation: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::unified_code
                .eq(unified_code)
                .and(dsl::unified_message.eq(unified_message))
                .and(dsl::locale.eq(locale))
                .and(dsl::translation.eq(translation)),
        )
        .await
    }
}
