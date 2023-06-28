use error_stack::IntoReport;
use storage_models::payment_method::PaymentMethodUpdateInternal;

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

    async fn insert_payment_method(
        &self,
        payment_method_new: storage::PaymentMethodNew,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        let mut payment_methods = self.payment_methods.lock().await;

        let payment_method = storage::PaymentMethod {
            #[allow(clippy::as_conversions)]
            id: payment_methods.len() as i32,
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
        };
        payment_methods.push(payment_method.clone());
        Ok(payment_method)
    }

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
