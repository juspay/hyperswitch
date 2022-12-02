use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::{
        api::CreateCustomerRequest,
        storage::{Customer, CustomerNew, CustomerUpdate},
    },
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
    ) -> CustomResult<Option<Customer>, errors::StorageError>;

    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: String,
        merchant_id: String,
        customer: CustomerUpdate,
    ) -> CustomResult<Customer, errors::StorageError>;

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Customer, errors::StorageError>;

    async fn insert_customer(
        &self,
        customer_data: CreateCustomerRequest,
    ) -> CustomResult<Customer, errors::StorageError>;
}

#[async_trait::async_trait]
impl CustomerInterface for super::Store {
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Option<Customer>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Customer::find_optional_by_customer_id_merchant_id(&conn, customer_id, merchant_id).await
    }

    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: String,
        merchant_id: String,
        customer: CustomerUpdate,
    ) -> CustomResult<Customer, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Customer::update_by_customer_id_merchant_id(&conn, customer_id, merchant_id, customer).await
    }

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Customer, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Customer::find_by_customer_id_merchant_id(&conn, customer_id, merchant_id).await
    }

    async fn insert_customer(
        &self,
        customer_data: CustomerNew,
    ) -> CustomResult<Customer, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        customer_data.insert_diesel(&conn).await
    }

    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Customer::delete_by_customer_id_merchant_id(&conn, customer_id, merchant_id).await
    }
}

#[async_trait::async_trait]
impl CustomerInterface for MockDb {
    #[allow(clippy::panic)]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Option<Customer>, errors::StorageError> {
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
        _customer: CustomerUpdate,
    ) -> CustomResult<Customer, errors::StorageError> {
        todo!()
    }

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<Customer, errors::StorageError> {
        todo!()
    }

    #[allow(clippy::panic)]
    async fn insert_customer(
        &self,
        customer_data: CustomerNew,
    ) -> CustomResult<Customer, errors::StorageError> {
        let mut customers = self.customers.lock().await;
        let customer = Customer {
            id: customers.len() as i32,
            customer_id: customer_data.customer_id,
            merchant_id: customer_data.merchant_id,
            name: customer_data.name,
            email: customer_data.email,
            phone: customer_data.phone,
            phone_country_code: customer_data.phone_country_code,
            description: customer_data.description,
            address: customer_data.address,
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
        todo!()
    }
}
