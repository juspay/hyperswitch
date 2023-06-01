use async_bb8_diesel::AsyncConnection;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, ExpressionMethods, Table};
use error_stack::{IntoReport, ResultExt};
use storage_models::{
    address::Address,
    customers::Customer,
    merchant_account::MerchantAccount,
    merchant_connector_account::MerchantConnectorAccount,
    query::generics::generic_filter,
    schema::{
        address::dsl as ad_dsl, customers::dsl as cu_dsl, merchant_account::dsl as ma_dsl,
        merchant_connector_account::dsl as mca_dsl,
    },
};

use crate::{
    connection,
    core::errors,
    db::{
        merchant_account::MerchantAccountInterface, merchant_key_store::MerchantKeyStoreInterface,
        MasterKeyInterface,
    },
    services::{self, Store},
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
            merchant_key_store, types,
        },
        storage,
    },
};

pub async fn create_merchant_key_store(
    state: &Store,
    merchant_id: &str,
    key: Vec<u8>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    crate::logger::info!("Trying to create MerchantKeyStore for {}", merchant_id);
    let master_key = state.get_master_key();
    let key_store = merchant_key_store::MerchantKeyStore {
        merchant_id: merchant_id.to_string(),
        key: types::encrypt(key.into(), master_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to decrypt data from key store")?,
        created_at: common_utils::date_time::now(),
    };

    match state.insert_merchant_key_store(key_store).await {
        Ok(_) => Ok(()),
        Err(err) => match err.current_context() {
            errors::StorageError::DatabaseError(f) => match f.current_context() {
                storage_models::errors::DatabaseError::UniqueViolation => Ok(()),
                _ => Err(err.change_context(errors::ApiErrorResponse::InternalServerError)),
            },
            _ => Err(err.change_context(errors::ApiErrorResponse::InternalServerError)),
        },
    }
}

pub async fn encrypt_merchant_account_fields(
    state: &Store,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let conn = connection::pg_connection_write(state)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let merchants: Vec<MerchantAccount> = generic_filter::<
        <MerchantAccount as HasTable>::Table,
        _,
        <<MerchantAccount as HasTable>::Table as Table>::PrimaryKey,
        _,
    >(
        &conn,
        ma_dsl::merchant_id.eq(ma_dsl::merchant_id),
        None,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    for mer in merchants.iter() {
        let key = services::generate_aes256_key()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        create_merchant_key_store(state, &mer.merchant_id, key.to_vec()).await?;
    }
    let mut domain_merchants = Vec::with_capacity(merchants.len());
    for mf in merchants.into_iter() {
        let merchant_id = mf.merchant_id.clone();
        let domain_merchant: domain::MerchantAccount = mf
            .convert(state, &merchant_id, state.get_migration_timestamp())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        domain_merchants.push(domain_merchant);
    }
    for m in domain_merchants {
        let merchant_id = m.merchant_id.clone();
        let updated_merchant_account = storage::MerchantAccountUpdate::Update {
            merchant_name: m.merchant_name.clone(),
            merchant_details: m.merchant_details.clone(),
            return_url: None,
            webhook_details: None,
            sub_merchants_enabled: None,
            parent_merchant_id: None,
            primary_business_details: None,
            enable_payment_response_hash: None,
            payment_response_hash_key: None,
            redirect_to_merchant_with_http_post: None,
            routing_algorithm: None,
            locker_id: None,
            publishable_key: None,
            metadata: None,
            intent_fulfillment_time: None,
            frm_routing_algorithm: None,
        };
        crate::logger::warn!("Started for {}", merchant_id);
        state
            .update_merchant(m, updated_merchant_account)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        encrypt_merchant_connector_account_fields(state, &merchant_id).await?;
        encrypt_customer_fields(state, &merchant_id).await?;
        encrypt_address_fields(state, &merchant_id).await?;
        crate::logger::warn!("Done for {}", merchant_id);
    }

    Ok(())
}

pub async fn encrypt_merchant_connector_account_fields(
    state: &Store,
    merchant_id: &str,
) -> CustomResult<(), errors::ApiErrorResponse> {
    crate::logger::warn!("Updating MerchantConnectorAccount for {}", merchant_id);
    let conn = connection::pg_connection_write(state)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let merchants: Vec<MerchantConnectorAccount> = generic_filter::<
        <MerchantConnectorAccount as HasTable>::Table,
        _,
        <<MerchantConnectorAccount as HasTable>::Table as Table>::PrimaryKey,
        _,
    >(
        &conn,
        mca_dsl::merchant_id.eq(merchant_id.to_string()),
        None,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let mut domain_merchants = Vec::with_capacity(merchants.len());
    for m in merchants.into_iter() {
        let merchant_id = m.merchant_id.clone();
        let domain_merchant: domain::MerchantConnectorAccount = m
            .convert(state, &merchant_id, state.get_migration_timestamp())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        domain_merchants.push(domain_merchant);
    }

    conn.transaction_async::<(), async_bb8_diesel::ConnectionError, _, _>(|conn| async move {
        for m in domain_merchants {
            let updated_merchant_connector_account =
                storage::MerchantConnectorAccountUpdate::Update {
                    merchant_id: None,
                    connector_name: None,
                    connector_type: None,
                    frm_configs: None,
                    test_mode: None,
                    disabled: None,
                    merchant_connector_id: None,
                    payment_methods_enabled: None,
                    metadata: None,
                    connector_account_details: Some(m.connector_account_details.clone()),
                    connector_webhook_details: None,
                };

            Conversion::convert(m)
                .await
                .map_err(|_| {
                    async_bb8_diesel::ConnectionError::Query(
                        diesel::result::Error::QueryBuilderError(
                            "Error while decrypting MerchantConnectorAccount".into(),
                        ),
                    )
                })?
                .update(&conn, updated_merchant_connector_account.into())
                .await
                .map_err(|_| {
                    async_bb8_diesel::ConnectionError::Query(
                        diesel::result::Error::QueryBuilderError(
                            "Error while updating MerchantConnectorAccount".into(),
                        ),
                    )
                })?;
        }
        Ok(())
    })
    .await
    .into_report()
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    crate::logger::warn!(
        "Done: Updating MerchantConnectorAccount for {}",
        merchant_id
    );
    Ok(())
}

pub async fn encrypt_customer_fields(
    state: &Store,
    merchant_id: &str,
) -> CustomResult<(), errors::ApiErrorResponse> {
    crate::logger::warn!("Updating Customer for {}", merchant_id);
    let conn = connection::pg_connection_write(state)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let merchants: Vec<Customer> = generic_filter::<
        <Customer as HasTable>::Table,
        _,
        <<Customer as HasTable>::Table as Table>::PrimaryKey,
        _,
    >(
        &conn,
        cu_dsl::merchant_id.eq(merchant_id.to_string()),
        None,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let mut domain_merchants = Vec::with_capacity(merchants.len());
    for m in merchants.into_iter() {
        let merchant_id = m.merchant_id.clone();
        let domain_merchant: domain::Customer = m
            .convert(state, &merchant_id, state.get_migration_timestamp())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        domain_merchants.push(domain_merchant);
    }

    conn.transaction_async::<(), async_bb8_diesel::ConnectionError, _, _>(|conn| async move {
        for m in domain_merchants {
            let update_customer = storage::CustomerUpdate::Update {
                name: m.name.clone(),
                email: m.email.clone(),
                phone: m.phone.clone(),
                description: None,
                metadata: None,
                phone_country_code: None,
                connector_customer: None,
            };

            Customer::update_by_customer_id_merchant_id(
                &conn,
                m.customer_id.to_string(),
                m.merchant_id.to_string(),
                update_customer.into(),
            )
            .await
            .map_err(|_| {
                async_bb8_diesel::ConnectionError::Query(diesel::result::Error::QueryBuilderError(
                    "Error while updating Customer".into(),
                ))
            })?;
        }

        Ok(())
    })
    .await
    .into_report()
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    crate::logger::warn!("Done: Updating Customer for {}", merchant_id);
    Ok(())
}

pub async fn encrypt_address_fields(
    state: &Store,
    merchant_id: &str,
) -> CustomResult<(), errors::ApiErrorResponse> {
    crate::logger::warn!("Updating Address for {}", merchant_id);
    let conn = connection::pg_connection_write(state)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let merchants: Vec<Address> = generic_filter::<
        <Address as HasTable>::Table,
        _,
        <<Address as HasTable>::Table as Table>::PrimaryKey,
        _,
    >(
        &conn,
        ad_dsl::merchant_id.eq(merchant_id.to_string()),
        None,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let mut domain_merchants = Vec::with_capacity(merchants.len());
    for m in merchants.into_iter() {
        let merchant_id = m.merchant_id.clone();
        let domain_merchant: domain::Address = m
            .convert(state, &merchant_id, state.get_migration_timestamp())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        domain_merchants.push(domain_merchant);
    }

    conn.transaction_async::<(), async_bb8_diesel::ConnectionError, _, _>(|conn| async move {
        for m in domain_merchants {
            let update_address = storage::address::AddressUpdate::Update {
                line1: m.line1.clone(),
                line2: m.line2.clone(),
                line3: m.line3.clone(),
                state: m.state.clone(),
                zip: m.zip.clone(),
                first_name: m.first_name.clone(),
                last_name: m.last_name.clone(),
                phone_number: m.phone_number.clone(),
                city: None,
                country: None,
                country_code: None,
            };

            Address::update_by_address_id(&conn, m.address_id, update_address.into())
                .await
                .map_err(|_| {
                    async_bb8_diesel::ConnectionError::Query(
                        diesel::result::Error::QueryBuilderError(
                            "Error while updating Address".into(),
                        ),
                    )
                })?;
        }
        Ok(())
    })
    .await
    .into_report()
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    crate::logger::warn!("Done: Updating Address for {}", merchant_id);
    Ok(())
}

///
/// # Panics
///
/// The functions runs at the start of the migration and tests, all the functional parts of
/// encryption.
///
#[allow(clippy::unwrap_used)]
pub async fn test_2_step_encryption(store: &Store) {
    use masking::ExposeInterface;
    let (encrypted_merchant_key, master_key) = {
        let master_key = store.get_master_key();
        let merchant_key: Vec<u8> = services::generate_aes256_key().unwrap().into();
        let encrypted_merchant_key =
            types::encrypt::<_, crate::pii::WithType>(merchant_key.into(), master_key)
                .await
                .unwrap()
                .into_encrypted();
        let encrypted_merchant_key =
            storage_models::encryption::Encryption::new(encrypted_merchant_key);
        (encrypted_merchant_key, master_key)
    };

    let dummy_data = "Hello, World!".to_string();
    let encrypted_dummy_data = storage_models::encryption::Encryption::new(
        types::encrypt::<_, crate::pii::WithType>(
            masking::Secret::new(dummy_data.clone()),
            &types::decrypt::<Vec<u8>, crate::pii::WithType>(
                Some(encrypted_merchant_key.clone()),
                master_key,
                0,
                0,
            )
            .await
            .unwrap()
            .unwrap()
            .into_inner()
            .expose(),
        )
        .await
        .unwrap()
        .into_encrypted(),
    );

    let dummy_data_returned = types::decrypt::<String, crate::pii::WithType>(
        Some(encrypted_dummy_data),
        &types::decrypt::<Vec<u8>, crate::pii::WithType>(
            Some(encrypted_merchant_key),
            master_key,
            0,
            0,
        )
        .await
        .unwrap()
        .unwrap()
        .into_inner()
        .expose(),
        0,
        0,
    )
    .await
    .unwrap()
    .unwrap()
    .into_inner()
    .expose();

    assert!(dummy_data_returned == dummy_data)
}
