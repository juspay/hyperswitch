use common_utils::{errors::CustomResult, ext_traits::AsyncExt};
use diesel::{associations::HasTable, ExpressionMethods, Table};
use error_stack::ResultExt;
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
        address::AddressInterface, customers::CustomerInterface,
        merchant_account::MerchantAccountInterface,
        merchant_connector_account::MerchantConnectorAccountInterface,
        merchant_key_store::MerchantKeyStoreInterface, MasterKeyInterface,
    },
    services::{self, Store},
    types::{
        domain::{
            address, behaviour::ReverseConversion, customer, merchant_account,
            merchant_connector_account, merchant_key_store, types,
        },
        storage,
    },
};

pub async fn crate_merchant_key_store(
    state: &Store,
    merchant_id: &str,
    key: Vec<u8>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let master_key = state.get_master_key();
    let key_store = merchant_key_store::MerchantKeyStore {
        merchant_id: merchant_id.to_string(),
        key: types::encrypt(key.into(), master_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to decrypt data from key store")?,
        created_at: common_utils::date_time::now(),
    };

    state
        .insert_merchant_key_store(key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(())
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

    let mut domain_merchants = Vec::with_capacity(merchants.len());
    for m in merchants.into_iter() {
        let merchant_id = m.merchant_id.clone();
        let domain_merchant: merchant_account::MerchantAccount = m
            .convert(state, &merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        domain_merchants.push(domain_merchant);
    }
    for m in domain_merchants {
        let merchant_id = m.merchant_id.clone();
        let key = services::generate_aes256_key()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        crate_merchant_key_store(state, &merchant_id, key.to_vec()).await?;

        let updated_merchant_account = storage::MerchantAccountUpdate::Update {
            merchant_name: m
                .merchant_name
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), &key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            merchant_details: m
                .merchant_details
                .clone()
                .async_map(|details| types::encrypt(details.into_inner(), &key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            return_url: None,
            webhook_details: None,
            sub_merchants_enabled: None,
            parent_merchant_id: None,
            api_key: None,
            primary_business_details: None,
            enable_payment_response_hash: None,
            payment_response_hash_key: None,
            redirect_to_merchant_with_http_post: None,
            routing_algorithm: None,
            locker_id: None,
            publishable_key: None,
            metadata: None,
        };
        state
            .update_merchant(m, updated_merchant_account)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        encrypt_merchant_connector_account_fields(state, &key, &merchant_id).await?;
        encrypt_customer_fields(state, &key, &merchant_id).await?;
        encrypt_address_fields(state, &key, &merchant_id).await?;
    }

    Ok(())
}

pub async fn encrypt_merchant_connector_account_fields(
    state: &Store,
    key: &[u8],
    merchant_id: &str,
) -> CustomResult<(), errors::ApiErrorResponse> {
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
        let domain_merchant: merchant_connector_account::MerchantConnectorAccount = m
            .convert(state, &merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        domain_merchants.push(domain_merchant);
    }

    for m in domain_merchants {
        let updated_merchant_connector_account = storage::MerchantConnectorAccountUpdate::Update {
            merchant_id: None,
            connector_name: None,
            connector_type: None,
            frm_configs: None,
            test_mode: None,
            disabled: None,
            merchant_connector_id: None,
            payment_methods_enabled: None,
            metadata: None,
            connector_account_details: Some(
                types::encrypt(m.connector_account_details.clone().into_inner(), key)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
            ),
        };
        state
            .update_merchant_connector_account(m, updated_merchant_connector_account.into())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
    }
    Ok(())
}

pub async fn encrypt_customer_fields(
    state: &Store,
    key: &[u8],
    merchant_id: &str,
) -> CustomResult<(), errors::ApiErrorResponse> {
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
        let domain_merchant: customer::Customer = m
            .convert(state, &merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        domain_merchants.push(domain_merchant);
    }

    for m in domain_merchants {
        let update_customer = storage::CustomerUpdate::Update {
            name: m
                .name
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            email: m
                .email
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            phone: m
                .phone
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            description: None,
            metadata: None,
            phone_country_code: None,
        };
        state
            .update_customer_by_customer_id_merchant_id(
                m.merchant_id.to_string(),
                m.customer_id.to_string(),
                update_customer,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
    }
    Ok(())
}

pub async fn encrypt_address_fields(
    state: &Store,
    key: &[u8],
    merchant_id: &str,
) -> CustomResult<(), errors::ApiErrorResponse> {
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
        let domain_merchant: address::Address = m
            .convert(state, &merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        domain_merchants.push(domain_merchant);
    }

    for m in domain_merchants {
        let update_address = storage::address::AddressUpdate::Update {
            line1: m
                .line1
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            line2: m
                .line2
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,

            line3: m
                .line3
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,

            state: m
                .state
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,

            zip: m
                .zip
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,

            first_name: m
                .first_name
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,

            last_name: m
                .last_name
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,

            phone_number: m
                .phone_number
                .clone()
                .async_map(|name| types::encrypt(name.into_inner(), key))
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            city: None,
            country: None,
            country_code: None,
        };
        state
            .update_address(m.address_id.to_string(), update_address)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
    }
    Ok(())
}
