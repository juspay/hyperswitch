
// use error_stack::report;

// use super::MockDb;
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     services::Store,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use diesel_models::unified_translations as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait UnifiedTranslationsInterface {
    type Error;
    async fn add_unfied_translation(
        &self,
        translation: storage::UnifiedTranslationsNew,
    ) -> CustomResult<storage::UnifiedTranslations, Self::Error>;

    async fn update_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
        data: storage::UnifiedTranslationsUpdate,
    ) -> CustomResult<storage::UnifiedTranslations, Self::Error>;

    async fn find_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> CustomResult<String, Self::Error>;

    async fn delete_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> CustomResult<bool, Self::Error>;
}