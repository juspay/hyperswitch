//! Translations

use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::schema::unified_translations;

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = unified_translations, primary_key(unified_code, unified_message, locale), check_for_backend(diesel::pg::Pg))]
pub struct UnifiedTranslations {
    pub unified_code: String,
    pub unified_message: String,
    pub locale: String,
    pub translation: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = unified_translations)]
pub struct UnifiedTranslationsNew {
    pub unified_code: String,
    pub unified_message: String,
    pub locale: String,
    pub translation: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = unified_translations)]
pub struct UnifiedTranslationsUpdateInternal {
    pub translation: Option<String>,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub struct UnifiedTranslationsUpdate {
    pub translation: Option<String>,
}

impl From<UnifiedTranslationsUpdate> for UnifiedTranslationsUpdateInternal {
    fn from(value: UnifiedTranslationsUpdate) -> Self {
        let now = common_utils::date_time::now();
        let UnifiedTranslationsUpdate { translation } = value;
        Self {
            translation,
            last_modified_at: now,
        }
    }
}
