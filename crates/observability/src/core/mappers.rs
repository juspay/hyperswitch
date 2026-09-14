use diesel_models::observability::{
    alerts_dicts::{AlertsDict, AlertsDictNew},
    raw_json::RawJson,
};
use error_stack::{report, ResultExt};

use crate::{
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
    types::{
        mappers::{
            MapperEntry, MapperListResponse, MapperRetireResponse, MapperSaveResponse,
            MapperUpsertRequest,
        },
        ReadStatus, UserName, WriteStatus,
    },
};

const NAME_MAX_CHARS: usize = 64;

const KEY_MAX_CHARS: usize = 255;

const USERNAME_MAX_CHARS: usize = 64;

const MAX_ENTRY_BYTES: usize = 1024 * 1024;

pub async fn list_mappers(state: AppState) -> ObservabilityApiResult<MapperListResponse> {
    let connection = state.database_connection().await?;

    let entries = AlertsDict::list_enabled(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list mapper entries")?;

    Ok(MapperListResponse {
        status: if entries.is_empty() {
            ReadStatus::Absent
        } else {
            ReadStatus::Found
        },
        entries: entries.into_iter().map(MapperEntry::from).collect(),
    })
}

pub async fn read_mapper(
    state: AppState,
    name: &str,
    key: &str,
) -> ObservabilityApiResult<MapperEntry> {
    let connection = state.database_connection().await?;

    AlertsDict::find_enabled_by_name_and_key(&connection, name, key)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to read a mapper entry")?
        .map(MapperEntry::from)
        .ok_or_else(|| report!(ObservabilityError::MapperEntryNotFound))
}

pub async fn upsert_mapper(
    state: AppState,
    request: MapperUpsertRequest,
    user: UserName,
) -> ObservabilityApiResult<MapperSaveResponse> {
    let name = trimmed_within(&request.name, "name", NAME_MAX_CHARS)?;
    let key = trimmed_within(&request.key, "key", KEY_MAX_CHARS)?;
    let username = user.to_option();

    if let Some(username) = username.as_deref() {
        trimmed_within(username, "user name", USERNAME_MAX_CHARS)?;
    }

    let product = request.product.map(RawJson::from);
    let values = request.values.map(RawJson::from);
    let metadata = request.metadata.map(RawJson::from);
    within_entry_cap([product.as_ref(), values.as_ref(), metadata.as_ref()])?;

    let connection = state.database_connection().await?;

    let entry = AlertsDictNew {
        name,
        key_: key,
        product,
        values_: values,
        ts_created: common_utils::date_time::now(),
        username,
        metadata,
    }
    .upsert(&connection)
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to save a mapper entry")?;

    Ok(MapperSaveResponse {
        status: WriteStatus::Saved,
        entry: MapperEntry::from(entry),
    })
}

pub async fn retire_mapper(
    state: AppState,
    name: &str,
    key: &str,
) -> ObservabilityApiResult<MapperRetireResponse> {
    let connection = state.database_connection().await?;

    AlertsDict::retire(&connection, name, key)
        .await
        .to_not_found_response(ObservabilityError::MapperEntryNotFound)
        .attach_printable("Failed to retire a mapper entry")?;

    Ok(MapperRetireResponse {
        status: WriteStatus::Retired,
    })
}

fn trimmed_within(
    value: &str,
    field: &'static str,
    max_chars: usize,
) -> ObservabilityApiResult<String> {
    let value = value.trim();

    if value.is_empty() {
        Err(report!(ObservabilityError::InvalidRequest)
            .attach_printable(format!("The mapper {field} is empty")))?;
    }

    let chars = value.chars().count();
    if chars > max_chars {
        Err(
            report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                "The mapper {field} is {chars} characters, over the {max_chars} the column holds"
            )),
        )?;
    }

    Ok(value.to_owned())
}

fn within_entry_cap(columns: [Option<&RawJson>; 3]) -> ObservabilityApiResult<()> {
    let bytes = columns
        .into_iter()
        .flatten()
        .map(RawJson::len)
        .sum::<usize>();

    if bytes > MAX_ENTRY_BYTES {
        Err(report!(ObservabilityError::EntryTooLarge {
            bytes,
            limit: MAX_ENTRY_BYTES,
        }))?;
    }

    Ok(())
}
