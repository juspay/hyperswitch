use error_stack::IntoReport;

use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    services::Store,
    types::storage::{self, PaymentLinkDbExt},
};

#[async_trait::async_trait]
pub trait PaymentLinkInterface {
    async fn find_payment_link_by_payment_link_id(
        &self,
        payment_link_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError>;

    async fn insert_payment_link(
        &self,
        _payment_link: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError>;

    async fn list_payment_link_by_merchant_id(
        &self,
        merchant_id: &str,
        payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
    ) -> CustomResult<Vec<storage::PaymentLink>, errors::StorageError>;
}

#[async_trait::async_trait]
impl PaymentLinkInterface for Store {
        /// Asynchronously finds a payment link by its payment link ID.
    ///
    /// # Arguments
    ///
    /// * `payment_link_id` - The ID of the payment link to be found.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing a `PaymentLink` if it is found, otherwise returns a `StorageError`.
    ///
    async fn find_payment_link_by_payment_link_id(
        &self,
        payment_link_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentLink::find_link_by_payment_link_id(&conn, payment_link_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously inserts a new payment link into the database using the provided payment link configuration. 
    /// 
    /// # Arguments
    /// 
    /// * `payment_link_config` - The configuration for the new payment link to be inserted.
    /// 
    /// # Returns
    /// 
    /// * A `CustomResult` containing the newly inserted `PaymentLink` if successful, otherwise an `errors::StorageError`.
    /// 
    async fn insert_payment_link(
        &self,
        payment_link_config: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payment_link_config
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously retrieves a list of payment links based on the merchant ID and payment link constraints provided. 
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - The ID of the merchant for which to retrieve payment links.
    /// * `payment_link_constraints` - The constraints to apply when querying for payment links.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `Vec` of `PaymentLink` objects if successful, otherwise an `errors::StorageError`.
    ///
    async fn list_payment_link_by_merchant_id(
        &self,
        merchant_id: &str,
        payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
    ) -> CustomResult<Vec<storage::PaymentLink>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentLink::filter_by_constraints(&conn, merchant_id, payment_link_constraints)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl PaymentLinkInterface for MockDb {
        /// Inserts a new payment link into the database.
    ///
    /// This method takes a `PaymentLinkNew` object as input and attempts to insert it into the database.
    /// If successful, it returns the newly inserted `PaymentLink` object. If it encounters an error,
    /// it returns a `StorageError` indicating the nature of the error.
    ///
    /// # Arguments
    ///
    /// * `payment_link` - The payment link object to be inserted into the database.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing either the newly inserted `PaymentLink` object or a `StorageError`
    /// if an error occurs during the insertion process.
    ///
    async fn insert_payment_link(
        &self,
        _payment_link: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds a payment link by its payment link ID.
    ///
    /// # Arguments
    /// * `_payment_link_id` - The ID of the payment link to find.
    ///
    /// # Returns
    /// * `CustomResult<storage::PaymentLink, errors::StorageError>` - A result containing the found payment link or a storage error.
    ///
    /// # Errors
    /// Returns a `StorageError::MockDbError` if the function is called on a `MockDb`.
    ///
    async fn find_payment_link_by_payment_link_id(
        &self,
        _payment_link_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`x
        Err(errors::StorageError::MockDbError)?
    }


        /// Asynchronously list payment links by merchant id.
    ///
    /// # Arguments
    /// 
    /// * `_merchant_id` - A reference to a string representing the merchant id.
    /// * `_payment_link_constraints` - A struct representing the constraints for listing payment links.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `PaymentLink` or a `StorageError` if an error occurs.
    ///
    /// # Errors
    ///
    /// If a `MockDb` error occurs, a `StorageError` with the specific error will be returned.
    ///
    async fn list_payment_link_by_merchant_id(
        &self,
        _merchant_id: &str,
        _payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
    ) -> CustomResult<Vec<storage::PaymentLink>, errors::StorageError> {
        // TODO: Implement function for `MockDb`x
        Err(errors::StorageError::MockDbError)?
    }
}
