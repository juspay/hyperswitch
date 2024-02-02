use diesel_models::payment_method::PaymentMethodUpdateInternal;
use error_stack::{IntoReport, ResultExt};

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait PaymentMethodInterface {
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError>;

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError>;

    async fn insert_payment_method(
        &self,
        payment_method_new: storage::PaymentMethodNew,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError>;

    async fn update_payment_method(
        &self,
        payment_method: storage::PaymentMethod,
        payment_method_update: storage::PaymentMethodUpdate,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError>;

    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError>;
}

#[async_trait::async_trait]
impl PaymentMethodInterface for Store {
        /// Asynchronously finds a payment method by its ID.
    ///
    /// # Arguments
    ///
    /// * `payment_method_id` - The ID of the payment method to find.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `storage::PaymentMethod` if the payment method is found, otherwise an `errors::StorageError`.
    ///
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::find_by_payment_method_id(&conn, payment_method_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously inserts a new payment method into the storage.
    ///
    /// # Arguments
    ///
    /// * `payment_method_new` - The new payment method to be inserted into the storage.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing the inserted `PaymentMethod` if successful, otherwise returns a `StorageError`.
    ///
    async fn insert_payment_method(
        &self,
        payment_method_new: storage::PaymentMethodNew,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payment_method_new
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Updates the given payment method with the provided payment method update in the database.
    ///
    /// # Arguments
    ///
    /// * `payment_method` - The payment method to be updated.
    /// * `payment_method_update` - The updated information for the payment method.
    ///
    /// # Returns
    ///
    /// The updated payment method if successful, otherwise returns a StorageError.
    ///
    async fn update_payment_method(
        &self,
        payment_method: storage::PaymentMethod,
        payment_method_update: storage::PaymentMethodUpdate,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payment_method
            .update_with_payment_method_id(&conn, payment_method_update)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds a list of payment methods belonging to a specific customer and merchant.
    ///
    /// # Arguments
    ///
    /// * `customer_id` - The ID of the customer whose payment methods are to be retrieved.
    /// * `merchant_id` - The ID of the merchant where the payment methods are associated.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `storage::PaymentMethod` if successful, otherwise an `errors::StorageError`.
    ///
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::find_by_customer_id_merchant_id(&conn, customer_id, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously deletes a payment method by merchant ID and payment method ID.
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PaymentMethod::delete_by_merchant_id_payment_method_id(
            &conn,
            merchant_id,
            payment_method_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }
}

#[async_trait::async_trait]
impl PaymentMethodInterface for MockDb {
        /// Asynchronously finds a payment method by its ID in the storage.
    /// If the payment method is found, it returns a `CustomResult` containing the payment method.
    /// If the payment method is not found, it returns a `CustomResult` containing a `StorageError` indicating that the value was not found.
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_method = payment_methods
            .iter()
            .find(|pm| pm.payment_method_id == payment_method_id)
            .cloned();

        match payment_method {
            Some(pm) => Ok(pm),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method".to_string(),
            )
            .into()),
        }
    }

        /// Asynchronously inserts a new payment method into the storage. Returns a result containing the newly inserted payment method or a `StorageError`.
    async fn insert_payment_method(
        &self,
        payment_method_new: storage::PaymentMethodNew,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let mut payment_methods = self.payment_methods.lock().await;

        let payment_method = storage::PaymentMethod {
            id: payment_methods
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
            customer_id: payment_method_new.customer_id,
            merchant_id: payment_method_new.merchant_id,
            payment_method_id: payment_method_new.payment_method_id,
            accepted_currency: payment_method_new.accepted_currency,
            scheme: payment_method_new.scheme,
            token: payment_method_new.token,
            cardholder_name: payment_method_new.cardholder_name,
            issuer_name: payment_method_new.issuer_name,
            issuer_country: payment_method_new.issuer_country,
            payer_country: payment_method_new.payer_country,
            is_stored: payment_method_new.is_stored,
            swift_code: payment_method_new.swift_code,
            direct_debit_token: payment_method_new.direct_debit_token,
            created_at: payment_method_new.created_at,
            last_modified: payment_method_new.last_modified,
            payment_method: payment_method_new.payment_method,
            payment_method_type: payment_method_new.payment_method_type,
            payment_method_issuer: payment_method_new.payment_method_issuer,
            payment_method_issuer_code: payment_method_new.payment_method_issuer_code,
            metadata: payment_method_new.metadata,
            payment_method_data: payment_method_new.payment_method_data,
        };
        payment_methods.push(payment_method.clone());
        Ok(payment_method)
    }

        /// Asynchronously finds payment methods by customer and merchant IDs from the storage. It searches for payment methods that match the provided customer ID and merchant ID, and returns a vector of the found payment methods. If no payment methods are found, it returns a custom result with a storage error indicating that the payment method could not be found.
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_methods_found: Vec<storage::PaymentMethod> = payment_methods
            .iter()
            .filter(|pm| pm.customer_id == customer_id && pm.merchant_id == merchant_id)
            .cloned()
            .collect();

        if payment_methods_found.is_empty() {
            Err(
                errors::StorageError::ValueNotFound("cannot find payment method".to_string())
                    .into(),
            )
        } else {
            Ok(payment_methods_found)
        }
    }

        /// Asynchronously deletes a payment method by the given merchant_id and payment_method_id.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string representing the merchant ID
    /// * `payment_method_id` - A reference to a string representing the payment method ID
    ///
    /// # Returns
    ///
    /// * `CustomResult<storage::PaymentMethod, errors::StorageError>` - A result containing the deleted payment method if successful, or a storage error if the payment method is not found
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let mut payment_methods = self.payment_methods.lock().await;
        match payment_methods.iter().position(|pm| {
            pm.merchant_id == merchant_id && pm.payment_method_id == payment_method_id
        }) {
            Some(index) => {
                let deleted_payment_method = payment_methods.remove(index);
                Ok(deleted_payment_method)
            }
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method to delete".to_string(),
            )
            .into()),
        }
    }

        /// Asynchronously updates the payment method using the provided payment method and payment method update.
    async fn update_payment_method(
        &self,
        payment_method: storage::PaymentMethod,
        payment_method_update: storage::PaymentMethodUpdate,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        match self
            .payment_methods
            .lock()
            .await
            .iter_mut()
            .find(|pm| pm.id == payment_method.id)
            .map(|pm| {
                let payment_method_updated =
                    PaymentMethodUpdateInternal::from(payment_method_update)
                        .create_payment_method(pm.clone());
                *pm = payment_method_updated.clone();
                payment_method_updated
            }) {
            Some(result) => Ok(result),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method to update".to_string(),
            )
            .into()),
        }
    }
}
