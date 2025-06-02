use api_models::profile_acquirer;
use common_utils::{date_time, types::keymanager::KeyManagerState};
use diesel_models::profile_acquirer::{ProfileAcquirer, ProfileAcquirerNew};
use error_stack::ResultExt;

use crate::{
    core::errors::{self, utils::StorageErrorExt, RouterResponse},
    services::api,
    types::{domain, transformers::ForeignFrom},
    SessionState,
};

impl ForeignFrom<ProfileAcquirer> for profile_acquirer::ProfileAcquirerResponse {
    fn foreign_from(db_acquirer: ProfileAcquirer) -> Self {
        Self {
            profile_acquirer_id: db_acquirer.profile_acquirer_id,
            acquirer_assigned_merchant_id: db_acquirer.acquirer_assigned_merchant_id,
            merchant_name: db_acquirer.merchant_name,
            mcc: db_acquirer.mcc,
            merchant_country_code: db_acquirer.merchant_country_code,
            network: db_acquirer.network,
            acquirer_bin: db_acquirer.acquirer_bin,
            acquirer_ica: db_acquirer.acquirer_ica,
            acquirer_fraud_rate: db_acquirer.acquirer_fraud_rate,
            profile_id: db_acquirer.profile_id,
        }
    }
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn create_profile_acquirer(
    state: SessionState,
    request: profile_acquirer::ProfileAcquirerCreate,
    merchant_context: domain::MerchantContext,
) -> RouterResponse<profile_acquirer::ProfileAcquirerResponse> {
    let db = state.store.as_ref();
    let now = date_time::now();
    let profile_acquirer_id = common_utils::generate_profile_acquirer_id_of_default_length();
    let key_manager_state: KeyManagerState = (&state).into();
    let merchant_key_store = merchant_context.get_merchant_key_store();

    let business_profile = db
        .find_business_profile_by_profile_id(
            &key_manager_state,
            merchant_key_store,
            &request.profile_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: request.profile_id.get_string_repr().to_owned(),
        })?;

    let existing_profile_acquirers = db
        .list_profile_acquirer_based_on_profile_id(business_profile.get_id())
        .await
        .inspect_err(|error| {
            router_env::logger::error!("Failed to list profile acquirers: {:?}", error);
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    (!existing_profile_acquirers.is_empty())
        .then(|| {
            has_duplicate_profile_acquirer(
                &request,
                &existing_profile_acquirers,
                &profile_acquirer_id,
            )
        })
        .transpose()?;

    let new_acquirer_entry = ProfileAcquirerNew {
        profile_acquirer_id: profile_acquirer_id.clone(),
        acquirer_assigned_merchant_id: request.acquirer_assigned_merchant_id,
        merchant_name: request.merchant_name,
        mcc: request.mcc,
        merchant_country_code: request.merchant_country_code,
        network: request.network,
        acquirer_bin: request.acquirer_bin,
        acquirer_ica: request.acquirer_ica,
        acquirer_fraud_rate: request.acquirer_fraud_rate,
        created_at: Some(now),
        last_modified_at: Some(now),
        profile_id: business_profile.get_id().clone(),
    };

    let created_acquirer = db
        .insert_profile_acquirer(new_acquirer_entry)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: format!(
                "Profile acquirer with id {} already exists.",
                profile_acquirer_id.get_string_repr()
            ),
        })?;

    Ok(api::ApplicationResponse::Json(
        profile_acquirer::ProfileAcquirerResponse::foreign_from(created_acquirer),
    ))
}

fn has_duplicate_profile_acquirer(
    request: &profile_acquirer::ProfileAcquirerCreate,
    existing_acquirers: &Vec<ProfileAcquirer>,
    profile_acquirer_id: &common_utils::id_type::ProfileAcquirerId,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    for acquirer in existing_acquirers {
        if acquirer.acquirer_assigned_merchant_id == request.acquirer_assigned_merchant_id
            && acquirer.merchant_name == request.merchant_name
            && acquirer.mcc == request.mcc
            && acquirer.merchant_country_code == request.merchant_country_code
            && acquirer.network == request.network
            && acquirer.acquirer_bin == request.acquirer_bin
            && acquirer.acquirer_ica == request.acquirer_ica
        {
            return Err(error_stack::Report::from(
                errors::ApiErrorResponse::GenericDuplicateError {
                    message: format!(
                        "Profile acquirer configuration with id {} already exists.",
                        profile_acquirer_id.get_string_repr()
                    ),
                },
            ));
        }
    }
    Ok(())
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn list_merchant_acquirers(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<Vec<profile_acquirer::ProfileAcquirerResponse>> {
    let db = state.store.as_ref();
    let key_manager_state: KeyManagerState = (&state).into();
    let merchant_key_store = merchant_context.get_merchant_key_store();

    let business_profile = db
        .find_business_profile_by_profile_id(&key_manager_state, merchant_key_store, &profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let profile_acquirers = db
        .list_profile_acquirer_based_on_profile_id(business_profile.get_id())
        .await
        .inspect_err(|error| {
            router_env::logger::error!("Failed to list profile acquirers: {:?}", error);
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let response = api::ApplicationResponse::Json(
        profile_acquirers
            .into_iter()
            .map(profile_acquirer::ProfileAcquirerResponse::foreign_from)
            .collect(),
    );
    Ok(response)
}
