use async_bb8_diesel::AsyncConnection;
use common_utils::errors::ErrorSwitchFrom;
use diesel_models::{
    errors::DatabaseError,
    observability::alerts_dicts::{AlertsDict, AlertsDictUpdate},
};
use error_stack::{report, ResultExt};

use crate::{
    auth::UserName,
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
    types::mappers::{
        MapperEntryDeleteResponse, MapperEntryListResponse, MapperEntryResponse,
        MapperEntrySaveRequest,
    },
};

const SUPERSEDED_ENTRIES_KEPT: i64 = 1;

pub async fn list_mappers(state: AppState) -> ObservabilityApiResult<MapperEntryListResponse> {
    let connection = state.database_connection().await?;

    let entries = AlertsDict::list_enabled(&connection)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list mapper entries")?
        .into_iter()
        .map(MapperEntryResponse::from)
        .collect::<Vec<_>>();

    Ok(MapperEntryListResponse {
        count: entries.len(),
        entries,
    })
}

pub async fn read_mapper(
    state: AppState,
    name: String,
    key: String,
) -> ObservabilityApiResult<MapperEntryResponse> {
    let connection = state.database_connection().await?;

    AlertsDict::find_enabled_by_name_and_key(&connection, &name, &key)
        .await
        .to_not_found_response(ObservabilityError::MapperEntryNotFound)
        .attach_printable("Failed to find a mapper entry")
        .map(MapperEntryResponse::from)
}

pub async fn save_mapper(
    state: AppState,
    request: MapperEntrySaveRequest,
    user_name: Option<UserName>,
) -> ObservabilityApiResult<MapperEntryResponse> {
    request.validate()?;

    let connection = state.database_connection().await?;
    let entry = request.into_insertable(
        common_utils::generate_uuid_v7(),
        user_name,
        common_utils::date_time::now(),
    );
    let name = entry.name.clone();
    let key = entry.key_.clone();

    let borrowed = &connection;
    borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            AlertsDict::lock_by_name_and_key(borrowed, &name, &key).await?;

            AlertsDict::update_enabled_by_name_and_key(
                borrowed,
                &name,
                &key,
                AlertsDictUpdate::Demote,
            )
            .await?;

            let saved = entry.insert(borrowed).await?;

            AlertsDict::delete_superseded_by_name_and_key(
                borrowed,
                &name,
                &key,
                SUPERSEDED_ENTRIES_KEPT,
            )
            .await?;

            Ok::<_, TransactionFailure>(saved)
        })
        .await
        .map_err(|TransactionFailure(error)| error)
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to save a mapper entry")
        .map(MapperEntryResponse::from)
}

pub async fn delete_mapper(
    state: AppState,
    name: String,
    key: String,
) -> ObservabilityApiResult<MapperEntryDeleteResponse> {
    let connection = state.database_connection().await?;

    let borrowed = &connection;
    let (entry_name, entry_key) = (&name, &key);
    let deleted = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            AlertsDict::lock_by_name_and_key(borrowed, entry_name, entry_key).await?;

            AlertsDict::delete_enabled_by_name_and_key(borrowed, entry_name, entry_key)
                .await
                .map_err(TransactionFailure::from)
        })
        .await
        .map_err(|TransactionFailure(error)| error)
        .to_not_found_response(ObservabilityError::MapperEntryNotFound)
        .attach_printable("Failed to delete a mapper entry")?;

    Ok(MapperEntryDeleteResponse { name, key, deleted })
}

struct TransactionFailure(error_stack::Report<DatabaseError>);

impl From<diesel::result::Error> for TransactionFailure {
    fn from(error: diesel::result::Error) -> Self {
        let database_error = DatabaseError::switch_from(&error);
        Self(report!(error).change_context(database_error))
    }
}

impl From<error_stack::Report<DatabaseError>> for TransactionFailure {
    fn from(error: error_stack::Report<DatabaseError>) -> Self {
        Self(error)
    }
}
