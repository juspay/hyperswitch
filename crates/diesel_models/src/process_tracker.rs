use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::process_tracker};

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Deserialize,
    Identifiable,
    Queryable,
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
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub schedule_time: Option<PrimitiveDateTime>,
    pub rule: String,
    pub tracking_data: serde_json::Value,
    pub business_status: String,
    pub status: storage_enums::ProcessTrackerStatus,
    #[diesel(deserialize_as = super::DieselArray<String>)]
    pub event: Vec<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub updated_at: PrimitiveDateTime,
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
    pub status: storage_enums::ProcessTrackerStatus,
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
        status: Option<storage_enums::ProcessTrackerStatus>,
        updated_at: Option<PrimitiveDateTime>,
    },
    StatusUpdate {
        status: storage_enums::ProcessTrackerStatus,
        business_status: Option<String>,
    },
    StatusRetryUpdate {
        status: storage_enums::ProcessTrackerStatus,
        retry_count: i32,
        schedule_time: PrimitiveDateTime,
    },
}

#[derive(Debug, Clone, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = process_tracker)]
pub struct ProcessTrackerUpdateInternal {
    name: Option<String>,
    retry_count: Option<i32>,
    schedule_time: Option<PrimitiveDateTime>,
    tracking_data: Option<serde_json::Value>,
    business_status: Option<String>,
    status: Option<storage_enums::ProcessTrackerStatus>,
    updated_at: Option<PrimitiveDateTime>,
}

impl Default for ProcessTrackerUpdateInternal {
        /// Creates a new instance of the struct with default values for all fields, except for `updated_at` which is set to the current date and time.
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
        /// Converts a ProcessTrackerUpdate enum into the current struct.
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

#[allow(dead_code)]
pub struct SchedulerOptions {
    looper_interval: common_utils::date_time::Milliseconds,
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
#[allow(dead_code)]
pub struct ProcessData {
    db_name: String,
    cache_name: String,
    process_tracker: ProcessTracker,
}
