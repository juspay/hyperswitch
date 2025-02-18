use common_utils::{
    crypto::Encryptable,
    errors::ReportSwitchExt,
    ext_traits::AsyncExt,
    id_type, pii, type_name,
    types::{
        keymanager::{Identifier, KeyManagerState, ToEncryptable},
        Description,
    },
};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Secret, SwitchStrategy};
use router_env::{instrument, tracing};

#[cfg(all(feature = "v2", feature = "customer_v2"))]
use crate::core::payment_methods::cards::create_encrypted_data;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use crate::utils::CustomerAddress;
use crate::{
    core::{
        errors::{self, StorageErrorExt},
        payment_methods::{cards, network_tokenization},
    },
    db::StorageInterface,
    pii::PeekInterface,
    routes::{metrics, SessionState},
    services,
    types::{
        api::customers,
        domain::{self, types},
        storage::{self, enums},
        transformers::ForeignFrom,
    },
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
    let key_manager_state = &(&state).into();

    let merchant_reference_id = customer_data.get_merchant_reference_id();

    let merchant_id = merchant_account.get_id();

    let merchant_reference_id_customer = MerchantReferenceIdForCustomer {
        merchant_reference_id: merchant_reference_id.as_ref(),
        merchant_id,
        merchant_account: &merchant_account,
        key_store: &key_store,
        key_manager_state,
    };

    // We first need to validate whether the customer with the given customer id already exists
    // this may seem like a redundant db call, as the insert_customer will anyway return this error
    //
    // Consider a scenario where the address is inserted and then when inserting the customer,
    // it errors out, now the address that was inserted is not deleted

    merchant_reference_id_customer
        .verify_if_merchant_reference_not_present_by_optional_merchant_reference_id(db)
        .await?;

    let domain_customer = customer_data
        .create_domain_model_from_request(
            db,
            &key_store,
            &merchant_reference_id,
            &merchant_account,
            key_manager_state,
            &state,
        )
        .await?;

    let customer = db
        .insert_customer(
            domain_customer,
            key_manager_state,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_duplicate_response(errors::CustomersErrorResponse::CustomerAlreadyExists)?;

    customer_data.generate_response(&customer)
}

#[async_trait::async_trait]
trait CustomerCreateBridge {
    async fn create_domain_model_from_request<'a>(
        &'a self,
        db: &'a dyn StorageInterface,
        key_store: &'a domain::MerchantKeyStore,
        merchant_reference_id: &'a Option<id_type::CustomerId>,
        merchant_account: &'a domain::MerchantAccount,
        key_manager_state: &'a KeyManagerState,
        state: &'a SessionState,
    ) -> errors::CustomResult<domain::Customer, errors::CustomersErrorResponse>;

    fn generate_response<'a>(
        &'a self,
        customer: &'a domain::Customer,
    ) -> errors::CustomerResponse<customers::CustomerResponse>;
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[async_trait::async_trait]
impl CustomerCreateBridge for customers::CustomerRequest {
    async fn create_domain_model_from_request<'a>(
        &'a self,
        db: &'a dyn StorageInterface,
        key_store: &'a domain::MerchantKeyStore,
        merchant_reference_id: &'a Option<id_type::CustomerId>,
        merchant_account: &'a domain::MerchantAccount,
        key_manager_state: &'a KeyManagerState,
        state: &'a SessionState,
    ) -> errors::CustomResult<domain::Customer, errors::CustomersErrorResponse> {
        // Setting default billing address to Db
        let address = self.get_address();
        let merchant_id = merchant_account.get_id();
        let key = key_store.key.get_inner().peek();

        let customer_billing_address_struct = AddressStructForDbEntry {
            address: address.as_ref(),
            customer_data: self,
            merchant_id,
            customer_id: merchant_reference_id.as_ref(),
            storage_scheme: merchant_account.storage_scheme,
            key_store,
            key_manager_state,
            state,
        };

        let address_from_db = customer_billing_address_struct
            .encrypt_customer_address_and_set_to_db(db)
            .await?;

        let encrypted_data = types::crypto_operation(
            key_manager_state,
            type_name!(domain::Customer),
            types::CryptoOperation::BatchEncrypt(
                domain::FromRequestEncryptableCustomer::to_encryptable(
                    domain::FromRequestEncryptableCustomer {
                        name: self.name.clone(),
                        email: self.email.clone().map(|a| a.expose().switch_strategy()),
                        phone: self.phone.clone(),
                    },
                ),
            ),
            Identifier::Merchant(key_store.merchant_id.clone()),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .switch()
        .attach_printable("Failed while encrypting Customer")?;

        let encryptable_customer =
            domain::FromRequestEncryptableCustomer::from_encryptable(encrypted_data)
                .change_context(errors::CustomersErrorResponse::InternalServerError)?;

        Ok(domain::Customer {
            customer_id: merchant_reference_id
                .to_owned()
                .ok_or(errors::CustomersErrorResponse::InternalServerError)?,
            merchant_id: merchant_id.to_owned(),
            name: encryptable_customer.name,
            email: encryptable_customer.email.map(|email| {
                let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> = Encryptable::new(
                    email.clone().into_inner().switch_strategy(),
                    email.into_encrypted(),
                );
                encryptable
            }),
            phone: encryptable_customer.phone,
            description: self.description.clone(),
            phone_country_code: self.phone_country_code.clone(),
            metadata: self.metadata.clone(),
            connector_customer: None,
            address_id: address_from_db.clone().map(|addr| addr.address_id),
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            default_payment_method_id: None,
            updated_by: None,
            version: hyperswitch_domain_models::consts::API_VERSION,
        })
    }

    fn generate_response<'a>(
        &'a self,
        customer: &'a domain::Customer,
    ) -> errors::CustomerResponse<customers::CustomerResponse> {
        let address = self.get_address();
        let address_details = address.map(api_models::payments::AddressDetails::from);

        Ok(services::ApplicationResponse::Json(
            customers::CustomerResponse::foreign_from((customer.clone(), address_details)),
        ))
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[async_trait::async_trait]
impl CustomerCreateBridge for customers::CustomerRequest {
    async fn create_domain_model_from_request<'a>(
        &'a self,
        _db: &'a dyn StorageInterface,
        key_store: &'a domain::MerchantKeyStore,
        merchant_reference_id: &'a Option<id_type::CustomerId>,
        merchant_account: &'a domain::MerchantAccount,
        key_state: &'a KeyManagerState,
        state: &'a SessionState,
    ) -> errors::CustomResult<domain::Customer, errors::CustomersErrorResponse> {
        let default_customer_billing_address = self.get_default_customer_billing_address();
        let encrypted_customer_billing_address = default_customer_billing_address
            .async_map(|billing_address| {
                create_encrypted_data(key_state, key_store, billing_address)
            })
            .await
            .transpose()
            .change_context(errors::CustomersErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt default customer billing address")?;
        let default_customer_shipping_address = self.get_default_customer_shipping_address();
        let encrypted_customer_shipping_address = default_customer_shipping_address
            .async_map(|shipping_address| {
                create_encrypted_data(key_state, key_store, shipping_address)
            })
            .await
            .transpose()
            .change_context(errors::CustomersErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt default customer shipping address")?;

        let merchant_id = merchant_account.get_id().clone();
        let key = key_store.key.get_inner().peek();

        let encrypted_data = types::crypto_operation(
            key_state,
            type_name!(domain::Customer),
            types::CryptoOperation::BatchEncrypt(
                domain::FromRequestEncryptableCustomer::to_encryptable(
                    domain::FromRequestEncryptableCustomer {
                        name: Some(self.name.clone()),
                        email: Some(self.email.clone().expose().switch_strategy()),
                        phone: self.phone.clone(),
                    },
                ),
            ),
            Identifier::Merchant(key_store.merchant_id.clone()),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .switch()
        .attach_printable("Failed while encrypting Customer")?;

        let encryptable_customer =
            domain::FromRequestEncryptableCustomer::from_encryptable(encrypted_data)
                .change_context(errors::CustomersErrorResponse::InternalServerError)?;

        Ok(domain::Customer {
            id: id_type::GlobalCustomerId::generate(&state.conf.cell_information.id),
            merchant_reference_id: merchant_reference_id.to_owned(),
            merchant_id,
            name: encryptable_customer.name,
            email: encryptable_customer.email.map(|email| {
                let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> = Encryptable::new(
                    email.clone().into_inner().switch_strategy(),
                    email.into_encrypted(),
                );
                encryptable
            }),
            phone: encryptable_customer.phone,
            description: self.description.clone(),
            phone_country_code: self.phone_country_code.clone(),
            metadata: self.metadata.clone(),
            connector_customer: None,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            default_payment_method_id: None,
            updated_by: None,
            default_billing_address: encrypted_customer_billing_address.map(Into::into),
            default_shipping_address: encrypted_customer_shipping_address.map(Into::into),
            version: hyperswitch_domain_models::consts::API_VERSION,
            status: common_enums::DeleteStatus::Active,
        })
    }

    fn generate_response<'a>(
        &'a self,
        customer: &'a domain::Customer,
    ) -> errors::CustomerResponse<customers::CustomerResponse> {
        Ok(services::ApplicationResponse::Json(
            customers::CustomerResponse::foreign_from(customer.clone()),
        ))
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
struct AddressStructForDbEntry<'a> {
    address: Option<&'a api_models::payments::AddressDetails>,
    customer_data: &'a customers::CustomerRequest,
    merchant_id: &'a id_type::MerchantId,
    customer_id: Option<&'a id_type::CustomerId>,
    storage_scheme: common_enums::MerchantStorageScheme,
    key_store: &'a domain::MerchantKeyStore,
    key_manager_state: &'a KeyManagerState,
    state: &'a SessionState,
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl AddressStructForDbEntry<'_> {
    async fn encrypt_customer_address_and_set_to_db(
        &self,
        db: &dyn StorageInterface,
    ) -> errors::CustomResult<Option<domain::Address>, errors::CustomersErrorResponse> {
        let encrypted_customer_address = self
            .address
            .async_map(|addr| async {
                self.customer_data
                    .get_domain_address(
                        self.state,
                        addr.clone(),
                        self.merchant_id,
                        self.customer_id
                            .ok_or(errors::CustomersErrorResponse::InternalServerError)?, // should we raise error since in v1 appilcation is supposed to have this id or generate it at this point.
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
                db.insert_address_for_customers(self.key_manager_state, encrypt_add, self.key_store)
                    .await
                    .switch()
                    .attach_printable("Failed while inserting new address")
            })
            .await
            .transpose()
    }
}

struct MerchantReferenceIdForCustomer<'a> {
    merchant_reference_id: Option<&'a id_type::CustomerId>,
    merchant_id: &'a id_type::MerchantId,
    merchant_account: &'a domain::MerchantAccount,
    key_store: &'a domain::MerchantKeyStore,
    key_manager_state: &'a KeyManagerState,
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl<'a> MerchantReferenceIdForCustomer<'a> {
    async fn verify_if_merchant_reference_not_present_by_optional_merchant_reference_id(
        &self,
        db: &dyn StorageInterface,
    ) -> Result<Option<()>, error_stack::Report<errors::CustomersErrorResponse>> {
        self.merchant_reference_id
            .async_map(|cust| async {
                self.verify_if_merchant_reference_not_present_by_merchant_reference_id(cust, db)
                    .await
            })
            .await
            .transpose()
    }

    async fn verify_if_merchant_reference_not_present_by_merchant_reference_id(
        &self,
        cus: &'a id_type::CustomerId,
        db: &dyn StorageInterface,
    ) -> Result<(), error_stack::Report<errors::CustomersErrorResponse>> {
        match db
            .find_customer_by_customer_id_merchant_id(
                self.key_manager_state,
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

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl<'a> MerchantReferenceIdForCustomer<'a> {
    async fn verify_if_merchant_reference_not_present_by_optional_merchant_reference_id(
        &self,
        db: &dyn StorageInterface,
    ) -> Result<Option<()>, error_stack::Report<errors::CustomersErrorResponse>> {
        self.merchant_reference_id
            .async_map(|merchant_ref| async {
                self.verify_if_merchant_reference_not_present_by_merchant_reference(
                    merchant_ref,
                    db,
                )
                .await
            })
            .await
            .transpose()
    }

    async fn verify_if_merchant_reference_not_present_by_merchant_reference(
        &self,
        merchant_ref: &'a id_type::CustomerId,
        db: &dyn StorageInterface,
    ) -> Result<(), error_stack::Report<errors::CustomersErrorResponse>> {
        match db
            .find_customer_by_merchant_reference_id_merchant_id(
                self.key_manager_state,
                merchant_ref,
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

#[cfg(all(any(feature = "v1", feature = "v2",), not(feature = "customer_v2")))]
#[instrument(skip(state))]
pub async fn retrieve_customer(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    _profile_id: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    customer_id: id_type::CustomerId,
) -> errors::CustomerResponse<customers::CustomerResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let response = db
        .find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
            key_manager_state,
            &customer_id,
            merchant_account.get_id(),
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .switch()?
        .ok_or(errors::CustomersErrorResponse::CustomerNotFound)?;

    let address = match &response.address_id {
        Some(address_id) => Some(api_models::payments::AddressDetails::from(
            db.find_address_by_address_id(key_manager_state, address_id, &key_store)
                .await
                .switch()?,
        )),
        None => None,
    };
    Ok(services::ApplicationResponse::Json(
        customers::CustomerResponse::foreign_from((response, address)),
    ))
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[instrument(skip(state))]
pub async fn retrieve_customer(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    id: id_type::GlobalCustomerId,
) -> errors::CustomerResponse<customers::CustomerResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let response = db
        .find_customer_by_global_id(
            key_manager_state,
            &id,
            merchant_account.get_id(),
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .switch()?;

    Ok(services::ApplicationResponse::Json(
        customers::CustomerResponse::foreign_from(response),
    ))
}

#[instrument(skip(state))]
pub async fn list_customers(
    state: SessionState,
    merchant_id: id_type::MerchantId,
    _profile_id_list: Option<Vec<id_type::ProfileId>>,
    key_store: domain::MerchantKeyStore,
    request: customers::CustomerListRequest,
) -> errors::CustomerResponse<Vec<customers::CustomerResponse>> {
    let db = state.store.as_ref();

    let customer_list_constraints = crate::db::customers::CustomerListConstraints {
        limit: request
            .limit
            .unwrap_or(crate::consts::DEFAULT_LIST_API_LIMIT),
        offset: request.offset,
    };

    let domain_customers = db
        .list_customers_by_merchant_id(
            &(&state).into(),
            &merchant_id,
            &key_store,
            customer_list_constraints,
        )
        .await
        .switch()?;

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    let customers = domain_customers
        .into_iter()
        .map(|domain_customer| customers::CustomerResponse::foreign_from((domain_customer, None)))
        .collect();

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    let customers = domain_customers
        .into_iter()
        .map(customers::CustomerResponse::foreign_from)
        .collect();

    Ok(services::ApplicationResponse::Json(customers))
}

#[cfg(all(
    feature = "v2",
    feature = "customer_v2",
    feature = "payment_methods_v2"
))]
#[instrument(skip_all)]
pub async fn delete_customer(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    id: id_type::GlobalCustomerId,
    key_store: domain::MerchantKeyStore,
) -> errors::CustomerResponse<customers::CustomerDeleteResponse> {
    let db = &*state.store;
    let key_manager_state = &(&state).into();
    id.redact_customer_details_and_generate_response(
        db,
        &key_store,
        &merchant_account,
        key_manager_state,
        &state,
    )
    .await
}

#[cfg(all(
    feature = "v2",
    feature = "customer_v2",
    feature = "payment_methods_v2"
))]
#[async_trait::async_trait]
impl CustomerDeleteBridge for id_type::GlobalCustomerId {
    async fn redact_customer_details_and_generate_response<'a>(
        &'a self,
        db: &'a dyn StorageInterface,
        key_store: &'a domain::MerchantKeyStore,
        merchant_account: &'a domain::MerchantAccount,
        key_manager_state: &'a KeyManagerState,
        state: &'a SessionState,
    ) -> errors::CustomerResponse<customers::CustomerDeleteResponse> {
        let customer_orig = db
            .find_customer_by_global_id(
                key_manager_state,
                self,
                merchant_account.get_id(),
                key_store,
                merchant_account.storage_scheme,
            )
            .await
            .switch()?;

        let merchant_reference_id = customer_orig.merchant_reference_id.clone();

        let customer_mandates = db.find_mandate_by_global_customer_id(self).await.switch()?;

        for mandate in customer_mandates.into_iter() {
            if mandate.mandate_status == enums::MandateStatus::Active {
                Err(errors::CustomersErrorResponse::MandateActive)?
            }
        }

        match db
            .find_payment_method_list_by_global_customer_id(
                key_manager_state,
                key_store,
                self,
                None,
            )
            .await
        {
            // check this in review
            Ok(customer_payment_methods) => {
                for pm in customer_payment_methods.into_iter() {
                    if pm.get_payment_method_type() == Some(enums::PaymentMethod::Card) {
                        cards::delete_card_by_locker_id(state, self, merchant_account.get_id())
                            .await
                            .switch()?;
                    }
                    // No solution as of now, need to discuss this further with payment_method_v2

                    // db.delete_payment_method(
                    //     key_manager_state,
                    //     key_store,
                    //     pm,
                    // )
                    // .await
                    // .switch()?;
                }
            }
            Err(error) => {
                if error.current_context().is_db_not_found() {
                    Ok(())
                } else {
                    Err(error)
                        .change_context(errors::CustomersErrorResponse::InternalServerError)
                        .attach_printable(
                            "failed find_payment_method_by_customer_id_merchant_id_list",
                        )
                }?
            }
        };

        let key = key_store.key.get_inner().peek();

        let identifier = Identifier::Merchant(key_store.merchant_id.clone());
        let redacted_encrypted_value: Encryptable<Secret<_>> = types::crypto_operation(
            key_manager_state,
            type_name!(storage::Address),
            types::CryptoOperation::Encrypt(REDACTED.to_string().into()),
            identifier.clone(),
            key,
        )
        .await
        .and_then(|val| val.try_into_operation())
        .switch()?;

        let redacted_encrypted_email = Encryptable::new(
            redacted_encrypted_value
                .clone()
                .into_inner()
                .switch_strategy(),
            redacted_encrypted_value.clone().into_encrypted(),
        );

        let updated_customer =
            storage::CustomerUpdate::Update(Box::new(storage::CustomerGeneralUpdate {
                name: Some(redacted_encrypted_value.clone()),
                email: Box::new(Some(redacted_encrypted_email)),
                phone: Box::new(Some(redacted_encrypted_value.clone())),
                description: Some(Description::from_str_unchecked(REDACTED)),
                phone_country_code: Some(REDACTED.to_string()),
                metadata: None,
                connector_customer: Box::new(None),
                default_billing_address: None,
                default_shipping_address: None,
                default_payment_method_id: None,
                status: Some(common_enums::DeleteStatus::Redacted),
            }));

        db.update_customer_by_global_id(
            key_manager_state,
            self,
            customer_orig,
            merchant_account.get_id(),
            updated_customer,
            key_store,
            merchant_account.storage_scheme,
        )
        .await
        .switch()?;

        let response = customers::CustomerDeleteResponse {
            id: self.clone(),
            merchant_reference_id,
            customer_deleted: true,
            address_deleted: true,
            payment_methods_deleted: true,
        };
        metrics::CUSTOMER_REDACTED.add(1, &[]);
        Ok(services::ApplicationResponse::Json(response))
    }
}

#[async_trait::async_trait]
trait CustomerDeleteBridge {
    async fn redact_customer_details_and_generate_response<'a>(
        &'a self,
        db: &'a dyn StorageInterface,
        key_store: &'a domain::MerchantKeyStore,
        merchant_account: &'a domain::MerchantAccount,
        key_manager_state: &'a KeyManagerState,
        state: &'a SessionState,
    ) -> errors::CustomerResponse<customers::CustomerDeleteResponse>;
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
pub async fn delete_customer(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    customer_id: id_type::CustomerId,
    key_store: domain::MerchantKeyStore,
) -> errors::CustomerResponse<customers::CustomerDeleteResponse> {
    let db = &*state.store;
    let key_manager_state = &(&state).into();
    customer_id
        .redact_customer_details_and_generate_response(
            db,
            &key_store,
            &merchant_account,
            key_manager_state,
            &state,
        )
        .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "payment_methods_v2")
))]
#[async_trait::async_trait]
impl CustomerDeleteBridge for id_type::CustomerId {
    async fn redact_customer_details_and_generate_response<'a>(
        &'a self,
        db: &'a dyn StorageInterface,
        key_store: &'a domain::MerchantKeyStore,
        merchant_account: &'a domain::MerchantAccount,
        key_manager_state: &'a KeyManagerState,
        state: &'a SessionState,
    ) -> errors::CustomerResponse<customers::CustomerDeleteResponse> {
        let customer_orig = db
            .find_customer_by_customer_id_merchant_id(
                key_manager_state,
                self,
                merchant_account.get_id(),
                key_store,
                merchant_account.storage_scheme,
            )
            .await
            .switch()?;

        let customer_mandates = db
            .find_mandate_by_merchant_id_customer_id(merchant_account.get_id(), self)
            .await
            .switch()?;

        for mandate in customer_mandates.into_iter() {
            if mandate.mandate_status == enums::MandateStatus::Active {
                Err(errors::CustomersErrorResponse::MandateActive)?
            }
        }

        match db
            .find_payment_method_by_customer_id_merchant_id_list(
                key_manager_state,
                key_store,
                self,
                merchant_account.get_id(),
                None,
            )
            .await
        {
            // check this in review
            Ok(customer_payment_methods) => {
                for pm in customer_payment_methods.into_iter() {
                    if pm.get_payment_method_type() == Some(enums::PaymentMethod::Card) {
                        cards::delete_card_from_locker(
                            state,
                            self,
                            merchant_account.get_id(),
                            pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
                        )
                        .await
                        .switch()?;

                        if let Some(network_token_ref_id) = pm.network_token_requestor_reference_id
                        {
                            network_tokenization::delete_network_token_from_locker_and_token_service(
                            state,
                            self,
                            merchant_account.get_id(),
                            pm.payment_method_id.clone(),
                            pm.network_token_locker_id,
                            network_token_ref_id,
                        )
                        .await
                        .switch()?;
                        }
                    }

                    db.delete_payment_method_by_merchant_id_payment_method_id(
                        key_manager_state,
                        key_store,
                        merchant_account.get_id(),
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
                        .attach_printable(
                            "failed find_payment_method_by_customer_id_merchant_id_list",
                        )
                }?
            }
        };

        let key = key_store.key.get_inner().peek();
        let identifier = Identifier::Merchant(key_store.merchant_id.clone());
        let redacted_encrypted_value: Encryptable<Secret<_>> = types::crypto_operation(
            key_manager_state,
            type_name!(storage::Address),
            types::CryptoOperation::Encrypt(REDACTED.to_string().into()),
            identifier.clone(),
            key,
        )
        .await
        .and_then(|val| val.try_into_operation())
        .switch()?;

        let redacted_encrypted_email = Encryptable::new(
            redacted_encrypted_value
                .clone()
                .into_inner()
                .switch_strategy(),
            redacted_encrypted_value.clone().into_encrypted(),
        );

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
                key_manager_state,
                self,
                merchant_account.get_id(),
                update_address,
                key_store,
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
                types::crypto_operation(
                    key_manager_state,
                    type_name!(storage::Customer),
                    types::CryptoOperation::Encrypt(REDACTED.to_string().into()),
                    identifier,
                    key,
                )
                .await
                .and_then(|val| val.try_into_operation())
                .switch()?,
            ),
            phone: Box::new(Some(redacted_encrypted_value.clone())),
            description: Some(Description::from_str_unchecked(REDACTED)),
            phone_country_code: Some(REDACTED.to_string()),
            metadata: None,
            connector_customer: Box::new(None),
            address_id: None,
        };

        db.update_customer_by_customer_id_merchant_id(
            key_manager_state,
            self.clone(),
            merchant_account.get_id().to_owned(),
            customer_orig,
            updated_customer,
            key_store,
            merchant_account.storage_scheme,
        )
        .await
        .switch()?;

        let response = customers::CustomerDeleteResponse {
            customer_id: self.clone(),
            customer_deleted: true,
            address_deleted: true,
            payment_methods_deleted: true,
        };
        metrics::CUSTOMER_REDACTED.add(1, &[]);
        Ok(services::ApplicationResponse::Json(response))
    }
}

