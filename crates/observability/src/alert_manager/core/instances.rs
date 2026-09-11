use std::cmp::Ordering;

use async_bb8_diesel::AsyncConnection;
use diesel_models::{
    observability::{
        alerts_main::{slack as slack_main, xyne as xyne_main, AnnouncementRow},
        merchants_alert_external::{
            slack as slack_instance, xyne as xyne_instance, MerchantInstanceRow,
        },
        merchants_alert_external_dimension::DimensionInstance,
    },
    DatabaseConnectionWithContext, StorageResult,
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use super::stamp;
use crate::{
    alert_manager::types::{
        instances::{
            DimensionInstanceEntry, DimensionInstanceWrite, DimensionReadResponse,
            DimensionSaveResponse, DimensionWriteRequest, InstanceReadResponse,
            InstanceSaveResponse, InstanceWriteRequest, MerchantInstanceEntry,
            MerchantInstanceWrite, Truncation,
        },
        lifecycle::Channel,
        ReadStatus, WriteStatus,
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    state::AppState,
};

const NAME_MAX_BYTES: usize = 64;

const VALUE_MAX_BYTES: usize = 255;

pub const TRUNCATION_KEY: &str = "truncated_by_impact";

pub async fn read_instances(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
) -> ObservabilityApiResult<InstanceReadResponse> {
    let connection = state.database_connection().await?;

    let rows = store::list(&connection, channel, announcement)
        .await
        .change_context(ObservabilityError::StorageUnavailable)
        .attach_printable("Failed to read the merchant alert instances")?;

    Ok(InstanceReadResponse {
        status: found_or_absent(rows.len()),
        merchants: rows.into_iter().map(MerchantInstanceEntry::from).collect(),
    })
}

pub async fn write_instances(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
    request: InstanceWriteRequest,
) -> ObservabilityApiResult<InstanceSaveResponse> {
    let connection = state.database_connection().await?;
    let parent = store::announcement(&connection, channel, announcement).await?;

    let now = stamp();
    let plan = InstancePlan::build(
        request.merchants,
        Parent {
            announcement,
            thread: parent.ts_slack,
        },
        now,
        state.conf.instances.max_merchants,
    )?;

    let borrowed = &connection;
    let rows = plan.rows;
    let applied = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            let removed = store::delete(borrowed, channel, announcement).await?;
            let stored = store::insert(borrowed, channel, rows).await?;

            Ok::<_, WriteFailure>(Applied { stored, removed })
        })
        .await
        .map_err(WriteFailure::into_report)?;

    Ok(InstanceSaveResponse {
        status: WriteStatus::Saved,
        ts_alert: now,
        merchants: applied.stored,
        removed: applied.removed,
        truncated: plan.truncated,
    })
}

pub async fn read_dimensions(
    state: AppState,
    announcement: uuid::Uuid,
) -> ObservabilityApiResult<DimensionReadResponse> {
    let connection = state.database_connection().await?;

    let rows = DimensionInstance::list_for_announcement(&connection, announcement)
        .await
        .change_context(ObservabilityError::StorageUnavailable)
        .attach_printable("Failed to read the alert dimension breakdown")?;

    Ok(DimensionReadResponse {
        status: found_or_absent(rows.len()),
        dimensions: rows.into_iter().map(DimensionInstanceEntry::from).collect(),
    })
}

pub async fn write_dimensions(
    state: AppState,
    announcement: uuid::Uuid,
    request: DimensionWriteRequest,
) -> ObservabilityApiResult<DimensionSaveResponse> {
    let connection = state.database_connection().await?;
    let parent = store::announcement(&connection, Channel::Slack, announcement).await?;

    let now = stamp();
    let plan = DimensionPlan::build(
        request.dimensions,
        Parent {
            announcement,
            thread: parent.ts_slack,
        },
        now,
        state.conf.instances.max_dimensions,
    )?;

    let borrowed = &connection;
    let rows = plan.rows;
    let applied = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            let removed =
                DimensionInstance::delete_for_announcement(borrowed, announcement).await?;
            let stored = DimensionInstance::insert_all(borrowed, rows).await?;

            Ok::<_, WriteFailure>(Applied { stored, removed })
        })
        .await
        .map_err(WriteFailure::into_report)?;

    Ok(DimensionSaveResponse {
        status: WriteStatus::Saved,
        ts_alert: now,
        dimensions: applied.stored,
        removed: applied.removed,
        truncated: plan.truncated,
    })
}

