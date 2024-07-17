#[cfg(not(feature = "v2"))]
use common_utils::{
    crypto::{Encryptable, GcmAes256},
    ext_traits::OptionExt,
};
use common_utils::{errors::ReportSwitchExt, ext_traits::AsyncExt, id_type};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::customer as customer_domain;
use masking::ExposeInterface;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, StorageErrorExt},
    db::StorageInterface,
    pii::PeekInterface,
    routes::SessionState,
    services,
    types::{
        api::customers,
        domain::{
            self,
            types::{self, AsyncLift},
        },
        transformers::ForeignFrom,
    },
};
#[cfg(not(feature = "v2"))]
use crate::{
    core::payment_methods::cards,
    routes::metrics,
    types::{
        domain::types::TypeEncryption,
        storage::{self, enums},
    },
    utils::CustomerAddress,
};

pub const REDACTED: &str = "Redacted";

#[instrument(skip(state))]
pub async fn create_customer(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    customer_data: customers::CustomerRequest,
) -> errors::CustomerResponse<customers::CustomerResponse> {
    let db: &dyn StorageInterface = state.store.as_ref();

    let merchant_reference_id = customer_data.get_merchant_reference_id();

    let merchant_id = &merchant_account.merchant_id;

    let merchant_ref_id_customer_struct = MerchantReferenceIdForCustomer {
        customer_id: merchant_reference_id.as_ref(),
        merchant_id,
        merchant_account: &merchant_account,
        key_store: &key_store,
    };

    // We first need to validate whether the customer with the given customer id already exists
    // this may seem like a redundant db call, as the insert_customer will anyway return this error
    //
    // Consider a scenario where the address is inserted and then when inserting the customer,
    // it errors out, now the address that was inserted is not deleted

    merchant_ref_id_customer_struct
        .verify_if_customer_not_present_by_optional_merchant_reference_id(db)
        .await?;

    customer_data
        .create_domain_model_from_request(db, &key_store, &merchant_reference_id, &merchant_account)
        .await
}

trait CustomerCreateBridge {
    async fn create_domain_model_from_request(
        self,
        db: &dyn StorageInterface,
        key: &domain::MerchantKeyStore,
        merchant_reference_id: &Option<id_type::CustomerId>,
        merchant_account: &domain::MerchantAccount,
    ) -> errors::CustomerResponse<customers::CustomerResponse>;
}

#[cfg(all(not(feature = "v2")))]
impl CustomerCreateBridge for customers::CustomerRequest {
    async fn create_domain_model_from_request(
        self,
        db: &dyn StorageInterface,
        key_store: &domain::MerchantKeyStore,
        merchant_reference_id: &Option<id_type::CustomerId>,
        merchant_account: &domain::MerchantAccount,
    ) -> errors::CustomerResponse<customers::CustomerResponse> {
        // Setting default billing address to Db
        let address = self.get_address();
        let merchant_id = &merchant_account.merchant_id;
        let key = key_store.key.get_inner().peek();

        let customer_billing_address_struct = AddressStructForDbEntry {
            address: address.as_ref(),
            customer_data: &self,
            merchant_id: &merchant_id,
            customer_id: merchant_reference_id.as_ref(),
            storage_scheme: merchant_account.storage_scheme,
            key_store: &key_store,
        };

        let address_from_db = customer_billing_address_struct
            .encrypt_customer_address_and_set_to_db(db)
            .await?;

        let domain_customer = async {
            Ok(customer_domain::Customer {
                customer_id: merchant_reference_id.to_owned().unwrap(),
                merchant_id: merchant_id.to_string(),
                name: self
                    .name
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                email: self
                    .email
                    .async_lift(|inner| {
                        types::encrypt_optional(inner.map(|inner| inner.expose()), key)
                    })
                    .await?,
                phone: self
                    .phone
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                description: self.description,
                phone_country_code: self.phone_country_code,
                metadata: self.metadata,
                id: None,
                connector_customer: None,
                address_id: address_from_db.clone().map(|addr| addr.address_id),
                created_at: common_utils::date_time::now(),
                modified_at: common_utils::date_time::now(),
                default_payment_method_id: None,
                updated_by: None,
                // default_billing_address: None,
                // default_shipping_address: None,
                // merchant_reference_id: None,
                // status: None
            })
        }
        .await
        .switch()
        .attach_printable("Failed while encrypting Customer")?;

        let customer = db
            .insert_customer(domain_customer, &key_store, merchant_account.storage_scheme)
            .await
            .to_duplicate_response(errors::CustomersErrorResponse::CustomerAlreadyExists)?;

        let address_details = address.map(api_models::payments::AddressDetails::from);

        Ok(services::ApplicationResponse::Json(
            customers::CustomerResponse::foreign_from((customer, address_details)),
        ))
    }
}

