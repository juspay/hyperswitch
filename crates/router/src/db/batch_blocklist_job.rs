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

    async fn list_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
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
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::BatchBlocklistJob>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::BatchBlocklistJob::list_by_merchant_id(&conn, merchant_id, limit, offset)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
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
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::BatchBlocklistJob::count_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
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
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::BatchBlocklistJob>, errors::StorageError> {
        Ok(self
            .batch_blocklist_jobs
            .lock()
            .await
            .iter()
            .filter(|j| j.merchant_id.get_string_repr() == merchant_id)
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
        job.updated_at = update.updated_at;
        Ok(job.clone())
    }

    #[instrument(skip_all)]
    async fn count_batch_blocklist_jobs_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<usize, errors::StorageError> {
        Ok(self
            .batch_blocklist_jobs
            .lock()
            .await
            .iter()
            .filter(|j| j.merchant_id.get_string_repr() == merchant_id)
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
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::BatchBlocklistJob>, errors::StorageError> {
        self.diesel_store
            .list_batch_blocklist_jobs_by_merchant_id(merchant_id, limit, offset)
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
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .count_batch_blocklist_jobs_by_merchant_id(merchant_id)
            .await
    }
}