struct Parent {
    announcement: uuid::Uuid,
    thread: Option<String>,
}

struct Applied {
    stored: usize,
    removed: usize,
}

fn found_or_absent(rows: usize) -> ReadStatus {
    if rows == 0 {
        ReadStatus::Absent
    } else {
        ReadStatus::Found
    }
}

#[derive(Debug)]
struct InstancePlan {
    rows: Vec<MerchantInstanceRow>,
    truncated: Option<Truncation>,
}

impl InstancePlan {
    fn build(
        writes: Vec<MerchantInstanceWrite>,
        parent: Parent,
        now: PrimitiveDateTime,
        limit: usize,
    ) -> ObservabilityApiResult<Self> {
        for write in &writes {
            fits(write.name.as_deref(), "name", NAME_MAX_BYTES)?;
            fits(write.product.as_deref(), "product", NAME_MAX_BYTES)?;
            fits(write.merchant_id.as_deref(), "merchant_id", NAME_MAX_BYTES)?;
            fits(write.priority.as_deref(), "priority", NAME_MAX_BYTES)?;
            fits(write.tenant_id.as_deref(), "tenant_id", NAME_MAX_BYTES)?;
            fits(write.attribution.as_deref(), "attribution", VALUE_MAX_BYTES)?;
            fits(write.ts_slack.as_deref(), "ts_slack", VALUE_MAX_BYTES)?;
        }

        let (kept, truncated) = keep_the_worst(writes, limit, |write| {
            impact(write.current_metric, write.expected_metric)
        });
        let marker = truncated.as_ref().map(marker_for);

        let rows = kept
            .into_iter()
            .map(|write| MerchantInstanceRow {
                id: Some(parent.announcement),
                id_merchant_table: uuid::Uuid::now_v7(),
                id_intermediate: write.id_intermediate,
                name: write.name,
                product: write.product,
                merchant_id: write.merchant_id,
                dimensions: write.dimensions,
                auxiliary_dimensions: write.auxiliary_dimensions,
                current_metric: write.current_metric,
                expected_metric: write.expected_metric,
                attribution: write.attribution,
                max_duration: write.max_duration,
                start_time: write.start_time,
                is_visible: Some(write.is_visible.unwrap_or(true)),
                recovered_ts: write.recovered_ts,
                ts_slack: write.ts_slack.or_else(|| parent.thread.clone()),
                ts_alert: Some(now),
                latest_ts_alert: write.latest_ts_alert,
                last_updated_at: Some(now),
                slack_info: write.slack_info,
                communication_info: write.communication_info,
                metadata: write.metadata,
                metadata_alert_details: record_truncation(
                    write.metadata_alert_details,
                    marker.as_ref(),
                ),
                priority: write.priority,
                tenant_id: write.tenant_id,
            })
            .collect();

        Ok(Self { rows, truncated })
    }
}

#[derive(Debug)]
struct DimensionPlan {
    rows: Vec<DimensionInstance>,
    truncated: Option<Truncation>,
}

