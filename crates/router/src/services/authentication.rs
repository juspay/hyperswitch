use actix_web::http::header::HeaderMap;
use api_models::{payment_methods::ListPaymentMethodRequest, payments::PaymentsRequest};
use async_trait::async_trait;
use error_stack::{report, IntoReport, ResultExt};

use crate::{
    core::errors::{self, RouterResult, StorageErrorExt},
    db::StorageInterface,
    routes::AppState,
    services::api,
    types::storage,
    utils::OptionExt,
};

#[async_trait]
pub trait AuthenticateAndFetch<T> {
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &AppState,
    ) -> RouterResult<T>;
}

#[derive(Debug)]
pub struct ApiKeyAuth;

#[async_trait]
impl AuthenticateAndFetch<storage::MerchantAccount> for ApiKeyAuth {
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &AppState,
    ) -> RouterResult<storage::MerchantAccount> {
        let api_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        state
            .store
            .find_merchant_account_by_api_key(api_key)
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Merchant not authenticated")
    }
}

#[derive(Debug)]
pub struct AdminApiAuth;

#[async_trait]
impl AuthenticateAndFetch<()> for AdminApiAuth {
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &AppState,
    ) -> RouterResult<()> {
        let admin_api_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        if admin_api_key != state.conf.keys.admin_api_key {
            Err(report!(errors::ApiErrorResponse::Unauthorized)
                .attach_printable("Admin Authentication Failure"))?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct MerchantIdAuth(pub String);

#[async_trait]
impl AuthenticateAndFetch<storage::MerchantAccount> for MerchantIdAuth {
    async fn authenticate_and_fetch(
        &self,
        _request_headers: &HeaderMap,
        state: &AppState,
    ) -> RouterResult<storage::MerchantAccount> {
        state
            .store
            .find_merchant_account_by_merchant_id(self.0.as_ref())
            .await
            .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::Unauthorized))
    }
}

#[derive(Debug)]
pub struct PublishableKeyAuth;

#[async_trait]
impl AuthenticateAndFetch<storage::MerchantAccount> for PublishableKeyAuth {
    async fn authenticate_and_fetch(
        &self,
        request_headers: &HeaderMap,
        state: &AppState,
    ) -> RouterResult<storage::MerchantAccount> {
        let publishable_key =
            get_api_key(request_headers).change_context(errors::ApiErrorResponse::Unauthorized)?;
        state
            .store
            .find_merchant_account_by_publishable_key(publishable_key)
            .await
            .change_context(errors::ApiErrorResponse::Unauthorized)
            .attach_printable("Merchant not authenticated")
    }
}

pub trait ClientSecretFetch {
    fn get_client_secret(&self) -> Option<&String>;
}

impl ClientSecretFetch for PaymentsRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl ClientSecretFetch for ListPaymentMethodRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

pub fn get_auth_type_and_flow(
    headers: &HeaderMap,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<storage::MerchantAccount>>,
    api::AuthFlow,
)> {
    let api_key = get_api_key(headers)?;

    if api_key.starts_with("pk_") {
        return Ok((Box::new(PublishableKeyAuth), api::AuthFlow::Client));
    }
    Ok((Box::new(PublishableKeyAuth), api::AuthFlow::Merchant))
}

pub fn check_client_secret_and_get_auth(
    headers: &HeaderMap,
    payload: &impl ClientSecretFetch,
) -> RouterResult<(
    Box<dyn AuthenticateAndFetch<storage::MerchantAccount>>,
    api::AuthFlow,
)> {
    let api_key = get_api_key(headers)?;

    if api_key.starts_with("pk_") {
        payload
            .get_client_secret()
            .check_value_present("client_secret")
            .map_err(|_| errors::ApiErrorResponse::MissingRequiredField {
                field_name: "client_secret".to_owned(),
            })?;
        return Ok((Box::new(PublishableKeyAuth), api::AuthFlow::Client));
    }

    if payload.get_client_secret().is_some() {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "client_secret is not a valid parameter".to_owned(),
        }
        .into());
    }

    Ok((Box::new(ApiKeyAuth), api::AuthFlow::Merchant))
}

pub async fn is_ephemeral_auth(
    headers: &HeaderMap,
    db: &dyn StorageInterface,
    customer_id: &str,
) -> RouterResult<Box<dyn AuthenticateAndFetch<storage::MerchantAccount>>> {
    let api_key = get_api_key(headers)?;

    if !api_key.starts_with("epk") {
        return Ok(Box::new(ApiKeyAuth));
    }

    let ephemeral_key = db
        .get_ephemeral_key(api_key)
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)?;

    if ephemeral_key.customer_id.ne(customer_id) {
        return Err(report!(errors::ApiErrorResponse::InvalidEphermeralKey));
    }

    Ok(Box::new(MerchantIdAuth(ephemeral_key.merchant_id)))
}

fn get_api_key(headers: &HeaderMap) -> RouterResult<&str> {
    headers
        .get("api-key")
        .get_required_value("api-key")?
        .to_str()
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert API key to string")
}
