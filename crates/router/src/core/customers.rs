use common_utils::crypto::{Encryptable, GcmAes256};
use error_stack::ResultExt;
use masking::ExposeInterface;
use router_env::{instrument, tracing};

use crate::{
    consts,
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        payment_methods::cards,
    },
    db::StorageInterface,
    pii::PeekInterface,
    routes::{metrics, AppState},
    services,
    types::{
        api::customers,
        domain::{
            self,
            types::{self, AsyncLift, TypeEncryption},
        },
        storage::{self, enums},
    },
    utils::generate_id,
};

pub const REDACTED: &str = "Redacted";

#[instrument(skip(db))]
pub async fn create_customer(
    db: &dyn StorageInterface,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    mut customer_data: customers::CustomerRequest,
) -> RouterResponse<customers::CustomerResponse> {
    let customer_id = &customer_data.customer_id;
    let merchant_id = &merchant_account.merchant_id;
    customer_data.merchant_id = merchant_id.to_owned();

    let key = key_store.key.get_inner().peek();
    let address_id = if let Some(addr) = &customer_data.address {
        let customer_address: api_models::payments::AddressDetails = addr.clone();

        let address = async {
            Ok(domain::Address {
                city: customer_address.city,
                country: customer_address.country,
                line1: customer_address
                    .line1
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                line2: customer_address
                    .line2
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                line3: customer_address
                    .line3
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                zip: customer_address
                    .zip
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                state: customer_address
                    .state
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                first_name: customer_address
                    .first_name
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                last_name: customer_address
                    .last_name
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                phone_number: customer_data
                    .phone
                    .clone()
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                country_code: customer_data.phone_country_code.clone(),
                customer_id: customer_id.to_string(),
                merchant_id: merchant_id.to_string(),
                id: None,
                address_id: generate_id(consts::ID_LENGTH, "add"),
                payment_id: None,
                created_at: common_utils::date_time::now(),
                modified_at: common_utils::date_time::now(),
            })
        }
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting address")?;

        Some(
            db.insert_address_customers(address, &key_store)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while inserting new address")?
                .address_id,
        )
    } else {
        None
    };

    let new_customer = async {
        Ok(domain::Customer {
            customer_id: customer_id.to_string(),
            merchant_id: merchant_id.to_string(),
            name: customer_data
                .name
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            email: customer_data
                .email
                .async_lift(|inner| types::encrypt_optional(inner.map(|inner| inner.expose()), key))
                .await?,
            phone: customer_data
                .phone
                .async_lift(|inner| types::encrypt_optional(inner, key))
                .await?,
            description: customer_data.description,
            phone_country_code: customer_data.phone_country_code,
            metadata: customer_data.metadata,
            id: None,
            connector_customer: None,
            address_id,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
        })
    }
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while encrypting Customer")?;

    let customer = match db.insert_customer(new_customer, &key_store).await {
        Ok(customer) => customer,
        Err(error) => {
            if error.current_context().is_db_unique_violation() {
                db.find_customer_by_customer_id_merchant_id(customer_id, merchant_id, &key_store)
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(format!(
                        "Failed while fetching Customer, customer_id: {customer_id}",
                    ))?
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
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: customers::CustomerId,
) -> RouterResponse<customers::CustomerResponse> {
    let response = db
        .find_customer_by_customer_id_merchant_id(
            &req.customer_id,
            &merchant_account.merchant_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    Ok(services::ApplicationResponse::Json(response.into()))
}

#[instrument(skip_all)]
pub async fn delete_customer(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    req: customers::CustomerId,
    key_store: domain::MerchantKeyStore,
) -> RouterResponse<customers::CustomerDeleteResponse> {
    let db = &state.store;

    db.find_customer_by_customer_id_merchant_id(
        &req.customer_id,
        &merchant_account.merchant_id,
        &key_store,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let customer_mandates = db
        .find_mandate_by_merchant_id_customer_id(&merchant_account.merchant_id, &req.customer_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;

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
                .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
            }
        }
        Err(error) => {
            if error.current_context().is_db_not_found() {
                Ok(())
            } else {
                Err(error)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("failed find_payment_method_by_customer_id_merchant_id_list")
            }?
        }
    };

    let key = key_store.key.get_inner().peek();

    let redacted_encrypted_value: Encryptable<masking::Secret<_>> =
        Encryptable::encrypt(REDACTED.to_string().into(), key, GcmAes256)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let update_address = storage::AddressUpdate::Update {
        city: Some(REDACTED.to_string()),
        country: None,
        line1: Some(redacted_encrypted_value.clone()),
        line2: Some(redacted_encrypted_value.clone()),
        line3: Some(redacted_encrypted_value.clone()),
        state: Some(redacted_encrypted_value.clone()),
        zip: Some(redacted_encrypted_value.clone()),
        first_name: Some(redacted_encrypted_value.clone()),
        last_name: Some(redacted_encrypted_value.clone()),
        phone_number: Some(redacted_encrypted_value.clone()),
        country_code: Some(REDACTED.to_string()),
    };

    match db
        .update_address_by_merchant_id_customer_id(
            &req.customer_id,
            &merchant_account.merchant_id,
            update_address,
            &key_store,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(error) => {
            if error.current_context().is_db_not_found() {
                Ok(())
            } else {
                Err(error)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("failed update_address_by_merchant_id_customer_id")
            }
        }
    }?;

    let updated_customer = storage::CustomerUpdate::Update {
        name: Some(redacted_encrypted_value.clone()),
        email: Some(
            Encryptable::encrypt(REDACTED.to_string().into(), key, GcmAes256)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
        ),
        phone: Some(redacted_encrypted_value.clone()),
        description: Some(REDACTED.to_string()),
        phone_country_code: Some(REDACTED.to_string()),
        metadata: None,
        connector_customer: None,
        address_id: None,
    };
    db.update_customer_by_customer_id_merchant_id(
        req.customer_id.clone(),
        merchant_account.merchant_id,
        updated_customer,
        &key_store,
    )
    .await
    .change_context(errors::ApiErrorResponse::CustomerNotFound)?;

    let response = customers::CustomerDeleteResponse {
        customer_id: req.customer_id,
        customer_deleted: true,
        address_deleted: true,
        payment_methods_deleted: true,
    };
    metrics::CUSTOMER_REDACTED.add(&metrics::CONTEXT, 1, &[]);
    Ok(services::ApplicationResponse::Json(response))
}

#[instrument(skip(db))]
pub async fn update_customer(
    db: &dyn StorageInterface,
    merchant_account: domain::MerchantAccount,
    update_customer: customers::CustomerRequest,
    key_store: domain::MerchantKeyStore,
) -> RouterResponse<customers::CustomerResponse> {
    //Add this in update call if customer can be updated anywhere else
    let customer = db
        .find_customer_by_customer_id_merchant_id(
            &update_customer.customer_id,
            &merchant_account.merchant_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let key = key_store.key.get_inner().peek();

    let address_id = if let Some(addr) = &update_customer.address {
        match customer.address_id {
            Some(address_id) => {
                let customer_address: api_models::payments::AddressDetails = addr.clone();
                let update_address = async {
                    Ok(storage::AddressUpdate::Update {
                        city: customer_address.city,
                        country: customer_address.country,
                        line1: customer_address
                            .line1
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        line2: customer_address
                            .line2
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        line3: customer_address
                            .line3
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        zip: customer_address
                            .zip
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        state: customer_address
                            .state
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        first_name: customer_address
                            .first_name
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        last_name: customer_address
                            .last_name
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        phone_number: update_customer
                            .phone
                            .clone()
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        country_code: update_customer.phone_country_code.clone(),
                    })
                }
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting Address while Update")?;
                db.update_address(address_id.clone(), update_address, &key_store)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(format!(
                        "Failed while updating address: merchant_id: {}, customer_id: {}",
                        merchant_account.merchant_id, update_customer.customer_id
                    ))?;
                Some(address_id)
            }
            None => {
                let customer_address: api_models::payments::AddressDetails = addr.clone();

                let address = async {
                    Ok(domain::Address {
                        city: customer_address.city,
                        country: customer_address.country,
                        line1: customer_address
                            .line1
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        line2: customer_address
                            .line2
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        line3: customer_address
                            .line3
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        zip: customer_address
                            .zip
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        state: customer_address
                            .state
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        first_name: customer_address
                            .first_name
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        last_name: customer_address
                            .last_name
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        phone_number: update_customer
                            .phone
                            .clone()
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                        country_code: update_customer.phone_country_code.clone(),
                        customer_id: customer.customer_id.to_string(),
                        merchant_id: merchant_account.merchant_id.to_string(),
                        id: None,
                        address_id: generate_id(consts::ID_LENGTH, "add"),
                        payment_id: None,
                        created_at: common_utils::date_time::now(),
                        modified_at: common_utils::date_time::now(),
                    })
                }
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting address")?;
                Some(
                    db.insert_address_customers(address, &key_store)
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while inserting new address")?
                        .address_id,
                )
            }
        }
    } else {
        None
    };

    let response = db
        .update_customer_by_customer_id_merchant_id(
            update_customer.customer_id.to_owned(),
            merchant_account.merchant_id.to_owned(),
            async {
                Ok(storage::CustomerUpdate::Update {
                    name: update_customer
                        .name
                        .async_lift(|inner| types::encrypt_optional(inner, key))
                        .await?,
                    email: update_customer
                        .email
                        .async_lift(|inner| {
                            types::encrypt_optional(inner.map(|inner| inner.expose()), key)
                        })
                        .await?,
                    phone: update_customer
                        .phone
                        .async_lift(|inner| types::encrypt_optional(inner, key))
                        .await?,
                    phone_country_code: update_customer.phone_country_code,
                    metadata: update_customer.metadata,
                    description: update_customer.description,
                    connector_customer: None,
                    address_id,
                })
            }
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while encrypting while updating customer")?,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let mut customer_update_response: customers::CustomerResponse = response.into();
    customer_update_response.address = update_customer.address;
    Ok(services::ApplicationResponse::Json(
        customer_update_response,
    ))
}
