use common_utils::ext_traits::Encode;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, errors, schema::process_tracker, StorageResult};

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

impl ProcessTracker {
    #[inline(always)]
    pub fn is_valid_business_status(&self, valid_statuses: &[&str]) -> bool {
        valid_statuses.iter().any(|&x| x == self.business_status)
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
    pub status: storage_enums::ProcessTrackerStatus,
    pub event: Vec<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

impl ProcessTrackerNew {
    pub fn new<T>(
        process_tracker_id: impl Into<String>,
        task: impl Into<String>,
        runner: ProcessTrackerRunner,
        tag: impl IntoIterator<Item = impl Into<String>>,
        tracking_data: T,
        schedule_time: PrimitiveDateTime,
    ) -> StorageResult<Self>
    where
        T: Serialize + std::fmt::Debug,
    {
        let current_time = common_utils::date_time::now();
        Ok(Self {
            id: process_tracker_id.into(),
            name: Some(task.into()),
            tag: tag.into_iter().map(Into::into).collect(),
            runner: Some(runner.to_string()),
            retry_count: 0,
            schedule_time: Some(schedule_time),
            rule: String::new(),
            tracking_data: tracking_data
                .encode_to_value()
                .change_context(errors::DatabaseError::Others)
                .attach_printable("Failed to serialize process tracker tracking data")?,
            business_status: String::from(business_status::PENDING),
            status: storage_enums::ProcessTrackerStatus::New,
            event: vec![],
            created_at: current_time,
            updated_at: current_time,
        })
    }
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

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    strum::EnumString,
    strum::Display,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum ProcessTrackerRunner {
    PaymentsSyncWorkflow,
    RefundWorkflowRouter,
    DeleteTokenizeDataWorkflow,
    ApiKeyExpiryWorkflow,
    OutgoingWebhookRetryWorkflow,
    AttachPayoutAccountWorkflow,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use common_utils::ext_traits::StringExt;

    use super::ProcessTrackerRunner;

    #[test]
    fn test_enum_to_string() {
        let string_format = "PAYMENTS_SYNC_WORKFLOW".to_string();
        let enum_format: ProcessTrackerRunner =
            string_format.parse_enum("ProcessTrackerRunner").unwrap();
        assert_eq!(enum_format, ProcessTrackerRunner::PaymentsSyncWorkflow);
    }
}

pub mod business_status {
    /// Indicates that an irrecoverable error occurred during the workflow execution.
    pub const GLOBAL_FAILURE: &str = "GLOBAL_FAILURE";

    /// Task successfully completed by consumer.
    /// A task that reaches this status should not be retried (rescheduled for execution) later.
    pub const COMPLETED_BY_PT: &str = "COMPLETED_BY_PT";

    /// An error occurred during the workflow execution which prevents further execution and
    /// retries.
    /// A task that reaches this status should not be retried (rescheduled for execution) later.
    pub const FAILURE: &str = "FAILURE";

    /// The resource associated with the task was removed, due to which further retries can/should
    /// not be done.
    pub const REVOKED: &str = "Revoked";

    /// The task was executed for the maximum possible number of times without a successful outcome.
    /// A task that reaches this status should not be retried (rescheduled for execution) later.
    pub const RETRIES_EXCEEDED: &str = "RETRIES_EXCEEDED";

    /// The outgoing webhook was successfully delivered in the initial attempt.
    /// Further retries of the task are not required.
    pub const INITIAL_DELIVERY_ATTEMPT_SUCCESSFUL: &str = "INITIAL_DELIVERY_ATTEMPT_SUCCESSFUL";

    /// Indicates that an error occurred during the workflow execution.
    /// This status is typically set by the workflow error handler.
    /// A task that reaches this status should not be retried (rescheduled for execution) later.
    pub const GLOBAL_ERROR: &str = "GLOBAL_ERROR";

    /// The resource associated with the task has been significantly modified since the task was
    /// created, due to which further retries of the current task are not required.
    /// A task that reaches this status should not be retried (rescheduled for execution) later.
    pub const RESOURCE_STATUS_MISMATCH: &str = "RESOURCE_STATUS_MISMATCH";

    /// Business status set for newly created tasks.
    pub const PENDING: &str = "Pending";
}
