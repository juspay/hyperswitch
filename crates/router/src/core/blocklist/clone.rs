//! Copying a business profile's blocklist entries onto its sibling profiles.
//!
//! A profile can hold a hundred thousand entries, so the copy runs as one background job whose
//! process tracker copies onto the target profiles one at a time, rather than inline.
use std::collections::HashSet;

use api_models::blocklist as api_blocklist;
use common_utils::{date_time, ext_traits::OptionExt, id_type};
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResult, StorageErrorExt},
    logger,
    routes::SessionState,
    types::{domain, storage},
};

const BLOCKLIST_CLONE_TASK: &str = "BLOCKLIST_PROFILE_CLONE";
const BLOCKLIST_CLONE_TAGS: [&str; 2] = ["BLOCKLIST", "CLONE"];
/// Matches the export's page size — both walk the same table the same way.
pub(crate) const CLONE_PAGE_SIZE: i64 = 5_000;

/// Loads the merchant's profiles once and checks the source and every target against them, rather
/// than reading each profile separately. Any profile outside the merchant fails the whole request.
async fn resolve_clone_targets(
    state: &SessionState,
    processor: &domain::Processor,
    source_profile_id: &id_type::ProfileId,
    target_profile_ids: &HashSet<id_type::ProfileId>,
) -> RouterResult<Vec<id_type::ProfileId>> {
    let merchant_profile_ids = state
        .store
        .list_profile_by_merchant_id(processor.get_key_store(), processor.get_account().get_id())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to list the merchant's business profiles")?
        .into_iter()
        .map(|profile| profile.get_id().clone())
        .collect::<HashSet<_>>();

    merchant_profile_ids
        .contains(source_profile_id)
        .then_some(())
        .ok_or(errors::ApiErrorResponse::ProfileNotFound {
            id: source_profile_id.get_string_repr().to_owned(),
        })?;

    target_profile_ids
        .iter()
        .map(|target_profile_id| -> RouterResult<id_type::ProfileId> {
            merchant_profile_ids
                .contains(target_profile_id)
                .then(|| target_profile_id.clone())
                .ok_or(
                    errors::ApiErrorResponse::ProfileNotFound {
                        id: target_profile_id.get_string_repr().to_owned(),
                    }
                    .into(),
                )
        })
        .collect()
}

/// Creates one job for the request, with a single process tracker that clones onto the targets in
/// order.
#[instrument(skip_all, fields(flow = ?router_env::Flow::CloneBlocklistEntries))]
pub async fn clone_blocklist_entries(
    state: &SessionState,
    platform: &domain::Platform,
    source_profile_id: Option<id_type::ProfileId>,
    request: api_blocklist::CloneBlocklistEntriesRequest,
) -> RouterResult<api_blocklist::CloneBlocklistEntriesResponse> {
    let processor = platform.get_processor();

    let source_profile_id = source_profile_id
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id".into(),
        })?;

    request
        .validate(&source_profile_id)
        .map_err(|message| errors::ApiErrorResponse::InvalidRequestData { message })?;

    let targets = resolve_clone_targets(
        state,
        processor,
        &source_profile_id,
        &request.target_profile_ids,
    )
    .await?;

    let job_id = enqueue_clone_job(state, platform, &source_profile_id, &targets).await?;

    Ok(api_blocklist::CloneBlocklistEntriesResponse {
        source_profile_id,
        job_id,
        status: common_enums::BatchBlocklistJobStatus::Initiated,
        target_profile_ids: targets,
    })
}

