pub use diesel_models::process_tracker::{
    ProcessData, ProcessTracker, ProcessTrackerNew, ProcessTrackerUpdate,
    ProcessTrackerUpdateInternal, SchedulerOptions,
};
use error_stack::ResultExt;
use serde::Serialize;
use time::PrimitiveDateTime;

use crate::{
    core::errors, db::StorageInterface, scheduler::metrics, types::storage::enums as storage_enums,
};

#[async_trait::async_trait]
pub trait ProcessTrackerExt {
    fn is_valid_business_status(&self, valid_statuses: &[&str]) -> bool;

    fn make_process_tracker_new<'a, T>(
        process_tracker_id: String,
        task: &'a str,
        runner: &'a str,
        tracking_data: T,
        schedule_time: PrimitiveDateTime,
    ) -> Result<ProcessTrackerNew, errors::ProcessTrackerError>
    where
        T: Serialize;

    async fn reset(
        self,
        db: &dyn StorageInterface,
        schedule_time: PrimitiveDateTime,
    ) -> Result<(), errors::ProcessTrackerError>;

    async fn retry(
        self,
        db: &dyn StorageInterface,
        schedule_time: PrimitiveDateTime,
    ) -> Result<(), errors::ProcessTrackerError>;

    async fn finish_with_status(
        self,
        db: &dyn StorageInterface,
        status: String,
    ) -> Result<(), errors::ProcessTrackerError>;
}

#[async_trait::async_trait]
impl ProcessTrackerExt for ProcessTracker {
    fn is_valid_business_status(&self, valid_statuses: &[&str]) -> bool {
        valid_statuses.iter().any(|x| x == &self.business_status)
    }

    fn make_process_tracker_new<'a, T>(
        process_tracker_id: String,
        task: &'a str,
        runner: &'a str,
        tracking_data: T,
        schedule_time: PrimitiveDateTime,
    ) -> Result<ProcessTrackerNew, errors::ProcessTrackerError>
    where
        T: Serialize,
    {
        let current_time = common_utils::date_time::now();
        Ok(ProcessTrackerNew {
            id: process_tracker_id,
            name: Some(String::from(task)),
            tag: vec![String::from("SYNC"), String::from("PAYMENT")],
            runner: Some(String::from(runner)),
            retry_count: 0,
            schedule_time: Some(schedule_time),
            rule: String::new(),
            tracking_data: serde_json::to_value(tracking_data)
                .map_err(|_| errors::ProcessTrackerError::SerializationFailed)?,
            business_status: String::from("Pending"),
            status: storage_enums::ProcessTrackerStatus::New,
            event: vec![],
            created_at: current_time,
            updated_at: current_time,
        })
    }

    async fn reset(
        self,
        db: &dyn StorageInterface,
        schedule_time: PrimitiveDateTime,
    ) -> Result<(), errors::ProcessTrackerError> {
        db.update_process_tracker(
            self.clone(),
            ProcessTrackerUpdate::StatusRetryUpdate {
                status: storage_enums::ProcessTrackerStatus::New,
                retry_count: 0,
                schedule_time,
            },
        )
        .await?;
        Ok(())
    }

    async fn retry(
        self,
        db: &dyn StorageInterface,
        schedule_time: PrimitiveDateTime,
    ) -> Result<(), errors::ProcessTrackerError> {
        metrics::TASK_RETRIED.add(&metrics::CONTEXT, 1, &[]);
        db.update_process_tracker(
            self.clone(),
            ProcessTrackerUpdate::StatusRetryUpdate {
                status: storage_enums::ProcessTrackerStatus::Pending,
                retry_count: self.retry_count + 1,
                schedule_time,
            },
        )
        .await?;
        Ok(())
    }

    async fn finish_with_status(
        self,
        db: &dyn StorageInterface,
        status: String,
    ) -> Result<(), errors::ProcessTrackerError> {
        db.update_process(
            self,
            ProcessTrackerUpdate::StatusUpdate {
                status: storage_enums::ProcessTrackerStatus::Finish,
                business_status: Some(status),
            },
        )
        .await
        .attach_printable("Failed while updating status of the process")?;
        metrics::TASK_FINISHED.add(&metrics::CONTEXT, 1, &[]);
        Ok(())
    }
}
