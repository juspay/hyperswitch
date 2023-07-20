use common_utils::ext_traits::AsyncExt;
use diesel_models::address::AddressUpdateInternal;
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
pub trait AddressInterface
where
    domain::Address: Conversion<DstType = storage::Address, NewDstType = storage::AddressNew>,
{
    async fn update_address(
        &self,
        address_id: String,
        address: storage::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn insert_address(
        &self,
        address: domain::Address,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn find_address(
        &self,
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address: storage::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Address>, errors::StorageError>;
}

#[async_trait::async_trait]
impl AddressInterface for Store {
    async fn find_address(
        &self,
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Address::find_by_address_id(&conn, address_id)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|address| async {
                address
                    .convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    async fn update_address(
        &self,
        address_id: String,
        address: storage::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Address::update_by_address_id(&conn, address_id, address.into())
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|address| async {
                address
                    .convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    async fn insert_address(
        &self,
        address: domain::Address,
        key_store: &domain::MerchantKeyStore,
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
                address
                    .convert(key_store.key.get_inner())
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
        key_store: &domain::MerchantKeyStore,
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
                output.push(
                    address
                        .convert(key_store.key.get_inner())
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
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        match self
            .addresses
            .lock()
            .await
            .iter()
            .find(|address| address.address_id == address_id)
        {
            Some(address) => address
                .clone()
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError),
            None => {
                return Err(
                    errors::StorageError::ValueNotFound("address not found".to_string()).into(),
                )
            }
        }
    }

    async fn update_address(
        &self,
        address_id: String,
        address_update: storage::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        match self
            .addresses
            .lock()
            .await
            .iter_mut()
            .find(|address| address.address_id == address_id)
            .map(|a| {
                let address_updated =
                    AddressUpdateInternal::from(address_update).create_address(a.clone());
                *a = address_updated.clone();
                address_updated
            }) {
            Some(address_updated) => address_updated
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find address to update".to_string(),
            )
            .into()),
        }
    }

    async fn insert_address(
        &self,
        address_new: domain::Address,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        let mut addresses = self.addresses.lock().await;

        let address = Conversion::convert(address_new)
            .await
            .change_context(errors::StorageError::EncryptionError)?;

        addresses.push(address.clone());

        address
            .convert(key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address_update: storage::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Address>, errors::StorageError> {
        match self
            .addresses
            .lock()
            .await
            .iter_mut()
            .find(|address| {
                address.customer_id == customer_id && address.merchant_id == merchant_id
            })
            .map(|a| {
                let address_updated =
                    AddressUpdateInternal::from(address_update).create_address(a.clone());
                *a = address_updated.clone();
                address_updated
            }) {
            Some(address) => {
                let address: domain::Address = address
                    .convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)?;
                Ok(vec![address])
            }
            None => {
                Err(errors::StorageError::ValueNotFound("address not found".to_string()).into())
            }
        }
    }
}
