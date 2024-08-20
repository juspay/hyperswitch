//! Translations

use common_utils::custom_serde;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::unified_translations;

#[derive(
    Clone,
    Debug,
    Insertable,
    Serialize,
    Deserialize,
    router_derive::DebugAsDisplay,
    Queryable,
    Selectable,
    Identifiable,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
)]
#[diesel(table_name = unified_translations, primary_key(unified_code, unified_message, locale), check_for_backend(diesel::pg::Pg))]
pub struct UnifiedTranslations {
    pub unified_code: String,
    pub unified_message: String,
    pub locale: String,
    pub translation: String,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "custom_serde::iso8601")]
    pub last_modified: PrimitiveDateTime,
}
#[derive(
    Clone, Debug, Insertable, Serialize, Deserialize, router_derive::DebugAsDisplay, Eq, PartialEq,
)]
#[diesel(table_name = unified_translations)]
pub struct UnifiedTranslationsNew {
    pub unified_code: String,
    pub unified_message: String,
    pub locale: String,
    pub translation: String,
}

#[derive(
    Clone,
    Debug,
    AsChangeset,
    router_derive::DebugAsDisplay,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Default,
)]
#[diesel(table_name = unified_translations)]
pub struct UnifiedTranslationsUpdateInternal {
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub locale: Option<String>,
    pub translation: Option<String>,
    pub last_modified: Option<PrimitiveDateTime>,
}

#[derive(Debug)]
pub struct UnifiedTranslationsUpdate {
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub locale: Option<String>,
    pub translation: Option<String>,
}

impl From<UnifiedTranslationsUpdate> for UnifiedTranslationsUpdateInternal {
    fn from(value: UnifiedTranslationsUpdate) -> Self {
        let now = Some(common_utils::date_time::now());
        let UnifiedTranslationsUpdate {
            unified_code,
            unified_message,
            locale,
            translation,
        } = value;
        Self {
            unified_code,
            unified_message,
            locale,
            translation,
            last_modified: now,
        }
    }
}
