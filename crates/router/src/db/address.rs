use error_stack::IntoReport;
use storage_models::address::AddressUpdateInternal;

use super::{MockDb, Store};
use crate::{
    connection,
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

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address: storage::AddressUpdate,
    ) -> CustomResult<Vec<storage::Address>, errors::StorageError>;
}

#[async_trait::async_trait]
impl AddressInterface for Store {
    async fn find_address(
        &self,
        address_id: &str,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
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
        let conn = connection::pg_connection_write(self).await?;
        storage::Address::update_by_address_id(&conn, address_id, address)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn insert_address(
        &self,
        address: storage::AddressNew,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        address
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address: storage::AddressUpdate,
    ) -> CustomResult<Vec<storage::Address>, errors::StorageError> {
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
    }
}

#[async_trait::async_trait]
impl AddressInterface for MockDb {
    #[allow(clippy::unwrap_used)]
    async fn find_address(
        &self,
        _address_id: &str,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        let addresses = self.addresses.lock().await;
        Ok(addresses
            .iter()
            .find(|address| address.address_id == _address_id)
            .unwrap()
            .clone())
    }

    #[allow(clippy::unwrap_used)]
    async fn update_address(
        &self,
        _address_id: String,
        _address: storage::AddressUpdate,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        let mut addresses = self.addresses.lock().await;
        let address = addresses
            .iter_mut()
            .find(|address| address.address_id == _address_id)
            .unwrap();

        let address_updated = AddressUpdateInternal::from(_address).create_address(address.clone());
        *address = address_updated.clone();
        Ok(address_updated)
    }

    #[allow(clippy::unwrap_used)]
    async fn insert_address(
        &self,
        _address: storage::AddressNew,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        let mut addresses = self.addresses.lock().await;
        let now = common_utils::date_time::now();

        let address = storage::Address {
            #[allow(clippy::as_conversions)]
            id: addresses.len() as i32,
            address_id: _address.address_id,
            city: _address.city,
            country: _address.country,
            line1: _address.line1,
            line2: _address.line2,
            line3: _address.line3,
            state: _address.state,
            zip: _address.zip,
            first_name: _address.first_name,
            last_name: _address.last_name,
            phone_number: _address.phone_number,
            country_code: _address.country_code,
            created_at: now,
            modified_at: now,
            customer_id: _address.customer_id,
            merchant_id: _address.merchant_id,
        };

        addresses.push(address.clone());

        Ok(address)
    }

    #[allow(clippy::unwrap_used)]
    async fn update_address_by_merchant_id_customer_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
        _address: storage::AddressUpdate,
    ) -> CustomResult<Vec<storage::Address>, errors::StorageError> {
        let addresses = self.addresses.lock().await;

        let address = addresses
            .iter()
            .find(|address| {
                address.customer_id == _customer_id && address.merchant_id == _merchant_id
            })
            .cloned()
            .map(|address| vec![address])
            .unwrap();

        Ok(address)
    }
}
