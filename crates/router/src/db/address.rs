use common_utils::ext_traits::AsyncExt;
use error_stack::{IntoReport, ResultExt};

use super::{MasterKeyInterface, MockDb, Store};
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
pub trait AddressInterface
where
    domain::Address: Conversion<DstType = storage::Address, NewDstType = storage::AddressNew>,
{
    async fn update_address(
        &self,
        address_id: String,
        address: storage::AddressUpdate,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn insert_address(
        &self,
        address: domain::Address,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn find_address(
        &self,
        address_id: &str,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address: storage::AddressUpdate,
    ) -> CustomResult<Vec<domain::Address>, errors::StorageError>;
}

#[async_trait::async_trait]
impl AddressInterface for Store {
    async fn find_address(
        &self,
        address_id: &str,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Address::find_by_address_id(&conn, address_id)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|address| async {
                let merchant_id = address.merchant_id.clone();
                address
                    .convert(self, &merchant_id, self.get_migration_timestamp())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    async fn update_address(
        &self,
        address_id: String,
        address: storage::AddressUpdate,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Address::update_by_address_id(&conn, address_id, address.into())
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|address| async {
                let merchant_id = address.merchant_id.clone();
                address
                    .convert(self, &merchant_id, self.get_migration_timestamp())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    async fn insert_address(
        &self,
        address: domain::Address,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        address
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|address| async {
                let merchant_id = address.merchant_id.clone();
                address
                    .convert(self, &merchant_id, self.get_migration_timestamp())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address: storage::AddressUpdate,
    ) -> CustomResult<Vec<domain::Address>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Address::update_by_merchant_id_customer_id(
            &conn,
            customer_id,
            merchant_id,
            address.into(),
        )
        .await
        .map_err(Into::into)
        .into_report()
        .async_and_then(|addresses| async {
            let mut output = Vec::with_capacity(addresses.len());
            for address in addresses.into_iter() {
                let merchant_id = address.merchant_id.clone();
                output.push(
                    address
                        .convert(self, &merchant_id, self.get_migration_timestamp())
                        .await
                        .change_context(errors::StorageError::DecryptionError)?,
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
    ) -> CustomResult<domain::Address, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_address(
        &self,
        _address_id: String,
        _address: storage::AddressUpdate,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_address(
        &self,
        _address: domain::Address,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_address_by_merchant_id_customer_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
        _address: storage::AddressUpdate,
    ) -> CustomResult<Vec<domain::Address>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