impl DimensionPlan {
    fn build(
        writes: Vec<DimensionInstanceWrite>,
        parent: Parent,
        now: PrimitiveDateTime,
        limit: usize,
    ) -> ObservabilityApiResult<Self> {
        for write in &writes {
            fits(write.name.as_deref(), "name", NAME_MAX_BYTES)?;
            fits(write.product.as_deref(), "product", NAME_MAX_BYTES)?;
            fits(
                write.dimension_key.as_deref(),
                "dimension_key",
                NAME_MAX_BYTES,
            )?;
            fits(write.priority.as_deref(), "priority", NAME_MAX_BYTES)?;
            fits(write.tenant_id.as_deref(), "tenant_id", NAME_MAX_BYTES)?;
            fits(
                write.dimension_value.as_deref(),
                "dimension_value",
                VALUE_MAX_BYTES,
            )?;
            fits(write.attribution.as_deref(), "attribution", VALUE_MAX_BYTES)?;
            fits(write.ts_slack.as_deref(), "ts_slack", VALUE_MAX_BYTES)?;
        }

        let (kept, truncated) = keep_the_worst(writes, limit, |write| {
            impact(write.current_metric, write.expected_metric)
        });
        let marker = truncated.as_ref().map(marker_for);

        let rows = kept
            .into_iter()
            .map(|write| DimensionInstance {
                id: Some(parent.announcement),
                id_merchant_table: uuid::Uuid::now_v7(),
                id_intermediate: write.id_intermediate,
                name: write.name,
                product: write.product,
                dimension_key: write.dimension_key,
                dimension_value: write.dimension_value,
                dimensions: write.dimensions,
                auxiliary_dimensions: write.auxiliary_dimensions,
                current_metric: write.current_metric,
                expected_metric: write.expected_metric,
                attribution: write.attribution,
                max_duration: write.max_duration,
                is_visible: Some(write.is_visible.unwrap_or(true)),
                start_time: write.start_time,
                recovered_ts: write.recovered_ts,
                ts_slack: write.ts_slack.or_else(|| parent.thread.clone()),
                ts_alert: Some(now),
                latest_ts_alert: write.latest_ts_alert,
                last_updated_at: Some(now),
                slack_info: write.slack_info,
                communication_info: write.communication_info,
                metadata: write.metadata,
                metadata_alert_details: record_truncation(
                    write.metadata_alert_details,
                    marker.as_ref(),
                ),
                priority: write.priority,
                tenant_id: write.tenant_id,
            })
            .collect();

        Ok(Self { rows, truncated })
    }
}

#[derive(Debug, PartialEq)]
enum Impact {
    Absolute,
    Gap(f64),
    Unknown,
}

fn impact(current: Option<f64>, expected: Option<f64>) -> Impact {
    match (current, expected) {
        (Some(current), Some(expected)) => Impact::Gap((expected - current).abs()),
        (Some(_), None) => Impact::Absolute,
        (None, _) => Impact::Unknown,
    }
}

fn worst_first(left: &Impact, right: &Impact) -> Ordering {
    match (left, right) {
        (Impact::Absolute, Impact::Absolute) | (Impact::Unknown, Impact::Unknown) => {
            Ordering::Equal
        }
        (Impact::Absolute, _) | (Impact::Gap(_), Impact::Unknown) => Ordering::Less,
        (_, Impact::Absolute) | (Impact::Unknown, Impact::Gap(_)) => Ordering::Greater,
        (Impact::Gap(left), Impact::Gap(right)) => right.total_cmp(left),
    }
}

fn keep_the_worst<T>(
    mut rows: Vec<T>,
    limit: usize,
    impact_of: impl Fn(&T) -> Impact,
) -> (Vec<T>, Option<Truncation>) {
    let received = rows.len();
    if received <= limit {
        return (rows, None);
    }

    rows.sort_by(|left, right| worst_first(&impact_of(left), &impact_of(right)));
    rows.truncate(limit);

    logger::warn!(
        received = received,
        stored = limit,
        dropped = received - limit,
        "An alert instance write was truncated at the row cap"
    );

    (
        rows,
        Some(Truncation {
            received,
            stored: limit,
            dropped: received - limit,
        }),
    )
}

fn marker_for(truncation: &Truncation) -> serde_json::Value {
    serde_json::json!({
        "received": truncation.received,
        "stored": truncation.stored,
        "dropped": truncation.dropped,
    })
}

fn record_truncation(
    details: Option<serde_json::Value>,
    marker: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let Some(marker) = marker else {
        return details;
    };

    match details {
        Some(serde_json::Value::Object(mut fields)) => {
            fields.insert(TRUNCATION_KEY.to_owned(), marker.clone());
            Some(serde_json::Value::Object(fields))
        }
        Some(other) => Some(serde_json::json!({
            TRUNCATION_KEY: marker.clone(),
            "details": other,
        })),
        None => Some(serde_json::json!({ TRUNCATION_KEY: marker.clone() })),
    }
}

