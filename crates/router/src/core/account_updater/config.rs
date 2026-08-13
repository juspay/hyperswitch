use common_utils::errors::CustomResult;
use error_stack::report;

use super::types::{
    AccountUpdaterCredentialSource, AccountUpdaterError, ResolvedAccountUpdaterConfig,
};
use crate::{
    core::configs::{self, dimension_config, dimension_state},
    routes::SessionState,
};

pub async fn resolve_account_updater_config<D>(
    state: &SessionState,
    dimensions: &D,
) -> CustomResult<Option<ResolvedAccountUpdaterConfig>, AccountUpdaterError>
where
    D: dimension_state::DimensionsBase,
{
    let store = state.store.as_ref();
    let superposition = state.superposition_service.as_ref();

    let enabled =
        configs::fetch_db_config_for_dimensions::<dimension_config::AccountUpdaterEnabled>(
            store,
            superposition,
            dimensions,
            None,
        )
        .await;

    let source = if enabled {
        configs::fetch_db_config_for_string_enum::<
            dimension_config::AccountUpdaterCredentialSource,
            AccountUpdaterCredentialSource,
        >(store, superposition, dimensions, None)
        .await
        .unwrap_or(AccountUpdaterCredentialSource::None)
    } else {
        AccountUpdaterCredentialSource::None
    };

    match source {
        AccountUpdaterCredentialSource::None => Ok(None),
        AccountUpdaterCredentialSource::Application => resolve_application_config(state).map(Some),
    }
}

fn resolve_application_config(
    state: &SessionState,
) -> CustomResult<ResolvedAccountUpdaterConfig, AccountUpdaterError> {
    state
        .conf
        .account_updater
        .as_ref()
        .map(|account_updater| ResolvedAccountUpdaterConfig::from(account_updater.get_inner()))
        .ok_or_else(|| {
            report!(AccountUpdaterError::MissingApplicationConfig).attach_printable(
                "Account Updater credential source is 'application' but the account_updater \
                 section is not configured",
            )
        })
}
