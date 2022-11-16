use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::{
        api::CreateCustomerRequest,
        storage::{Customer, CustomerNew, CustomerUpdate},
    },
};

#[async_trait::async_trait]
pub trait ICustomer {
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
impl ICustomer for Store {
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Option<Customer>, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        Customer::find_optional_by_customer_id_merchant_id(&conn, customer_id, merchant_id).await
    }

    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: String,
        merchant_id: String,
        customer: CustomerUpdate,
    ) -> CustomResult<Customer, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        Customer::update_by_customer_id_merchant_id(&conn, customer_id, merchant_id, customer).await
    }

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Customer, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        Customer::find_by_customer_id_merchant_id(&conn, customer_id, merchant_id).await
    }

    async fn insert_customer(
        &self,
        customer_data: CustomerNew,
    ) -> CustomResult<Customer, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        customer_data.insert(&conn).await
    }

    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        Customer::delete_by_customer_id_merchant_id(&conn, customer_id, merchant_id).await
    }
}
