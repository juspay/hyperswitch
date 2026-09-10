//! Per-request logic for per-merchant alert instances and their per-dimension breakdown.
//!
//! Two resources, both hanging off an announcement: the merchants an announcement was about, and
//! the dimension breakdown behind it. Each write replaces everything stored under that
//! announcement, so a rerun of the same alert manager pass is idempotent rather than doubling the
//! table.
//!
//! ## One handler, two channels, three tables
//!
//! `merchants_alert_external` and `merchants_alert_external_xyne` are the same table once per
//! delivery channel. The models are generated per table because diesel needs a struct per table —
//! but everything that decides anything is written once here and takes the channel as an argument.
//! The only code that branches on it is [`store`], whose functions are a two-arm match around one
//! query each. `merchants_alert_external_dimension` exists once and needs no channel at all; it
//! shares the policy below rather than a copy of it.
//!
//! ## The row cap truncates, and says so
//!
//! One alert across many connectors becomes many rows, and a *broad* outage is exactly when the
//! alert matters most — so a write over the cap is **not** refused. It is cut down to the cap and
//! stored, which is the opposite of what [`super::lifecycle`] does with its alert cap, and
//! deliberately so: losing the whole write there costs a rerun, losing it here costs the record of
//! who was affected during the worst incident of the day.
//!
//! Two things make the cut safe to reason about:
//!
//! * **The rows kept are the worst ones.** Rows are ordered by [`Impact`] before the cut, so what
//!   survives is what a human reading the alert would have looked at first.
//! * **The cut is recorded on every row it kept**, under
//!   [`TRUNCATION_KEY`](self::TRUNCATION_KEY) in `metadata_alert_details`, as well as in the
//!   response. A breakdown that quietly arrived shortened would make an outage look narrower than
//!   it was, and nothing downstream would ever know to ask.
//!
//! The cap is also a hard limit rather than only a policy: these tables are 25 and 26 columns
//! wide, and Postgres accepts at most 65535 bind parameters in one statement, so a batch insert
//! stops working somewhere around 2,500 rows however anyone feels about it. See
//! [`crate::settings::InstanceSettings`].
//!
//! ## Absent is not zero
//!
//! `current_metric` and `expected_metric` are stored exactly as they arrive, including absent. The
//! ordering above is where that matters most: an absolute — zero volume, zero success — reports no
//! expected value, and scoring it as `expected - current` with both defaulted to `0` would rank a
//! total outage as *no impact at all* and drop it first. [`Impact::Absolute`] ranks above every
//! measured gap instead.
//!
//! ## Whose clock
//!
//! `ts_alert` and `last_updated_at` are this service's, as everywhere else in this crate. Neither
//! column has a `DEFAULT` any more — `ts_alert` used to carry `CURRENT_TIMESTAMP` — so the handler
//! supplies both, along with `id_merchant_table`, which used to carry `gen_random_uuid()`, and
//! `is_visible`, which used to carry `TRUE`.

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

/// `name`, `product`, `merchant_id`, `dimension_key`, `priority` and `tenant_id` are all
/// `VARCHAR(64)`.
const NAME_MAX_BYTES: usize = 64;

/// `attribution`, `dimension_value` and `ts_slack` are all `VARCHAR(255)`.
const VALUE_MAX_BYTES: usize = 255;

/// The key a truncated write records itself under, inside `metadata_alert_details`.
///
/// Named for the ordering as well as the fact, so a row found in isolation says both that the
/// breakdown is partial and that what survived is the worst of it.
pub const TRUNCATION_KEY: &str = "truncated_by_impact";

/// Every instance recorded against one announcement.
pub async fn read_instances(
    state: AppState,
    channel: Channel,
    announcement: uuid::Uuid,
) -> ObservabilityApiResult<InstanceReadResponse> {
    let connection = state.database_connection().await?;

    // No check that the announcement itself exists: an announcement with no instances and an
    // announcement that was never made both have nothing to return, and the second is what a read
    // after the cascade correctly reports.
    let rows = store::list(&connection, channel, announcement)
        .await
        .change_context(ObservabilityError::StorageUnavailable)
        .attach_printable("Failed to read the merchant alert instances")?;

    Ok(InstanceReadResponse {
        status: found_or_absent(rows.len()),
        merchants: rows.into_iter().map(MerchantInstanceEntry::from).collect(),
    })
}

