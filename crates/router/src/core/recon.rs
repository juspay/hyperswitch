use api_models::recon as recon_api;
#[cfg(feature = "email")]
use common_utils::{ext_traits::AsyncExt, types::user::ThemeLineage};
use error_stack::ResultExt;
#[cfg(feature = "email")]
use masking::{ExposeInterface, PeekInterface, Secret};

#[cfg(feature = "email")]
use crate::{
    consts, services::email::types as email_types, types::domain, utils::user::theme as theme_utils,
};
use crate::{
    core::errors::{self, RouterResponse, UserErrors, UserResponse},
    services::{api as service_api, authentication},
    types::{
        api::{self as api_types, enums},
        storage,
        transformers::ForeignTryFrom,
    },
    utils::user as user_utils,
    SessionState,
};

#[allow(unused_variables)]
pub async fn send_recon_request(
    state: SessionState,
    auth_data: authentication::AuthenticationDataWithUser,
) -> RouterResponse<recon_api::ReconStatusResponse> {
    #[cfg(not(feature = "email"))]
    return Ok(service_api::ApplicationResponse::Json(
        recon_api::ReconStatusResponse {
            recon_status: enums::ReconStatus::NotRequested,
        },
    ));

    #[cfg(feature = "email")]
    {
        let user_in_db = &auth_data.user;
        let merchant_id = auth_data.merchant_account.get_id().clone();

        let theme = theme_utils::get_most_specific_theme_using_lineage(
            &state.clone(),
            ThemeLineage::Merchant {
                tenant_id: state.tenant.tenant_id.clone(),
                org_id: auth_data.merchant_account.get_org_id().clone(),
                merchant_id: merchant_id.clone(),
            },
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "Failed to fetch theme for merchant_id = {merchant_id:?}",
        ))?;

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
            subject: format!(
                "{} {}",
                consts::EMAIL_SUBJECT_DASHBOARD_FEATURE_REQUEST,
                user_email.expose().peek()
            ),
            theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
            theme_config: theme
                .map(|theme| theme.email_config())
                .unwrap_or(state.conf.theme.email_config.clone()),
        };
        state
            .email_client
            .compose_and_send_email(
                user_utils::get_base_url(&state),
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

                let response = db
                    .update_merchant(
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
}

pub async fn generate_recon_token(
    state: SessionState,
    user_with_role: authentication::UserFromTokenWithRoleInfo,
) -> RouterResponse<recon_api::ReconTokenResponse> {
    let user = user_with_role.user;
    let token = authentication::ReconToken::new_token(
        user.user_id.clone(),
        user.merchant_id.clone(),
        &state.conf,
        user.org_id.clone(),
        user.profile_id.clone(),
        user.tenant_id,
        user_with_role.role_info,
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

    let updated_merchant_account = storage::MerchantAccountUpdate::ReconUpdate {
        recon_status: req.recon_status,
    };
    let merchant_id = auth.merchant_account.get_id().clone();

    let updated_merchant_account = db
        .update_merchant(
            auth.merchant_account.clone(),
            updated_merchant_account,
            &auth.key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Failed while updating merchant's recon status: {merchant_id:?}")
        })?;

    #[cfg(feature = "email")]
    {
        let user_email = &req.user_email.clone();

        let theme = theme_utils::get_most_specific_theme_using_lineage(
            &state.clone(),
            ThemeLineage::Merchant {
                tenant_id: state.tenant.tenant_id.clone(),
                org_id: auth.merchant_account.get_org_id().clone(),
                merchant_id: merchant_id.clone(),
            },
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "Failed to fetch theme for merchant_id = {merchant_id:?}",
        ))?;

        let email_contents = email_types::ReconActivation {
            recipient_email: domain::UserEmail::from_pii_email(user_email.clone())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to convert recipient's email to UserEmail from pii::Email",
                )?,
            user_name: domain::UserName::new(Secret::new("HyperSwitch User".to_string()))
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to form username")?,
            subject: consts::EMAIL_SUBJECT_APPROVAL_RECON_REQUEST,
            theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
            theme_config: theme
                .map(|theme| theme.email_config())
                .unwrap_or(state.conf.theme.email_config.clone()),
        };
        if req.recon_status == enums::ReconStatus::Active {
            let _ = state
                .email_client
                .compose_and_send_email(
                    user_utils::get_base_url(&state),
                    Box::new(email_contents),
                    state.conf.proxy.https_url.as_ref(),
                )
                .await
                .inspect_err(|err| {
                    router_env::logger::error!(
                        "Failed to compose and send email notifying them of recon activation: {}",
                        err
                    )
                })
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to compose and send email for ReconActivation");
        }
    }

    Ok(service_api::ApplicationResponse::Json(
        api_types::MerchantAccountResponse::foreign_try_from(updated_merchant_account)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "merchant_account",
            })?,
    ))
}

pub async fn verify_recon_token(
    state: SessionState,
    user_with_role: authentication::UserFromTokenWithRoleInfo,
) -> UserResponse<recon_api::VerifyTokenResponse> {
    let user = user_with_role.user;
    let user_in_db = user
        .get_user_from_db(&state)
        .await
        .attach_printable_lazy(|| {
            format!(
                "Failed to fetch the user from DB for user_id - {}",
                user.user_id
            )
        })?;

    let acl = user_with_role.role_info.get_recon_acl();
    let optional_acl_str = serde_json::to_string(&acl)
        .inspect_err(|err| router_env::logger::error!("Failed to serialize acl to string: {}", err))
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to serialize acl to string. Using empty ACL")
        .ok();

    Ok(service_api::ApplicationResponse::Json(
        recon_api::VerifyTokenResponse {
            merchant_id: user.merchant_id.to_owned(),
            user_email: user_in_db.0.email,
            acl: optional_acl_str,
        },
    ))
}
