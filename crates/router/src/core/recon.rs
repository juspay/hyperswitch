use api_models::recon as recon_api;
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
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
        domain::{UserEmail, UserName},
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
    let merchant_id = user.merchant_id;

    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch merchant's key store")?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(key_manager_state, &merchant_id, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch merchant's account")?;

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

    state
        .email_client
        .compose_and_send_email(
            Box::new(email_contents),
            state.conf.proxy.https_url.as_ref(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to compose and send email for ProFeatureRequest [Recon]")
        .async_and_then(|_| async {
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
        })
        .await
}

pub async fn generate_recon_token(
    state: SessionState,
    user: UserFromToken,
) -> RouterResponse<recon_api::ReconTokenResponse> {
    let token = AuthToken::new_token(
        user.user_id.clone(),
        user.merchant_id.clone(),
        user.role_id.clone(),
        &state.conf,
        user.org_id.clone(),
        user.profile_id.clone(),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "Failed to create recon token for params [user_id, org_id, mid, pid] [{}, {:?}, {:?}, {:?}]",
            user.user_id, user.org_id, user.merchant_id, user.profile_id,
        )
    })?;

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
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "merchant's key store not found".to_string(),
        })?;

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
