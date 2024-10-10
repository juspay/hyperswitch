use api_models::recon as recon_api;
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret};

use crate::{
    consts,
    core::errors::{self, RouterResponse, UserErrors},
    services::{api as service_api, authentication, email::types as email_types},
    types::{
        api::{self as api_types, enums},
        domain, storage,
        transformers::ForeignTryFrom,
    },
    SessionState,
};

pub async fn send_recon_request(
    state: SessionState,
    auth_data: authentication::AuthenticationDataWithUser,
) -> RouterResponse<recon_api::ReconStatusResponse> {
    let user_in_db = &auth_data.user;
    let merchant_id = auth_data.merchant_account.get_id().clone();

    let user_email = user_in_db.email.clone();
    let email_contents = email_types::ProFeatureRequest {
        feature_name: consts::RECON_FEATURE_TAG.to_string(),
        merchant_id: merchant_id.clone(),
        user_name: domain::UserName::new(user_in_db.name.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to form username")?,
        user_email: domain::UserEmail::from_pii_email(user_email.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert recipient's email to UserEmail")?,
        recipient_email: domain::UserEmail::from_pii_email(
            state.conf.email.recon_recipient_email.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert recipient's email to UserEmail")?,
        settings: state.conf.clone(),
        subject: format!(
            "{} {}",
            consts::EMAIL_SUBJECT_DASHBOARD_FEATURE_REQUEST,
            user_email.expose().peek()
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
            let db = &*state.store;
            let key_manager_state = &(&state).into();

            let response = db
                .update_merchant(
                    key_manager_state,
                    auth_data.merchant_account,
                    updated_merchant_account,
                    &auth_data.key_store,
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
    user: authentication::UserFromToken,
) -> RouterResponse<recon_api::ReconTokenResponse> {
    let token = authentication::AuthToken::new_token(
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
    auth: authentication::AuthenticationData,
    req: recon_api::ReconUpdateMerchantRequest,
) -> RouterResponse<api_types::MerchantAccountResponse> {
    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let updated_merchant_account = storage::MerchantAccountUpdate::ReconUpdate {
        recon_status: req.recon_status,
    };
    let merchant_id = auth.merchant_account.get_id().clone();

    let updated_merchant_account = db
        .update_merchant(
            key_manager_state,
            auth.merchant_account,
            updated_merchant_account,
            &auth.key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Failed while updating merchant's recon status: {merchant_id:?}")
        })?;

    let user_email = &req.user_email.clone();
    let email_contents = email_types::ReconActivation {
        recipient_email: domain::UserEmail::from_pii_email(user_email.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert recipient's email to UserEmail from pii::Email")?,
        user_name: domain::UserName::new(Secret::new("HyperSwitch User".to_string()))
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to form username")?,
        settings: state.conf.clone(),
        subject: consts::EMAIL_SUBJECT_APPROVAL_RECON_REQUEST,
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
        api_types::MerchantAccountResponse::foreign_try_from(updated_merchant_account)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "merchant_account",
            })?,
    ))
}
