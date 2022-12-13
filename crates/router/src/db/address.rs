use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait AddressInterface {
    async fn update_address(
        &self,
        address_id: String,
        address: storage::AddressUpdate,
    ) -> CustomResult<storage::Address, errors::StorageError>;
    async fn insert_address(
        &self,
        address: storage::AddressNew,
    ) -> CustomResult<storage::Address, errors::StorageError>;
    async fn find_address(
        &self,
        address_id: &str,
    ) -> CustomResult<storage::Address, errors::StorageError>;
}

#[async_trait::async_trait]
impl AddressInterface for Store {
    async fn find_address(
        &self,
        address_id: &str,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        storage::Address::find_by_address_id(&conn, address_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_address(
        &self,
        address_id: String,
        address: storage::AddressUpdate,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        storage::Address::update_by_address_id(&conn, address_id, address)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn insert_address(
        &self,
        address: storage::AddressNew,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        address
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl AddressInterface for MockDb {
    async fn find_address(
        &self,
        _address_id: &str,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        todo!()
    }

    async fn update_address(
        &self,
        _address_id: String,
        _address: storage::AddressUpdate,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        todo!()
    }

    async fn insert_address(
        &self,
        _address: storage::AddressNew,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        todo!()
    }
}