fn fits(value: Option<&str>, field: &'static str, max_bytes: usize) -> ObservabilityApiResult<()> {
    if let Some(value) = value {
        if value.len() > max_bytes {
            Err(
                report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                    "The instance {field} is {} bytes, over the {max_bytes} the column holds",
                    value.len()
                )),
            )?;
        }
    }

    Ok(())
}

enum WriteFailure {
    Storage(error_stack::Report<diesel_models::errors::DatabaseError>),
    Transaction(diesel::result::Error),
}

impl WriteFailure {
    fn into_report(self) -> error_stack::Report<ObservabilityError> {
        match self {
            Self::Storage(error) => error
                .change_context(ObservabilityError::StorageUnavailable)
                .attach_printable("Failed to write the alert instances"),
            Self::Transaction(error) => report!(ObservabilityError::StorageUnavailable)
                .attach_printable(format!(
                    "The alert instance write transaction failed: {error}"
                )),
        }
    }
}

impl From<diesel::result::Error> for WriteFailure {
    fn from(error: diesel::result::Error) -> Self {
        Self::Transaction(error)
    }
}

impl From<error_stack::Report<diesel_models::errors::DatabaseError>> for WriteFailure {
    fn from(error: error_stack::Report<diesel_models::errors::DatabaseError>) -> Self {
        Self::Storage(error)
    }
}

mod store {
    use super::{
        report, slack_instance, slack_main, xyne_instance, xyne_main, AnnouncementRow, Channel,
        DatabaseConnectionWithContext, MerchantInstanceRow, ObservabilityApiResult,
        ObservabilityError, ResultExt, StorageResult,
    };

    pub(super) async fn announcement(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        id: uuid::Uuid,
    ) -> ObservabilityApiResult<AnnouncementRow> {
        match channel {
            Channel::Slack => slack_main::Announcement::find_by_id(conn, id).await,
            Channel::Xyne => xyne_main::Announcement::find_by_id(conn, id).await,
        }
        .change_context(ObservabilityError::StorageUnavailable)
        .attach_printable("Failed to look up the announcement an instance write references")?
        .ok_or_else(|| report!(ObservabilityError::UnknownAnnouncement { id: id.to_string() }))
    }

    pub(super) async fn list(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        announcement: uuid::Uuid,
    ) -> StorageResult<Vec<MerchantInstanceRow>> {
        match channel {
            Channel::Slack => {
                slack_instance::MerchantInstance::list_for_announcement(conn, announcement).await
            }
            Channel::Xyne => {
                xyne_instance::MerchantInstance::list_for_announcement(conn, announcement).await
            }
        }
    }

    pub(super) async fn delete(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        announcement: uuid::Uuid,
    ) -> StorageResult<usize> {
        match channel {
            Channel::Slack => {
                slack_instance::MerchantInstance::delete_for_announcement(conn, announcement).await
            }
            Channel::Xyne => {
                xyne_instance::MerchantInstance::delete_for_announcement(conn, announcement).await
            }
        }
    }

