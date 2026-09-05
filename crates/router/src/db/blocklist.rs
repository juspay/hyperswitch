use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
use storage_impl::MockDb;

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    types::storage,
};

#[async_trait::async_trait]
pub trait BlocklistInterface {
    async fn insert_blocklist_entry(
        &self,
        pm_blocklist_new: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn find_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn find_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn find_blocklist_entries_by_processor_merchant_id_profile_id_fingerprint_ids(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_ids: Vec<String>,
    ) -> CustomResult<Option<storage::Blocklist>, errors::StorageError>;

    /// Batched BIN-only lookup: every BIN-kind blocklist entry for this merchant/profile
    /// whose BIN is in `card_bins`, in a single query. PAN-fingerprint entries are excluded.
    async fn list_blocklist_entries_by_processor_merchant_id_profile_id_card_bins(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        card_bins: Vec<String>,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;

    async fn delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    async fn delete_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError>;

    /// `profile_id` is `None` for any caller that has no resolved profile - publishable-key and
    /// SDK auth, and an API key used without `X-Profile-Id`. Unlike the write paths, listing does
    /// not fall back to the default profile or error; absent means merchant-wide, as before
    /// profile scoping.
    async fn list_blocklist_entries_by_processor_merchant_id_profile_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;

    async fn get_blocklist_entries_count_by_processor_merchant_id_profile_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError>;

    /// Counts blocklist entries grouped by fingerprint length, computed via a SQL `GROUP BY`
    /// rather than fetching every row - backs `/blocklist/count`'s `counts_by_length`.
    async fn count_blocklist_entries_by_fingerprint_length_processor_merchant_id_profile_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<Vec<(i32, i64)>, errors::StorageError>;

    async fn list_blocklist_entries_by_processor_merchant_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;

    async fn list_blocklist_entries_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError>;

    async fn get_blocklist_entries_count_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError>;

    /// Bulk insert entries with `ON CONFLICT DO NOTHING`.
    /// Returns the number of rows *newly* inserted.
    /// Rows already present are silently skipped (not an error).
    async fn bulk_insert_blocklist_entries(
        &self,
        entries: Vec<storage::BlocklistNew>,
    ) -> CustomResult<usize, errors::StorageError>;
}

#[async_trait::async_trait]
impl BlocklistInterface for Store {
    #[instrument(skip_all)]
    async fn insert_blocklist_entry(
        &self,
        pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        pm_blocklist
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        // Stagger release fallback: first try processor_merchant_id, if not found fallback to merchant_id
        // For old records processor_merchant_id is NULL, so we use merchant_id (which has the same value)
        let result = storage::Blocklist::find_by_processor_merchant_id_fingerprint_id(
            &conn,
            processor_merchant_id,
            fingerprint_id,
        )
        .await;

        match result {
            Ok(blocklist_entry) => Ok(blocklist_entry),
            Err(error) => {
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) {
                    storage::Blocklist::find_by_merchant_id_fingerprint_id(
                        &conn,
                        processor_merchant_id,
                        fingerprint_id,
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                } else {
                    Err(report!(errors::StorageError::from(error)))
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn find_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let result = storage::Blocklist::find_by_processor_merchant_id_profile_id_fingerprint_id(
            &conn,
            processor_merchant_id,
            profile_id,
            fingerprint_id,
        )
        .await;

        match result {
            Ok(blocklist_entry) => Ok(blocklist_entry),
            Err(error) => {
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) {
                    storage::Blocklist::find_by_merchant_id_profile_id_fingerprint_id(
                        &conn,
                        processor_merchant_id,
                        profile_id,
                        fingerprint_id,
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                } else {
                    Err(report!(errors::StorageError::from(error)))
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn find_blocklist_entries_by_processor_merchant_id_profile_id_fingerprint_ids(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_ids: Vec<String>,
    ) -> CustomResult<Option<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::find_by_processor_merchant_id_profile_id_fingerprint_ids(
            &conn,
            processor_merchant_id,
            profile_id,
            fingerprint_ids,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id_profile_id_card_bins(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        card_bins: Vec<String>,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::find_by_processor_merchant_id_profile_id_card_bins(
            &conn,
            processor_merchant_id,
            profile_id,
            card_bins,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::list_by_processor_merchant_id(&conn, processor_merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::list_by_processor_merchant_id_data_kind(
            &conn,
            processor_merchant_id,
            data_kind,
            limit,
            offset,
        )
        .await
        .change_context(errors::StorageError::DatabaseError(report!(
            diesel_models::errors::DatabaseError::Others
        )))
    }

    #[instrument(skip_all)]
    async fn get_blocklist_entries_count_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::get_count_by_processor_merchant_id_data_kind(
            &conn,
            processor_merchant_id,
            data_kind,
        )
        .await
        .change_context(errors::StorageError::DatabaseError(report!(
            diesel_models::errors::DatabaseError::Others
        )))
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        // Stagger release fallback: first try processor_merchant_id, if not found fallback to merchant_id
        // For old records processor_merchant_id is NULL, so we delete by merchant_id (which has the same value)
        let result = storage::Blocklist::delete_by_processor_merchant_id_fingerprint_id(
            &conn,
            processor_merchant_id,
            fingerprint_id,
        )
        .await;

        match result {
            Ok(blocklist) => Ok(blocklist),
            Err(error) => {
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) {
                    storage::Blocklist::delete_by_merchant_id_fingerprint_id(
                        &conn,
                        processor_merchant_id,
                        fingerprint_id,
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                } else {
                    Err(report!(errors::StorageError::from(error)))
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let result = storage::Blocklist::delete_by_processor_merchant_id_profile_id_fingerprint_id(
            &conn,
            processor_merchant_id,
            profile_id,
            fingerprint_id,
        )
        .await;

        match result {
            Ok(blocklist) => Ok(blocklist),
            Err(error) => {
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) {
                    storage::Blocklist::delete_by_merchant_id_profile_id_fingerprint_id(
                        &conn,
                        processor_merchant_id,
                        profile_id,
                        fingerprint_id,
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                } else {
                    Err(report!(errors::StorageError::from(error)))
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id_profile_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        match profile_id {
            Some(profile_id) => {
                storage::Blocklist::list_by_processor_merchant_id_profile_id_data_kind(
                    &conn,
                    processor_merchant_id,
                    profile_id,
                    data_kind,
                    limit,
                    offset,
                )
                .await
            }
            None => {
                storage::Blocklist::list_by_processor_merchant_id_data_kind(
                    &conn,
                    processor_merchant_id,
                    data_kind,
                    limit,
                    offset,
                )
                .await
            }
        }
        .change_context(errors::StorageError::DatabaseError(report!(
            diesel_models::errors::DatabaseError::Others
        )))
    }

    #[instrument(skip_all)]
    async fn get_blocklist_entries_count_by_processor_merchant_id_profile_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        match profile_id {
            Some(profile_id) => {
                storage::Blocklist::get_count_by_processor_merchant_id_profile_id_data_kind(
                    &conn,
                    processor_merchant_id,
                    profile_id,
                    data_kind,
                )
                .await
            }
            None => {
                storage::Blocklist::get_count_by_processor_merchant_id_data_kind(
                    &conn,
                    processor_merchant_id,
                    data_kind,
                )
                .await
            }
        }
        .change_context(errors::StorageError::DatabaseError(report!(
            diesel_models::errors::DatabaseError::Others
        )))
    }

    #[instrument(skip_all)]
    async fn count_blocklist_entries_by_fingerprint_length_processor_merchant_id_profile_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<Vec<(i32, i64)>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Blocklist::count_by_fingerprint_length_processor_merchant_id_profile_id_data_kind(
            &conn,
            processor_merchant_id,
            profile_id,
            data_kind,
        )
        .await
        .change_context(errors::StorageError::DatabaseError(report!(
            diesel_models::errors::DatabaseError::Others
        )))
    }

    #[instrument(skip_all)]
    async fn bulk_insert_blocklist_entries(
        &self,
        entries: Vec<storage::BlocklistNew>,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BlocklistNew::bulk_insert_on_conflict_do_nothing(&conn, entries)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl BlocklistInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_blocklist_entry(
        &self,
        _pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _profile_id: &common_utils::id_type::ProfileId,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_blocklist_entries_by_processor_merchant_id_profile_id_fingerprint_ids(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _profile_id: &common_utils::id_type::ProfileId,
        _fingerprint_ids: Vec<String>,
    ) -> CustomResult<Option<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_processor_merchant_id_profile_id_card_bins(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _profile_id: &common_utils::id_type::ProfileId,
        _card_bins: Vec<String>,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_processor_merchant_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_processor_merchant_id_data_kind(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _data_kind: common_enums::BlocklistDataKind,
        _limit: i64,
        _offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn get_blocklist_entries_count_by_processor_merchant_id_data_kind(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _profile_id: &common_utils::id_type::ProfileId,
        _fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_blocklist_entries_by_processor_merchant_id_profile_id_data_kind(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _profile_id: Option<&common_utils::id_type::ProfileId>,
        _data_kind: common_enums::BlocklistDataKind,
        _limit: i64,
        _offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn get_blocklist_entries_count_by_processor_merchant_id_profile_id_data_kind(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _profile_id: Option<&common_utils::id_type::ProfileId>,
        _data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn count_blocklist_entries_by_fingerprint_length_processor_merchant_id_profile_id_data_kind(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _profile_id: &common_utils::id_type::ProfileId,
        _data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<Vec<(i32, i64)>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn bulk_insert_blocklist_entries(
        &self,
        entries: Vec<storage::BlocklistNew>,
    ) -> CustomResult<usize, errors::StorageError> {
        let mut blocklists = self.blocklists.lock().await;
        let mut inserted = 0usize;
        for entry in entries {
            let already_exists = blocklists.iter().any(|b| {
                b.processor_merchant_id == entry.processor_merchant_id
                    && b.profile_id == entry.profile_id
                    && b.fingerprint_id == entry.fingerprint_id
            });
            if !already_exists {
                blocklists.push(storage::Blocklist {
                    merchant_id: entry.merchant_id,
                    fingerprint_id: entry.fingerprint_id,
                    data_kind: entry.data_kind,
                    metadata: entry.metadata,
                    created_at: entry.created_at,
                    processor_merchant_id: entry.processor_merchant_id,
                    created_by: entry.created_by,
                    profile_id: entry.profile_id,
                });
                inserted += 1;
            }
        }
        Ok(inserted)
    }
}

#[async_trait::async_trait]
impl BlocklistInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_blocklist_entry(
        &self,
        pm_blocklist: storage::BlocklistNew,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store.insert_blocklist_entry(pm_blocklist).await
    }

    #[instrument(skip_all)]
    async fn find_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .find_blocklist_entry_by_processor_merchant_id_fingerprint_id(
                processor_merchant_id,
                fingerprint_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn find_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .find_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
                processor_merchant_id,
                profile_id,
                fingerprint_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn find_blocklist_entries_by_processor_merchant_id_profile_id_fingerprint_ids(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_ids: Vec<String>,
    ) -> CustomResult<Option<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .find_blocklist_entries_by_processor_merchant_id_profile_id_fingerprint_ids(
                processor_merchant_id,
                profile_id,
                fingerprint_ids,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id_profile_id_card_bins(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        card_bins: Vec<String>,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_processor_merchant_id_profile_id_card_bins(
                processor_merchant_id,
                profile_id,
                card_bins,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .delete_blocklist_entry_by_processor_merchant_id_fingerprint_id(
                processor_merchant_id,
                fingerprint_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_processor_merchant_id_data_kind(
                processor_merchant_id,
                data_kind,
                limit,
                offset,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn delete_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        fingerprint_id: &str,
    ) -> CustomResult<storage::Blocklist, errors::StorageError> {
        self.diesel_store
            .delete_blocklist_entry_by_processor_merchant_id_profile_id_fingerprint_id(
                processor_merchant_id,
                profile_id,
                fingerprint_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id_profile_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_processor_merchant_id_profile_id_data_kind(
                processor_merchant_id,
                profile_id,
                data_kind,
                limit,
                offset,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn get_blocklist_entries_count_by_processor_merchant_id_profile_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .get_blocklist_entries_count_by_processor_merchant_id_profile_id_data_kind(
                processor_merchant_id,
                profile_id,
                data_kind,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn get_blocklist_entries_count_by_processor_merchant_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .get_blocklist_entries_count_by_processor_merchant_id_data_kind(
                processor_merchant_id,
                data_kind,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn count_blocklist_entries_by_fingerprint_length_processor_merchant_id_profile_id_data_kind(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        data_kind: common_enums::BlocklistDataKind,
    ) -> CustomResult<Vec<(i32, i64)>, errors::StorageError> {
        self.diesel_store
            .count_blocklist_entries_by_fingerprint_length_processor_merchant_id_profile_id_data_kind(
                processor_merchant_id,
                profile_id,
                data_kind,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn list_blocklist_entries_by_processor_merchant_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<storage::Blocklist>, errors::StorageError> {
        self.diesel_store
            .list_blocklist_entries_by_processor_merchant_id(processor_merchant_id)
            .await
    }

    #[instrument(skip_all)]
    async fn bulk_insert_blocklist_entries(
        &self,
        entries: Vec<storage::BlocklistNew>,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .bulk_insert_blocklist_entries(entries)
            .await
    }
}
