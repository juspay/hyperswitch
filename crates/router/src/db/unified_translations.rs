use diesel_models::unified_translations as storage;
use error_stack::report;

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

#[async_trait::async_trait]
pub trait UnifiedTranslationsInterface {
    async fn add_unfied_translation(
        &self,
        translation: storage::UnifiedTranslationsNew,
    ) -> CustomResult<storage::UnifiedTranslations, errors::StorageError>;

    async fn update_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
        data: storage::UnifiedTranslationsUpdate,
    ) -> CustomResult<storage::UnifiedTranslations, errors::StorageError>;

    async fn find_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> CustomResult<String, errors::StorageError>;

    async fn delete_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl UnifiedTranslationsInterface for Store {
    async fn add_unfied_translation(
        &self,
        translation: storage::UnifiedTranslationsNew,
    ) -> CustomResult<storage::UnifiedTranslations, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        translation
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn update_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
        data: storage::UnifiedTranslationsUpdate,
    ) -> CustomResult<storage::UnifiedTranslations, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UnifiedTranslations::update_by_unified_code_unified_message_locale(
            &conn,
            unified_code,
            unified_message,
            locale,
            data,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> CustomResult<String, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let translations =
            storage::UnifiedTranslations::find_by_unified_code_unified_message_locale(
                &conn,
                unified_code,
                unified_message,
                locale,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;
        Ok(translations.translation)
    }

    async fn delete_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::UnifiedTranslations::delete_by_unified_code_unified_message_locale(
            &conn,
            unified_code,
            unified_message,
            locale,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl UnifiedTranslationsInterface for MockDb {
    async fn add_unfied_translation(
        &self,
        _translation: storage::UnifiedTranslationsNew,
    ) -> CustomResult<storage::UnifiedTranslations, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_translation(
        &self,
        _unified_code: String,
        _unified_message: String,
        _locale: String,
    ) -> CustomResult<String, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_translation(
        &self,
        _unified_code: String,
        _unified_message: String,
        _locale: String,
        _data: storage::UnifiedTranslationsUpdate,
    ) -> CustomResult<storage::UnifiedTranslations, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_translation(
        &self,
        _unified_code: String,
        _unified_message: String,
        _locale: String,
    ) -> CustomResult<bool, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
