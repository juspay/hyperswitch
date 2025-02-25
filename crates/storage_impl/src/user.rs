
mod local {
    pub use diesel_models::user::theme::*;
    pub use diesel_models::user::*;
}

use futures::{future::try_join_all, FutureExt};
#[cfg(feature = "v1")]
use diesel_models::user::sample_data::PaymentAttemptBatchNew;
use diesel_models::{
    dispute::{Dispute, DisputeNew},
    errors::DatabaseError,
    query::user::sample_data as sample_data_queries,
    refund::{Refund, RefundNew},
};
use hyperswitch_domain_models::{
    behaviour::Conversion,
    merchant_key_store::MerchantKeyStore,
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
};

use common_utils::{errors::CustomResult, types::{theme::ThemeLineage, keymanager::KeyManagerState}};
use local as storage;
// use diesel_models::{user as storage};
use error_stack::{report, Report, ResultExt};
use router_env::{instrument, tracing};
use sample::{domain::user as domain, user::{UserInterface, ThemeInterface, BatchSampleDataInterface}};

use crate::{connection, errors, DatabaseStore, RouterStore, DataModelExt};

#[async_trait::async_trait]
impl<T: DatabaseStore> UserInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_user(
        &self,
        user_data: storage::UserNew,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        user_data
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_user_by_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_by_user_email(&conn, user_email.get_inner())
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_user_by_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_by_user_id(&conn, user_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_user_by_user_id(
        &self,
        user_id: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::update_by_user_id(&conn, user_id, user)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_user_by_email(
        &self,
        user_email: &domain::UserEmail,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::update_by_user_email(&conn, user_email.get_inner(), user)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::User::delete_by_user_id(&conn, user_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_users_by_user_ids(
        &self,
        user_ids: Vec<String>,
    ) -> CustomResult<Vec<storage::User>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::User::find_users_by_user_ids(&conn, user_ids)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}


#[async_trait::async_trait]
impl<T: DatabaseStore> ThemeInterface for RouterStore<T> {
    type Error = errors::StorageError;

    async fn insert_theme(
        &self,
        theme: storage::ThemeNew,
    ) -> CustomResult<storage::Theme, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        theme
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_theme_by_theme_id(
        &self,
        theme_id: String,
    ) -> CustomResult<storage::Theme, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Theme::find_by_theme_id(&conn, theme_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_most_specific_theme_in_lineage(
        &self,
        lineage: ThemeLineage,
    ) -> CustomResult<storage::Theme, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Theme::find_most_specific_theme_in_lineage(&conn, lineage)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_theme_by_lineage(
        &self,
        lineage: ThemeLineage,
    ) -> CustomResult<storage::Theme, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Theme::find_by_lineage(&conn, lineage)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn delete_theme_by_lineage_and_theme_id(
        &self,
        theme_id: String,
        lineage: ThemeLineage,
    ) -> CustomResult<storage::Theme, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Theme::delete_by_theme_id_and_lineage(&conn, theme_id, lineage)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}


#[async_trait::async_trait]
impl<T: DatabaseStore> BatchSampleDataInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[cfg(feature = "v1")]
    async fn insert_payment_intents_batch_for_sample_data(
        &self,
        state: &KeyManagerState,
        batch: Vec<PaymentIntent>,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        let new_intents = try_join_all(batch.into_iter().map(|payment_intent| async {
            payment_intent
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)
        }))
        .await?;
        sample_data_queries::insert_payment_intents(&conn, new_intents)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|v| {
                try_join_all(v.into_iter().map(|payment_intent| {
                    PaymentIntent::convert_back(
                        state,
                        payment_intent,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                }))
                .map(|join_result| join_result.change_context(errors::StorageError::DecryptionError))
            })?
            .await
    }

    #[cfg(feature = "v1")]
    async fn insert_payment_attempts_batch_for_sample_data(
        &self,
        batch: Vec<PaymentAttemptBatchNew>,
    ) -> CustomResult<Vec<PaymentAttempt>, errors::StorageError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        sample_data_queries::insert_payment_attempts(&conn, batch)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|res| {
                res.into_iter()
                    .map(PaymentAttempt::from_storage_model)
                    .collect()
            })
    }

    #[cfg(feature = "v1")]
    async fn insert_refunds_batch_for_sample_data(
        &self,
        batch: Vec<RefundNew>,
    ) -> CustomResult<Vec<Refund>, errors::StorageError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        sample_data_queries::insert_refunds(&conn, batch)
            .await
            .map_err(diesel_error_to_data_error)
    }

    #[cfg(feature = "v1")]
    async fn insert_disputes_batch_for_sample_data(
        &self,
        batch: Vec<DisputeNew>,
    ) -> CustomResult<Vec<Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        sample_data_queries::insert_disputes(&conn, batch)
            .await
            .map_err(diesel_error_to_data_error)
    }

    #[cfg(feature = "v1")]
    async fn delete_payment_intents_for_sample_data(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        sample_data_queries::delete_payment_intents(&conn, merchant_id)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|v| {
                try_join_all(v.into_iter().map(|payment_intent| {
                    PaymentIntent::convert_back(
                        state,
                        payment_intent,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                }))
                .map(|join_result| join_result.change_context(errors::StorageError::DecryptionError))
            })?
            .await
    }

    #[cfg(feature = "v1")]
    async fn delete_payment_attempts_for_sample_data(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<PaymentAttempt>, errors::StorageError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        sample_data_queries::delete_payment_attempts(&conn, merchant_id)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|res| {
                res.into_iter()
                    .map(PaymentAttempt::from_storage_model)
                    .collect()
            })
    }

    #[cfg(feature = "v1")]
    async fn delete_refunds_for_sample_data(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<Refund>, errors::StorageError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        sample_data_queries::delete_refunds(&conn, merchant_id)
            .await
            .map_err(diesel_error_to_data_error)
    }

    #[cfg(feature = "v1")]
    async fn delete_disputes_for_sample_data(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        sample_data_queries::delete_disputes(&conn, merchant_id)
            .await
            .map_err(diesel_error_to_data_error)
    }
}

// TODO: This error conversion is re-used from storage_impl and is not DRY when it should be
// Ideally the impl's here should be defined in that crate avoiding this re-definition
fn diesel_error_to_data_error(diesel_error: Report<DatabaseError>) -> Report<errors::StorageError> {
    let new_err = match diesel_error.current_context() {
        DatabaseError::DatabaseConnectionError => errors::StorageError::DatabaseConnectionError,
        DatabaseError::NotFound => errors::StorageError::ValueNotFound("Value not found".to_string()),
        DatabaseError::UniqueViolation => errors::StorageError::DuplicateValue {
            entity: "entity ",
            key: None,
        },
        err => errors::StorageError::DatabaseError(error_stack::report!(*err)),
    };
    diesel_error.change_context(new_err)
}