#[cfg(all(feature = "v2"))]
impl CustomerCreateBridge for customers::CustomerRequest {
    async fn create_domain_model_from_request(
        self,
        db: &dyn StorageInterface,
        key_store: &domain::MerchantKeyStore,
        merchant_reference_id: &Option<id_type::CustomerId>,
        merchant_account: &domain::MerchantAccount,
    ) -> errors::CustomerResponse<customers::CustomerResponse> {
        let _default_customer_billing_address = self.get_default_customer_billing_address();
        let _default_customer_shipping_address = self.get_default_customer_shipping_address();
        let merchant_id = merchant_account.merchant_id.clone();
        let key = key_store.key.get_inner().peek();

        // We first need to validate whether the customer with the given customer id already exists
        // this may seem like a redundant db call, as the insert_customer will anyway return this error
        //
        // Consider a scenario where the address is inserted and then when inserting the customer,
        // it errors out, now the address that was inserted is not deleted

        let domain_customer = async {
            Ok(customer_domain::Customer {
                customer_id: merchant_reference_id.to_owned().unwrap(),
                merchant_id: merchant_id.to_string(),
                name: Some(types::encrypt(self.name, key).await?),
                email: Some(types::encrypt(self.email.expose(), key).await?),
                phone: self
                    .phone
                    .async_lift(|inner| types::encrypt_optional(inner, key))
                    .await?,
                description: self.description,
                phone_country_code: self.phone_country_code,
                metadata: self.metadata,
                id: None,
                connector_customer: None,
                address_id: None,
                created_at: common_utils::date_time::now(),
                modified_at: common_utils::date_time::now(),
                default_payment_method_id: None,
                updated_by: None,
                // default_billing_address: default_customer_billing_address,
                // default_shipping_address: default_customer_shipping_address,
                // merchant_reference_id,
                // status: Some(customer_domain::SoftDeleteStatus::Active)
            })
        }
        .await
        .switch()
        .attach_printable("Failed while encrypting Customer")?;

        let customer = db
            .insert_customer(domain_customer, &key_store, merchant_account.storage_scheme)
            .await
            .to_duplicate_response(errors::CustomersErrorResponse::CustomerAlreadyExists)?;

        Ok(services::ApplicationResponse::Json(
            customers::CustomerResponse::foreign_from(customer),
        ))
    }
}

#[cfg(not(feature = "v2"))]
struct AddressStructForDbEntry<'a> {
    address: Option<&'a api_models::payments::AddressDetails>,
    customer_data: &'a customers::CustomerRequest,
    merchant_id: &'a str,
    customer_id: Option<&'a id_type::CustomerId>,
    storage_scheme: common_enums::MerchantStorageScheme,
    key_store: &'a domain::MerchantKeyStore,
}

#[cfg(not(feature = "v2"))]
impl<'a> AddressStructForDbEntry<'a> {
    async fn encrypt_customer_address_and_set_to_db(
        &self,
        db: &dyn StorageInterface,
    ) -> errors::CustomResult<Option<domain::Address>, errors::CustomersErrorResponse> {
        let encrypted_customer_address = self
            .address
            .async_map(|addr| async {
                self.customer_data
                    .get_domain_address(
                        addr.clone(),
                        self.merchant_id,
                        self.customer_id.unwrap(),
                        self.key_store.key.get_inner().peek(),
                        self.storage_scheme,
                    )
                    .await
                    .switch()
                    .attach_printable("Failed while encrypting address")
            })
            .await
            .transpose()?;

        encrypted_customer_address
            .async_map(|encrypt_add| async {
                db.insert_address_for_customers(encrypt_add, self.key_store)
                    .await
                    .switch()
                    .attach_printable("Failed while inserting new address")
            })
            .await
            .transpose()
    }
}

struct MerchantReferenceIdForCustomer<'a> {
    customer_id: Option<&'a id_type::CustomerId>,
    merchant_id: &'a str,
    merchant_account: &'a domain::MerchantAccount,
    key_store: &'a domain::MerchantKeyStore,
}

impl<'a> MerchantReferenceIdForCustomer<'a> {
    async fn verify_if_customer_not_present_by_optional_merchant_reference_id(
        &self,
        db: &dyn StorageInterface,
    ) -> Result<Option<()>, error_stack::Report<errors::CustomersErrorResponse>> {
        self.customer_id
            .async_map(|cust| async {
                self.verify_if_customer_not_present_by_merchant_reference(cust, db)
                    .await
            })
            .await
            .transpose()
    }

