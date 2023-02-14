use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::{
        customers::REDACTED,
        errors::{self, CustomResult},
    },
    types::storage,
};

#[async_trait::async_trait]
pub trait CustomerInterface {
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Option<storage::Customer>, errors::StorageError>;

    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: String,
        merchant_id: String,
        customer: storage::CustomerUpdate,
    ) -> CustomResult<storage::Customer, errors::StorageError>;

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::Customer, errors::StorageError>;

    async fn insert_customer(
        &self,
        customer_data: storage::CustomerNew,
    ) -> CustomResult<storage::Customer, errors::StorageError>;
}

#[async_trait::async_trait]
impl CustomerInterface for Store {
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Option<storage::Customer>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        let maybe_customer = storage::Customer::find_optional_by_customer_id_merchant_id(
            &conn,
            customer_id,
            merchant_id,
        )
        .await
        .map_err(Into::into)
        .into_report()?;
        maybe_customer.map_or(Ok(None), |customer| {
            // in the future, once #![feature(is_some_and)] is stable, we can make this more concise:
            // `if customer.name.is_some_and(|ref name| name == REDACTED) ...`
            match customer.name {
                Some(ref name) if name == REDACTED => Err(errors::StorageError::CustomerRedacted)?,
                _ => Ok(Some(customer)),
            }
        })
    }

    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: String,
        merchant_id: String,
        customer: storage::CustomerUpdate,
    ) -> CustomResult<storage::Customer, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::Customer::update_by_customer_id_merchant_id(
            &conn,
            customer_id,
            merchant_id,
            customer,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::Customer, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        let customer =
            storage::Customer::find_by_customer_id_merchant_id(&conn, customer_id, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()?;
        match customer.name {
            Some(ref name) if name == REDACTED => Err(errors::StorageError::CustomerRedacted)?,
            _ => Ok(customer),
        }
    }

    async fn insert_customer(
        &self,
        customer_data: storage::CustomerNew,
    ) -> CustomResult<storage::Customer, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        customer_data
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
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
    ) -> CustomResult<Option<storage::Customer>, errors::StorageError> {
        let customers = self.customers.lock().await;

        Ok(customers
            .iter()
            .find(|customer| {
                customer.customer_id == customer_id && customer.merchant_id == merchant_id
            })
            .cloned())
    }

    async fn update_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: String,
        _merchant_id: String,
        _customer: storage::CustomerUpdate,
    ) -> CustomResult<storage::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<storage::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_customer(
        &self,
        customer_data: storage::CustomerNew,
    ) -> CustomResult<storage::Customer, errors::StorageError> {
        let mut customers = self.customers.lock().await;
        let customer = storage::Customer {
            #[allow(clippy::as_conversions)]
            id: customers.len() as i32,
            customer_id: customer_data.customer_id,
            merchant_id: customer_data.merchant_id,
            name: customer_data.name,
            email: customer_data.email,
            phone: customer_data.phone,
            phone_country_code: customer_data.phone_country_code,
            description: customer_data.description,
            created_at: common_utils::date_time::now(),
            metadata: customer_data.metadata,
        };
        customers.push(customer.clone());
        Ok(customer)
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
