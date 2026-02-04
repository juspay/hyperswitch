use common_utils::ext_traits::AsyncExt;
use diesel_models::business_profile::{self, ProfileUpdateInternal};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    business_profile as domain,
    business_profile::ProfileInterface,
    merchant_key_store::MerchantKeyStore,
};
use router_env::{instrument, tracing};

use crate::{
    kv_router_store,
    utils::{pg_accounts_connection_read, pg_accounts_connection_write},
    CustomResult, DatabaseStore, KeyManagerState, MockDb, RouterStore, StorageError,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> ProfileInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .insert_business_profile(key_manager_state, merchant_key_store, business_profile)
            .await
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .find_business_profile_by_profile_id(key_manager_state, merchant_key_store, profile_id)
            .await
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .find_business_profile_by_merchant_id_profile_id(
                key_manager_state,
                merchant_key_store,
                merchant_id,
                profile_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .find_business_profile_by_profile_name_merchant_id(
                key_manager_state,
                merchant_key_store,
                profile_name,
                merchant_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .update_profile_by_profile_id(
                key_manager_state,
                merchant_key_store,
                current_state,
                profile_update,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        self.router_store
            .delete_profile_by_profile_id_merchant_id(profile_id, merchant_id)
            .await
    }

    #[instrument(skip_all)]
    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, StorageError> {
        self.router_store
            .list_profile_by_merchant_id(key_manager_state, merchant_key_store, merchant_id)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ProfileInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_write(self).await?;
        business_profile
            .construct_new()
            .await
            .change_context(StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.call_database(
            key_manager_state,
            merchant_key_store,
            business_profile::Profile::find_by_profile_id(&conn, profile_id),
        )
        .await
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.call_database(
            key_manager_state,
            merchant_key_store,
            business_profile::Profile::find_by_merchant_id_profile_id(
                &conn,
                merchant_id,
                profile_id,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.call_database(
            key_manager_state,
            merchant_key_store,
            business_profile::Profile::find_by_profile_name_merchant_id(
                &conn,
                profile_name,
                merchant_id,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_write(self).await?;

        let (profile_update_internal, current_storage_profile) = match profile_update {
            domain::ProfileUpdate::WebhooksUpdate { webhook_details } => {
                let new_json =
                    serde_json::to_value(webhook_details).unwrap_or_else(|_| serde_json::json!({}));
                let current_storage_profile =
                    business_profile::Profile::find_by_profile_id(&conn, current_state.get_id())
                        .await
                        .map_err(|error| report!(StorageError::from(error)))?;

                let merged_webhook_details = if let Some(current_storage_webhook) =
                    current_storage_profile.webhook_details.clone()
                {
                    current_storage_webhook.merge_with_json(new_json)
                } else {
                    business_profile::WebhookDetailsStorage::from_json(new_json)
                };

                (
                    ProfileUpdateInternal {
                        webhook_details: Some(merged_webhook_details),
                        modified_at: common_utils::date_time::now(),
                        profile_name: None,
                        return_url: None,
                        enable_payment_response_hash: None,
                        payment_response_hash_key: None,
                        redirect_to_merchant_with_http_post: None,
                        metadata: None,
                        routing_algorithm: None,
                        intent_fulfillment_time: None,
                        frm_routing_algorithm: None,
                        payout_routing_algorithm: None,
                        is_recon_enabled: None,
                        applepay_verified_domains: None,
                        payment_link_config: None,
                        session_expiry: None,
                        authentication_connector_details: None,
                        payout_link_config: None,
                        is_extended_card_info_enabled: None,
                        extended_card_info_config: None,
                        is_connector_agnostic_mit_enabled: None,
                        use_billing_as_payment_method_billing: None,
                        collect_shipping_details_from_wallet_connector: None,
                        collect_billing_details_from_wallet_connector: None,
                        outgoing_webhook_custom_http_headers: None,
                        always_collect_billing_details_from_wallet_connector: None,
                        always_collect_shipping_details_from_wallet_connector: None,
                        tax_connector_id: None,
                        is_tax_connector_enabled: None,
                        is_l2_l3_enabled: None,
                        dynamic_routing_algorithm: None,
                        is_network_tokenization_enabled: None,
                        is_auto_retries_enabled: None,
                        max_auto_retries_enabled: None,
                        always_request_extended_authorization: None,
                        is_click_to_pay_enabled: None,
                        authentication_product_ids: None,
                        card_testing_guard_config: None,
                        card_testing_secret_key: None,
                        is_clear_pan_retries_enabled: None,
                        force_3ds_challenge: None,
                        is_debit_routing_enabled: None,
                        merchant_business_country: None,
                        is_iframe_redirection_enabled: None,
                        is_pre_network_tokenization_enabled: None,
                        three_ds_decision_rule_algorithm: None,
                        acquirer_config_map: None,
                        merchant_category_code: None,
                        merchant_country_code: None,
                        dispute_polling_interval: None,
                        is_manual_retry_enabled: None,
                        always_enable_overcapture: None,
                        is_external_vault_enabled: None,
                        external_vault_connector_details: None,
                        billing_processor_id: None,
                    },
                    current_storage_profile,
                )
            }
            #[cfg(feature = "v1")]
            domain::ProfileUpdate::Update(update) => {
                if update.webhook_details.is_some() {
                    let new_json = serde_json::to_value(update.webhook_details.clone())
                        .unwrap_or_else(|_| serde_json::json!({}));

                    let current_storage_profile = business_profile::Profile::find_by_profile_id(
                        &conn,
                        current_state.get_id(),
                    )
                    .await
                    .map_err(|error| report!(StorageError::from(error)))?;

                    let merged_webhook_details = if let Some(current_storage_webhook) =
                        current_storage_profile.webhook_details.clone()
                    {
                        current_storage_webhook.merge_with_json(new_json)
                    } else {
                        business_profile::WebhookDetailsStorage::from_json(new_json)
                    };

                    let mut profile_update_internal =
                        ProfileUpdateInternal::from(domain::ProfileUpdate::Update(update));
                    profile_update_internal.webhook_details = Some(merged_webhook_details);

                    (profile_update_internal, current_storage_profile)
                } else {
                    let storage_profile = Conversion::convert(current_state.clone())
                        .await
                        .change_context(StorageError::EncryptionError)?;
                    (
                        ProfileUpdateInternal::from(domain::ProfileUpdate::Update(update)),
                        storage_profile,
                    )
                }
            }
            #[cfg(feature = "v2")]
            domain::ProfileUpdate::Update(update) => {
                if update.webhook_details.is_some() {
                    let new_json = serde_json::to_value(update.webhook_details.clone())
                        .unwrap_or_else(|_| serde_json::json!({}));

                    let current_storage_profile = business_profile::Profile::find_by_profile_id(
                        &conn,
                        current_state.get_id(),
                    )
                    .await
                    .map_err(|error| report!(StorageError::from(error)))?;

                    let merged_webhook_details = if let Some(current_storage_webhook) =
                        current_storage_profile.webhook_details.clone()
                    {
                        current_storage_webhook.merge_with_json(new_json)
                    } else {
                        business_profile::WebhookDetailsStorage::from_json(new_json)
                    };

                    let mut profile_update_internal =
                        ProfileUpdateInternal::from(domain::ProfileUpdate::Update(update));
                    profile_update_internal.webhook_details = Some(merged_webhook_details);

                    (profile_update_internal, current_storage_profile)
                } else {
                    let storage_profile = Conversion::convert(current_state.clone())
                        .await
                        .change_context(StorageError::EncryptionError)?;
                    (
                        ProfileUpdateInternal::from(domain::ProfileUpdate::Update(update)),
                        storage_profile,
                    )
                }
            }
            other => {
                let storage_profile = Conversion::convert(current_state.clone())
                    .await
                    .change_context(StorageError::EncryptionError)?;
                (ProfileUpdateInternal::from(other), storage_profile)
            }
        };

        current_storage_profile
            .update_by_profile_id(&conn, profile_update_internal)
            .await
            .map_err(|error| report!(StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        let conn = pg_accounts_connection_write(self).await?;
        business_profile::Profile::delete_by_profile_id_merchant_id(&conn, profile_id, merchant_id)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.find_resources(
            key_manager_state,
            merchant_key_store,
            business_profile::Profile::list_profile_by_merchant_id(&conn, merchant_id),
        )
        .await
    }
}

#[async_trait::async_trait]
impl ProfileInterface for MockDb {
    type Error = StorageError;
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, StorageError> {
        let stored_business_profile = Conversion::convert(business_profile)
            .await
            .change_context(StorageError::EncryptionError)?;

        self.business_profiles
            .lock()
            .await
            .push(stored_business_profile.clone());

        stored_business_profile
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.business_profiles
            .lock()
            .await
            .iter()
            .find(|business_profile| business_profile.get_id() == profile_id)
            .cloned()
            .async_map(|business_profile| async {
                business_profile
                    .convert(
                        key_manager_state,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                StorageError::ValueNotFound(format!(
                    "No business profile found for profile_id = {profile_id:?}"
                ))
                .into(),
            )
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.business_profiles
            .lock()
            .await
            .iter()
            .find(|business_profile| {
                business_profile.merchant_id == *merchant_id
                    && business_profile.get_id() == profile_id
            })
            .cloned()
            .async_map(|business_profile| async {
                business_profile
                    .convert(
                        key_manager_state,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                StorageError::ValueNotFound(format!(
                    "No business profile found for merchant_id = {merchant_id:?} and profile_id = {profile_id:?}"
                ))
                .into(),
            )
    }

    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, StorageError> {
        let profile_id = current_state.get_id().to_owned();
        self.business_profiles
            .lock()
            .await
            .iter_mut()
            .find(|business_profile| business_profile.get_id() == current_state.get_id())
            .async_map(|business_profile| async {
                let profile_update_internal = match profile_update {
                    domain::ProfileUpdate::WebhooksUpdate { webhook_details } => {
                        let new_json = serde_json::to_value(webhook_details)
                            .unwrap_or_else(|_| serde_json::json!({}));

                        let merged_webhook_details = if let Some(current_storage_webhook) =
                            business_profile.webhook_details.clone()
                        {
                            current_storage_webhook.merge_with_json(new_json)
                        } else {
                            business_profile::WebhookDetailsStorage::from_json(new_json)
                        };

                        ProfileUpdateInternal {
                            webhook_details: Some(merged_webhook_details),
                            modified_at: common_utils::date_time::now(),
                            profile_name: None,
                            return_url: None,
                            enable_payment_response_hash: None,
                            payment_response_hash_key: None,
                            redirect_to_merchant_with_http_post: None,
                            metadata: None,
                            routing_algorithm: None,
                            intent_fulfillment_time: None,
                            frm_routing_algorithm: None,
                            payout_routing_algorithm: None,
                            is_recon_enabled: None,
                            applepay_verified_domains: None,
                            payment_link_config: None,
                            session_expiry: None,
                            authentication_connector_details: None,
                            payout_link_config: None,
                            is_extended_card_info_enabled: None,
                            extended_card_info_config: None,
                            is_connector_agnostic_mit_enabled: None,
                            use_billing_as_payment_method_billing: None,
                            collect_shipping_details_from_wallet_connector: None,
                            collect_billing_details_from_wallet_connector: None,
                            outgoing_webhook_custom_http_headers: None,
                            always_collect_billing_details_from_wallet_connector: None,
                            always_collect_shipping_details_from_wallet_connector: None,
                            tax_connector_id: None,
                            is_tax_connector_enabled: None,
                            is_l2_l3_enabled: None,
                            dynamic_routing_algorithm: None,
                            is_network_tokenization_enabled: None,
                            is_auto_retries_enabled: None,
                            max_auto_retries_enabled: None,
                            always_request_extended_authorization: None,
                            is_click_to_pay_enabled: None,
                            authentication_product_ids: None,
                            card_testing_guard_config: None,
                            card_testing_secret_key: None,
                            is_clear_pan_retries_enabled: None,
                            force_3ds_challenge: None,
                            is_debit_routing_enabled: None,
                            merchant_business_country: None,
                            is_iframe_redirection_enabled: None,
                            is_pre_network_tokenization_enabled: None,
                            three_ds_decision_rule_algorithm: None,
                            acquirer_config_map: None,
                            merchant_category_code: None,
                            merchant_country_code: None,
                            dispute_polling_interval: None,
                            is_manual_retry_enabled: None,
                            always_enable_overcapture: None,
                            is_external_vault_enabled: None,
                            external_vault_connector_details: None,
                            billing_processor_id: None,
                        }
                    }
                    #[cfg(feature = "v1")]
                    domain::ProfileUpdate::Update(update) => {
                        if update.webhook_details.is_some() {
                            let new_json = serde_json::to_value(update.webhook_details.clone())
                                .unwrap_or_else(|_| serde_json::json!({}));

                            let merged_webhook_details = if let Some(current_storage_webhook) =
                                business_profile.webhook_details.clone()
                            {
                                current_storage_webhook.merge_with_json(new_json)
                            } else {
                                business_profile::WebhookDetailsStorage::from_json(new_json)
                            };

                            let mut update_internal =
                                ProfileUpdateInternal::from(domain::ProfileUpdate::Update(update));
                            update_internal.webhook_details = Some(merged_webhook_details);
                            update_internal
                        } else {
                            ProfileUpdateInternal::from(domain::ProfileUpdate::Update(update))
                        }
                    }
                    #[cfg(feature = "v2")]
                    domain::ProfileUpdate::Update(update) => {
                        if update.webhook_details.is_some() {
                            let new_json = serde_json::to_value(update.webhook_details.clone())
                                .unwrap_or_else(|_| serde_json::json!({}));

                            let merged_webhook_details = if let Some(current_storage_webhook) =
                                business_profile.webhook_details.clone()
                            {
                                current_storage_webhook.merge_with_json(new_json)
                            } else {
                                business_profile::WebhookDetailsStorage::from_json(new_json)
                            };

                            let mut update_internal =
                                ProfileUpdateInternal::from(domain::ProfileUpdate::Update(update));
                            update_internal.webhook_details = Some(merged_webhook_details);
                            update_internal
                        } else {
                            ProfileUpdateInternal::from(domain::ProfileUpdate::Update(update))
                        }
                    }
                    other => ProfileUpdateInternal::from(other),
                };

                let profile_updated = profile_update_internal.apply_changeset(
                    Conversion::convert(current_state)
                        .await
                        .change_context(StorageError::EncryptionError)?,
                );
                *business_profile = profile_updated.clone();

                profile_updated
                    .convert(
                        key_manager_state,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                StorageError::ValueNotFound(format!(
                    "No business profile found for profile_id = {profile_id:?}",
                ))
                .into(),
            )
    }

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        let mut business_profiles = self.business_profiles.lock().await;
        let index = business_profiles
            .iter()
            .position(|business_profile| {
                business_profile.get_id() == profile_id
                    && business_profile.merchant_id == *merchant_id
            })
            .ok_or::<StorageError>(StorageError::ValueNotFound(format!(
                "No business profile found for profile_id = {profile_id:?} and merchant_id = {merchant_id:?}"
            )))?;
        business_profiles.remove(index);
        Ok(true)
    }

    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, StorageError> {
        let business_profiles = self
            .business_profiles
            .lock()
            .await
            .iter()
            .filter(|business_profile| business_profile.merchant_id == *merchant_id)
            .cloned()
            .collect::<Vec<_>>();

        let mut domain_business_profiles = Vec::with_capacity(business_profiles.len());

        for business_profile in business_profiles {
            let domain_profile = business_profile
                .convert(
                    key_manager_state,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)?;
            domain_business_profiles.push(domain_profile);
        }

        Ok(domain_business_profiles)
    }

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.business_profiles
            .lock()
            .await
            .iter()
            .find(|business_profile| {
                business_profile.profile_name == profile_name
                    && business_profile.merchant_id == *merchant_id
            })
            .cloned()
            .async_map(|business_profile| async {
                business_profile
                    .convert(
                        key_manager_state,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                StorageError::ValueNotFound(format!(
                    "No business profile found for profile_name = {profile_name} and merchant_id = {merchant_id:?}"

                ))
                .into(),
            )
    }
}
