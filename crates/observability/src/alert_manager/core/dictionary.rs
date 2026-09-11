//! Per-request logic for the mappers dictionary:

use diesel_models::observability::{
    alerts_dicts::{AlertsDict, AlertsDictNew},
    raw_json::RawJson,
};
use error_stack::{report, ResultExt};

use crate::{
    alert_manager::types::{
        dictionary::{
            DictionaryDeleteResponse, DictionaryEntry, DictionaryListResponse,
            DictionaryReadResponse, DictionarySaveResponse, DictionaryUpsertRequest,
        },
        ReadStatus, UserName, WriteStatus,
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    state::AppState,
};

/// `alerts_dicts.name` is `VARCHAR(64)`.
const NAME_MAX_BYTES: usize = 64;

/// `alerts_dicts.key_` is `VARCHAR(255)`.
const KEY_MAX_BYTES: usize = 255;

/// `alerts_dicts.username` is `VARCHAR(64)`.
const USERNAME_MAX_BYTES: usize = 64;

/// Every live entry.
pub async fn list(state: AppState) -> ObservabilityApiResult<DictionaryListResponse> {
    let connection = state.database_connection().await?;

    let entries = AlertsDict::list_enabled(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list dictionary entries")?;

    Ok(DictionaryListResponse {
        status: if entries.is_empty() {
            ReadStatus::Absent
        } else {
            ReadStatus::Found
        },
        entries: entries.into_iter().map(DictionaryEntry::from).collect(),
    })
}

/// One live entry, or the fact that there is none.
pub async fn read(
    state: AppState,
    name: &str,
    key: &str,
) -> ObservabilityApiResult<DictionaryReadResponse> {
    let connection = state.database_connection().await?;

    let entry = AlertsDict::find_enabled_by_name_and_key(&connection, name, key)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to read a dictionary entry")?;

    Ok(DictionaryReadResponse {
        status: entry
            .as_ref()
            .map_or(ReadStatus::Absent, |_| ReadStatus::Found),
        entry: entry.map(DictionaryEntry::from),
    })
}

/// Save an entry, replacing the live row for its key.
pub async fn upsert(
    state: AppState,
    request: DictionaryUpsertRequest,
    user: UserName,
) -> ObservabilityApiResult<DictionarySaveResponse> {
    let name = validated(&request.name, "name", NAME_MAX_BYTES)?;
    let key = validated(&request.key_, "key_", KEY_MAX_BYTES)?;
    let username = user.to_option();

    if let Some(username) = username.as_deref() {
        validated(username, "user name", USERNAME_MAX_BYTES)?;
    }

    let product = request.product.map(RawJson::from);
    let values = request.values_.map(RawJson::from);
    let metadata = request.metadata.map(RawJson::from);
    within_cap(
        state.conf.dictionary.max_entry_bytes,
        [product.as_ref(), values.as_ref(), metadata.as_ref()],
    )?;

    let connection = state.database_connection().await?;

    let entry = AlertsDictNew {
        name,
        key_: key,
        product,
        values_: values,
        // The service's clock, not the caller's:
        ts_created: common_utils::date_time::now(),
        username,
        metadata,
    }
    .upsert(&connection)
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to save a dictionary entry")?;

    Ok(DictionarySaveResponse {
        status: WriteStatus::Saved,
        entry: DictionaryEntry::from(entry),
    })
}

/// Retire an entry, keeping it as history.
pub async fn retire(
    state: AppState,
    name: &str,
    key: &str,
) -> ObservabilityApiResult<DictionaryDeleteResponse> {
    let connection = state.database_connection().await?;

    let retired = AlertsDict::retire(&connection, name, key)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to retire a dictionary entry")?;

    Ok(DictionaryDeleteResponse {
        status: retired.map_or(WriteStatus::Absent, |_| WriteStatus::Retired),
    })
}

/// Trim a required field and check it against its column's width.
fn validated(value: &str, field: &'static str, max_bytes: usize) -> ObservabilityApiResult<String> {
    let value = value.trim();

    if value.is_empty() {
        Err(report!(ObservabilityError::InvalidRequest)
            .attach_printable(format!("The dictionary {field} is empty")))?;
    }

    if value.len() > max_bytes {
        Err(
            report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                "The dictionary {field} is {} bytes, over the {max_bytes} the column holds",
                value.len()
            )),
        )?;
    }

    Ok(value.to_owned())
}

/// Reject an entry whose JSON is larger than the configured cap.
fn within_cap(limit: usize, columns: [Option<&RawJson>; 3]) -> ObservabilityApiResult<()> {
    let bytes = columns
        .into_iter()
        .flatten()
        .map(RawJson::len)
        .sum::<usize>();

    if bytes > limit {
        // Logged with the sizes, which the response deliberately does not carry:
        logger::warn!(
            bytes = bytes,
            limit = limit,
            "Dictionary entry rejected: over the configured size cap"
        );
        Err(report!(ObservabilityError::EntryTooLarge { bytes, limit }))?;
    }

    Ok(())
}
