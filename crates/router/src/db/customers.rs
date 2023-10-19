use common_utils::ext_traits::AsyncExt;
use error_stack::{IntoReport, ResultExt};
use futures::future::try_join_all;
use masking::PeekInterface;
use router_env::{instrument, tracing};

use super::{MockDb, Store};
use crate::{
    connection,
    core::{
        customers::REDACTED,
        errors::{self, CustomResult},
    },
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage,
    },
};

#[async_trait::async_trait]
pub trait CustomerInterface
where
    domain::Customer: Conversion<DstType = storage::Customer, NewDstType = storage::CustomerNew>,
{
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError>;

    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: String,
        merchant_id: String,
        customer: storage::CustomerUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError>;

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError>;

    async fn list_customers_by_merchant_id(
        &self,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Customer>, errors::StorageError>;

    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError>;
}

#[async_trait::async_trait]
impl CustomerInterface for Store {
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let maybe_customer: Option<domain::Customer> =
            storage::Customer::find_optional_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
            )
            .await
            .map_err(Into::into)
            .into_report()?
            .async_map(|c| async {
                c.convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?;
        maybe_customer.map_or(Ok(None), |customer| {
            // in the future, once #![feature(is_some_and)] is stable, we can make this more concise:
            // `if customer.name.is_some_and(|ref name| name == REDACTED) ...`
            match customer.name {
                Some(ref name) if name.peek() == REDACTED => {
                    Err(errors::StorageError::CustomerRedacted)?
                }
                _ => Ok(Some(customer)),
            }
        })
    }

    #[instrument(skip_all)]
    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: String,
        merchant_id: String,
        customer: storage::CustomerUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Customer::update_by_customer_id_merchant_id(
            &conn,
            customer_id,
            merchant_id,
            customer.into(),
        )
        .await
        .map_err(Into::into)
        .into_report()
        .async_and_then(|c| async {
            c.convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)
        })
        .await
    }

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let customer: domain::Customer =
            storage::Customer::find_by_customer_id_merchant_id(&conn, customer_id, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
                .async_and_then(|c| async {
                    c.convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await?;
        match customer.name {
            Some(ref name) if name.peek() == REDACTED => {
                Err(errors::StorageError::CustomerRedacted)?
            }
            _ => Ok(customer),
        }
    }

    async fn list_customers_by_merchant_id(
        &self,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Customer>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        let encrypted_customers = storage::Customer::list_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()?;

        let customers = try_join_all(encrypted_customers.into_iter().map(
            |encrypted_customer| async {
                encrypted_customer
                    .convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            },
        ))
        .await?;

        Ok(customers)
    }

    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        customer_data
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|c| async {
                c.convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Customer::delete_by_customer_id_merchant_id(&conn, customer_id, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl CustomerInterface for MockDb {
    #[allow(clippy::panic)]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        let customers = self.customers.lock().await;
        let customer = customers
            .iter()
            .find(|customer| {
                customer.customer_id == customer_id && customer.merchant_id == merchant_id
            })
            .cloned();
        customer
            .async_map(|c| async {
                c.convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()
    }

    async fn list_customers_by_merchant_id(
        &self,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Customer>, errors::StorageError> {
        let customers = self.customers.lock().await;

        let customers = try_join_all(
            customers
                .iter()
                .filter(|customer| customer.merchant_id == merchant_id)
                .map(|customer| async {
                    customer
                        .to_owned()
                        .convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                }),
        )
        .await?;

        Ok(customers)
    }

    #[instrument(skip_all)]
    async fn update_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: String,
        _merchant_id: String,
        _customer: storage::CustomerUpdate,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let mut customers = self.customers.lock().await;

        let customer = Conversion::convert(customer_data)
            .await
            .change_context(errors::StorageError::EncryptionError)?;

        customers.push(customer.clone());

        customer
            .convert(key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
