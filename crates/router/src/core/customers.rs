use error_stack::ResultExt;
use router_env::{tracing, tracing::instrument};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    db::StorageInterface,
    services,
    types::{api::customers, storage},
};

#[instrument(skip(db))]
pub async fn create_customer(
    db: &dyn StorageInterface,
    merchant_account: storage::MerchantAccount,
    customer_data: customers::CreateCustomerRequest,
) -> RouterResponse<customers::CustomerResponse> {
    let mut customer_data = customer_data.validate()?;
    let customer_id = customer_data.customer_id.to_owned();
    let merchant_id = merchant_account.merchant_id.to_owned();
    customer_data.merchant_id = merchant_id.to_owned();

    let customer = match db.insert_customer(customer_data).await {
        Ok(customer) => customer,
        Err(error) => match error.current_context() {
            errors::StorageError::DatabaseError(errors::DatabaseError::UniqueViolation) => db
                .find_customer_by_customer_id_merchant_id(&customer_id, &merchant_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            _ => Err(error.change_context(errors::ApiErrorResponse::InternalServerError))?,
        },
    };
    Ok(services::BachResponse::Json(customer))
}

#[instrument(skip(db))]
pub async fn retrieve_customer(
    db: &dyn StorageInterface,
    merchant_account: storage::MerchantAccount,
    req: customers::CustomerId,
) -> RouterResponse<customers::CustomerResponse> {
    let response = db
        .find_customer_by_customer_id_merchant_id(&req.customer_id, &merchant_account.merchant_id)
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::CustomerNotFound))?;

    Ok(services::BachResponse::Json(response))
}

#[instrument(skip(db))]
pub async fn delete_customer(
    db: &dyn StorageInterface,
    merchant_account: storage::MerchantAccount,
    req: customers::CustomerId,
) -> RouterResponse<customers::CustomerDeleteResponse> {
    let response = db
        .delete_customer_by_customer_id_merchant_id(&req.customer_id, &merchant_account.merchant_id)
        .await
        .map(|response| customers::CustomerDeleteResponse {
            customer_id: req.customer_id,
            deleted: response,
        })
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::CustomerNotFound))?;
    Ok(services::BachResponse::Json(response))
}

#[instrument(skip(db))]
pub async fn update_customer(
    db: &dyn StorageInterface,
    merchant_account: storage::MerchantAccount,
    update_customer: customers::CustomerUpdateRequest,
) -> RouterResponse<customers::CustomerResponse> {
    let update_customer = update_customer.validate()?;

    let response = db
        .update_customer_by_customer_id_merchant_id(
            update_customer.customer_id.to_owned(),
            merchant_account.merchant_id.to_owned(),
            storage::CustomerUpdate::Update {
                name: update_customer.name,
                email: update_customer.email,
                phone: update_customer.phone,
                phone_country_code: update_customer.phone_country_code,
                address: update_customer.address,
                metadata: update_customer.metadata,
                description: update_customer.description,
            },
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::CustomerNotFound))?;
    Ok(services::BachResponse::Json(response))
}
