use error_stack::report;
use router_env::{instrument, tracing};
use storage_impl::MockDb;

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    types::storage,
};

/// The listing predicate the SQL queries apply, restated for `MockDb` so the two agree.
fn matches_job_scope(
    job: &storage::BatchBlocklistJob,
    merchant_id: &str,
    profile_id: Option<&common_utils::id_type::ProfileId>,
    job_types: &[common_enums::BatchBlocklistJobType],
) -> bool {
    job.merchant_id.get_string_repr() == merchant_id
        && job
            .job_type
            .is_some_and(|job_type| job_types.contains(&job_type))
        && profile_id.is_none_or(|profile_id| {
            job.profile_id
                .as_ref()
                .is_none_or(|job_profile_id| job_profile_id == profile_id)
        })
}

#[async_trait::async_trait]
pub trait BatchBlocklistJobInterface {
    async fn insert_batch_blocklist_job(
        &self,
        new: storage::BatchBlocklistJobNew,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError>;

    async fn find_batch_blocklist_job_by_id_merchant_id(
        &self,
        id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError>;

    /// `profile_id` restricts the page to that profile; `None` lists the whole merchant.
    async fn list_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        job_types: Vec<common_enums::BatchBlocklistJobType>,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::BatchBlocklistJob>, errors::StorageError>;

    async fn update_batch_blocklist_job_by_id_merchant_id(
        &self,
        id: &str,
        merchant_id: &str,
        update: storage::BatchBlocklistJobUpdate,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError>;

    async fn count_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        job_types: Vec<common_enums::BatchBlocklistJobType>,
    ) -> CustomResult<usize, errors::StorageError>;
}

#[async_trait::async_trait]
impl BatchBlocklistJobInterface for Store {
    #[instrument(skip_all)]
    async fn insert_batch_blocklist_job(
        &self,
        new: storage::BatchBlocklistJobNew,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_batch_blocklist_job_by_id_merchant_id(
        &self,
        id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::BatchBlocklistJob::find_by_id_merchant_id(&conn, id, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        job_types: Vec<common_enums::BatchBlocklistJobType>,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::BatchBlocklistJob>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        match profile_id {
            Some(profile_id) => storage::BatchBlocklistJob::list_by_merchant_id_profile_id(
                &conn,
                merchant_id,
                profile_id,
                job_types,
                limit,
                offset,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error))),
            None => storage::BatchBlocklistJob::list_by_merchant_id(
                &conn,
                merchant_id,
                job_types,
                limit,
                offset,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error))),
        }
    }

    #[instrument(skip_all)]
    async fn update_batch_blocklist_job_by_id_merchant_id(
        &self,
        id: &str,
        merchant_id: &str,
        update: storage::BatchBlocklistJobUpdate,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::BatchBlocklistJob::update_by_id_merchant_id(&conn, id, merchant_id, update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn count_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        job_types: Vec<common_enums::BatchBlocklistJobType>,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        match profile_id {
            Some(profile_id) => storage::BatchBlocklistJob::count_by_merchant_id_profile_id(
                &conn,
                merchant_id,
                profile_id,
                job_types,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error))),
            None => storage::BatchBlocklistJob::count_by_merchant_id(&conn, merchant_id, job_types)
                .await
                .map_err(|error| report!(errors::StorageError::from(error))),
        }
    }
}

