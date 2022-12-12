use error_stack::ResultExt;
use router_env::{tracing, tracing::instrument};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    db::StorageInterface,
    services,
    types::{
        api::customers::{self, CustomerRequestExt},
        storage,
    },
};

#[instrument(skip(db))]
pub async fn create_customer(
    db: &dyn StorageInterface,
    merchant_account: storage::MerchantAccount,
    customer_data: customers::CustomerRequest,
) -> RouterResponse<customers::CustomerResponse> {
    let mut customer_data = customer_data.validate()?;
    let customer_id = customer_data.customer_id.to_owned();
    let merchant_id = merchant_account.merchant_id.to_owned();
    customer_data.merchant_id = merchant_id.to_owned();

    let new_customer = storage::CustomerNew {
        customer_id: customer_id.clone(),
        merchant_id: merchant_id.clone(),
        name: customer_data.name,
        email: customer_data.email,
        phone: customer_data.phone,
        description: customer_data.description,
        phone_country_code: customer_data.phone_country_code,
        metadata: customer_data.metadata,
    };

    let customer = match db.insert_customer(new_customer).await {
        Ok(customer) => customer,
        Err(error) => {
            if error.current_context().is_db_unique_violation() {
                db.find_customer_by_customer_id_merchant_id(&customer_id, &merchant_id)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?
            } else {
                Err(error.change_context(errors::ApiErrorResponse::InternalServerError))?
            }
        }
    };

    Ok(services::BachResponse::Json(customer.into()))
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

    Ok(services::BachResponse::Json(response.into()))
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
    update_customer: customers::CustomerRequest,
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
                metadata: update_customer.metadata,
                description: update_customer.description,
            },
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::CustomerNotFound))?;

    Ok(services::BachResponse::Json(response.into()))
}