    pub(super) async fn insert(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        rows: Vec<MerchantInstanceRow>,
    ) -> StorageResult<usize> {
        match channel {
            Channel::Slack => slack_instance::MerchantInstance::insert_all(conn, rows).await,
            Channel::Xyne => xyne_instance::MerchantInstance::insert_all(conn, rows).await,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn now() -> PrimitiveDateTime {
        common_utils::date_time::now()
    }

    fn parent() -> Parent {
        Parent {
            announcement: uuid::Uuid::now_v7(),
            thread: Some("1757400000.000100".to_owned()),
        }
    }

    fn merchant(id: &str, current: Option<f64>, expected: Option<f64>) -> MerchantInstanceWrite {
        MerchantInstanceWrite {
            id_intermediate: None,
            name: Some("sr_drop".to_owned()),
            product: Some("payments".to_owned()),
            merchant_id: Some(id.to_owned()),
            dimensions: None,
            auxiliary_dimensions: None,
            current_metric: current,
            expected_metric: expected,
            attribution: None,
            max_duration: None,
            start_time: None,
            is_visible: None,
            recovered_ts: None,
            ts_slack: None,
            latest_ts_alert: None,
            slack_info: None,
            communication_info: None,
            metadata: None,
            metadata_alert_details: None,
            priority: Some("SEV2".to_owned()),
            tenant_id: None,
        }
    }

    fn dimension(
        value: &str,
        current: Option<f64>,
        expected: Option<f64>,
    ) -> DimensionInstanceWrite {
        DimensionInstanceWrite {
            id_intermediate: None,
            name: Some("sr_drop".to_owned()),
            product: Some("payments".to_owned()),
            dimension_key: Some("connector".to_owned()),
            dimension_value: Some(value.to_owned()),
            dimensions: None,
            auxiliary_dimensions: None,
            current_metric: current,
            expected_metric: expected,
            attribution: None,
            max_duration: None,
            is_visible: None,
            start_time: None,
            recovered_ts: None,
            ts_slack: None,
            latest_ts_alert: None,
            slack_info: None,
            communication_info: None,
            metadata: None,
            metadata_alert_details: None,
            priority: None,
            tenant_id: None,
        }
    }

    fn merchants_of(plan: &InstancePlan) -> Vec<String> {
        plan.rows
            .iter()
            .map(|row| row.merchant_id.clone().unwrap_or_default())
            .collect()
    }

    #[test]
    fn every_row_is_given_an_id_of_its_own() {
        let plan = InstancePlan::build(
            vec![merchant("m1", None, None), merchant("m2", None, None)],
            parent(),
            now(),
            10,
        )
        .unwrap();

        assert_ne!(plan.rows[0].id_merchant_table, uuid::Uuid::nil());
        assert_ne!(
            plan.rows[0].id_merchant_table, plan.rows[1].id_merchant_table,
            "two rows in one write were given the same primary key"
        );
    }

    #[test]
    fn every_row_is_stamped_with_the_servers_clock() {
        let at = now();
        let plan = InstancePlan::build(
            vec![merchant("m1", None, None), merchant("m2", None, None)],
            parent(),
            at,
            10,
        )
        .unwrap();

        for row in &plan.rows {
            assert_eq!(row.ts_alert, Some(at));
            assert_eq!(row.last_updated_at, Some(at));
        }
    }

    #[test]
    fn a_row_that_says_nothing_about_visibility_is_visible() {
        let mut hidden = merchant("m2", None, None);
        hidden.is_visible = Some(false);

        let plan = InstancePlan::build(
            vec![merchant("m1", None, None), hidden],
            parent(),
            now(),
            10,
        )
        .unwrap();

        assert_eq!(plan.rows[0].is_visible, Some(true));
        assert_eq!(plan.rows[1].is_visible, Some(false));
    }

    #[test]
    fn a_breakdown_row_is_given_the_same_defaults() {
        let at = now();
        let plan =
            DimensionPlan::build(vec![dimension("stripe", None, None)], parent(), at, 10).unwrap();

        assert_ne!(plan.rows[0].id_merchant_table, uuid::Uuid::nil());
        assert_eq!(plan.rows[0].ts_alert, Some(at));
        assert_eq!(plan.rows[0].last_updated_at, Some(at));
        assert_eq!(plan.rows[0].is_visible, Some(true));
    }

    #[test]
    fn every_row_points_at_the_announcement_it_was_written_under() {
        let parent = parent();
        let announcement = parent.announcement;
        let plan = InstancePlan::build(
            vec![merchant("m1", None, None), merchant("m2", None, None)],
            parent,
            now(),
            10,
        )
        .unwrap();

        for row in &plan.rows {
            assert_eq!(row.id, Some(announcement));
        }
    }

    #[test]
    fn a_row_without_a_thread_takes_the_announcements() {
        let mut own = merchant("m2", None, None);
        own.ts_slack = Some("1757400000.999999".to_owned());

        let plan = InstancePlan::build(vec![merchant("m1", None, None), own], parent(), now(), 10)
            .unwrap();

        assert_eq!(plan.rows[0].ts_slack.as_deref(), Some("1757400000.000100"));
        assert_eq!(plan.rows[1].ts_slack.as_deref(), Some("1757400000.999999"));
    }

    #[test]
    fn an_instance_recorded_before_its_announcement_has_no_thread_to_store() {
        let plan = InstancePlan::build(
            vec![merchant("m1", None, None)],
            Parent {
                announcement: uuid::Uuid::now_v7(),
                thread: None,
            },
            now(),
            10,
        )
        .unwrap();

        assert!(plan.rows[0].ts_slack.is_none());
    }

    #[test]
    fn an_absent_expected_metric_is_stored_absent_rather_than_as_zero() {
        let plan = InstancePlan::build(vec![merchant("m1", Some(0.0), None)], parent(), now(), 10)
            .unwrap();

        assert_eq!(plan.rows[0].current_metric, Some(0.0));
        assert!(plan.rows[0].expected_metric.is_none());
    }

    #[test]
    fn a_write_inside_the_cap_is_stored_untouched() {
        let plan = InstancePlan::build(
            vec![
                merchant("m1", Some(10.0), Some(90.0)),
                merchant("m2", Some(89.0), Some(90.0)),
            ],
            parent(),
            now(),
            10,
        )
        .unwrap();

        assert!(plan.truncated.is_none());
        assert_eq!(merchants_of(&plan), vec!["m1", "m2"]);
    }

    #[test]
    fn a_write_over_the_cap_keeps_the_most_impacted_rows() {
        let plan = InstancePlan::build(
            vec![
                merchant("barely", Some(89.0), Some(90.0)),
                merchant("badly", Some(10.0), Some(90.0)),
                merchant("somewhat", Some(60.0), Some(90.0)),
            ],
            parent(),
            now(),
            2,
        )
        .unwrap();

        assert_eq!(merchants_of(&plan), vec!["badly", "somewhat"]);
        assert_eq!(
            plan.truncated,
            Some(Truncation {
                received: 3,
                stored: 2,
                dropped: 1,
            })
        );
    }

    #[test]
    fn an_absolute_outranks_every_measured_gap() {
        let plan = InstancePlan::build(
            vec![
                merchant("small_gap", Some(89.0), Some(90.0)),
                merchant("zero_volume", Some(0.0), None),
                merchant("large_gap", Some(1.0), Some(90.0)),
            ],
            parent(),
            now(),
            2,
        )
        .unwrap();

        assert_eq!(merchants_of(&plan), vec!["zero_volume", "large_gap"]);
    }

    #[test]
    fn a_row_that_measured_nothing_is_dropped_first() {
        let plan = InstancePlan::build(
            vec![
                merchant("nothing", None, None),
                merchant("barely", Some(89.9), Some(90.0)),
            ],
            parent(),
            now(),
            1,
        )
        .unwrap();

        assert_eq!(merchants_of(&plan), vec!["barely"]);
    }

    #[test]
    fn impact_is_the_distance_from_what_was_expected_in_either_direction() {
        assert_eq!(impact(Some(1.0), Some(3.0)), Impact::Gap(2.0));
        assert_eq!(impact(Some(5.0), Some(3.0)), Impact::Gap(2.0));
        assert_eq!(impact(Some(0.0), None), Impact::Absolute);
        assert_eq!(impact(None, Some(90.0)), Impact::Unknown);
        assert_eq!(impact(None, None), Impact::Unknown);
    }

    #[test]
    fn rows_of_equal_impact_keep_the_order_they_arrived_in() {
        let of = |plan: &DimensionPlan| {
            plan.rows
                .iter()
                .map(|row| row.dimension_value.clone().unwrap_or_default())
                .collect::<Vec<_>>()
        };
        let build = || {
            DimensionPlan::build(
                vec![
                    dimension("adyen", Some(10.0), Some(90.0)),
                    dimension("stripe", Some(10.0), Some(90.0)),
                    dimension("checkout", Some(10.0), Some(90.0)),
                ],
                parent(),
                now(),
                2,
            )
            .unwrap()
        };

        assert_eq!(of(&build()), vec!["adyen", "stripe"]);
        assert_eq!(of(&build()), of(&build()));
    }

    #[test]
    fn a_truncated_write_records_itself_on_every_row_it_kept() {
        let plan = DimensionPlan::build(
            vec![
                dimension("adyen", Some(10.0), Some(90.0)),
                dimension("stripe", Some(20.0), Some(90.0)),
            ],
            parent(),
            now(),
            1,
        )
        .unwrap();

        let marker = &plan.rows[0].metadata_alert_details.as_ref().unwrap()[TRUNCATION_KEY];

        assert_eq!(marker["received"], 2);
        assert_eq!(marker["stored"], 1);
        assert_eq!(marker["dropped"], 1);
    }

    #[test]
    fn a_write_that_was_not_cut_leaves_no_marker() {
        let mut carrying = merchant("m1", None, None);
        carrying.metadata_alert_details = Some(serde_json::json!({ "detail": "kept" }));

        let plan = InstancePlan::build(vec![carrying], parent(), now(), 10).unwrap();

        assert_eq!(
            plan.rows[0].metadata_alert_details,
            Some(serde_json::json!({ "detail": "kept" }))
        );
    }

    #[test]
    fn the_marker_is_added_to_the_callers_document_rather_than_replacing_it() {
        let mut carrying = merchant("m1", Some(1.0), Some(90.0));
        carrying.metadata_alert_details = Some(serde_json::json!({ "detail": "kept" }));

        let plan = InstancePlan::build(
            vec![carrying, merchant("m2", Some(89.0), Some(90.0))],
            parent(),
            now(),
            1,
        )
        .unwrap();

        let stored = plan.rows[0].metadata_alert_details.as_ref().unwrap();
        assert_eq!(stored["detail"], "kept");
        assert_eq!(stored[TRUNCATION_KEY]["dropped"], 1);
    }

    #[test]
    fn a_document_that_is_not_an_object_is_kept_beside_the_marker() {
        let mut carrying = merchant("m1", Some(1.0), Some(90.0));
        carrying.metadata_alert_details = Some(serde_json::json!(["kept"]));

        let plan = InstancePlan::build(
            vec![carrying, merchant("m2", Some(89.0), Some(90.0))],
            parent(),
            now(),
            1,
        )
        .unwrap();

        let stored = plan.rows[0].metadata_alert_details.as_ref().unwrap();
        assert_eq!(stored["details"], serde_json::json!(["kept"]));
        assert_eq!(stored[TRUNCATION_KEY]["received"], 2);
    }

    #[test]
    fn a_write_carrying_nothing_stores_nothing_and_is_not_a_truncation() {
        let plan = InstancePlan::build(Vec::new(), parent(), now(), 10).unwrap();

        assert!(plan.rows.is_empty());
        assert!(plan.truncated.is_none());
    }

    #[test]
    fn a_value_wider_than_its_column_is_rejected_before_the_query_runs() {
        let mut wide_merchant = merchant("m1", None, None);
        wide_merchant.merchant_id = Some("m".repeat(NAME_MAX_BYTES + 1));
        assert!(InstancePlan::build(vec![wide_merchant], parent(), now(), 10).is_err());

        let mut long_thread = merchant("m1", None, None);
        long_thread.ts_slack = Some("t".repeat(VALUE_MAX_BYTES + 1));
        assert!(InstancePlan::build(vec![long_thread], parent(), now(), 10).is_err());

        let mut wide_value = dimension("stripe", None, None);
        wide_value.dimension_value = Some("v".repeat(VALUE_MAX_BYTES + 1));
        assert!(DimensionPlan::build(vec![wide_value], parent(), now(), 10).is_err());
    }

    #[test]
    fn a_row_that_would_be_dropped_is_still_checked() {
        let mut wide = merchant("m2", Some(89.0), Some(90.0));
        wide.priority = Some("p".repeat(NAME_MAX_BYTES + 1));

        assert!(InstancePlan::build(
            vec![merchant("m1", Some(1.0), Some(90.0)), wide],
            parent(),
            now(),
            1,
        )
        .is_err());
    }

    #[test]
    fn an_absent_value_is_not_measured_against_its_column() {
        let mut bare = merchant("m1", None, None);
        bare.name = None;
        bare.merchant_id = None;
        bare.priority = None;

        assert!(InstancePlan::build(vec![bare], parent(), now(), 10).is_ok());
    }
}
