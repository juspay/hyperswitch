use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use storage_models::errors as storage_errors;

use crate::{
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        payment_methods::cards,
    },
    db::StorageInterface,
    pii::PeekInterface,
    routes::AppState,
    services,
    types::{
        api::customers::{self, CustomerRequestExt},
        storage::{self, enums},
    },
};

pub const REDACTED: &str = "Redacted";

#[instrument(skip(db))]
pub async fn create_customer(
    db: &dyn StorageInterface,
    merchant_account: storage::MerchantAccount,
    customer_data: customers::CustomerRequest,
) -> RouterResponse<customers::CustomerResponse> {
    let mut customer_data = customer_data.validate()?;
    let customer_id = &customer_data.customer_id;
    let merchant_id = &merchant_account.merchant_id;
    customer_data.merchant_id = merchant_id.to_owned();

    if let Some(addr) = &customer_data.address {
        let customer_address: api_models::payments::AddressDetails = addr
            .peek()
            .clone()
            .parse_value("AddressDetails")
            .change_context(errors::ApiErrorResponse::AddressNotFound)?;
        db.insert_address(storage::AddressNew {
            city: customer_address.city,
            country: customer_address.country,
            line1: customer_address.line1,
            line2: customer_address.line2,
            line3: customer_address.line3,
            zip: customer_address.zip,
            state: customer_address.state,
            first_name: customer_address.first_name,
            last_name: customer_address.last_name,
            phone_number: customer_data.phone.clone(),
            country_code: customer_data.phone_country_code.clone(),
            customer_id: customer_id.to_string(),
            merchant_id: merchant_id.to_string(),
            ..Default::default()
        })
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while inserting new address")?;
    };

    let new_customer = storage::CustomerNew {
        customer_id: customer_id.to_string(),
        merchant_id: merchant_id.to_string(),
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
                db.find_customer_by_customer_id_merchant_id(customer_id, merchant_id)
                    .await
                    .map_err(|err| {
                        err.to_not_found_response(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable(format!(
                                "Failed while fetching Customer, customer_id: {customer_id}",
                            ))
                    })?
            } else {
                Err(error
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while inserting new customer"))?
            }
        }
    };
    let mut customer_response: customers::CustomerResponse = customer.into();
    customer_response.address = customer_data.address;

    Ok(services::ApplicationResponse::Json(customer_response))
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

    Ok(services::ApplicationResponse::Json(response.into()))
}

