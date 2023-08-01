use common_utils::errors::{CustomResult};
use common_utils::pii::REDACTED;
use masking::Secret;
use crate::services::{Store, MockDb};
use crate::cache::Cacheable;
use crate::db::cache::publish_and_redact;
use crate::domain::{MerchantAccountUpdate};
use crate::{self as types, cache, CardInfo, enums, EphemeralKeyNew, EphemeralKey};
use crate::{domain::behaviour::Conversion, connection};
use crate::AddressNew;
use crate::address::AddressUpdateInternal;
use error_stack::{IntoReport, ResultExt};
use crate::merchant_key_store;
use crate::{domain, errors};
use crate::domain::CustomerUpdate;
use crate::db::cache::get_or_populate_redis;
use crate::{ReverseLookupNew, ReverseLookup};

#[async_trait::async_trait]
pub trait ReverseLookupInterface {
    async fn insert_reverse_lookup(
        &self,
        _new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::StorageError>;
    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError>;
}

#[async_trait::async_trait]
impl ReverseLookupInterface for Store {
    async fn insert_reverse_lookup(
        &self,
        new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn).await.map_err(Into::into).into_report()
    }

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        let database_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            ReverseLookup::find_by_lookup_id(id, &conn)
                .await
                .map_err(Into::into)
                .into_report()
        };
        get_or_populate_redis(self, id, database_call).await
    }
}

#[async_trait::async_trait]
impl ReverseLookupInterface for MockDb {
    async fn insert_reverse_lookup(
        &self,
        _new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        Err(errors::StorageError::MockDbError.into())
    }
    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        Err(errors::StorageError::MockDbError.into())
    }
}
