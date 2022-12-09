use error_stack::ResultExt;
use router_env::{tracing, tracing::instrument};

use crate::{
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        payment_methods::cards,
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{api::customers, storage},
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
        address: customer_data.address,
        metadata: customer_data.metadata,
    };

    let customer = match db.insert_customer(new_customer).await {
        Ok(customer) => customer,
        Err(error) => match error.current_context() {
            errors::StorageError::DatabaseError(errors::DatabaseError::UniqueViolation) => db
                .find_customer_by_customer_id_merchant_id(&customer_id, &merchant_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            _ => Err(error.change_context(errors::ApiErrorResponse::InternalServerError))?,
        },
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

#[instrument(skip_all)]
pub async fn delete_customer(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: customers::CustomerId,
) -> RouterResponse<customers::CustomerDeleteResponse> {
    let db = &state.store;
    //TODO check if there are any existing mandates/subscriptions that exist for the current customer
    let resp = db
        .find_payment_method_by_customer_id_merchant_id_list(
            &req.customer_id,
            &merchant_account.merchant_id,
        )
        .await
        .map_err(|err| {
            err.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;
    for pm in resp.into_iter() {
        cards::delete_card(state, &merchant_account.merchant_id, &pm.payment_method_id).await?;
    }
    //delete address from address table
    let payment_intent = db
        .filter_payment_intent_by_customer_id(&req.customer_id)
        .await
        .map_err(|err| err.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;
    for pi in payment_intent.into_iter() {
        if let Some(address_id) = pi.billing_address_id {
            db.delete_address_by_address_id(&address_id)
                .await
                .change_context(errors::ApiErrorResponse::AddressNotFound)?; //DeleteAddressFailed ?
        }
        if let Some(address_id) = pi.shipping_address_id {
            db.delete_address_by_address_id(&address_id)
                .await
                .change_context(errors::ApiErrorResponse::AddressNotFound)?; //DeleteAddressFailed ?
        }
    }
    let response = db
        .delete_customer_by_customer_id_merchant_id(&req.customer_id, &merchant_account.merchant_id)
        .await
        .map(|response| customers::CustomerDeleteResponse {
            customer_id: req.customer_id,
            customer_deleted: response,
            address_deleted: true,
            payment_methods_deleted: true,
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
                address: update_customer.address,
                metadata: update_customer.metadata,
                description: update_customer.description,
            },
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::CustomerNotFound))?;

    Ok(services::BachResponse::Json(response.into()))
}
