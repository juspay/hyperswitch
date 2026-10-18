use api_models::revenue_recovery_data_backfill::{
    CsvParsingError, RetryStatsMigrationCsvResult, RetryStatsMigrationResponse,
};
use futures::StreamExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    revenue_recovery::{
        retry_stats_cluster_key::RetryStatsClusterKey, retry_stats_document::StatsDocument,
    },
};
use router_env::logger;

use super::record;
use crate::{
    core::errors::RouterResult,
    routes::{app::SessionStateInfo, SessionState},
};

/// Batch-load pre-aggregated retry-stats documents from a CSV upload into
/// `revenue_recovery_retry_stats`.
pub async fn migrate_retry_stats_from_csv(
    state: SessionState,
    csv_result: RetryStatsMigrationCsvResult,
) -> RouterResult<ApplicationResponse<RetryStatsMigrationResponse>> {
    // Phase 1: validate & parse every row into a typed (cluster key, stats document)
    // pair BEFORE anything touches the database, so a single bad row can never leave a
    // partially migrated batch behind.
    let (ok_rows, failed_rows): (Vec<_>, Vec<_>) = csv_result
        .records
        .iter()
        .map(|(row_number, record)| {
            validate_row(record)
                .map(|(key, doc)| (*row_number, key, doc))
                .map_err(|error| CsvParsingError {
                    row_number: *row_number,
                    error,
                })
        })
        .partition(Result::is_ok);
    let validated_rows: Vec<(usize, RetryStatsClusterKey, StatsDocument)> =
        ok_rows.into_iter().flatten().collect();

    // Reject the whole batch if any cluster_key appears more than once.
    let duplicate_errors = validated_rows.iter().scan(
        std::collections::HashSet::new(),
        |seen_keys, (row_number, key, _doc)| {
            Some(
                (!seen_keys.insert(key.as_db_string())).then(|| CsvParsingError {
                    row_number: *row_number,
                    error: "duplicate cluster_key present".to_string(),
                }),
            )
        },
    );

    let validation_errors: Vec<CsvParsingError> = failed_rows
        .into_iter()
        .filter_map(Result::err)
        .chain(duplicate_errors.flatten())
        .collect();

    let response = if !validation_errors.is_empty() {
        logger::warn!(
            failed_records = validation_errors.len(),
            total_rows = csv_result.records.len(),
            "revenue_recovery_retry_stats_migration: batch rejected during validation; \
             nothing was persisted"
        );
        RetryStatsMigrationResponse {
            processed_records: 0,
            failed_records: validation_errors.len(),
            row_errors: validation_errors,
        }
    } else {
        // Phase 2: the whole batch is valid; persist every document sequentially.
        // Failures here are lock/DB errors, reported per row. Rerunning the file is
        // always safe because every write is a whole-document replace.
        let state = &state;
        let outcomes: Vec<Result<(), CsvParsingError>> = futures::stream::iter(&validated_rows)
            .then(|(row_number, key, doc)| async move {
                match record::replace_retry_stats_document(state, key, doc).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(CsvParsingError {
                        row_number: *row_number,
                        error: "per-key redis lock stayed contended; row skipped, retry the file"
                            .to_string(),
                    }),
                    Err(error) => Err(CsvParsingError {
                        row_number: *row_number,
                        error: format!("failed to persist stats document: {error:?}"),
                    }),
                }
            })
            .collect()
            .await;

        let row_errors: Vec<CsvParsingError> =
            outcomes.into_iter().filter_map(Result::err).collect();
        let processed_records = validated_rows.len() - row_errors.len();

        logger::info!(
            processed_records,
            failed_records = row_errors.len(),
            "revenue_recovery_retry_stats_migration: batch completed"
        );
        RetryStatsMigrationResponse {
            processed_records,
            failed_records: row_errors.len(),
            row_errors,
        }
    };

    Ok(ApplicationResponse::Json(response))
}

/// Parse and validate one migration row into its typed (cluster key, stats document)
/// pair.
fn validate_row(
    record: &api_models::revenue_recovery_data_backfill::RetryStatsMigrationRecord,
) -> Result<(RetryStatsClusterKey, StatsDocument), String> {
    let key = RetryStatsClusterKey::from_db_string(&record.cluster_key)
        .ok_or_else(|| format!("invalid cluster_key '{}'", record.cluster_key))?;

    let doc: StatsDocument = serde_json::from_str(&record.stats)
        .map_err(|error| format!("stats is not a valid stats document: {error}"))?;

    doc.validate_invariants()?;

    Ok((key, doc))
}