    async fn verify_if_customer_not_present_by_merchant_reference(
        &self,
        cus: &'a id_type::CustomerId,
        db: &dyn StorageInterface,
    ) -> Result<(), error_stack::Report<errors::CustomersErrorResponse>> {
        match db
            .find_customer_by_customer_id_merchant_id(
                cus,
                self.merchant_id,
                self.key_store,
                self.merchant_account.storage_scheme,
            )
            .await
        {
            Err(err) => {
                if !err.current_context().is_db_not_found() {
                    Err(err).switch()
                } else {
                    Ok(())
                }
            }
            Ok(_) => Err(report!(
                errors::CustomersErrorResponse::CustomerAlreadyExists
            )),
        }
    }
}

#[cfg(all(not(feature = "v2")))]
#[instrument(skip(state))]
pub async fn retrieve_customer(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: customers::CustomerId,
) -> errors::CustomerResponse<customers::CustomerResponse> {
    let db = state.store.as_ref();
    let response = db
        .find_customer_by_customer_id_merchant_id(
            &req.get_merchant_reference_id(),
            &merchant_account.merchant_id,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .switch()?;
    let address = match &response.address_id {
        Some(address_id) => Some(api_models::payments::AddressDetails::from(
            db.find_address_by_address_id(address_id, &key_store)
                .await
                .switch()?,
        )),
        None => None,
    };
    Ok(services::ApplicationResponse::Json(
        customers::CustomerResponse::foreign_from((response, address)),
    ))
}

#[cfg(not(feature = "v2"))]
#[instrument(skip(state))]
pub async fn list_customers(
    state: SessionState,
    merchant_id: String,
    key_store: domain::MerchantKeyStore,
) -> errors::CustomerResponse<Vec<customers::CustomerResponse>> {
    let db = state.store.as_ref();

    let domain_customers = db
        .list_customers_by_merchant_id(&merchant_id, &key_store)
        .await
        .switch()?;

    let customers = domain_customers
        .into_iter()
        .map(|domain_customer| customers::CustomerResponse::foreign_from((domain_customer, None)))
        .collect();

    Ok(services::ApplicationResponse::Json(customers))
}

#[cfg(not(feature = "v2"))]
#[instrument(skip_all)]
pub async fn delete_customer(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    req: customers::CustomerId,
    key_store: domain::MerchantKeyStore,
) -> errors::CustomerResponse<customers::CustomerDeleteResponse> {
    let db = &state.store;

    let customer_orig = db
        .find_customer_by_customer_id_merchant_id(
            &req.customer_id,
            &merchant_account.merchant_id,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .switch()?;

    let customer_mandates = db
        .find_mandate_by_merchant_id_customer_id(&merchant_account.merchant_id, &req.customer_id)
        .await
        .switch()?;

    for mandate in customer_mandates.into_iter() {
        if mandate.mandate_status == enums::MandateStatus::Active {
            Err(errors::CustomersErrorResponse::MandateActive)?
        }
    }

    match db
        .find_payment_method_by_customer_id_merchant_id_list(
            &req.customer_id,
            &merchant_account.merchant_id,
            None,
        )
        .await
    {
        // check this in review
        Ok(customer_payment_methods) => {
            for pm in customer_payment_methods.into_iter() {
                if pm.payment_method == Some(enums::PaymentMethod::Card) {
                    cards::delete_card_from_locker(
                        &state,
                        &req.customer_id,
                        &merchant_account.merchant_id,
                        pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
                    )
                    .await
                    .switch()?;
                }
                db.delete_payment_method_by_merchant_id_payment_method_id(
                    &merchant_account.merchant_id,
                    &pm.payment_method_id,
                )
                .await
                .switch()?;
            }
        }
        Err(error) => {
            if error.current_context().is_db_not_found() {
                Ok(())
            } else {
                Err(error)
                    .change_context(errors::CustomersErrorResponse::InternalServerError)
                    .attach_printable("failed find_payment_method_by_customer_id_merchant_id_list")
            }?
        }
    };

    let key = key_store.key.get_inner().peek();

    let redacted_encrypted_value: Encryptable<masking::Secret<_>> =
        Encryptable::encrypt(REDACTED.to_string().into(), key, GcmAes256)
            .await
            .switch()?;

    let redacted_encrypted_email: Encryptable<
        masking::Secret<_, common_utils::pii::EmailStrategy>,
    > = Encryptable::encrypt(REDACTED.to_string().into(), key, GcmAes256)
        .await
        .switch()?;

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
        updated_by: merchant_account.storage_scheme.to_string(),
        email: Some(redacted_encrypted_email),
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
                    .change_context(errors::CustomersErrorResponse::InternalServerError)
                    .attach_printable("failed update_address_by_merchant_id_customer_id")
            }
        }
    }?;

    let updated_customer = storage::CustomerUpdate::Update {
        name: Some(redacted_encrypted_value.clone()),
        email: Some(
            Encryptable::encrypt(REDACTED.to_string().into(), key, GcmAes256)
                .await
                .switch()?,
        ),
        phone: Box::new(Some(redacted_encrypted_value.clone())),
        description: Some(REDACTED.to_string()),
        phone_country_code: Some(REDACTED.to_string()),
        metadata: None,
        connector_customer: None,
        address_id: None,
    };
    db.update_customer_by_customer_id_merchant_id(
        req.customer_id.clone(),
        merchant_account.merchant_id,
        customer_orig,
        updated_customer,
        &key_store,
        merchant_account.storage_scheme,
    )
    .await
    .switch()?;

    let response = customers::CustomerDeleteResponse {
        customer_id: req.customer_id,
        customer_deleted: true,
        address_deleted: true,
        payment_methods_deleted: true,
    };
    metrics::CUSTOMER_REDACTED.add(&metrics::CONTEXT, 1, &[]);
    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(not(feature = "v2"))]