/// Creates the job row and the single process tracker that clones onto every target in order. The
/// job's generic row counters intentionally stay at zero; per-target status and progress live in
/// its metadata.
async fn enqueue_clone_job(
    state: &SessionState,
    platform: &domain::Platform,
    source_profile_id: &id_type::ProfileId,
    targets: &[id_type::ProfileId],
) -> RouterResult<String> {
    let processor_merchant_id = platform.get_processor().get_account().get_id();
    let job_id = common_utils::generate_id(crate::consts::ID_LENGTH, "blkclone");
    let now = date_time::now();
    let target_profile_ids = targets.to_vec();

    let metadata = storage::BlocklistProfileCloneJobMetadata {
        targets: target_profile_ids
            .iter()
            .map(|profile_id| storage::BlocklistProfileCloneTargetMetadata {
                profile_id: profile_id.clone(),
                status: common_enums::BatchBlocklistJobStatus::Initiated,
                processed_rows: 0,
                error_message: None,
            })
            .collect(),
    };

    let job_new = storage::BatchBlocklistJobNew {
        id: job_id.clone(),
        merchant_id: processor_merchant_id.clone(),
        status: common_enums::BatchBlocklistJobStatus::Initiated,
        total_rows: 0,
        succeeded_rows: 0,
        failed_rows: 0,
        created_at: now,
        updated_at: now,
        profile_id: source_profile_id.clone(),
        job_type: common_enums::BatchBlocklistJobType::ProfileClone,
        file_name: None,
        metadata: Some(metadata.into()),
    };

    state
        .store
        .insert_batch_blocklist_job(job_new)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::InternalServerError)?;

    let tracking_data = storage::BlocklistProfileCloneTrackingData {
        job_id: job_id.clone(),
        merchant_id: platform.get_provider().get_account().get_id().clone(),
        processor_merchant_id: Some(processor_merchant_id.clone()),
        source_profile_id: source_profile_id.clone(),
        target_profile_ids,
        current_target: 0,
        snapshot_at: now,
        draining_legacy_rows: false,
        last_fingerprint_id: String::new(),
        processed_rows: 0,
    };

    let process_tracker_entry = storage::ProcessTrackerNew::new(
        job_id.clone(),
        BLOCKLIST_CLONE_TASK,
        storage::ProcessTrackerRunner::BlocklistProfileCloneWorkflow,
        BLOCKLIST_CLONE_TAGS,
        tracking_data,
        None,
        now,
        common_types::consts::API_VERSION,
        common_enums::ApplicationSource::Main,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to create ProcessTrackerNew for a blocklist profile clone")?;

    match state.store.insert_process(process_tracker_entry).await {
        Ok(_) => {
            logger::info!(
                job_id = %job_id,
                source_profile_id = ?source_profile_id,
                target_count = targets.len(),
                "Blocklist profile clone job initiated"
            );
            Ok(job_id)
        }
        Err(error) => {
            state
                .store
                .update_batch_blocklist_job_by_id_merchant_id(
                    &job_id,
                    processor_merchant_id.get_string_repr(),
                    storage::BatchBlocklistJobUpdate {
                        status: Some(common_enums::BatchBlocklistJobStatus::Failed),
                        succeeded_rows: None,
                        failed_rows: None,
                        total_rows: None,
                        file_key: None,
                        error_message: Some("Failed to enqueue the clone process".to_string()),
                        expires_at: None,
                        metadata: None,
                        updated_at: date_time::now(),
                    },
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to mark an unenqueued blocklist profile clone job failed",
                )?;
            Err(error)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to enqueue the blocklist profile clone process")
        }
    }
}

/// Rewrites one source row as an insert against the target profile. `created_by` is carried over
/// rather than restamped with the cloning operator: it records who originally blocked the entry.
/// `created_at` is refreshed because listings are ordered by it.
pub(crate) fn to_target_entry(
    entry: storage::Blocklist,
    target_profile_id: &id_type::ProfileId,
    created_at: time::PrimitiveDateTime,
) -> storage::BlocklistNew {
    storage::BlocklistNew {
        merchant_id: entry.merchant_id,
        fingerprint_id: entry.fingerprint_id,
        data_kind: entry.data_kind,
        metadata: entry.metadata,
        created_at,
        processor_merchant_id: entry.processor_merchant_id,
        created_by: entry.created_by,
        profile_id: Some(target_profile_id.clone()),
    }
}