#[async_trait::async_trait]
impl BatchBlocklistJobInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_batch_blocklist_job(
        &self,
        new: storage::BatchBlocklistJobNew,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        let mut jobs = self.batch_blocklist_jobs.lock().await;
        if jobs.iter().any(|j| j.id == new.id) {
            Err(errors::StorageError::MockDbError)?
        }
        let job = storage::BatchBlocklistJob {
            id: new.id,
            merchant_id: new.merchant_id,
            status: new.status,
            total_rows: new.total_rows,
            succeeded_rows: new.succeeded_rows,
            failed_rows: new.failed_rows,
            created_at: new.created_at,
            updated_at: new.updated_at,
            profile_id: Some(new.profile_id),
            job_type: Some(new.job_type),
            file_name: new.file_name,
            file_key: None,
            error_message: None,
            expires_at: None,
        };
        jobs.push(job.clone());
        Ok(job)
    }

    #[instrument(skip_all)]
    async fn find_batch_blocklist_job_by_id_merchant_id(
        &self,
        id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        self.batch_blocklist_jobs
            .lock()
            .await
            .iter()
            .find(|j| j.id == id && j.merchant_id.get_string_repr() == merchant_id)
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "BatchBlocklistJob not found for id = {id} and merchant_id = {merchant_id}"
                ))
                .into(),
            )
    }

    #[instrument(skip_all)]
    async fn list_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        job_types: Vec<common_enums::BatchBlocklistJobType>,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::BatchBlocklistJob>, errors::StorageError> {
        Ok(self
            .batch_blocklist_jobs
            .lock()
            .await
            .iter()
            .filter(|j| matches_job_scope(j, merchant_id, profile_id, &job_types))
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .skip(usize::try_from(offset).unwrap_or(0))
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect())
    }

    #[instrument(skip_all)]
    async fn update_batch_blocklist_job_by_id_merchant_id(
        &self,
        id: &str,
        merchant_id: &str,
        update: storage::BatchBlocklistJobUpdate,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        let mut jobs = self.batch_blocklist_jobs.lock().await;
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id && j.merchant_id.get_string_repr() == merchant_id)
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "BatchBlocklistJob not found for id = {id} and merchant_id = {merchant_id}"
            )))?;
        if let Some(status) = update.status {
            job.status = status;
        }
        if let Some(succeeded_rows) = update.succeeded_rows {
            job.succeeded_rows = succeeded_rows;
        }
        if let Some(failed_rows) = update.failed_rows {
            job.failed_rows = failed_rows;
        }
        if let Some(total_rows) = update.total_rows {
            job.total_rows = total_rows;
        }
        if let Some(file_key) = update.file_key {
            job.file_key = Some(file_key);
        }
        if let Some(error_message) = update.error_message {
            job.error_message = Some(error_message);
        }
        if let Some(expires_at) = update.expires_at {
            job.expires_at = Some(expires_at);
        }
        job.updated_at = update.updated_at;
        Ok(job.clone())
    }

    #[instrument(skip_all)]
    async fn count_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        job_types: Vec<common_enums::BatchBlocklistJobType>,
    ) -> CustomResult<usize, errors::StorageError> {
        Ok(self
            .batch_blocklist_jobs
            .lock()
            .await
            .iter()
            .filter(|j| matches_job_scope(j, merchant_id, profile_id, &job_types))
            .count())
    }
}

#[async_trait::async_trait]
impl BatchBlocklistJobInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_batch_blocklist_job(
        &self,
        new: storage::BatchBlocklistJobNew,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        self.diesel_store.insert_batch_blocklist_job(new).await
    }

    #[instrument(skip_all)]
    async fn find_batch_blocklist_job_by_id_merchant_id(
        &self,
        id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        self.diesel_store
            .find_batch_blocklist_job_by_id_merchant_id(id, merchant_id)
            .await
    }

    #[instrument(skip_all)]
    async fn list_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        job_types: Vec<common_enums::BatchBlocklistJobType>,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::BatchBlocklistJob>, errors::StorageError> {
        self.diesel_store
            .list_batch_blocklist_jobs_by_merchant_id(
                merchant_id,
                profile_id,
                job_types,
                limit,
                offset,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn update_batch_blocklist_job_by_id_merchant_id(
        &self,
        id: &str,
        merchant_id: &str,
        update: storage::BatchBlocklistJobUpdate,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        self.diesel_store
            .update_batch_blocklist_job_by_id_merchant_id(id, merchant_id, update)
            .await
    }

    #[instrument(skip_all)]
    async fn count_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
        profile_id: Option<&common_utils::id_type::ProfileId>,
        job_types: Vec<common_enums::BatchBlocklistJobType>,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .count_batch_blocklist_jobs_by_merchant_id(merchant_id, profile_id, job_types)
            .await
    }
}
