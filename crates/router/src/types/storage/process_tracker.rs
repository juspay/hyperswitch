#![allow(dead_code)]

use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{core::errors, db, scheduler::metrics, schema::process_tracker, types::storage::enums};

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Identifiable,
    Queryable,
    Deserialize,
    Serialize,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = process_tracker)]
pub struct ProcessTracker {
    pub id: String,
    pub name: Option<String>,
    #[diesel(deserialize_as = super::DieselArray<String>)]
    pub tag: Vec<String>,
    pub runner: Option<String>,
    pub retry_count: i32,
    pub schedule_time: Option<PrimitiveDateTime>,
    pub rule: String,
    pub tracking_data: serde_json::Value,
    pub business_status: String,
    pub status: enums::ProcessTrackerStatus,
    #[diesel(deserialize_as = super::DieselArray<String>)]
    pub event: Vec<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

impl ProcessTracker {
    pub fn is_valid_business_status(&self, valid_statuses: &[&str]) -> bool {
        valid_statuses.iter().any(|x| x == &self.business_status)
    }
    pub fn make_process_tracker_new<'a, T>(
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
            status: enums::ProcessTrackerStatus::New,
            event: vec![],
            created_at: current_time,
            updated_at: current_time,
        })
    }

    pub async fn retry(
        self,
        db: &dyn db::Db,
        schedule_time: PrimitiveDateTime,
    ) -> Result<(), errors::ProcessTrackerError> {
        metrics::TASK_RETRIED.add(1, &[]);
        db.update_process_tracker(
            self.clone(),
            ProcessTrackerUpdate::StatusRetryUpdate {
                status: enums::ProcessTrackerStatus::Pending,
                retry_count: self.retry_count + 1,
                schedule_time,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn finish_with_status(
        self,
        db: &dyn db::Db,
        status: String,
    ) -> Result<(), errors::ProcessTrackerError> {
        db.update_process(
            self,
            ProcessTrackerUpdate::StatusUpdate {
                status: enums::ProcessTrackerStatus::Finish,
                business_status: Some(status),
            },
        )
        .await
        .attach_printable("Failed while updating status of the process")?;
        metrics::TASK_FINISHED.add(1, &[]);
        Ok(())
    }
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = process_tracker)]
pub struct ProcessTrackerNew {
    pub id: String,
    pub name: Option<String>,
    pub tag: Vec<String>,
    pub runner: Option<String>,
    pub retry_count: i32,
    pub schedule_time: Option<PrimitiveDateTime>,
    pub rule: String,
    pub tracking_data: serde_json::Value,
    pub business_status: String,
    pub status: enums::ProcessTrackerStatus,
    pub event: Vec<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub enum ProcessTrackerUpdate {
    Update {
        name: Option<String>,
        retry_count: Option<i32>,
        schedule_time: Option<PrimitiveDateTime>,
        tracking_data: Option<serde_json::Value>,
        business_status: Option<String>,
        status: Option<enums::ProcessTrackerStatus>,
        updated_at: Option<PrimitiveDateTime>,
    },
    StatusUpdate {
        status: enums::ProcessTrackerStatus,
        business_status: Option<String>,
    },
    StatusRetryUpdate {
        status: enums::ProcessTrackerStatus,
        retry_count: i32,
        schedule_time: PrimitiveDateTime,
    },
}

#[derive(Debug, Clone, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = process_tracker)]
pub(super) struct ProcessTrackerUpdateInternal {
    name: Option<String>,
    retry_count: Option<i32>,
    schedule_time: Option<PrimitiveDateTime>,
    tracking_data: Option<serde_json::Value>,
    business_status: Option<String>,
    status: Option<enums::ProcessTrackerStatus>,
    updated_at: Option<PrimitiveDateTime>,
}

impl Default for ProcessTrackerUpdateInternal {
    fn default() -> Self {
        Self {
            name: Option::default(),
            retry_count: Option::default(),
            schedule_time: Option::default(),
            tracking_data: Option::default(),
            business_status: Option::default(),
            status: Option::default(),
            updated_at: Some(common_utils::date_time::now()),
        }
    }
}

impl From<ProcessTrackerUpdate> for ProcessTrackerUpdateInternal {
    fn from(process_tracker_update: ProcessTrackerUpdate) -> Self {
        match process_tracker_update {
            ProcessTrackerUpdate::Update {
                name,
                retry_count,
                schedule_time,
                tracking_data,
                business_status,
                status,
                updated_at,
            } => Self {
                name,
                retry_count,
                schedule_time,
                tracking_data,
                business_status,
                status,
                updated_at,
            },
            ProcessTrackerUpdate::StatusUpdate {
                status,
                business_status,
            } => Self {
                status: Some(status),
                business_status,
                ..Default::default()
            },
            ProcessTrackerUpdate::StatusRetryUpdate {
                status,
                retry_count,
                schedule_time,
            } => Self {
                status: Some(status),
                retry_count: Some(retry_count),
                schedule_time: Some(schedule_time),
                ..Default::default()
            },
        }
    }
}

// TODO: Move this to a utility module?
pub struct Milliseconds(i32);

pub struct SchedulerOptions {
    looper_interval: Milliseconds,
    db_name: String,
    cache_name: String,
    schema_name: String,
    cache_expiry: i32,
    runners: Vec<String>,
    fetch_limit: i32,
    fetch_limit_product_factor: i32,
    query_order: String,
}

#[derive(Debug, Clone)]
pub struct ProcessData {
    db_name: String,
    cache_name: String,
    process_tracker: ProcessTracker,
}
