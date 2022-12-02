use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{Address, AddressNew, AddressUpdate},
};

#[async_trait::async_trait]
pub trait AddressInterface {
    async fn update_address(
        &self,
        address_id: String,
        address: AddressUpdate,
    ) -> CustomResult<Address, errors::StorageError>;
    async fn insert_address(
        &self,
        address: AddressNew,
    ) -> CustomResult<Address, errors::StorageError>;
    async fn find_address(&self, address_id: &str) -> CustomResult<Address, errors::StorageError>;
}

#[async_trait::async_trait]
impl AddressInterface for super::Store {
    async fn find_address(&self, address_id: &str) -> CustomResult<Address, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Address::find_by_address_id(&conn, address_id).await
    }

    async fn update_address(
        &self,
        address_id: String,
        address: AddressUpdate,
    ) -> CustomResult<Address, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        Address::update_by_address_id(&conn, address_id, address).await
    }

    async fn insert_address(
        &self,
        address: AddressNew,
    ) -> CustomResult<Address, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        address.insert(&conn).await
    }
}

#[async_trait::async_trait]
impl AddressInterface for MockDb {
    async fn find_address(&self, address_id: &str) -> CustomResult<Address, errors::StorageError> {
        todo!()
    }

    async fn update_address(
        &self,
        address_id: String,
        address: AddressUpdate,
    ) -> CustomResult<Address, errors::StorageError> {
        todo!()
    }

    async fn insert_address(
        &self,
        address: AddressNew,
    ) -> CustomResult<Address, errors::StorageError> {
        todo!()
    }
}
