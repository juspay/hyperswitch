use diesel_models::payment_method::PaymentMethodUpdateInternal;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

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

    async fn find_payment_method_by_locker_id(
        &self,
        locker_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError>;

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError>;

    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError>;

    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError>;

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
    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
    async fn find_payment_method_by_locker_id(
        &self,
        locker_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::find_by_locker_id(&conn, locker_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::get_count_by_customer_id_merchant_id_status(
            &conn,
            customer_id,
            merchant_id,
            status,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::find_by_customer_id_merchant_id(
            &conn,
            customer_id,
            merchant_id,
            limit,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::find_by_customer_id_merchant_id_status(
            &conn,
            customer_id,
            merchant_id,
            status,
            limit,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

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

    async fn find_payment_method_by_locker_id(
        &self,
        locker_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_method = payment_methods
            .iter()
            .find(|pm| pm.locker_id == Some(locker_id.to_string()))
            .cloned();

        match payment_method {
            Some(pm) => Ok(pm),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method".to_string(),
            )
            .into()),
        }
    }

    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let count = payment_methods
            .iter()
            .filter(|pm| {
                pm.customer_id == customer_id
                    && pm.merchant_id == merchant_id
                    && pm.status == status
            })
            .count();
        count
            .try_into()
            .into_report()
            .change_context(errors::StorageError::MockDbError)
    }

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
            locker_id: payment_method_new.locker_id,
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
            last_used_at: payment_method_new.last_used_at,
            connector_mandate_details: payment_method_new.connector_mandate_details,
            customer_acceptance: payment_method_new.customer_acceptance,
            status: payment_method_new.status,
            network_transaction_id: payment_method_new.network_transaction_id,
        };
        payment_methods.push(payment_method.clone());
        Ok(payment_method)
    }

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
        _limit: Option<i64>,
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

    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_methods_found: Vec<storage::PaymentMethod> = payment_methods
            .iter()
            .filter(|pm| {
                pm.customer_id == customer_id
                    && pm.merchant_id == merchant_id
                    && pm.status == status
            })
            .cloned()
            .collect();

        if payment_methods_found.is_empty() {
            Err(errors::StorageError::ValueNotFound(
                "cannot find payment methods".to_string(),
            ))
            .into_report()
        } else {
            Ok(payment_methods_found)
        }
    }

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

    async fn update_payment_method(
        &self,
        payment_method: storage::PaymentMethod,
        payment_method_update: storage::PaymentMethodUpdate,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let pm_update_res = self
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
            });

        match pm_update_res {
            Some(result) => Ok(result),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method to update".to_string(),
            )
            .into()),
        }
    }
}
