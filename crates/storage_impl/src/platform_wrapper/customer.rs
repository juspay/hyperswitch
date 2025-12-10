use common_utils::{errors::CustomResult, id_type};
use hyperswitch_domain_models::{
    customer::{Customer, CustomerInterface, CustomerUpdate},
    platform::Provider,
};

use crate::StorageError;

pub async fn insert_customer<S>(
    store: &S,
    provider: &Provider,
    customer_data: Customer,
) -> CustomResult<Customer, StorageError>
where
    S: CustomerInterface<Error = StorageError> + ?Sized,
{
    store
        .insert_customer(
            customer_data,
            provider.get_key_store(),
            provider.get_account().storage_scheme,
        )
        .await
}

#[cfg(feature = "v1")]
pub async fn find_customer_optional_by_customer_id_merchant_id<S>(
    store: &S,
    provider: &Provider,
    customer_id: &id_type::CustomerId,
) -> CustomResult<Option<Customer>, StorageError>
where
    S: CustomerInterface<Error = StorageError> + ?Sized,
{
    store
        .find_customer_optional_by_customer_id_merchant_id(
            customer_id,
            provider.get_account().get_id(),
            provider.get_key_store(),
            provider.get_account().storage_scheme,
        )
        .await
}

#[cfg(feature = "v1")]
pub async fn find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id<S>(
    store: &S,
    provider: &Provider,
    customer_id: &id_type::CustomerId,
) -> CustomResult<Option<Customer>, StorageError>
where
    S: CustomerInterface<Error = StorageError> + ?Sized,
{
    store
        .find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
            customer_id,
            provider.get_account().get_id(),
            provider.get_key_store(),
            provider.get_account().storage_scheme,
        )
        .await
}

#[cfg(feature = "v1")]
pub async fn update_customer_by_customer_id_merchant_id<S>(
    store: &S,
    provider: &Provider,
    customer_id: id_type::CustomerId,
    customer: Customer,
    customer_update: CustomerUpdate,
) -> CustomResult<Customer, StorageError>
where
    S: CustomerInterface<Error = StorageError> + ?Sized,
{
    store
        .update_customer_by_customer_id_merchant_id(
            customer_id,
            provider.get_account().get_id().to_owned(),
            customer,
            customer_update,
            provider.get_key_store(),
            provider.get_account().storage_scheme,
        )
        .await
}

#[cfg(feature = "v2")]
pub async fn update_customer_by_global_id<S>(
    store: &S,
    provider: &Provider,
    customer_id: &id_type::GlobalCustomerId,
    customer: Customer,
    customer_update: CustomerUpdate,
) -> CustomResult<Customer, StorageError>
where
    S: CustomerInterface<Error = StorageError> + ?Sized,
{
    store
        .update_customer_by_global_id(
            customer_id,
            customer,
            customer_update,
            provider.get_key_store(),
            provider.get_account().storage_scheme,
        )
        .await
}
