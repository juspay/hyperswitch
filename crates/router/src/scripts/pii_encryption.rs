use common_utils::errors::CustomResult;

use crate::{core::errors, services::Store};

pub async fn crate_merchant_key_store(state: &Store) -> CustomResult<(), errors::ApiErrorResponse> {
    Ok(())
}

pub async fn encrypt_merchant_account_fields(
    state: &Store,
) -> CustomResult<(), errors::ApiErrorResponse> {
    Ok(())
}

pub async fn encrypt_merchant_connector_account_fields(
    state: &Store,
) -> CustomResult<(), errors::ApiErrorResponse> {
    Ok(())
}

pub async fn encrypt_customer_fields(state: &Store) -> CustomResult<(), errors::ApiErrorResponse> {
    Ok(())
}

pub async fn encrypt_address_fields(state: &Store) -> CustomResult<(), errors::ApiErrorResponse> {
    Ok(())
}
