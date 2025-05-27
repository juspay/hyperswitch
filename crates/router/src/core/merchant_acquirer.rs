use api_models::merchant_acquirer;
use common_utils::{date_time, types::keymanager::KeyManagerState};
use diesel_models::merchant_acquirer::{MerchantAcquirer, MerchantAcquirerNew};
use error_stack::ResultExt;

use crate::{
    core::errors::{self, utils::StorageErrorExt, RouterResponse},
    services::api,
    types::domain,
    SessionState,
};

fn to_api_response(db_acquirer: MerchantAcquirer) -> merchant_acquirer::MerchantAcquirerResponse {
    merchant_acquirer::MerchantAcquirerResponse {
        merchant_acquirer_id: db_acquirer.merchant_acquirer_id,
        acquirer_assigned_merchant_id: db_acquirer.acquirer_assigned_merchant_id,
        merchant_name: db_acquirer.merchant_name,
        mcc: db_acquirer.mcc,
        merchant_country_code: db_acquirer.merchant_country_code,
        network: db_acquirer.network,
        acquirer_bin: db_acquirer.acquirer_bin,
        acquirer_ica: db_acquirer.acquirer_ica,
        acquirer_fraud_rate: db_acquirer.acquirer_fraud_rate,
        profile_id: db_acquirer.profile_id,
        created_at: db_acquirer.created_at,
    }
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn create_merchant_acquirer(
    state: SessionState,
    request: merchant_acquirer::MerchantAcquirerCreate,
    merchant_context: domain::MerchantContext,
) -> RouterResponse<merchant_acquirer::MerchantAcquirerResponse> {
    let db = state.store.as_ref();
    let now = date_time::now();
    let merchant_acquirer_id = common_utils::generate_merchant_acquirer_id_of_default_length();
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

    let existing_merchant_acquirer = db
        .list_merchant_acquirer_based_on_profile_id(business_profile.get_id())
        .await
        .ok();

    existing_merchant_acquirer
        .map(|merchant_acquirers| {
            if has_duplicate_merchant_acquirer(&request, &merchant_acquirers) {
                Err(error_stack::Report::from(
                    errors::ApiErrorResponse::GenericDuplicateError {
                        message: format!(
                            "Merchant acquirer configuration with id {} already exists.",
                            merchant_acquirer_id.get_string_repr()
                        ),
                    },
                ))
            } else {
                Ok(())
            }
        })
        .transpose()?;

    let new_acquirer_entry = MerchantAcquirerNew {
        merchant_acquirer_id: merchant_acquirer_id.clone(),
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
        .insert_merchant_acquirer(new_acquirer_entry)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: format!(
                "Merchant acquirer with id {} already exists.",
                merchant_acquirer_id.get_string_repr()
            ),
        })?;

    let updated_acquirer_list = match business_profile.merchant_acquirer_ids.clone() {
        Some(mut ids) => {
            ids.push(merchant_acquirer_id);
            ids
        }
        None => vec![merchant_acquirer_id],
    };

    let profile_update = domain::ProfileUpdate::MerchantAcquirerUpdate {
        merchant_acquirer_ids: Some(updated_acquirer_list),
    };

    db.update_profile_by_profile_id(
        &key_manager_state,
        merchant_key_store,
        business_profile,
        profile_update,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update merchant acquirer ids in business profile")?;

    Ok(api::ApplicationResponse::Json(to_api_response(
        created_acquirer,
    )))
}

fn has_duplicate_merchant_acquirer(
    request: &merchant_acquirer::MerchantAcquirerCreate,
    existing_acquirers: &Vec<MerchantAcquirer>,
) -> bool {
    for acquirer in existing_acquirers {
        if acquirer.acquirer_assigned_merchant_id == request.acquirer_assigned_merchant_id
            && acquirer.merchant_name == request.merchant_name
            && acquirer.mcc == request.mcc
            && acquirer.merchant_country_code == request.merchant_country_code
            && acquirer.network == request.network
            && acquirer.acquirer_bin == request.acquirer_bin
            && acquirer.acquirer_ica == request.acquirer_ica
        {
            return true;
        }
    }
    false
}
