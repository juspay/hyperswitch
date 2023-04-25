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
        address_id: &str,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        match self
            .addresses
            .lock()
            .await
            .iter()
            .find(|address| address.address_id == address_id)
        {
            Some(address) => return Ok(address.clone()),
            None => {
                return Err(
                    errors::StorageError::ValueNotFound("address not found".to_string()).into(),
                )
            }
        }
    }

    #[allow(clippy::unwrap_used)]
    async fn update_address(
        &self,
        address_id: String,
        address_update: storage::AddressUpdate,
    ) -> CustomResult<storage::Address, errors::StorageError> {
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
            Some(address_updated) => Ok(address_updated),
            None => {
                return Err(errors::StorageError::ValueNotFound(
                    "cannot find address to update".to_string(),
                )
                .into())
            }
        }
    }

    #[allow(clippy::unwrap_used)]
    async fn insert_address(
        &self,
        address_new: storage::AddressNew,
    ) -> CustomResult<storage::Address, errors::StorageError> {
        let mut addresses = self.addresses.lock().await;
        let now = common_utils::date_time::now();

        let address = storage::Address {
            #[allow(clippy::as_conversions)]
            id: addresses.len() as i32,
            address_id: address_new.address_id,
            city: address_new.city,
            country: address_new.country,
            line1: address_new.line1,
            line2: address_new.line2,
            line3: address_new.line3,
            state: address_new.state,
            zip: address_new.zip,
            first_name: address_new.first_name,
            last_name: address_new.last_name,
            phone_number: address_new.phone_number,
            country_code: address_new.country_code,
            created_at: now,
            modified_at: now,
            customer_id: address_new.customer_id,
            merchant_id: address_new.merchant_id,
        };

        addresses.push(address.clone());

        Ok(address)
    }

    #[allow(clippy::unwrap_used)]
    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address_update: storage::AddressUpdate,
    ) -> CustomResult<Vec<storage::Address>, errors::StorageError> {
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
            Some(address) => Ok(vec![address.clone()]),
            None => {
                return Err(
                    errors::StorageError::ValueNotFound("address not found".to_string()).into(),
                )
            }
        }
    }
}
