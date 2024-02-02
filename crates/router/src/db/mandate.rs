use error_stack::{IntoReport, ResultExt};

use super::{MockDb, Store};
use crate::{
    connection,
    core::{errors, errors::CustomResult},
    types::storage::{self, MandateDbExt},
};

#[async_trait::async_trait]
pub trait MandateInterface {
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
    ) -> CustomResult<storage::Mandate, errors::StorageError>;

    async fn find_mandate_by_merchant_id_connector_mandate_id(
        &self,
        merchant_id: &str,
        connector_mandate_id: &str,
    ) -> CustomResult<storage::Mandate, errors::StorageError>;

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError>;

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate: storage::MandateUpdate,
    ) -> CustomResult<storage::Mandate, errors::StorageError>;

    async fn find_mandates_by_merchant_id(
        &self,
        merchant_id: &str,
        mandate_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError>;

    async fn insert_mandate(
        &self,
        mandate: storage::MandateNew,
    ) -> CustomResult<storage::Mandate, errors::StorageError>;
}

#[async_trait::async_trait]
impl MandateInterface for Store {
        /// Asynchronously finds a mandate by the given merchant ID and mandate ID.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string reference representing the merchant ID to search for.
    /// * `mandate_id` - A string reference representing the mandate ID to search for.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `storage::Mandate` if the mandate is found, or an `errors::StorageError` if an error occurs.
    ///
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Mandate::find_by_merchant_id_mandate_id(&conn, merchant_id, mandate_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds a mandate by its merchant ID and connector mandate ID in the storage. Returns a `CustomResult` containing the found `storage::Mandate` or a `StorageError` if an error occurs.
    async fn find_mandate_by_merchant_id_connector_mandate_id(
        &self,
        merchant_id: &str,
        connector_mandate_id: &str,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Mandate::find_by_merchant_id_connector_mandate_id(
            &conn,
            merchant_id,
            connector_mandate_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

        /// Asynchronously finds a mandate by the given merchant ID and customer ID.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - A string slice representing the merchant ID.
    /// * `customer_id` - A string slice representing the customer ID.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `storage::Mandate` if successful, otherwise an `errors::StorageError`.
    /// 
    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Mandate::find_by_merchant_id_customer_id(&conn, merchant_id, customer_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously updates a mandate by the given merchant id and mandate id using the provided mandate update information.
    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate: storage::MandateUpdate,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Mandate::update_by_merchant_id_mandate_id(&conn, merchant_id, mandate_id, mandate)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds mandates by the given merchant ID and mandate constraints.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    /// * `mandate_constraints` - A `MandateListConstraints` object representing the constraints to filter the mandates.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `Mandate` objects if successful, otherwise a `StorageError`.
    /// 
    async fn find_mandates_by_merchant_id(
        &self,
        merchant_id: &str,
        mandate_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Mandate::filter_by_constraints(&conn, merchant_id, mandate_constraints)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously inserts a new mandate into the storage.
    /// 
    /// # Arguments
    /// * `mandate` - A `storage::MandateNew` struct representing the new mandate to be inserted.
    /// 
    /// # Returns
    /// A `CustomResult` containing the inserted `storage::Mandate` or a `StorageError` if the insertion fails.
    /// 
    async fn insert_mandate(
        &self,
        mandate: storage::MandateNew,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        mandate
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl MandateInterface for MockDb {
        /// Asynchronously finds a mandate by the given merchant ID and mandate ID in the storage. Returns a CustomResult with the found mandate if it exists, otherwise returns a StorageError indicating that the mandate was not found.
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.mandates
            .lock()
            .await
            .iter()
            .find(|mandate| mandate.merchant_id == merchant_id && mandate.mandate_id == mandate_id)
            .cloned()
            .ok_or_else(|| errors::StorageError::ValueNotFound("mandate not found".to_string()))
            .map_err(|err| err.into())
    }

        /// Asynchronously finds a mandate by merchant ID and connector mandate ID in the storage. 
    /// Returns a `CustomResult` containing the found `storage::Mandate` if it exists, 
    /// otherwise returns a `StorageError` indicating that the mandate was not found.
    async fn find_mandate_by_merchant_id_connector_mandate_id(
        &self,
        merchant_id: &str,
        connector_mandate_id: &str,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.mandates
            .lock()
            .await
            .iter()
            .find(|mandate| {
                mandate.merchant_id == merchant_id
                    && mandate.connector_mandate_id == Some(connector_mandate_id.to_string())
            })
            .cloned()
            .ok_or_else(|| errors::StorageError::ValueNotFound("mandate not found".to_string()))
            .map_err(|err| err.into())
    }

        /// Asynchronously finds a mandate by merchant ID and customer ID.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    /// * `customer_id` - A reference to a string representing the customer ID.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `storage::Mandate` or a `StorageError`.
    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        return Ok(self
            .mandates
            .lock()
            .await
            .iter()
            .filter(|mandate| {
                mandate.merchant_id == merchant_id && mandate.customer_id == customer_id
            })
            .cloned()
            .collect());
    }

        /// Updates a mandate based on the merchant ID and mandate ID with the provided mandate update.
    /// Returns a result containing the updated mandate or a storage error if the mandate is not found.
    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate_update: storage::MandateUpdate,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        let mut mandates = self.mandates.lock().await;
        match mandates
            .iter_mut()
            .find(|mandate| mandate.merchant_id == merchant_id && mandate.mandate_id == mandate_id)
        {
            Some(mandate) => {
                match mandate_update {
                    storage::MandateUpdate::StatusUpdate { mandate_status } => {
                        mandate.mandate_status = mandate_status;
                    }
                    storage::MandateUpdate::CaptureAmountUpdate { amount_captured } => {
                        mandate.amount_captured = amount_captured;
                    }
                    storage::MandateUpdate::ConnectorReferenceUpdate {
                        connector_mandate_ids,
                    } => {
                        mandate.connector_mandate_ids = connector_mandate_ids;
                    }

                    diesel_models::MandateUpdate::ConnectorMandateIdUpdate {
                        connector_mandate_id,
                        connector_mandate_ids,
                        payment_method_id,
                        original_payment_id,
                    } => {
                        mandate.connector_mandate_ids = connector_mandate_ids;
                        mandate.connector_mandate_id = connector_mandate_id;
                        mandate.payment_method_id = payment_method_id;
                        mandate.original_payment_id = original_payment_id
                    }
                }
                Ok(mandate.clone())
            }
            None => {
                Err(errors::StorageError::ValueNotFound("mandate not found".to_string()).into())
            }
        }
    }

        /// Asynchronously finds mandates by merchant ID based on the provided constraints.
    /// Returns a vector of storage mandates or a storage error.
    async fn find_mandates_by_merchant_id(
        &self,
        merchant_id: &str,
        mandate_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        let mandates = self.mandates.lock().await;
        let mandates_iter = mandates.iter().filter(|mandate| {
            let mut checker = mandate.merchant_id == merchant_id;
            if let Some(created_time) = mandate_constraints.created_time {
                checker &= mandate.created_at == created_time;
            }
            if let Some(created_time_lt) = mandate_constraints.created_time_lt {
                checker &= mandate.created_at < created_time_lt;
            }
            if let Some(created_time_gt) = mandate_constraints.created_time_gt {
                checker &= mandate.created_at > created_time_gt;
            }
            if let Some(created_time_lte) = mandate_constraints.created_time_lte {
                checker &= mandate.created_at <= created_time_lte;
            }
            if let Some(created_time_gte) = mandate_constraints.created_time_gte {
                checker &= mandate.created_at >= created_time_gte;
            }
            if let Some(connector) = &mandate_constraints.connector {
                checker &= mandate.connector == *connector;
            }
            if let Some(mandate_status) = mandate_constraints.mandate_status {
                checker &= mandate.mandate_status == mandate_status;
            }
            checker
        });
    
        let mandates: Vec<storage::Mandate> = if let Some(limit) = mandate_constraints.limit {
            #[allow(clippy::as_conversions)]
            mandates_iter
                .take((if limit < 0 { 0 } else { limit }) as usize)
                .cloned()
                .collect()
        } else {
            mandates_iter.cloned().collect()
        };
        Ok(mandates)
    }

        /// Asynchronously inserts a new mandate into the storage and returns the inserted mandate.
    async fn insert_mandate(
        &self,
        mandate_new: storage::MandateNew,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        let mut mandates = self.mandates.lock().await;
        let mandate = storage::Mandate {
            id: mandates
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
            mandate_id: mandate_new.mandate_id.clone(),
            customer_id: mandate_new.customer_id,
            merchant_id: mandate_new.merchant_id,
            original_payment_id: mandate_new.original_payment_id,
            payment_method_id: mandate_new.payment_method_id,
            mandate_status: mandate_new.mandate_status,
            mandate_type: mandate_new.mandate_type,
            customer_accepted_at: mandate_new.customer_accepted_at,
            customer_ip_address: mandate_new.customer_ip_address,
            customer_user_agent: mandate_new.customer_user_agent,
            network_transaction_id: mandate_new.network_transaction_id,
            previous_attempt_id: mandate_new.previous_attempt_id,
            created_at: mandate_new
                .created_at
                .unwrap_or_else(common_utils::date_time::now),
            mandate_amount: mandate_new.mandate_amount,
            mandate_currency: mandate_new.mandate_currency,
            amount_captured: mandate_new.amount_captured,
            connector: mandate_new.connector,
            connector_mandate_id: mandate_new.connector_mandate_id,
            start_date: mandate_new.start_date,
            end_date: mandate_new.end_date,
            metadata: mandate_new.metadata,
            connector_mandate_ids: mandate_new.connector_mandate_ids,
            merchant_connector_id: mandate_new.merchant_connector_id,
        };
        mandates.push(mandate.clone());
        Ok(mandate)
    }
}
