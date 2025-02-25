use std::str::FromStr;

use api_models::enums::PayoutConnectors;
use common_utils::{errors::CustomResult, ext_traits::Encode, fallback_reverse_lookup_not_found};
use diesel_models::{
    enums::MerchantStorageScheme,
    kv,
    payout_attempt::{
        PayoutAttempt as DieselPayoutAttempt, PayoutAttemptNew as DieselPayoutAttemptNew,
        PayoutAttemptUpdate as DieselPayoutAttemptUpdate,
    },
    reverse_lookup::ReverseLookup,
    ReverseLookupNew,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors,
    payouts::{
        payout_attempt::{
            PayoutAttempt, PayoutAttemptInterface, PayoutAttemptNew, PayoutAttemptUpdate,
            PayoutListFilters,
        },
        payouts::Payouts,
    },
};
use redis_interface::HsetnxReply;
use router_env::{instrument, logger, tracing};

use crate::{
    diesel_error_to_data_error,
    errors::RedisErrorExt,
    kv_router_store::KVRouterStore,
    lookup::ReverseLookupInterface,
    redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
    utils::{self, pg_connection_read, pg_connection_write},
    DataModelExt, DatabaseStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> PayoutAttemptInterface for KVRouterStore<T> {
    #[instrument(skip_all)]
    async fn insert_payout_attempt(
        &self,
        new_payout_attempt: PayoutAttemptNew,
        payouts: &Payouts,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPayoutAttempt>(
            self,
            storage_scheme,
            Op::Insert,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .insert_payout_attempt(new_payout_attempt, payouts, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let merchant_id = new_payout_attempt.merchant_id.clone();
                let payout_attempt_id = new_payout_attempt.payout_id.clone();
                let key = PartitionKey::MerchantIdPayoutAttemptId {
                    merchant_id: &merchant_id,
                    payout_attempt_id: &payout_attempt_id,
                };
                let key_str = key.to_string();
                let created_attempt = PayoutAttempt {
                    payout_attempt_id: new_payout_attempt.payout_attempt_id.clone(),
                    payout_id: new_payout_attempt.payout_id.clone(),
                    additional_payout_method_data: new_payout_attempt
                        .additional_payout_method_data
                        .clone(),
                    customer_id: new_payout_attempt.customer_id.clone(),
                    merchant_id: new_payout_attempt.merchant_id.clone(),
                    address_id: new_payout_attempt.address_id.clone(),
                    connector: new_payout_attempt.connector.clone(),
                    connector_payout_id: new_payout_attempt.connector_payout_id.clone(),
                    payout_token: new_payout_attempt.payout_token.clone(),
                    status: new_payout_attempt.status,
                    is_eligible: new_payout_attempt.is_eligible,
                    error_message: new_payout_attempt.error_message.clone(),
                    error_code: new_payout_attempt.error_code.clone(),
                    business_country: new_payout_attempt.business_country,
                    business_label: new_payout_attempt.business_label.clone(),
                    created_at: new_payout_attempt.created_at,
                    last_modified_at: new_payout_attempt.last_modified_at,
                    profile_id: new_payout_attempt.profile_id.clone(),
                    merchant_connector_id: new_payout_attempt.merchant_connector_id.clone(),
                    routing_info: new_payout_attempt.routing_info.clone(),
                    unified_code: new_payout_attempt.unified_code.clone(),
                    unified_message: new_payout_attempt.unified_message.clone(),
                };

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Insert {
                        insertable: Box::new(kv::Insertable::PayoutAttempt(
                            new_payout_attempt.to_storage_model(),
                        )),
                    },
                };

                // Reverse lookup for payout_attempt_id
                let field = format!("poa_{}", created_attempt.payout_attempt_id);
                let reverse_lookup = ReverseLookupNew {
                    lookup_id: format!(
                        "poa_{}_{}",
                        &created_attempt.merchant_id.get_string_repr(),
                        &created_attempt.payout_attempt_id,
                    ),
                    pk_id: key_str.clone(),
                    sk_id: field.clone(),
                    source: "payout_attempt".to_string(),
                    updated_by: storage_scheme.to_string(),
                };
                self.insert_reverse_lookup(reverse_lookup, storage_scheme)
                    .await?;

                match Box::pin(kv_wrapper::<DieselPayoutAttempt, _, _>(
                    self,
                    KvOperation::<DieselPayoutAttempt>::HSetNx(
                        &field,
                        &created_attempt.clone().to_storage_model(),
                        redis_entry,
                    ),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hsetnx()
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                        entity: "payout attempt",
                        key: Some(key_str),
                    }
                    .into()),
                    Ok(HsetnxReply::KeySet) => Ok(created_attempt),
                    Err(error) => Err(error.change_context(errors::StorageError::KVError)),
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn update_payout_attempt(
        &self,
        this: &PayoutAttempt,
        payout_update: PayoutAttemptUpdate,
        payouts: &Payouts,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError> {
        let key = PartitionKey::MerchantIdPayoutAttemptId {
            merchant_id: &this.merchant_id,
            payout_attempt_id: &this.payout_id,
        };
        let field = format!("poa_{}", this.payout_attempt_id);
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPayoutAttempt>(
            self,
            storage_scheme,
            Op::Update(key.clone(), &field, None),
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_payout_attempt(this, payout_update, payouts, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let key_str = key.to_string();

                let diesel_payout_update = payout_update.clone().to_storage_model();
                let origin_diesel_payout = this.clone().to_storage_model();

                let diesel_payout = diesel_payout_update
                    .clone()
                    .apply_changeset(origin_diesel_payout.clone());
                // Check for database presence as well Maybe use a read replica here ?

                let redis_value = diesel_payout
                    .encode_to_string_of_json()
                    .change_context(errors::StorageError::SerializationFailed)?;

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Update {
                        updatable: Box::new(kv::Updateable::PayoutAttemptUpdate(
                            kv::PayoutAttemptUpdateMems {
                                orig: origin_diesel_payout,
                                update_data: diesel_payout_update,
                            },
                        )),
                    },
                };

                let updated_attempt = PayoutAttempt::from_storage_model(
                    payout_update
                        .to_storage_model()
                        .apply_changeset(this.clone().to_storage_model()),
                );
                let old_connector_payout_id = this.connector_payout_id.clone();

                match (
                    old_connector_payout_id,
                    updated_attempt.connector_payout_id.clone(),
                ) {
                    (Some(old), Some(new)) if old != new => {
                        add_connector_payout_id_to_reverse_lookup(
                            self,
                            key_str.as_str(),
                            &this.merchant_id,
                            updated_attempt.payout_attempt_id.as_str(),
                            new.as_str(),
                            storage_scheme,
                        )
                        .await?;
                    }
                    (None, Some(new)) => {
                        add_connector_payout_id_to_reverse_lookup(
                            self,
                            key_str.as_str(),
                            &this.merchant_id,
                            updated_attempt.payout_attempt_id.as_str(),
                            new.as_str(),
                            storage_scheme,
                        )
                        .await?;
                    }
                    _ => {}
                }

                Box::pin(kv_wrapper::<(), _, _>(
                    self,
                    KvOperation::<DieselPayoutAttempt>::Hset((&field, redis_value), redis_entry),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hset()
                .change_context(errors::StorageError::KVError)?;

                Ok(PayoutAttempt::from_storage_model(diesel_payout))
            }
        }
    }

    #[instrument(skip_all)]
    async fn find_payout_attempt_by_merchant_id_payout_attempt_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_attempt_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPayoutAttempt>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_payout_attempt_by_merchant_id_payout_attempt_id(
                        merchant_id,
                        payout_attempt_id,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let lookup_id =
                    format!("poa_{}_{payout_attempt_id}", merchant_id.get_string_repr());
                let lookup = fallback_reverse_lookup_not_found!(
                    self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                        .await,
                    self.router_store
                        .find_payout_attempt_by_merchant_id_payout_attempt_id(
                            merchant_id,
                            payout_attempt_id,
                            storage_scheme
                        )
                        .await
                );
                let key = PartitionKey::CombinationKey {
                    combination: &lookup.pk_id,
                };
                Box::pin(utils::try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper(
                            self,
                            KvOperation::<DieselPayoutAttempt>::HGet(&lookup.sk_id),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    || async {
                        self.router_store
                            .find_payout_attempt_by_merchant_id_payout_attempt_id(
                                merchant_id,
                                payout_attempt_id,
                                storage_scheme,
                            )
                            .await
                    },
                ))
                .await
            }
        }
    }

    #[instrument(skip_all)]
    async fn find_payout_attempt_by_merchant_id_connector_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_payout_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_payout_attempt_by_merchant_id_connector_payout_id(
                        merchant_id,
                        connector_payout_id,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let lookup_id = format!(
                    "po_conn_payout_{}_{connector_payout_id}",
                    merchant_id.get_string_repr()
                );
                let lookup = fallback_reverse_lookup_not_found!(
                    self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                        .await,
                    self.router_store
                        .find_payout_attempt_by_merchant_id_connector_payout_id(
                            merchant_id,
                            connector_payout_id,
                            storage_scheme,
                        )
                        .await
                );
                let key = PartitionKey::CombinationKey {
                    combination: &lookup.pk_id,
                };
                Box::pin(utils::try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper(
                            self,
                            KvOperation::<DieselPayoutAttempt>::HGet(&lookup.sk_id),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    || async {
                        self.router_store
                            .find_payout_attempt_by_merchant_id_connector_payout_id(
                                merchant_id,
                                connector_payout_id,
                                storage_scheme,
                            )
                            .await
                    },
                ))
                .await
            }
        }
    }

    #[instrument(skip_all)]
    async fn get_filters_for_payouts(
        &self,
        payouts: &[Payouts],
        merchant_id: &common_utils::id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutListFilters, errors::StorageError> {
        self.router_store
            .get_filters_for_payouts(payouts, merchant_id, storage_scheme)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PayoutAttemptInterface for crate::RouterStore<T> {
    #[instrument(skip_all)]
    async fn insert_payout_attempt(
        &self,
        new: PayoutAttemptNew,
        _payouts: &Payouts,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        new.to_storage_model()
            .insert(&conn)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(PayoutAttempt::from_storage_model)
    }

    #[instrument(skip_all)]
    async fn update_payout_attempt(
        &self,
        this: &PayoutAttempt,
        payout: PayoutAttemptUpdate,
        _payouts: &Payouts,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        this.clone()
            .to_storage_model()
            .update_with_attempt_id(&conn, payout.to_storage_model())
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(PayoutAttempt::from_storage_model)
    }

    #[instrument(skip_all)]
    async fn find_payout_attempt_by_merchant_id_payout_attempt_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_attempt_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPayoutAttempt::find_by_merchant_id_payout_attempt_id(
            &conn,
            merchant_id,
            payout_attempt_id,
        )
        .await
        .map(PayoutAttempt::from_storage_model)
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
    }

    #[instrument(skip_all)]
    async fn find_payout_attempt_by_merchant_id_connector_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_payout_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPayoutAttempt::find_by_merchant_id_connector_payout_id(
            &conn,
            merchant_id,
            connector_payout_id,
        )
        .await
        .map(PayoutAttempt::from_storage_model)
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
    }

    #[instrument(skip_all)]
    async fn get_filters_for_payouts(
        &self,
        payouts: &[Payouts],
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PayoutListFilters, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        let payouts = payouts
            .iter()
            .cloned()
            .map(|payouts| payouts.to_storage_model())
            .collect::<Vec<diesel_models::Payouts>>();
        DieselPayoutAttempt::get_filters_for_payouts(&conn, payouts.as_slice(), merchant_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(
                |(connector, currency, status, payout_method)| PayoutListFilters {
                    connector: connector
                        .iter()
                        .filter_map(|c| {
                            PayoutConnectors::from_str(c)
                                .map_err(|e| {
                                    logger::error!(
                                        "Failed to parse payout connector '{}' - {}",
                                        c,
                                        e
                                    );
                                })
                                .ok()
                        })
                        .collect(),
                    currency,
                    status,
                    payout_method,
                },
            )
    }
}

impl DataModelExt for PayoutAttempt {
    type StorageModel = DieselPayoutAttempt;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPayoutAttempt {
            payout_attempt_id: self.payout_attempt_id,
            payout_id: self.payout_id,
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            address_id: self.address_id,
            connector: self.connector,
            connector_payout_id: self.connector_payout_id,
            payout_token: self.payout_token,
            status: self.status,
            is_eligible: self.is_eligible,
            error_message: self.error_message,
            error_code: self.error_code,
            business_country: self.business_country,
            business_label: self.business_label,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            profile_id: self.profile_id,
            merchant_connector_id: self.merchant_connector_id,
            routing_info: self.routing_info,
            unified_code: self.unified_code,
            unified_message: self.unified_message,
            additional_payout_method_data: self.additional_payout_method_data,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            payout_attempt_id: storage_model.payout_attempt_id,
            payout_id: storage_model.payout_id,
            customer_id: storage_model.customer_id,
            merchant_id: storage_model.merchant_id,
            address_id: storage_model.address_id,
            connector: storage_model.connector,
            connector_payout_id: storage_model.connector_payout_id,
            payout_token: storage_model.payout_token,
            status: storage_model.status,
            is_eligible: storage_model.is_eligible,
            error_message: storage_model.error_message,
            error_code: storage_model.error_code,
            business_country: storage_model.business_country,
            business_label: storage_model.business_label,
            created_at: storage_model.created_at,
            last_modified_at: storage_model.last_modified_at,
            profile_id: storage_model.profile_id,
            merchant_connector_id: storage_model.merchant_connector_id,
            routing_info: storage_model.routing_info,
            unified_code: storage_model.unified_code,
            unified_message: storage_model.unified_message,
            additional_payout_method_data: storage_model.additional_payout_method_data,
        }
    }
}
impl DataModelExt for PayoutAttemptNew {
    type StorageModel = DieselPayoutAttemptNew;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPayoutAttemptNew {
            payout_attempt_id: self.payout_attempt_id,
            payout_id: self.payout_id,
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            address_id: self.address_id,
            connector: self.connector,
            connector_payout_id: self.connector_payout_id,
            payout_token: self.payout_token,
            status: self.status,
            is_eligible: self.is_eligible,
            error_message: self.error_message,
            error_code: self.error_code,
            business_country: self.business_country,
            business_label: self.business_label,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            profile_id: self.profile_id,
            merchant_connector_id: self.merchant_connector_id,
            routing_info: self.routing_info,
            unified_code: self.unified_code,
            unified_message: self.unified_message,
            additional_payout_method_data: self.additional_payout_method_data,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            payout_attempt_id: storage_model.payout_attempt_id,
            payout_id: storage_model.payout_id,
            customer_id: storage_model.customer_id,
            merchant_id: storage_model.merchant_id,
            address_id: storage_model.address_id,
            connector: storage_model.connector,
            connector_payout_id: storage_model.connector_payout_id,
            payout_token: storage_model.payout_token,
            status: storage_model.status,
            is_eligible: storage_model.is_eligible,
            error_message: storage_model.error_message,
            error_code: storage_model.error_code,
            business_country: storage_model.business_country,
            business_label: storage_model.business_label,
            created_at: storage_model.created_at,
            last_modified_at: storage_model.last_modified_at,
            profile_id: storage_model.profile_id,
            merchant_connector_id: storage_model.merchant_connector_id,
            routing_info: storage_model.routing_info,
            unified_code: storage_model.unified_code,
            unified_message: storage_model.unified_message,
            additional_payout_method_data: storage_model.additional_payout_method_data,
        }
    }
}
impl DataModelExt for PayoutAttemptUpdate {
    type StorageModel = DieselPayoutAttemptUpdate;
    fn to_storage_model(self) -> Self::StorageModel {
        match self {
            Self::StatusUpdate {
                connector_payout_id,
                status,
                error_message,
                error_code,
                is_eligible,
                unified_code,
                unified_message,
            } => DieselPayoutAttemptUpdate::StatusUpdate {
                connector_payout_id,
                status,
                error_message,
                error_code,
                is_eligible,
                unified_code,
                unified_message,
            },
            Self::PayoutTokenUpdate { payout_token } => {
                DieselPayoutAttemptUpdate::PayoutTokenUpdate { payout_token }
            }
            Self::BusinessUpdate {
                business_country,
                business_label,
                address_id,
                customer_id,
            } => DieselPayoutAttemptUpdate::BusinessUpdate {
                business_country,
                business_label,
                address_id,
                customer_id,
            },
            Self::UpdateRouting {
                connector,
                routing_info,
                merchant_connector_id,
            } => DieselPayoutAttemptUpdate::UpdateRouting {
                connector,
                routing_info,
                merchant_connector_id,
            },
            Self::AdditionalPayoutMethodDataUpdate {
                additional_payout_method_data,
            } => DieselPayoutAttemptUpdate::AdditionalPayoutMethodDataUpdate {
                additional_payout_method_data,
            },
        }
    }

    #[allow(clippy::todo)]
    fn from_storage_model(_storage_model: Self::StorageModel) -> Self {
        todo!("Reverse map should no longer be needed")
    }
}

#[inline]
#[instrument(skip_all)]
async fn add_connector_payout_id_to_reverse_lookup<T: DatabaseStore>(
    store: &KVRouterStore<T>,
    key: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    updated_attempt_attempt_id: &str,
    connector_payout_id: &str,
    storage_scheme: MerchantStorageScheme,
) -> CustomResult<ReverseLookup, errors::StorageError> {
    let field = format!("poa_{}", updated_attempt_attempt_id);
    let reverse_lookup_new = ReverseLookupNew {
        lookup_id: format!(
            "po_conn_payout_{}_{}",
            merchant_id.get_string_repr(),
            connector_payout_id
        ),
        pk_id: key.to_owned(),
        sk_id: field.clone(),
        source: "payout_attempt".to_string(),
        updated_by: storage_scheme.to_string(),
    };
    store
        .insert_reverse_lookup(reverse_lookup_new, storage_scheme)
        .await
}
