use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "dummy_connector")]
use api_models::user::sample_data::SampleDataRequest;
use api_models::{
    errors::types::ApiErrorResponse,
    user::{self as user_api},
};
use common_utils::errors::ReportSwitchExt;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, user as user_core},
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
    utils::user::dashboard_metadata::{parse_string_to_enums, set_ip_address_if_required},
};

#[cfg(feature = "email")]
/// Handles user signup with a merchant ID request and returns an HTTP response.
pub async fn user_signup_with_merchant_id(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SignUpWithMerchantIdRequest>,
) -> HttpResponse {
    let flow = Flow::UserSignUpWithMerchantId;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body| user_core::signup_with_merchant_id(state, req_body),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Handles the user signup process by extracting the request payload from the provided JSON, creating a user signup flow, and then passing the flow, app state, HTTP request, request payload, and various other parameters to the server_wrap function. The server_wrap function handles the actual signup process by calling the signup function from the user_core module and applying the NoAuth authentication method with NotApplicable locking action.
pub async fn user_signup(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SignUpRequest>,
) -> HttpResponse {
    let flow = Flow::UserSignUp;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body| user_core::signup(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Handles the user sign-in process without performing invite checks. This method wraps the sign-in request in a server context, and then calls the `signin_without_invite_checks` method from the user core module. This method does not require authentication and does not involve any locking action.
pub async fn user_signin_without_invite_checks(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SignInRequest>,
) -> HttpResponse {
    let flow = Flow::UserSignInWithoutInviteChecks;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body| user_core::signin_without_invite_checks(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Handles the user sign-in process by taking in the App state, HTTP request, and JSON payload,
/// and then asynchronously wraps the sign-in request using the server_wrap function from the api module.
pub async fn user_signin(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SignInRequest>,
) -> HttpResponse {
    let flow = Flow::UserSignIn;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body| user_core::signin(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
/// This method handles the user's request to connect an account by wrapping the request in a server_wrap function and executing it asynchronously. It creates a flow for the user connection account operation, extracts the request payload from the JSON body, and then passes it along with the state and HTTP request to the user_core::connect_account function. The authentication method used is NoAuth, and the method does not apply any locking action. The result of the server_wrap function is awaited and returned as an HttpResponse.
pub async fn user_connect_account(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::ConnectAccountRequest>,
) -> HttpResponse {
    let flow = Flow::UserConnectAccount;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body| user_core::connect_account(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Handles the signout flow for the user. It takes the current application state and the HTTP request as input, and uses the server_wrap function to wrap the signout logic in an asynchronous context. It then waits for the result using the await keyword and returns the HttpResponse.
pub async fn signout(state: web::Data<AppState>, http_req: HttpRequest) -> HttpResponse {
    let flow = Flow::Signout;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        (),
        |state, user, _| user_core::signout(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously handles a request to change a user's password by calling the `change_password` function from the `user_core` module wrapped in the `api::server_wrap` function. The `change_password` function requires the `state`, `req`, and `user` as parameters, and the `api::server_wrap` function handles the authentication, authorization, and locking aspects of the request. The method returns an `HttpResponse` indicating the success or failure of the password change request.
pub async fn change_password(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::ChangePasswordRequest>,
) -> HttpResponse {
    let flow = Flow::ChangePassword;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, user, req| user_core::change_password(state, req, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously sets the metadata for a dashboard using the provided JSON payload and request information. This method first creates a flow for setting dashboard metadata, then extracts the payload from the JSON request. It then checks for any errors in setting the IP address using the request headers. If an error is found, it returns an error response. Otherwise, it wraps the flow, state, request, payload, and authentication information in a server response and awaits the result.
pub async fn set_dashboard_metadata(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::dashboard_metadata::SetMetaDataRequest>,
) -> HttpResponse {
    let flow = Flow::SetDashboardMetadata;
    let mut payload = json_payload.into_inner();

    if let Err(e) = common_utils::errors::ReportSwitchExt::<(), ApiErrorResponse>::switch(
        set_ip_address_if_required(&mut payload, req.headers()),
    ) {
        return api::log_and_return_error_response(e);
    }

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        user_core::dashboard_metadata::set_metadata,
        &auth::JWTAuth(Permission::MerchantAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Retrieves multiple dashboard metadata based on the provided request.
pub async fn get_multiple_dashboard_metadata(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<user_api::dashboard_metadata::GetMultipleMetaDataRequest>,
) -> HttpResponse {
    let flow = Flow::GetMutltipleDashboardMetadata;
    let payload = match ReportSwitchExt::<_, ApiErrorResponse>::switch(parse_string_to_enums(
        query.into_inner().keys,
    )) {
        Ok(payload) => payload,
        Err(e) => {
            return api::log_and_return_error_response(e);
        }
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        user_core::dashboard_metadata::get_multiple_metadata,
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Handles the signup process for internal users. This method takes the application state, HTTP request, and JSON payload as input, and then uses the server_wrap function to handle the internal user signup flow. It creates a new internal user using the user_core::create_internal_user method and admin API authentication. Finally, it awaits the result and returns an HTTP response.
pub async fn internal_user_signup(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::CreateInternalUserRequest>,
) -> HttpResponse {
    let flow = Flow::InternalUserSignup;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, _, req| user_core::create_internal_user(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously handles the switching of merchant ID by taking in the application state, HTTP request,
/// and a JSON payload containing the switch merchant ID request. It then creates a flow for switching the
/// merchant, wraps the server API call with the necessary parameters and authentication, and awaits the result.
pub async fn switch_merchant_id(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SwitchMerchantIdRequest>,
) -> HttpResponse {
    let flow = Flow::SwitchMerchant;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, user, req| user_core::switch_merchant_id(state, req, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously handles the creation of a merchant account for a user. It takes in the application state, HTTP request, and JSON payload containing the user's merchant create information. It then wraps the process in a box and awaits the server wrap operation, which includes creating a merchant account for the user using the provided state, authentication token, and JSON payload.
pub async fn user_merchant_account_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::UserMerchantCreate>,
) -> HttpResponse {
    let flow = Flow::UserMerchantAccountCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::UserFromToken, json_payload| {
            user_core::create_merchant_account(state, auth, json_payload)
        },
        &auth::JWTAuth(Permission::MerchantAccountCreate),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "dummy_connector")]
/// Asynchronously generates sample data for a user and returns an actix_web::Responder.
pub async fn generate_sample_data(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<SampleDataRequest>,
) -> impl actix_web::Responder {
    use crate::core::user::sample_data;

    let flow = Flow::GenerateSampleData;
    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        payload.into_inner(),
        sample_data::generate_sample_data_for_user,
        &auth::JWTAuth(Permission::PaymentWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[cfg(feature = "dummy_connector")]
/// Asynchronously deletes sample data for a user by handling the HTTP request and calling the `delete_sample_data_for_user` function from the `sample_data` module. 
pub async fn delete_sample_data(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<SampleDataRequest>,
) -> impl actix_web::Responder {
    use crate::core::user::sample_data;

    let flow = Flow::DeleteSampleData;
    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        payload.into_inner(),
        sample_data::delete_sample_data_for_user,
        &auth::JWTAuth(Permission::PaymentWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously retrieves a list of merchant IDs associated with a user and returns an HttpResponse.
pub async fn list_merchant_ids_for_user(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::UserMerchantAccountList;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, user, _| user_core::list_merchant_ids_for_user(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Retrieves user details from the server using the provided `AppState` and `HttpRequest`.
pub async fn get_user_details(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::GetUserDetails;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user, _| user_core::get_users_for_merchant_account(state, user),
        &auth::JWTAuth(Permission::UsersRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
/// Handles the request to initiate the forgot password flow for a user. 
/// This method takes the application state, the HTTP request, and the user's forgot password request payload as input. 
/// It then invokes the `api::server_wrap` method with the appropriate parameters to handle the forgot password flow, 
/// including invoking the `user_core::forgot_password` method to process the forgot password request. 
pub async fn forgot_password(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<user_api::ForgotPasswordRequest>,
) -> HttpResponse {
    let flow = Flow::ForgotPassword;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        |state, _, payload| user_core::forgot_password(state, payload),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
/// Handles the reset password HTTP request by delegating the processing to the `user_core::reset_password` function
pub async fn reset_password(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<user_api::ResetPasswordRequest>,
) -> HttpResponse {
    let flow = Flow::ResetPassword;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        |state, _, payload| user_core::reset_password(state, payload),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Handles the invitation of a user by wrapping the invite_user function from user_core module
/// in a server_wrap function from the api module. This method takes the current application state,
/// the HTTP request, and the user invitation request payload as input, and returns an HTTP response.
pub async fn invite_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<user_api::InviteUserRequest>,
) -> HttpResponse {
    let flow = Flow::InviteUser;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        |state, user, payload| user_core::invite_user(state, payload, user),
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Handles the invitation of multiple users by wrapping the invite_multiple_user function from the user_core module in an API server wrap. It takes the AppSate, HttpRequest, and a JSON payload containing multiple user invitation requests as input, and returns an HttpResponse.
pub async fn invite_multiple_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<Vec<user_api::InviteUserRequest>>,
) -> HttpResponse {
    let flow = Flow::InviteMultipleUser;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        user_core::invite_multiple_user,
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
/// Handles the verification of the user's email without performing invite checks. This method takes in the application state, the HTTP request, and the JSON payload containing the user's email verification request. It then initiates the email verification process by calling the user_core::verify_email_without_invite_checks function and returns the result as an HTTP response. This method uses the flow type VerifyEmailWithoutInviteChecks and does not require any authentication for the verification process.
pub async fn verify_email_without_invite_checks(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::VerifyEmailRequest>,
) -> HttpResponse {
    let flow = Flow::VerifyEmailWithoutInviteChecks;
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        json_payload.into_inner(),
        |state, _, req_payload| user_core::verify_email_without_invite_checks(state, req_payload),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
/// This method handles the verification of email for a user. It takes in the application state, an HTTP request, and a JSON payload containing the email verification request. It then uses the `api::server_wrap` function to wrap the verification process, including calling `user_core::verify_email` to perform the actual email verification. The method returns an HTTP response.
pub async fn verify_email(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::VerifyEmailRequest>,
) -> HttpResponse {
    let flow = Flow::VerifyEmail;
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        json_payload.into_inner(),
        |state, _, req_payload| user_core::verify_email(state, req_payload),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
/// Handles the verification of an email request by sending a verification email to the specified email address.
pub async fn verify_email_request(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SendVerifyEmailRequest>,
) -> HttpResponse {
    let flow = Flow::VerifyEmailRequest;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, _, req_body| user_core::send_verification_mail(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "recon")]
/// Asynchronously verifies a reconciliation token by calling the user_core::verify_token function. 
/// It uses the provided AppState and HttpRequest to perform the verification process, and then returns an HttpResponse. 
pub async fn verify_recon_token(state: web::Data<AppState>, http_req: HttpRequest) -> HttpResponse {
    let flow = Flow::ReconVerifyToken;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        (),
        |state, user, _req| user_core::verify_token(state, user),
        &auth::ReconJWT,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously updates the user account details by wrapping the update_user_details function from the user_core module in a server_wrap function from the api module. It takes the AppSate, HttpRequest, and a JSON payload containing the user's updated account details as input parameters. It then returns an HttpResponse after awaiting the completion of the server_wrap function.
pub async fn update_user_account_details(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::UpdateUserAccountDetailsRequest>,
) -> HttpResponse {
    let flow = Flow::UpdateUserAccountDetails;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        user_core::update_user_details,
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