#[instrument(skip(state))]
pub async fn update_customer(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    update_customer: customers::CustomerRequest,
    key_store: domain::MerchantKeyStore,
) -> errors::CustomerResponse<customers::CustomerResponse> {
    let db = state.store.as_ref();
    //Add this in update call if customer can be updated anywhere else

    let customer_id = update_customer
        .customer_id
        .as_ref()
        .get_required_value("customer_id")
        .change_context(errors::CustomersErrorResponse::InternalServerError)
        .attach("Missing required field `customer_id`")?;

    let customer = db
        .find_customer_by_customer_id_merchant_id(
            customer_id,
            &merchant_account.merchant_id,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .switch()?;

    let key = key_store.key.get_inner().peek();

    let address = if let Some(addr) = &update_customer.address {
        match customer.address_id.clone() {
            Some(address_id) => {
                let customer_address: api_models::payments::AddressDetails = addr.clone();
                let update_address = update_customer
                    .get_address_update(customer_address, key, merchant_account.storage_scheme)
                    .await
                    .switch()
                    .attach_printable("Failed while encrypting Address while Update")?;
                Some(
                    db.update_address(address_id, update_address, &key_store)
                        .await
                        .switch()
                        .attach_printable(format!(
                            "Failed while updating address: merchant_id: {}, customer_id: {:?}",
                            merchant_account.merchant_id, customer_id
                        ))?,
                )
            }
            None => {
                let customer_address: api_models::payments::AddressDetails = addr.clone();

                let address = update_customer
                    .get_domain_address(
                        customer_address,
                        &merchant_account.merchant_id,
                        &customer.customer_id,
                        key,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .switch()
                    .attach_printable("Failed while encrypting address")?;
                Some(
                    db.insert_address_for_customers(address, &key_store)
                        .await
                        .switch()
                        .attach_printable("Failed while inserting new address")?,
                )
            }
        }
    } else {
        match &customer.address_id {
            Some(address_id) => Some(
                db.find_address_by_address_id(address_id, &key_store)
                    .await
                    .switch()?,
            ),
            None => None,
        }
    };

    let response = db
        .update_customer_by_customer_id_merchant_id(
            customer_id.to_owned(),
            merchant_account.merchant_id.to_owned(),
            customer,
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
                    phone: Box::new(
                        update_customer
                            .phone
                            .async_lift(|inner| types::encrypt_optional(inner, key))
                            .await?,
                    ),
                    phone_country_code: update_customer.phone_country_code,
                    metadata: update_customer.metadata,
                    description: update_customer.description,
                    connector_customer: None,
                    address_id: address.clone().map(|addr| addr.address_id),
                })
            }
            .await
            .switch()
            .attach_printable("Failed while encrypting while updating customer")?,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .switch()?;

    Ok(services::ApplicationResponse::Json(
        customers::CustomerResponse::foreign_from((response, update_customer.address)),
    ))
}

pub async fn migrate_customers(
    state: SessionState,
    customers: Vec<customers::CustomerRequest>,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
) -> errors::CustomerResponse<()> {
    for customer in customers {
        match create_customer(
            state.clone(),
            merchant_account.clone(),
            key_store.clone(),
            customer,
        )
        .await
        {
            Ok(_) => (),
            Err(e) => match e.current_context() {
                errors::CustomersErrorResponse::CustomerAlreadyExists => (),
                _ => return Err(e),
            },
        }
    }
    Ok(services::ApplicationResponse::Json(()))
}
