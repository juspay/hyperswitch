use diesel_models::{address::AddressUpdateInternal, enums::MerchantStorageScheme};
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage as storage_types,
    },
};

#[async_trait::async_trait]
pub trait AddressInterface
where
    domain::Address:
        Conversion<DstType = storage_types::Address, NewDstType = storage_types::AddressNew>,
{
    async fn update_address(
        &self,
        address_id: String,
        address: storage_types::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn update_address_for_payments(
        &self,
        this: domain::Address,
        address: domain::AddressUpdate,
        payment_id: String,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn find_address_by_address_id(
        &self,
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn insert_address_for_payments(
        &self,
        payment_id: &str,
        address: domain::Address,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn insert_address_for_customers(
        &self,
        address: domain::Address,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn find_address_by_merchant_id_payment_id_address_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Address, errors::StorageError>;

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address: storage_types::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Address>, errors::StorageError>;
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use common_utils::ext_traits::AsyncExt;
    use error_stack::{IntoReport, ResultExt};
    use router_env::{instrument, tracing};

    use super::AddressInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::{
            domain::{
                self,
                behaviour::{Conversion, ReverseConversion},
            },
            storage::{self as storage_types, enums::MerchantStorageScheme},
        },
    };
    #[async_trait::async_trait]
    impl AddressInterface for Store {
        async fn find_address_by_address_id(
            &self,
            address_id: &str,
            key_store: &domain::MerchantKeyStore,
        ) -> CustomResult<domain::Address, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Address::find_by_address_id(&conn, address_id)
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

        async fn find_address_by_merchant_id_payment_id_address_id(
            &self,
            merchant_id: &str,
            payment_id: &str,
            address_id: &str,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Address, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Address::find_by_merchant_id_payment_id_address_id(
                &conn,
                merchant_id,
                payment_id,
                address_id,
            )
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

        #[instrument(skip_all)]
        async fn update_address(
            &self,
            address_id: String,
            address: storage_types::AddressUpdate,
            key_store: &domain::MerchantKeyStore,
        ) -> CustomResult<domain::Address, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Address::update_by_address_id(&conn, address_id, address.into())
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

        async fn update_address_for_payments(
            &self,
            this: domain::Address,
            address_update: domain::AddressUpdate,
            _payment_id: String,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Address, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            let address = Conversion::convert(this)
                .await
                .change_context(errors::StorageError::EncryptionError)?;
            address
                .update(&conn, address_update.into())
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

        async fn insert_address_for_payments(
            &self,
            _payment_id: &str,
            address: domain::Address,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
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

        async fn insert_address_for_customers(
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
            address: storage_types::AddressUpdate,
            key_store: &domain::MerchantKeyStore,
        ) -> CustomResult<Vec<domain::Address>, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Address::update_by_merchant_id_customer_id(
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
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::ext_traits::AsyncExt;
    use diesel_models::{enums::MerchantStorageScheme, AddressUpdateInternal};
    use error_stack::{IntoReport, ResultExt};
    use redis_interface::HsetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{kv_wrapper, KvOperation};

    use super::AddressInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::{
            domain::{
                self,
                behaviour::{Conversion, ReverseConversion},
            },
            storage::{self as storage_types, kv},
        },
        utils::db_utils,
    };
    #[async_trait::async_trait]
    impl AddressInterface for Store {
        async fn find_address_by_address_id(
            &self,
            address_id: &str,
            key_store: &domain::MerchantKeyStore,
        ) -> CustomResult<domain::Address, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Address::find_by_address_id(&conn, address_id)
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

        async fn find_address_by_merchant_id_payment_id_address_id(
            &self,
            merchant_id: &str,
            payment_id: &str,
            address_id: &str,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Address, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::Address::find_by_merchant_id_payment_id_address_id(
                    &conn,
                    merchant_id,
                    payment_id,
                    address_id,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            let address = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = format!("mid_{}_pid_{}", merchant_id, payment_id);
                    let field = format!("add_{}", address_id);
                    db_utils::try_redis_get_else_try_database_get(
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<diesel_models::Address>::HGet(&field),
                                key,
                            )
                            .await?
                            .try_into_hget()
                        },
                        database_call,
                    )
                    .await
                }
            }?;
            address
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[instrument(skip_all)]
        async fn update_address(
            &self,
            address_id: String,
            address: storage_types::AddressUpdate,
            key_store: &domain::MerchantKeyStore,
        ) -> CustomResult<domain::Address, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Address::update_by_address_id(&conn, address_id, address.into())
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

        async fn update_address_for_payments(
            &self,
            this: domain::Address,
            address_update: domain::AddressUpdate,
            payment_id: String,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Address, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            let address = Conversion::convert(this)
                .await
                .change_context(errors::StorageError::EncryptionError)?;
            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => {
                    address
                        .update(&conn, address_update.into())
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
                MerchantStorageScheme::RedisKv => {
                    let key = format!("mid_{}_pid_{}", address.merchant_id.clone(), payment_id);
                    let field = format!("add_{}", address.address_id);
                    let updated_address = AddressUpdateInternal::from(address_update.clone())
                        .create_address(address.clone());
                    let redis_value = serde_json::to_string(&updated_address)
                        .into_report()
                        .change_context(errors::StorageError::KVError)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::AddressUpdate(Box::new(
                                kv::AddressUpdateMems {
                                    orig: address,
                                    update_data: address_update.into(),
                                },
                            )),
                        },
                    };

                    kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::Hset::<storage_types::Address>(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        &key,
                    )
                    .await
                    .change_context(errors::StorageError::KVError)?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    updated_address
                        .convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                }
            }
        }

        async fn insert_address_for_payments(
            &self,
            payment_id: &str,
            address: domain::Address,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Address, errors::StorageError> {
            let address_new = address
                .clone()
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)?;
            let merchant_id = address_new.merchant_id.clone();
            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    address_new
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
                MerchantStorageScheme::RedisKv => {
                    let key = format!("mid_{}_pid_{}", merchant_id, payment_id);
                    let field = format!("add_{}", &address_new.address_id);
                    let created_address = diesel_models::Address {
                        id: Some(0i32),
                        address_id: address_new.address_id.clone(),
                        city: address_new.city.clone(),
                        country: address_new.country,
                        line1: address_new.line1.clone(),
                        line2: address_new.line2.clone(),
                        line3: address_new.line3.clone(),
                        state: address_new.state.clone(),
                        zip: address_new.zip.clone(),
                        first_name: address_new.first_name.clone(),
                        last_name: address_new.last_name.clone(),
                        phone_number: address_new.phone_number.clone(),
                        country_code: address_new.country_code.clone(),
                        created_at: address_new.created_at,
                        modified_at: address_new.modified_at,
                        customer_id: address_new.customer_id.clone(),
                        merchant_id: address_new.merchant_id.clone(),
                        payment_id: address_new.payment_id.clone(),
                        updated_by: storage_scheme.to_string(),
                    };

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: kv::Insertable::Address(Box::new(address_new)),
                        },
                    };

                    match kv_wrapper::<diesel_models::Address, _, _>(
                        self,
                        KvOperation::HSetNx::<diesel_models::Address>(
                            &field,
                            &created_address,
                            redis_entry,
                        ),
                        &key,
                    )
                    .await
                    .change_context(errors::StorageError::KVError)?
                    .try_into_hsetnx()
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "address",
                            key: Some(created_address.address_id),
                        })
                        .into_report(),
                        Ok(HsetnxReply::KeySet) => Ok(created_address
                            .convert(key_store.key.get_inner())
                            .await
                            .change_context(errors::StorageError::DecryptionError)?),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }
        }

        async fn insert_address_for_customers(
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
            address: storage_types::AddressUpdate,
            key_store: &domain::MerchantKeyStore,
        ) -> CustomResult<Vec<domain::Address>, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Address::update_by_merchant_id_customer_id(
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
}

#[async_trait::async_trait]
impl AddressInterface for MockDb {
    async fn find_address_by_address_id(
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

    async fn find_address_by_merchant_id_payment_id_address_id(
        &self,
        _merchant_id: &str,
        _payment_id: &str,
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
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

    #[instrument(skip_all)]
    async fn update_address(
        &self,
        address_id: String,
        address_update: storage_types::AddressUpdate,
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

    async fn update_address_for_payments(
        &self,
        this: domain::Address,
        address_update: domain::AddressUpdate,
        _payment_id: String,
        key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        match self
            .addresses
            .lock()
            .await
            .iter_mut()
            .find(|address| address.address_id == this.address_id)
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

    async fn insert_address_for_payments(
        &self,
        _payment_id: &str,
        address_new: domain::Address,
        key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
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

    async fn insert_address_for_customers(
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
        address_update: storage_types::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Address>, errors::StorageError> {
        match self
            .addresses
            .lock()
            .await
            .iter_mut()
            .find(|address| {
                address.customer_id == Some(customer_id.to_string())
                    && address.merchant_id == merchant_id
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
