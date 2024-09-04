use api_models::recon as recon_api;
use diesel_models::enums::UserRoleVersion;
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt, UserErrors},
    services::{
        api as service_api,
        authentication::{AuthToken, UserFromToken},
        email::types as email_types,
    },
    types::{
        api::{self as api_types, enums},
        domain::{UserEmail, UserFromStorage, UserName},
        storage,
        transformers::ForeignTryFrom,
    },
    SessionState,
};

pub async fn send_recon_request(
    state: SessionState,
    user: UserFromToken,
) -> RouterResponse<recon_api::ReconStatusResponse> {
    let global_db = &*state.global_store;
    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let user_role = db
        .find_user_role_by_user_id_and_lineage(
            &user.user_id,
            &user.org_id,
            &user.merchant_id,
            user.profile_id.as_ref(),
            UserRoleVersion::V1,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    let merchant_id = user_role
        .merchant_id
        .ok_or(errors::ApiErrorResponse::InternalServerError)?;
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // Ensure the merchant in user role is the parent of the business profile
    match user.profile_id {
        Some(profile_id) => {
            let business_profile = db
                .find_business_profile_by_profile_id(key_manager_state, &key_store, &profile_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

            if user.merchant_id != merchant_id || user.merchant_id != business_profile.merchant_id {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Profile id authentication failed. Please use the correct JWT token"
                        .to_string(),
                }))
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(key_manager_state, &merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let user_from_db = global_db
        .find_user_by_id(&user.user_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let email_contents = email_types::ProFeatureRequest {
        feature_name: "RECONCILIATION & SETTLEMENT".to_string(),
        merchant_id: merchant_id.clone(),
        user_name: UserName::new(user_from_db.name)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to form username")?,
        recipient_email: UserEmail::new(Secret::new("biz@hyperswitch.io".to_string()))
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert recipient's email to UserEmail")?,
        settings: state.conf.clone(),
        subject: format!(
            "Dashboard Pro Feature Request by {}",
            user_from_db.email.expose().peek()
        ),
    };

    let is_email_sent = state
        .email_client
        .compose_and_send_email(
            Box::new(email_contents),
            state.conf.proxy.https_url.as_ref(),
        )
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to compose and send email for ProFeatureRequest")
        .is_ok();

    if is_email_sent {
        let updated_merchant_account = storage::MerchantAccountUpdate::ReconUpdate {
            recon_status: enums::ReconStatus::Requested,
        };

        let response = db
            .update_merchant(
                key_manager_state,
                merchant_account,
                updated_merchant_account,
                &key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!("Failed while updating merchant's recon status: {merchant_id:?}")
            })?;

        Ok(service_api::ApplicationResponse::Json(
            recon_api::ReconStatusResponse {
                recon_status: response.recon_status,
            },
        ))
    } else {
        Ok(service_api::ApplicationResponse::Json(
            recon_api::ReconStatusResponse {
                recon_status: enums::ReconStatus::NotRequested,
            },
        ))
    }
}

pub async fn generate_recon_token(
    state: SessionState,
    req: UserFromToken,
) -> RouterResponse<recon_api::ReconTokenResponse> {
    let global_db = &*state.global_store;
    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let user: UserFromStorage = global_db
        .find_user_by_id(&req.user_id)
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(errors::ApiErrorResponse::InvalidJwtToken)
            } else {
                e.change_context(errors::ApiErrorResponse::InternalServerError)
            }
        })?
        .into();
    let user_role = db
        .find_user_role_by_user_id_and_lineage(
            &user.0.user_id,
            &req.org_id,
            &req.merchant_id,
            req.profile_id.as_ref(),
            UserRoleVersion::V1,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_id = user_role
        .merchant_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)?;
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // Ensure the merchant in user role is the parent of the business profile
    match req.profile_id {
        Some(ref profile_id) => {
            let business_profile = db
                .find_business_profile_by_profile_id(key_manager_state, &key_store, profile_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

            if req.merchant_id != merchant_id || req.merchant_id != business_profile.merchant_id {
                Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Profile id authentication failed. Please use the correct JWT token"
                        .to_string(),
                }))
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }?;

    let token = AuthToken::new_token(
        req.user_id,
        req.merchant_id,
        req.role_id,
        &state.conf,
        req.org_id,
        req.profile_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(service_api::ApplicationResponse::Json(
        recon_api::ReconTokenResponse {
            token: token.into(),
        },
    ))
}

pub async fn recon_merchant_account_update(
    state: SessionState,
    req: recon_api::ReconUpdateMerchantRequest,
) -> RouterResponse<api_types::MerchantAccountResponse> {
    let merchant_id = &req.merchant_id.clone();
    let user_email = &req.user_email.clone();
    let db = &*state.store;
    let key_manager_state = &(&state).into();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &req.merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(key_manager_state, merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let updated_merchant_account = storage::MerchantAccountUpdate::ReconUpdate {
        recon_status: req.recon_status,
    };

    let response = db
        .update_merchant(
            key_manager_state,
            merchant_account,
            updated_merchant_account,
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Failed while updating merchant's recon status: {merchant_id:?}")
        })?;

    let email_contents = email_types::ReconActivation {
        recipient_email: UserEmail::from_pii_email(user_email.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert recipient's email to UserEmail from pii::Email")?,
        user_name: UserName::new(Secret::new("HyperSwitch User".to_string()))
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to form username")?,
        settings: state.conf.clone(),
        subject: "Approval of Recon Request - Access Granted to Recon Dashboard",
    };

    if req.recon_status == enums::ReconStatus::Active {
        let _ = state
            .email_client
            .compose_and_send_email(
                Box::new(email_contents),
                state.conf.proxy.https_url.as_ref(),
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to compose and send email for ReconActivation")
            .is_ok();
    }

    Ok(service_api::ApplicationResponse::Json(
        api_types::MerchantAccountResponse::foreign_try_from(response).change_context(
            errors::ApiErrorResponse::InvalidDataValue {
                field_name: "merchant_account",
            },
        )?,
    ))
}