/// Replace every instance recorded against one announcement, in one transaction.
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

    // The connection handed to the closure is another handle to the one `connection` holds, so
    // every query issued through it below runs inside this transaction. Remove first, then write:
    // a write replaces what the announcement already carries.
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

/// The whole of one announcement's dimension breakdown.
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

/// Replace one announcement's dimension breakdown, in one transaction.
///
/// No channel: the breakdown table exists once and references `alerts_main`, so the announcement
/// is looked up there whatever channel it was delivered on.
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

/// The announcement a write hangs off, and the thread it opened.
///
/// Carried together because both come from the same lookup and both end up on every row: the
/// announcement as the `id` that cascades, and its thread as the `ts_slack` a caller did not send.
struct Parent {
    announcement: uuid::Uuid,
    thread: Option<String>,
}

/// What a write left behind.
struct Applied {
    stored: usize,
    removed: usize,
}

/// Whether a read found anything.
///
/// A store that could not be read never reaches here — that is a `503`, for the reason
/// [`super::lifecycle`] gives at length.
fn found_or_absent(rows: usize) -> ReadStatus {
    if rows == 0 {
        ReadStatus::Absent
    } else {
        ReadStatus::Found
    }
}

/// The rows one write will store, and what it dropped to get there.
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
                // Minted here: the column lost `gen_random_uuid()`, so without this every insert
                // would collide on the primary key.
                id_merchant_table: uuid::Uuid::now_v7(),
                id_intermediate: write.id_intermediate,
                name: write.name,
                product: write.product,
                merchant_id: write.merchant_id,
                dimensions: write.dimensions,
                auxiliary_dimensions: write.auxiliary_dimensions,
                // Stored exactly as they arrived, absent included. See the module docs.
                current_metric: write.current_metric,
                expected_metric: write.expected_metric,
                attribution: write.attribution,
                max_duration: write.max_duration,
                start_time: write.start_time,
                // The column lost `DEFAULT TRUE`, and a row nobody can see is not what a caller
                // that said nothing asked for.
                is_visible: Some(write.is_visible.unwrap_or(true)),
                recovered_ts: write.recovered_ts,
                // The announcement's thread when the caller sent none, and `null` when the
                // announcement has none either — an instance recorded before its announcement
                // reached a channel has no thread to point at.
                ts_slack: write.ts_slack.or_else(|| parent.thread.clone()),
                // The column lost `CURRENT_TIMESTAMP`; this service's clock replaces it.
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

/// The breakdown rows one write will store, and what it dropped to get there.
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

/// How badly one row is affected, as far as the two metrics can say.
///
/// Ordered, and the order is the whole reason this is a type rather than a subtraction: it decides
/// which rows survive a truncation.
#[derive(Debug, PartialEq)]
enum Impact {
    /// The detector reported an observation and expected nothing — an absolute, which fires only
    /// on zero volume or zero success. There is no gap to measure because *everything* is gone, so
    /// this outranks every measured one. Scoring it as a gap against a defaulted `0` would rank a
    /// total outage as unaffected and drop it first.
    Absolute,
    /// How far the observation is from what was expected.
    Gap(f64),
    /// Nothing was reported to compare. Last, because a row that says nothing is the one worth
    /// dropping when something has to be.
    Unknown,
}

/// What a pair of metrics says about one row.
fn impact(current: Option<f64>, expected: Option<f64>) -> Impact {
    match (current, expected) {
        (Some(current), Some(expected)) => Impact::Gap((expected - current).abs()),
        (Some(_), None) => Impact::Absolute,
        // An expectation with nothing observed against it measures nothing.
        (None, _) => Impact::Unknown,
    }
}

/// Order two rows worst first.
fn worst_first(left: &Impact, right: &Impact) -> Ordering {
    match (left, right) {
        (Impact::Absolute, Impact::Absolute) | (Impact::Unknown, Impact::Unknown) => {
            Ordering::Equal
        }
        (Impact::Absolute, _) | (Impact::Gap(_), Impact::Unknown) => Ordering::Less,
        (_, Impact::Absolute) | (Impact::Unknown, Impact::Gap(_)) => Ordering::Greater,
        // `total_cmp` rather than `partial_cmp`: a comparator that returns `Equal` for values it
        // cannot order makes the sort's result depend on the input order in ways nobody can
        // predict.
        (Impact::Gap(left), Impact::Gap(right)) => right.total_cmp(left),
    }
}

