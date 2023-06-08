use error_stack::IntoReport;
use storage_models::mandate::{Mandate, MandateUpdate};
use time::PrimitiveDateTime;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult, StorageError},
    types::{
        storage::{self, MandateDbExt},
        transformers::ForeignInto,
    },
};

#[async_trait::async_trait]
pub trait MandateInterface {
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
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
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        match self
            .mandate_info
            .lock()
            .await
            .iter()
            .find(|mandate| mandate.merchant_id == merchant_id && mandate.mandate_id == mandate_id)
        {
            Some(mandate) => Ok(mandate.clone()),
            None => Err(StorageError::ValueNotFound("mandate not found".to_string()).into()),
        }
    }

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        return Ok(self
            .mandate_info
            .lock()
            .await
            .iter()
            .filter(|mandate| {
                mandate.merchant_id == merchant_id && mandate.customer_id == customer_id
            })
            .cloned()
            .collect());
    }

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate_update: storage::MandateUpdate,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        let mut mandate_info = self.mandate_info.lock().await;
        match mandate_info
            .iter_mut()
            .find(|mandate| mandate.merchant_id == merchant_id && mandate.mandate_id == mandate_id)
        {
            Some(mandate) => {
                match mandate_update {
                    MandateUpdate::StatusUpdate { mandate_status } => {
                        mandate.mandate_status = mandate_status;
                    }
                    MandateUpdate::CaptureAmountUpdate { amount_captured } => {
                        mandate.amount_captured = amount_captured;
                    }
                    MandateUpdate::ConnectorReferenceUpdate {
                        connector_mandate_ids,
                    } => {
                        mandate.connector_mandate_ids = connector_mandate_ids;
                    }
                }
                Ok(mandate.clone())
            }
            None => Err(StorageError::ValueNotFound("mandate not found".to_string()).into()),
        }
        // [#172]: Implement function for `MockDb`
        // Err(errors::StorageError::MockDbError)?
    }

    async fn find_mandates_by_merchant_id(
        &self,
        merchant_id: &str,
        mandate_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        let mandate_info_iter = self.mandate_info.lock().await;
        let mandate_info_iter = mandate_info_iter.iter().filter(|mandate| {
            let mut checker = mandate.merchant_id == merchant_id;
            if let Some(created_time) = mandate_constraints.created_time {
                // filter = filter.filter(dsl::created_at.eq(created_time));
                checker &= mandate.created_at == created_time;
            }
            if let Some(created_time_lt) = mandate_constraints.created_time_lt {
                // filter = filter.filter(dsl::created_at.lt(created_time_lt));
                checker &= mandate.created_at < created_time_lt;
            }
            if let Some(created_time_gt) = mandate_constraints.created_time_gt {
                // filter = filter.filter(dsl::created_at.gt(created_time_gt));
                checker &= mandate.created_at > created_time_gt;
            }
            if let Some(created_time_lte) = mandate_constraints.created_time_lte {
                // filter = filter.filter(dsl::created_at.le(created_time_lte));
                checker &= mandate.created_at <= created_time_lte;
            }
            if let Some(created_time_gte) = mandate_constraints.created_time_gte {
                // filter = filter.filter(dsl::created_at.ge(created_time_gte));
                checker &= mandate.created_at >= created_time_gte;
            }
            if let Some(connector) = &mandate_constraints.connector {
                checker &= mandate.connector == *connector;
                // filter = filter.filter(dsl::connector.eq(connector));
            }
            if let Some(mandate_status) = mandate_constraints.mandate_status {
                let storage_mandate_status: storage_models::enums::MandateStatus =
                    mandate_status.foreign_into();
                // filter = filter.filter(dsl::mandate_status.eq(storage_mandate_status));
                checker &= mandate.mandate_status == storage_mandate_status;
            }
            checker
        });

        let mandates: Vec<storage::Mandate> = if let Some(limit) = mandate_constraints.limit {
            mandate_info_iter.take(limit).cloned().collect()
        } else {
            mandate_info_iter.cloned().collect()
        };
        Ok(mandates)

        // Err(errors::StorageError::MockDbError)?
        // return Ok(self
        //     .mandate_info
        //     .lock()
        //     .await
        //     .iter()
        //     .filter(|mandate| {
        //         mandate.merchant_id == merchant_id
        //     })
        //     .cloned()
        //     .collect());
    }

    async fn insert_mandate(
        &self,
        mandate_new: storage::MandateNew,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        let mut mandate_info = self.mandate_info.lock().await;
        let mandate = Mandate {
            id: mandate_info.len() as i32,
            mandate_id: mandate_new.mandate_id,
            customer_id: mandate_new.customer_id,
            merchant_id: mandate_new.merchant_id,
            payment_method_id: mandate_new.payment_method_id,
            mandate_status: mandate_new.mandate_status,
            mandate_type: mandate_new.mandate_type,
            customer_accepted_at: mandate_new.customer_accepted_at,
            customer_ip_address: mandate_new.customer_ip_address,
            customer_user_agent: mandate_new.customer_user_agent,
            network_transaction_id: mandate_new.network_transaction_id,
            previous_attempt_id: mandate_new.previous_attempt_id,
            created_at: mandate_new.created_at.unwrap(),
            mandate_amount: mandate_new.mandate_amount,
            mandate_currency: mandate_new.mandate_currency,
            amount_captured: mandate_new.amount_captured,
            connector: mandate_new.connector,
            connector_mandate_id: mandate_new.connector_mandate_id,
            start_date: mandate_new.start_date,
            end_date: mandate_new.end_date,
            metadata: mandate_new.metadata,
            connector_mandate_ids: mandate_new.connector_mandate_ids,
        };
        mandate_info.push(mandate.clone());
        Ok(mandate)
        // [#172]: Implement function for `MockDb`
        // Err(errors::StorageError::MockDbError)?
    }
}
