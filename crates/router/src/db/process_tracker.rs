//! Gate for inserts into the `process_tracker` table.
//!
//! Every production insert into `process_tracker` MUST go through
//! [`insert_process_if_task_creation_enabled`] so that the
//! `[scheduler] task_creation_enabled` configuration key can suppress task
//! creation globally (for example, during incidents) without touching the
//! flow code that constructs the tasks.

use router_env::logger;

use super::{errors, storage, CustomResult, StorageInterface};

/// Inserts `process_tracker_entry` into the `process_tracker` table unless
/// task creation is disabled via `[scheduler] task_creation_enabled = false`.
///
/// When task creation is enabled, the row is inserted and `TASKS_ADDED_COUNT`
/// is incremented with `flow_attributes` (if provided). When task creation is
/// disabled, no row is written. One tagged log line and a
/// [`PROCESS_TRACKER_TASK_CREATION_SKIPPED_COUNT`] metric increment are
/// emitted instead, and a synthesized (not persisted)
/// [`storage::ProcessTracker`] is returned so that callers that consume the
/// inserted row keep working.
///
/// Callers that historically incremented `TASKS_ADDED_COUNT` on a successful
/// insert pass their existing `flow` attributes here; callers that never
/// counted use `None`.
///
/// Warning: a task skipped this way is never rescheduled, not even after the
/// toggle is re-enabled. Objects created while the toggle is off have no
/// scheduler tasks and need manual recovery (for example, a payments PSync
/// or a refund retrieve).
///
/// [`PROCESS_TRACKER_TASK_CREATION_SKIPPED_COUNT`]: crate::routes::metrics::PROCESS_TRACKER_TASK_CREATION_SKIPPED_COUNT
pub async fn insert_process_if_task_creation_enabled(
    db: &dyn StorageInterface,
    process_tracker_entry: storage::ProcessTrackerNew,
    scheduler_settings: &scheduler::SchedulerSettings,
    flow_attributes: Option<&[router_env::opentelemetry::KeyValue]>,
) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
    if scheduler_settings.task_creation_enabled {
        let inserted = db.insert_process(process_tracker_entry).await?;
        if let Some(flow_attributes) = flow_attributes {
            crate::routes::metrics::TASKS_ADDED_COUNT.add(1, flow_attributes);
        }
        Ok(inserted)
    } else {
        let task_name = process_tracker_entry.name.clone().unwrap_or_default();
        let runner_name = process_tracker_entry.runner.clone().unwrap_or_default();
        logger::info!(
            process_tracker_id = %process_tracker_entry.id,
            process_tracker_task = %task_name,
            process_tracker_runner = %runner_name,
            "[scheduler] task_creation_enabled is false, skipping process_tracker insert"
        );
        crate::routes::metrics::PROCESS_TRACKER_TASK_CREATION_SKIPPED_COUNT.add(
            1,
            router_env::metric_attributes!(("task", task_name), ("runner", runner_name)),
        );
        Ok(process_tracker_entry.into())
    }
}

#[cfg(test)]
mod tests {
    use common_enums::{ApiVersion, ApplicationSource};
    use common_utils::types::keymanager::KeyManagerState;
    use storage_impl::MockDb;

    use super::*;

    fn sample_process_tracker_entry() -> storage::ProcessTrackerNew {
        #[allow(clippy::expect_used)]
        storage::ProcessTrackerNew::new(
            "process_tracker_id",
            "TEST_TASK",
            storage::ProcessTrackerRunner::OutgoingWebhookRetryWorkflow,
            ["TEST"],
            "test_tracking_data",
            None,
            common_utils::date_time::now(),
            ApiVersion::V1,
            ApplicationSource::Main,
        )
        .expect("Failed to construct sample process tracker entry")
    }

    #[tokio::test]
    async fn should_insert_process_when_task_creation_is_enabled() {
        #[allow(clippy::expect_used)]
        let db = MockDb::new(
            &redis_interface::RedisSettings::default(),
            KeyManagerState::mock(),
        )
        .await
        .expect("Failed to create MockDb");

        let scheduler_settings = scheduler::SchedulerSettings::default();
        assert!(scheduler_settings.task_creation_enabled);

        let task = insert_process_if_task_creation_enabled(
            &db,
            sample_process_tracker_entry(),
            &scheduler_settings,
            None,
        )
        .await
        .expect("Insert should succeed when task creation is enabled");
        assert_eq!(task.id, "process_tracker_id");

        assert_eq!(db.processes.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn should_skip_insert_when_task_creation_is_disabled() {
        #[allow(clippy::expect_used)]
        let db = MockDb::new(
            &redis_interface::RedisSettings::default(),
            KeyManagerState::mock(),
        )
        .await
        .expect("Failed to create MockDb");

        let scheduler_settings = scheduler::SchedulerSettings {
            task_creation_enabled: false,
            ..Default::default()
        };

        let task = insert_process_if_task_creation_enabled(
            &db,
            sample_process_tracker_entry(),
            &scheduler_settings,
            None,
        )
        .await
        .expect("Skip path should return a synthesized process tracker entry");
        assert_eq!(task.id, "process_tracker_id");

        assert!(db.processes.lock().await.is_empty());
    }
}