/// Cut a write down to the row cap, keeping the most impacted rows.
///
/// A write that fits is returned untouched and unsorted, so the caller's own order — which is
/// usually meaningful — survives when nothing has to be dropped. The sort is stable, so rows of
/// equal impact keep their relative order too and two runs over the same input keep the same rows.
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

/// The marker a truncated write leaves on every row it kept.
fn marker_for(truncation: &Truncation) -> serde_json::Value {
    serde_json::json!({
        "received": truncation.received,
        "stored": truncation.stored,
        "dropped": truncation.dropped,
    })
}

/// Add the marker to one row's `metadata_alert_details`, if there is one to add.
///
/// The caller's document is added to, never replaced: a service that quietly overwrote the field
/// it was handed would trade one silent loss for another.
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
        // Nothing to insert into, so the caller's value is kept beside the marker rather than
        // dropped. Callers send an object or nothing; this is what happens if one does not.
        Some(other) => Some(serde_json::json!({
            TRUNCATION_KEY: marker.clone(),
            "details": other,
        })),
        None => Some(serde_json::json!({ TRUNCATION_KEY: marker.clone() })),
    }
}

/// Check a value against its column's width.
///
/// An absent value is fine — no column here carries `NOT NULL`, so a `null` is a stored fact.
/// Checked before the query runs for the reason [`super::lifecycle`] checks its own: Postgres
/// rejects the same values as an opaque `22001` with a `500` attached, and one over-wide row would
/// fail the whole batch.
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

/// Why a write did not apply.
///
/// A type of its own because the transaction helper needs an error it can build from a diesel
/// failure. Both variants are the store failing rather than the request being wrong — the request
/// is checked before the transaction opens.
enum WriteFailure {
    /// A query failed.
    Storage(error_stack::Report<diesel_models::errors::DatabaseError>),
    /// The transaction itself failed to begin, commit or roll back.
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

/// The only code in this module that knows there are two channels.
///
/// Each function is one query behind a two-arm match. The models are generated per table because
/// diesel needs a struct per table; they take and return the channel-agnostic row, so nothing
/// above here is written twice.
mod store {
    use super::{
        report, slack_instance, slack_main, xyne_instance, xyne_main, AnnouncementRow, Channel,
        DatabaseConnectionWithContext, MerchantInstanceRow, ObservabilityApiResult,
        ObservabilityError, ResultExt, StorageResult,
    };

    /// The announcement a write hangs off, or [`ObservabilityError::UnknownAnnouncement`].
    ///
    /// Checked before anything is written rather than left to the foreign key, which would arrive
    /// as an opaque constraint failure naming neither the row nor the id — the same reason
    /// [`super::super::lifecycle`] checks its references up front. The same error as there, and
    /// deliberately not a `404`: it is the same condition, the id is one this service handed out,
    /// and a caller should not have two codes to branch on for it.
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

    /// One affected merchant, reporting the pair of metrics a test cares about.
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

    // -----------------------------------------------------------------------
    // The defaults these columns lost
    // -----------------------------------------------------------------------

    /// `id_merchant_table` lost `gen_random_uuid()`, so an unminted id would make every row after
    /// the first collide on the primary key — and the first one collide with the previous write.
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

    /// `ts_alert` lost `CURRENT_TIMESTAMP` and `last_updated_at` never had one. Both are this
    /// service's clock, and the same instant across the write.
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

    /// `is_visible` lost `DEFAULT TRUE`, and a row nobody can see is not what a caller that said
    /// nothing asked for. An explicit `false` still means false.
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

    /// The same three defaults, on the breakdown table.
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

    // -----------------------------------------------------------------------
    // The announcement, and the thread it opened
    // -----------------------------------------------------------------------

    /// Every row points at the announcement in the path, which is what makes the cascade the API's
    /// job rather than the caller's.
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