#[instrument(skip(state))]
pub async fn update_customer(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    update_customer: customers::CustomerUpdateRequestInternal,
    key_store: domain::MerchantKeyStore,
) -> errors::CustomerResponse<customers::CustomerResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    //Add this in update call if customer can be updated anywhere else

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    let verify_id_for_update_customer = VerifyIdForUpdateCustomer {
        merchant_reference_id: &update_customer.customer_id,
        merchant_account: &merchant_account,
        key_store: &key_store,
        key_manager_state,
    };

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    let verify_id_for_update_customer = VerifyIdForUpdateCustomer {
        id: &update_customer.id,
        merchant_account: &merchant_account,
        key_store: &key_store,
        key_manager_state,
    };

    let customer = verify_id_for_update_customer
        .verify_id_and_get_customer_object(db)
        .await?;

    let updated_customer = update_customer
        .request
        .create_domain_model_from_request(
            db,
            &key_store,
            &merchant_account,
            key_manager_state,
            &state,
            &customer,
        )
        .await?;

    update_customer.request.generate_response(&updated_customer)
}

#[async_trait::async_trait]
trait CustomerUpdateBridge {
    async fn create_domain_model_from_request<'a>(
        &'a self,
        db: &'a dyn StorageInterface,
        key_store: &'a domain::MerchantKeyStore,
        merchant_account: &'a domain::MerchantAccount,
        key_manager_state: &'a KeyManagerState,
        state: &'a SessionState,
        domain_customer: &'a domain::Customer,
    ) -> errors::CustomResult<domain::Customer, errors::CustomersErrorResponse>;

    fn generate_response<'a>(
        &'a self,
        customer: &'a domain::Customer,
    ) -> errors::CustomerResponse<customers::CustomerResponse>;
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
struct AddressStructForDbUpdate<'a> {
    update_customer: &'a customers::CustomerUpdateRequest,
    merchant_account: &'a domain::MerchantAccount,
    key_store: &'a domain::MerchantKeyStore,
    key_manager_state: &'a KeyManagerState,
    state: &'a SessionState,
    domain_customer: &'a domain::Customer,
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl AddressStructForDbUpdate<'_> {
    async fn update_address_if_sent(
        &self,
        db: &dyn StorageInterface,
    ) -> errors::CustomResult<Option<domain::Address>, errors::CustomersErrorResponse> {
        let address = if let Some(addr) = &self.update_customer.address {
            match self.domain_customer.address_id.clone() {
                Some(address_id) => {
                    let customer_address: api_models::payments::AddressDetails = addr.clone();
                    let update_address = self
                        .update_customer
                        .get_address_update(
                            self.state,
                            customer_address,
                            self.key_store.key.get_inner().peek(),
                            self.merchant_account.storage_scheme,
                            self.merchant_account.get_id().clone(),
                        )
                        .await
                        .switch()
                        .attach_printable("Failed while encrypting Address while Update")?;
                    Some(
                        db.update_address(
                            self.key_manager_state,
                            address_id,
                            update_address,
                            self.key_store,
                        )
                        .await
                        .switch()
                        .attach_printable(format!(
                            "Failed while updating address: merchant_id: {:?}, customer_id: {:?}",
                            self.merchant_account.get_id(),
                            self.domain_customer.customer_id
                        ))?,
                    )
                }
                None => {
                    let customer_address: api_models::payments::AddressDetails = addr.clone();

                    let address = self
                        .update_customer
                        .get_domain_address(
                            self.state,
                            customer_address,
                            self.merchant_account.get_id(),
                            &self.domain_customer.customer_id,
                            self.key_store.key.get_inner().peek(),
                            self.merchant_account.storage_scheme,
                        )
                        .await
                        .switch()
                        .attach_printable("Failed while encrypting address")?;
                    Some(
                        db.insert_address_for_customers(
                            self.key_manager_state,
                            address,
                            self.key_store,
                        )
                        .await
                        .switch()
                        .attach_printable("Failed while inserting new address")?,
                    )
                }
            }
        } else {
            match &self.domain_customer.address_id {
                Some(address_id) => Some(
                    db.find_address_by_address_id(
                        self.key_manager_state,
                        address_id,
                        self.key_store,
                    )
                    .await
                    .switch()?,
                ),
                None => None,
            }
        };
        Ok(address)
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[derive(Debug)]
struct VerifyIdForUpdateCustomer<'a> {
    merchant_reference_id: &'a id_type::CustomerId,
    merchant_account: &'a domain::MerchantAccount,
    key_store: &'a domain::MerchantKeyStore,
    key_manager_state: &'a KeyManagerState,
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[derive(Debug)]
struct VerifyIdForUpdateCustomer<'a> {
    id: &'a id_type::GlobalCustomerId,
    merchant_account: &'a domain::MerchantAccount,
    key_store: &'a domain::MerchantKeyStore,
    key_manager_state: &'a KeyManagerState,
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl VerifyIdForUpdateCustomer<'_> {
    async fn verify_id_and_get_customer_object(
        &self,
        db: &dyn StorageInterface,
    ) -> Result<domain::Customer, error_stack::Report<errors::CustomersErrorResponse>> {
        let customer = db
            .find_customer_by_customer_id_merchant_id(
                self.key_manager_state,
                self.merchant_reference_id,
                self.merchant_account.get_id(),
                self.key_store,
                self.merchant_account.storage_scheme,
            )
            .await
            .switch()?;

        Ok(customer)
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl VerifyIdForUpdateCustomer<'_> {
    async fn verify_id_and_get_customer_object(
        &self,
        db: &dyn StorageInterface,
    ) -> Result<domain::Customer, error_stack::Report<errors::CustomersErrorResponse>> {
        let customer = db
            .find_customer_by_global_id(
                self.key_manager_state,
                self.id,
                self.merchant_account.get_id(),
                self.key_store,
                self.merchant_account.storage_scheme,
            )
            .await
            .switch()?;

        Ok(customer)
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[async_trait::async_trait]
impl CustomerUpdateBridge for customers::CustomerUpdateRequest {
    async fn create_domain_model_from_request<'a>(
        &'a self,
        db: &'a dyn StorageInterface,
        key_store: &'a domain::MerchantKeyStore,
        merchant_account: &'a domain::MerchantAccount,
        key_manager_state: &'a KeyManagerState,
        state: &'a SessionState,
        domain_customer: &'a domain::Customer,
    ) -> errors::CustomResult<domain::Customer, errors::CustomersErrorResponse> {
        let update_address_for_update_customer = AddressStructForDbUpdate {
            update_customer: self,
            merchant_account,
            key_store,
            key_manager_state,
            state,
            domain_customer,
        };

        let address = update_address_for_update_customer
            .update_address_if_sent(db)
            .await?;

        let key = key_store.key.get_inner().peek();

        let encrypted_data = types::crypto_operation(
            key_manager_state,
            type_name!(domain::Customer),
            types::CryptoOperation::BatchEncrypt(
                domain::FromRequestEncryptableCustomer::to_encryptable(
                    domain::FromRequestEncryptableCustomer {
                        name: self.name.clone(),
                        email: self
                            .email
                            .as_ref()
                            .map(|a| a.clone().expose().switch_strategy()),
                        phone: self.phone.clone(),
                    },
                ),
            ),
            Identifier::Merchant(key_store.merchant_id.clone()),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .switch()?;

        let encryptable_customer =
            domain::FromRequestEncryptableCustomer::from_encryptable(encrypted_data)
                .change_context(errors::CustomersErrorResponse::InternalServerError)?;

        let response = db
            .update_customer_by_customer_id_merchant_id(
                key_manager_state,
                domain_customer.customer_id.to_owned(),
                merchant_account.get_id().to_owned(),
                domain_customer.to_owned(),
                storage::CustomerUpdate::Update {
                    name: encryptable_customer.name,
                    email: encryptable_customer.email.map(|email| {
                        let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> =
                            Encryptable::new(
                                email.clone().into_inner().switch_strategy(),
                                email.into_encrypted(),
                            );
                        encryptable
                    }),
                    phone: Box::new(encryptable_customer.phone),
                    phone_country_code: self.phone_country_code.clone(),
                    metadata: self.metadata.clone(),
                    description: self.description.clone(),
                    connector_customer: Box::new(None),
                    address_id: address.clone().map(|addr| addr.address_id),
                },
                key_store,
                merchant_account.storage_scheme,
            )
            .await
            .switch()?;

        Ok(response)
    }

    fn generate_response<'a>(
        &'a self,
        customer: &'a domain::Customer,
    ) -> errors::CustomerResponse<customers::CustomerResponse> {
        let address = self.get_address();
        let address_details = address.map(api_models::payments::AddressDetails::from);

        Ok(services::ApplicationResponse::Json(
            customers::CustomerResponse::foreign_from((customer.clone(), address_details)),
        ))
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[async_trait::async_trait]
impl CustomerUpdateBridge for customers::CustomerUpdateRequest {
    async fn create_domain_model_from_request<'a>(
        &'a self,
        db: &'a dyn StorageInterface,
        key_store: &'a domain::MerchantKeyStore,
        merchant_account: &'a domain::MerchantAccount,
        key_manager_state: &'a KeyManagerState,
        state: &'a SessionState,
        domain_customer: &'a domain::Customer,
    ) -> errors::CustomResult<domain::Customer, errors::CustomersErrorResponse> {
        let default_billing_address = self.get_default_customer_billing_address();
        let encrypted_customer_billing_address = default_billing_address
            .async_map(|billing_address| {
                create_encrypted_data(key_manager_state, key_store, billing_address)
            })
            .await
            .transpose()
            .change_context(errors::CustomersErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt default customer billing address")?;

        let default_shipping_address = self.get_default_customer_shipping_address();
        let encrypted_customer_shipping_address = default_shipping_address
            .async_map(|shipping_address| {
                create_encrypted_data(key_manager_state, key_store, shipping_address)
            })
            .await
            .transpose()
            .change_context(errors::CustomersErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt default customer shipping address")?;

        let key = key_store.key.get_inner().peek();

        let encrypted_data = types::crypto_operation(
            key_manager_state,
            type_name!(domain::Customer),
            types::CryptoOperation::BatchEncrypt(
                domain::FromRequestEncryptableCustomer::to_encryptable(
                    domain::FromRequestEncryptableCustomer {
                        name: self.name.clone(),
                        email: self
                            .email
                            .as_ref()
                            .map(|a| a.clone().expose().switch_strategy()),
                        phone: self.phone.clone(),
                    },
                ),
            ),
            Identifier::Merchant(key_store.merchant_id.clone()),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .switch()?;

        let encryptable_customer =
            domain::FromRequestEncryptableCustomer::from_encryptable(encrypted_data)
                .change_context(errors::CustomersErrorResponse::InternalServerError)?;

        let response = db
            .update_customer_by_global_id(
                key_manager_state,
                &domain_customer.id,
                domain_customer.to_owned(),
                merchant_account.get_id(),
                storage::CustomerUpdate::Update(Box::new(storage::CustomerGeneralUpdate {
                    name: encryptable_customer.name,
                    email: Box::new(encryptable_customer.email.map(|email| {
                        let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> =
                            Encryptable::new(
                                email.clone().into_inner().switch_strategy(),
                                email.into_encrypted(),
                            );
                        encryptable
                    })),
                    phone: Box::new(encryptable_customer.phone),
                    phone_country_code: self.phone_country_code.clone(),
                    metadata: self.metadata.clone(),
                    description: self.description.clone(),
                    connector_customer: Box::new(None),
                    default_billing_address: encrypted_customer_billing_address.map(Into::into),
                    default_shipping_address: encrypted_customer_shipping_address.map(Into::into),
                    default_payment_method_id: Some(self.default_payment_method_id.clone()),
                    status: None,
                })),
                key_store,
                merchant_account.storage_scheme,
            )
            .await
            .switch()?;
        Ok(response)
    }

    fn generate_response<'a>(
        &'a self,
        customer: &'a domain::Customer,
    ) -> errors::CustomerResponse<customers::CustomerResponse> {
        Ok(services::ApplicationResponse::Json(
            customers::CustomerResponse::foreign_from(customer.clone()),
        ))
    }
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
