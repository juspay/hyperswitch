use common_utils::ext_traits::AsyncExt;
use error_stack::{IntoReport, ResultExt};

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage,
    },
};

#[async_trait::async_trait]
pub trait AddressInterface {
    async fn update_address(
        &self,
        address_id: String,
        address: storage::AddressUpdate,
    ) -> CustomResult<domain::address::Address, errors::StorageError>;

    async fn insert_address(
        &self,
        address: domain::address::Address,
    ) -> CustomResult<domain::address::Address, errors::StorageError>;

    async fn find_address(
        &self,
        address_id: &str,
    ) -> CustomResult<domain::address::Address, errors::StorageError>;

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address: storage::AddressUpdate,
    ) -> CustomResult<Vec<domain::address::Address>, errors::StorageError>;
}

#[async_trait::async_trait]
impl AddressInterface for Store {
    async fn find_address(
        &self,
        address_id: &str,
    ) -> CustomResult<domain::address::Address, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Address::find_by_address_id(&conn, address_id)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|address| async {
                address
                    .convert()
                    .await
                    .change_context(errors::StorageError::DeserializationFailed)
            })
            .await
    }

    async fn update_address(
        &self,
        address_id: String,
        address: storage::AddressUpdate,
    ) -> CustomResult<domain::address::Address, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Address::update_by_address_id(&conn, address_id, address)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|address| async {
                address
                    .convert()
                    .await
                    .change_context(errors::StorageError::DeserializationFailed)
            })
            .await
    }

    async fn insert_address(
        &self,
        address: domain::address::Address,
    ) -> CustomResult<domain::address::Address, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        address
            .construct_new()
            .await
            .change_context(errors::StorageError::DeserializationFailed)?
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|address| async {
                address
                    .convert()
                    .await
                    .change_context(errors::StorageError::DeserializationFailed)
            })
            .await
    }

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address: storage::AddressUpdate,
    ) -> CustomResult<Vec<domain::address::Address>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Address::update_by_merchant_id_customer_id(
            &conn,
            customer_id,
            merchant_id,
            address,
        )
        .await
        .map_err(Into::into)
        .into_report()
        .async_and_then(|addresses| async {
            let mut output = Vec::with_capacity(addresses.len());
            for address in addresses.into_iter() {
                output.push(
                    address
                        .convert()
                        .await
                        .change_context(errors::StorageError::DeserializationFailed)?,
                )
            }
            Ok(output)
        })
        .await
    }
}

#[async_trait::async_trait]
impl AddressInterface for MockDb {
    async fn find_address(
        &self,
        _address_id: &str,
    ) -> CustomResult<domain::address::Address, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_address(
        &self,
        _address_id: String,
        _address: storage::AddressUpdate,
    ) -> CustomResult<domain::address::Address, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_address(
        &self,
        _address: domain::address::Address,
    ) -> CustomResult<domain::address::Address, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_address_by_merchant_id_customer_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
        _address: storage::AddressUpdate,
    ) -> CustomResult<Vec<domain::address::Address>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
