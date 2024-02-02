use actix_web::{web, HttpRequest, HttpResponse};
use api_models::recon as recon_api;
use common_enums::ReconStatus;
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret};
use router_env::Flow;

use super::AppState;
use crate::{
    core::{
        api_locking,
        errors::{self, RouterResponse, RouterResult, StorageErrorExt, UserErrors},
    },
    services::{
        api as service_api, api,
        authentication::{self as auth, ReconUser, UserFromToken},
        email::types as email_types,
        recon::ReconToken,
    },
    types::{
        api::{self as api_types, enums},
        domain::{UserEmail, UserFromStorage, UserName},
        storage,
    },
};

/// Asynchronously handles the update merchant request by wrapping the call to recon_merchant_account_update
/// within the server_wrap function, and returning the HttpResponse.
pub async fn update_merchant(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<recon_api::ReconUpdateMerchantRequest>,
) -> HttpResponse {
    let flow = Flow::ReconMerchantUpdate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _user, req| recon_merchant_account_update(state, req),
        &auth::ReconAdmin,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Handles a request for reconnaissance by wrapping the request in a server process and awaiting the response.
pub async fn request_for_recon(state: web::Data<AppState>, http_req: HttpRequest) -> HttpResponse {
    let flow = Flow::ReconServiceRequest;
    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        (),
        |state, user: UserFromToken, _req| send_recon_request(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously handles a request to retrieve a reconnaissance token. This method
/// takes the application state and the HTTP request as input, and returns an HTTP
/// response. It creates a ReconTokenRequest flow, then uses a server wrap function
/// to generate a reconnaissance token for the given user using the app state and
/// request data. The generated token is then returned in the HTTP response.
pub async fn get_recon_token(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::ReconTokenRequest;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, user: ReconUser, _| generate_recon_token(state, user),
        &auth::ReconJWT,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously sends a reconciliation request for the given user's merchant account. It first retrieves the user's information from the database, including the associated merchant ID and key store. Then it composes and sends an email for a pro feature request related to reconciliation and settlement. If the email is successfully sent, it updates the merchant account's reconciliation status to "Requested" and returns the updated status. If the email fails to send, it returns a response with the reconciliation status as "NotRequested".
pub async fn send_recon_request(
    state: AppState,
    user: UserFromToken,
) -> RouterResponse<recon_api::ReconStatusResponse> {
    let db = &*state.store;
    let user_from_db = db
        .find_user_by_id(&user.user_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let merchant_id = db
        .find_user_role_by_user_id(&user.user_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?
        .merchant_id;
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            merchant_id.as_str(),
            &db.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    let merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id.as_str(), &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

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
            .update_merchant(merchant_account, updated_merchant_account, &key_store)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!("Failed while updating merchant's recon status: {merchant_id}")
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

/// Handles the update of a merchant account's recon status. Retrieves the merchant's key store
/// and account from the database, updates the recon status, sends an email notification if the
/// recon status is set to Active, and returns the updated merchant account response.
pub async fn recon_merchant_account_update(
    state: AppState,
    req: recon_api::ReconUpdateMerchantRequest,
) -> RouterResponse<api_types::MerchantAccountResponse> {
    let merchant_id = &req.merchant_id.clone();
    let user_email = &req.user_email.clone();

    let db = &*state.store;

    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            &req.merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let updated_merchant_account = storage::MerchantAccountUpdate::ReconUpdate {
        recon_status: req.recon_status,
    };

    let response = db
        .update_merchant(merchant_account, updated_merchant_account, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Failed while updating merchant's recon status: {merchant_id}")
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

    if req.recon_status == ReconStatus::Active {
        let _is_email_sent = state
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
        response
            .try_into()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "merchant_account",
            })?,
    ))
}

/// This method is used to generate a recon token for a given user using the provided state and request. It first retrieves the user from the database based on the user ID in the request. It then generates a recon authentication token for the user using the state. Finally, it returns the recon token as a JSON response.
pub async fn generate_recon_token(
    state: AppState,
    req: ReconUser,
) -> RouterResponse<recon_api::ReconTokenResponse> {
    let db = &*state.store;
    let user = db
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

    let token = Box::pin(get_recon_auth_token(user, state))
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(service_api::ApplicationResponse::Json(
        recon_api::ReconTokenResponse { token },
    ))
}

/// Asynchronously generates a recon authentication token for the given user using the provided application state configuration.
pub async fn get_recon_auth_token(
    user: UserFromStorage,
    state: AppState,
) -> RouterResult<Secret<String>> {
    ReconToken::new_token(user.0.user_id.clone(), &state.conf).await
}
