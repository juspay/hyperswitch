use diesel_models::observability::{
    alerts_dicts::{AlertsDict, AlertsDictNew},
    raw_json::RawJson,
};
use error_stack::{report, ResultExt};

use crate::{
    alert_manager::types::{
        mappers::{
            MapperDeleteResponse, MapperEntry, MapperListResponse, MapperReadResponse,
            MapperSaveResponse, MapperUpsertRequest,
        },
        ReadStatus, UserName, WriteStatus,
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    state::AppState,
};

const NAME_MAX_BYTES: usize = 64;

const KEY_MAX_BYTES: usize = 255;

const USERNAME_MAX_BYTES: usize = 64;

pub async fn list(state: AppState) -> ObservabilityApiResult<MapperListResponse> {
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

pub async fn read(
    state: AppState,
    name: &str,
    key: &str,
) -> ObservabilityApiResult<MapperReadResponse> {
    let connection = state.database_connection().await?;

    let entry = AlertsDict::find_enabled_by_name_and_key(&connection, name, key)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to read a mapper entry")?;

    Ok(MapperReadResponse {
        status: entry
            .as_ref()
            .map_or(ReadStatus::Absent, |_| ReadStatus::Found),
        entry: entry.map(MapperEntry::from),
    })
}

pub async fn upsert(
    state: AppState,
    request: MapperUpsertRequest,
    user: UserName,
) -> ObservabilityApiResult<MapperSaveResponse> {
    let name = validated(&request.name, "name", NAME_MAX_BYTES)?;
    let key = validated(&request.key, "key", KEY_MAX_BYTES)?;
    let username = user.to_option();

    if let Some(username) = username.as_deref() {
        validated(username, "user name", USERNAME_MAX_BYTES)?;
    }

    let product = request.product.map(RawJson::from);
    let values = request.values.map(RawJson::from);
    let metadata = request.metadata.map(RawJson::from);
    within_cap(
        state.conf.mappers.max_entry_bytes,
        [product.as_ref(), values.as_ref(), metadata.as_ref()],
    )?;

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

pub async fn retire(
    state: AppState,
    name: &str,
    key: &str,
) -> ObservabilityApiResult<MapperDeleteResponse> {
    let connection = state.database_connection().await?;

    let retired = AlertsDict::retire(&connection, name, key)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to retire a mapper entry")?;

    Ok(MapperDeleteResponse {
        status: retired.map_or(WriteStatus::Absent, |_| WriteStatus::Retired),
    })
}

fn validated(value: &str, field: &'static str, max_bytes: usize) -> ObservabilityApiResult<String> {
    let value = value.trim();

    if value.is_empty() {
        Err(report!(ObservabilityError::InvalidRequest)
            .attach_printable(format!("The mapper {field} is empty")))?;
    }

    if value.len() > max_bytes {
        Err(
            report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                "The mapper {field} is {} bytes, over the {max_bytes} the column holds",
                value.len()
            )),
        )?;
    }

    Ok(value.to_owned())
}

fn within_cap(limit: usize, columns: [Option<&RawJson>; 3]) -> ObservabilityApiResult<()> {
    let bytes = columns
        .into_iter()
        .flatten()
        .map(RawJson::len)
        .sum::<usize>();

    if bytes > limit {
        logger::warn!(
            bytes = bytes,
            limit = limit,
            "Mapper entry rejected: over the configured size cap"
        );
        Err(report!(ObservabilityError::EntryTooLarge { bytes, limit }))?;
    }

    Ok(())
}
