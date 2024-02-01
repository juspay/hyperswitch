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
        /// This method is used to find a customer by their customer ID and merchant ID, with an optional decryption step using the provided key store. It first establishes a connection to the database, then attempts to find the customer based on the given IDs. If a customer is found, their data is decrypted using the key from the key store. If the customer's name is redacted, an error is returned. The method returns a Result containing an Option of the found customer or an error if the operation fails.
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
        /// Asynchronously updates a customer by their ID and merchant ID using the provided customer update data and merchant key store. Returns a custom result containing the updated customer or a storage error.
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

        /// Asynchronously finds a customer by their customer ID and merchant ID using the provided MerchantKeyStore for decryption. 
    /// Returns a result containing the customer details if successful, or a StorageError if an error occurs during the process.
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

        /// Asynchronously retrieves a list of customers belonging to a specified merchant by their merchant ID,
    /// using the provided merchant key store to decrypt the customer data.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - A reference to a string representing the ID of the merchant whose customers should be retrieved.
    /// * `key_store` - A reference to a `MerchantKeyStore` containing the necessary keys for decrypting the customer data.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `Customer` objects belonging to the specified merchant,
    /// or a `StorageError` if an error occurs during the retrieval or decryption process.
    /// 
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

        /// Asynchronously inserts a new customer into the database after encrypting the customer data using the merchant's key from the key store,
    /// and then decrypting the data after insertion. Returns a Result containing the inserted customer or a StorageError if an error occurs.
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

        /// Asynchronously deletes a customer by their customer ID and merchant ID from the database.
    /// 
    /// # Arguments
    /// 
    /// * `customer_id` - The unique identifier of the customer to be deleted.
    /// * `merchant_id` - The unique identifier of the merchant.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a boolean value indicating whether the deletion was successful
    /// or an error of type `errors::StorageError`.
    /// 
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
        /// Asynchronously finds a customer by their customer ID and merchant ID, and optionally returns the customer if found. 
    /// 
    /// # Arguments
    /// 
    /// * `customer_id` - A string slice containing the customer ID to search for.
    /// * `merchant_id` - A string slice containing the merchant ID to search for.
    /// * `key_store` - A reference to the MerchantKeyStore used for converting the customer's data.
    /// 
    /// # Returns
    /// 
    /// * `CustomResult<Option<domain::Customer>, errors::StorageError>` - A custom result that may contain the customer if found, or a StorageError if an error occurs during the process.
    /// 
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

        /// Retrieves a list of customers associated with the given merchant ID, decrypts the customer data using the provided merchant key store, and returns the list of customers. If any decryption errors occur, a StorageError is returned.
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
        /// Asynchronously updates a customer by customer ID and merchant ID in the storage.
    ///
    /// # Arguments
    /// * `_customer_id` - The ID of the customer to be updated.
    /// * `_merchant_id` - The ID of the merchant associated with the customer.
    /// * `_customer` - The updated customer data.
    /// * `_key_store` - The key store for the merchant.
    ///
    /// # Returns
    /// A `CustomResult` containing the updated customer if successful, or a `StorageError` if an error occurs.
    ///
    /// # Errors
    /// This method will return a `StorageError::MockDbError` when the function is implemented for `MockDb`.
    ///
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

        /// Asynchronously finds a customer by their customer ID and merchant ID using the provided merchant key store.
    ///
    /// # Arguments
    ///
    /// * `_customer_id` - The ID of the customer to find
    /// * `_merchant_id` - The ID of the merchant for which the customer is associated
    /// * `_key_store` - The key store used to retrieve merchant keys
    ///
    /// # Returns
    ///
    /// * `Ok(domain::Customer)` - If the customer is found
    /// * `Err(errors::StorageError)` - If there is an error, such as a database error or a mock database error
    ///
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
        /// Asynchronously inserts a new customer into the storage, encrypting the customer data using the provided merchant key store, and returning the inserted customer data or an error if encryption or decryption fails.
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

        /// Deletes a customer by customer ID and merchant ID from the database.
    /// 
    /// # Arguments
    /// 
    /// * `_customer_id` - A string slice representing the customer ID.
    /// * `_merchant_id` - A string slice representing the merchant ID.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a boolean value indicating whether the deletion was successful or an error of type `StorageError`.
    /// 
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
