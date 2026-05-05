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
        _new: storage::BatchBlocklistJobNew,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn find_batch_blocklist_job_by_id_merchant_id(
        &self,
        _id: &str,
        _merchant_id: &str,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn list_batch_blocklist_jobs_by_merchant_id(
        &self,
        _merchant_id: &str,
        _limit: i64,
        _offset: i64,
    ) -> CustomResult<Vec<storage::BatchBlocklistJob>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn update_batch_blocklist_job_by_id_merchant_id(
        &self,
        _id: &str,
        _merchant_id: &str,
        _update: storage::BatchBlocklistJobUpdate,
    ) -> CustomResult<storage::BatchBlocklistJob, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn count_batch_blocklist_jobs_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<usize, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
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