    /// The ticket's `ts_slack` edge case. A caller that sends none takes the announcement's, so
    /// the thread does not have to be carried around by hand.
    #[test]
    fn a_row_without_a_thread_takes_the_announcements() {
        let mut own = merchant("m2", None, None);
        own.ts_slack = Some("1757400000.999999".to_owned());

        let plan = InstancePlan::build(vec![merchant("m1", None, None), own], parent(), now(), 10)
            .unwrap();

        assert_eq!(plan.rows[0].ts_slack.as_deref(), Some("1757400000.000100"));
        assert_eq!(plan.rows[1].ts_slack.as_deref(), Some("1757400000.999999"));
    }

    /// The other half of it: an instance recorded before its announcement reached a channel has no
    /// thread anywhere, and stores `null` rather than being refused. The column is not `NOT NULL`
    /// any more, so that is a value it can hold.
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

    // -----------------------------------------------------------------------
    // Absent metrics
    // -----------------------------------------------------------------------

    /// The ticket's other edge case. "Observed 0, expected 0" reads as healthy, so an absolute has
    /// to be able to store an absent expectation rather than a zero standing in for one.
    #[test]
    fn an_absent_expected_metric_is_stored_absent_rather_than_as_zero() {
        let plan = InstancePlan::build(vec![merchant("m1", Some(0.0), None)], parent(), now(), 10)
            .unwrap();

        assert_eq!(plan.rows[0].current_metric, Some(0.0));
        assert!(plan.rows[0].expected_metric.is_none());
    }

    // -----------------------------------------------------------------------
    // The row cap
    // -----------------------------------------------------------------------

    /// A write that fits is stored whole, in the order it arrived, and reports no truncation at
    /// all rather than a truncation of nothing.
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

    /// The decision this resource turns on: the write is cut down, not refused, and what survives
    /// is the worst of it.
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

    /// The trap the whole ordering exists for. An absolute — zero volume, zero success — reports
    /// no expected value, so scoring it as `expected - current` with both defaulted to `0` would
    /// rank a total outage as *no impact* and drop it first, which is exactly the row a human
    /// needed to see.
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

    /// And a row that reported nothing to compare is the one worth dropping when something has to
    /// be.
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

    /// A gap is a distance, so a metric that overshot what was expected is as far out as one that
    /// fell as far short — a doubled refund rate is not a healthy row.
    #[test]
    fn impact_is_the_distance_from_what_was_expected_in_either_direction() {
        assert_eq!(impact(Some(1.0), Some(3.0)), Impact::Gap(2.0));
        assert_eq!(impact(Some(5.0), Some(3.0)), Impact::Gap(2.0));
        assert_eq!(impact(Some(0.0), None), Impact::Absolute);
        assert_eq!(impact(None, Some(90.0)), Impact::Unknown);
        assert_eq!(impact(None, None), Impact::Unknown);
    }

    /// Two runs over the same breakdown must keep the same rows, or a dimension would flicker in
    /// and out of the record between runs for no reason anybody could see.
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

    /// The other half of the decision: a cut that nothing downstream can see would make an outage
    /// look narrower than it was. Every row that survived says so, not just the response.
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

    /// A write that was stored whole leaves the caller's document exactly as it arrived.
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

    /// The marker is added to what the caller sent, never over it: trading one silent loss for
    /// another is not an improvement.
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

    /// And a caller that sent something that is not an object keeps it, beside the marker rather
    /// than under it.
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

    /// An empty write is a legitimate one — it clears what the announcement carried.
    #[test]
    fn a_write_carrying_nothing_stores_nothing_and_is_not_a_truncation() {
        let plan = InstancePlan::build(Vec::new(), parent(), now(), 10).unwrap();

        assert!(plan.rows.is_empty());
        assert!(plan.truncated.is_none());
    }

    // -----------------------------------------------------------------------
    // Column widths
    // -----------------------------------------------------------------------

    /// Postgres would reject these too, as a `22001` that arrives as an opaque failure with a
    /// `500` attached — and one over-wide row would take the whole batch with it.
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

    /// A row over the cap is still checked: the width check runs over everything the caller sent,
    /// before anything is dropped, so a request cannot smuggle an unstorable row in behind rows
    /// that will be cut.
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

    /// No column here is `NOT NULL`, so an absent value is a stored fact and not a short one.
    #[test]
    fn an_absent_value_is_not_measured_against_its_column() {
        let mut bare = merchant("m1", None, None);
        bare.name = None;
        bare.merchant_id = None;
        bare.priority = None;

        assert!(InstancePlan::build(vec![bare], parent(), now(), 10).is_ok());
    }
}