#[instrument(skip_all)]
pub async fn delete_customer(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: customers::CustomerId,
) -> RouterResponse<customers::CustomerDeleteResponse> {
    let db = &state.store;

    db.find_customer_by_customer_id_merchant_id(&req.customer_id, &merchant_account.merchant_id)
        .await
        .map_err(|err| err.to_not_found_response(errors::ApiErrorResponse::CustomerNotFound))?;

    let customer_mandates = db
        .find_mandate_by_merchant_id_customer_id(&merchant_account.merchant_id, &req.customer_id)
        .await
        .map_err(|err| err.to_not_found_response(errors::ApiErrorResponse::MandateNotFound))?;

    for mandate in customer_mandates.into_iter() {
        if mandate.mandate_status == enums::MandateStatus::Active {
            Err(errors::ApiErrorResponse::MandateActive)?
        }
    }

    match db
        .find_payment_method_by_customer_id_merchant_id_list(
            &req.customer_id,
            &merchant_account.merchant_id,
        )
        .await
    {
        Ok(customer_payment_methods) => {
            for pm in customer_payment_methods.into_iter() {
                if pm.payment_method == enums::PaymentMethod::Card {
                    cards::delete_card_from_locker(
                        state,
                        &req.customer_id,
                        &merchant_account.merchant_id,
                        &pm.payment_method_id,
                    )
                    .await?;
                }
                db.delete_payment_method_by_merchant_id_payment_method_id(
                    &merchant_account.merchant_id,
                    &pm.payment_method_id,
                )
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
                })?;
            }
        }
        Err(error) => match error.current_context() {
            errors::StorageError::DatabaseError(err) => match err.current_context() {
                storage_errors::DatabaseError::NotFound => Ok(()),
                _ => Err(errors::ApiErrorResponse::InternalServerError),
            },
            _ => Err(errors::ApiErrorResponse::InternalServerError),
        }?,
    };

    let update_address = storage::AddressUpdate::Update {
        city: Some(REDACTED.to_string()),
        country: Some(REDACTED.to_string()),
        line1: Some(REDACTED.to_string().into()),
        line2: Some(REDACTED.to_string().into()),
        line3: Some(REDACTED.to_string().into()),
        state: Some(REDACTED.to_string().into()),
        zip: Some(REDACTED.to_string().into()),
        first_name: Some(REDACTED.to_string().into()),
        last_name: Some(REDACTED.to_string().into()),
        phone_number: Some(REDACTED.to_string().into()),
        country_code: Some(REDACTED.to_string()),
    };

    match db
        .update_address_by_merchant_id_customer_id(
            &req.customer_id,
            &merchant_account.merchant_id,
            update_address,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(error) => match error.current_context() {
            errors::StorageError::DatabaseError(err) => match err.current_context() {
                storage_errors::DatabaseError::NotFound => Ok(()),
                _ => Err(errors::ApiErrorResponse::InternalServerError),
            },
            _ => Err(errors::ApiErrorResponse::InternalServerError),
        },
    }?;

    let updated_customer = storage::CustomerUpdate::Update {
        name: Some(REDACTED.to_string()),
        email: Some(REDACTED.to_string().into()),
        phone: Some(REDACTED.to_string().into()),
        description: Some(REDACTED.to_string()),
        phone_country_code: Some(REDACTED.to_string()),
        metadata: None,
    };
    db.update_customer_by_customer_id_merchant_id(
        req.customer_id.clone(),
        merchant_account.merchant_id,
        updated_customer,
    )
    .await
    .change_context(errors::ApiErrorResponse::CustomerNotFound)?;

    let response = customers::CustomerDeleteResponse {
        customer_id: req.customer_id,
        customer_deleted: true,
        address_deleted: true,
        payment_methods_deleted: true,
    };
    Ok(services::ApplicationResponse::Json(response))
}

#[instrument(skip(db))]
pub async fn update_customer(
    db: &dyn StorageInterface,
    merchant_account: storage::MerchantAccount,
    update_customer: customers::CustomerRequest,
) -> RouterResponse<customers::CustomerResponse> {
    let update_customer = update_customer.validate()?;
    //Add this in update call if customer can be updated anywhere else
    db.find_customer_by_customer_id_merchant_id(
        &update_customer.customer_id,
        &merchant_account.merchant_id,
    )
    .await
    .map_err(|err| err.to_not_found_response(errors::ApiErrorResponse::CustomerNotFound))?;

    if let Some(addr) = &update_customer.address {
        let customer_address: api_models::payments::AddressDetails = addr
            .peek()
            .clone()
            .parse_value("AddressDetails")
            .change_context(errors::ApiErrorResponse::AddressNotFound)?;
        let update_address = storage::AddressUpdate::Update {
            city: customer_address.city,
            country: customer_address.country,
            line1: customer_address.line1,
            line2: customer_address.line2,
            line3: customer_address.line3,
            zip: customer_address.zip,
            state: customer_address.state,
            first_name: customer_address.first_name,
            last_name: customer_address.last_name,
            phone_number: update_customer.phone.clone(),
            country_code: update_customer.phone_country_code.clone(),
        };
        db.update_address_by_merchant_id_customer_id(
            &update_customer.customer_id,
            &merchant_account.merchant_id,
            update_address,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "Failed while updating address: merchant_id: {}, customer_id: {}",
            merchant_account.merchant_id, update_customer.customer_id
        ))?;
    };

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

    let mut customer_update_response: customers::CustomerResponse = response.into();
    customer_update_response.address = update_customer.address;
    Ok(services::ApplicationResponse::Json(
        customer_update_response,
    ))
}
