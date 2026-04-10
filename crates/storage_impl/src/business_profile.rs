use common_utils::ext_traits::AsyncExt;
use diesel_models::business_profile::{self as diesel, ProfileUpdateInternal};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    business_profile::{
        ProfileInterface, {self as domain},
    },
    merchant_key_store::MerchantKeyStore,
};
use router_env::{instrument, tracing};

use crate::{
    behaviour::{Conversion, ForeignFrom, ReverseConversion},
    kv_router_store,
    utils::{pg_accounts_connection_read, pg_accounts_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore, StorageError,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> ProfileInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        merchant_key_store: &MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .insert_business_profile(merchant_key_store, business_profile)
            .await
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .find_business_profile_by_profile_id(merchant_key_store, profile_id)
            .await
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .find_business_profile_by_merchant_id_profile_id(
                merchant_key_store,
                merchant_id,
                profile_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .find_business_profile_by_profile_name_merchant_id(
                merchant_key_store,
                profile_name,
                merchant_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .update_profile_by_profile_id(merchant_key_store, current_state, profile_update)
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
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, StorageError> {
        self.router_store
            .list_profile_by_merchant_id(merchant_key_store, merchant_id)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ProfileInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
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
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.call_database_new(
            merchant_key_store,
            diesel::Profile::find_by_profile_id(&conn, profile_id),
        )
        .await
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.call_database_new(
            merchant_key_store,
            diesel::Profile::find_by_merchant_id_profile_id(&conn, merchant_id, profile_id),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.call_database_new(
            merchant_key_store,
            diesel::Profile::find_by_profile_name_merchant_id(&conn, profile_name, merchant_id),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_write(self).await?;
        Conversion::convert(current_state)
            .await
            .change_context(StorageError::EncryptionError)?
            .update_by_profile_id(&conn, ProfileUpdateInternal::foreign_from(profile_update))
            .await
            .map_err(|error| report!(StorageError::from(error)))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
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
        diesel::Profile::delete_by_profile_id_merchant_id(&conn, profile_id, merchant_id)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_profile_by_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.find_resources_new(
            merchant_key_store,
            diesel::Profile::list_profile_by_merchant_id(&conn, merchant_id),
        )
        .await
    }
}

#[async_trait::async_trait]
impl ProfileInterface for MockDb {
    type Error = StorageError;
    async fn insert_business_profile(
        &self,
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
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn find_business_profile_by_profile_id(
        &self,
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
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
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
                        self.get_keymanager_state().attach_printable("Missing KeyManagerState")?,
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
                let profile_updated = ProfileUpdateInternal::foreign_from(profile_update)
                    .apply_changeset(
                        Conversion::convert(current_state)
                            .await
                            .change_context(StorageError::EncryptionError)?,
                    );
                *business_profile = profile_updated.clone();

                profile_updated
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
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
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
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
                        self.get_keymanager_state().attach_printable("Missing KeyManagerState")?,
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

use common_utils::{
    date_time, encryption::Encryption, errors::ValidationError, type_name, types::keymanager,
};
#[cfg(feature = "v2")]
use diesel_models::business_profile::RevenueRecoveryAlgorithmData;
use hyperswitch_domain_models::{
    // behaviour::Conversion,
    type_encryption::{crypto_operation, AsyncLift, CryptoOperation},
};
use hyperswitch_masking::{PeekInterface, Secret};

#[cfg(feature = "v1")]
impl ForeignFrom<domain::ProfileUpdate> for ProfileUpdateInternal {
    fn foreign_from(profile_update: domain::ProfileUpdate) -> Self {
        let now = date_time::now();

        match profile_update {
            domain::ProfileUpdate::Update(update) => {
                let domain::ProfileGeneralUpdate {
                    profile_name,
                    return_url,
                    enable_payment_response_hash,
                    payment_response_hash_key,
                    redirect_to_merchant_with_http_post,
                    webhook_details,
                    metadata,
                    routing_algorithm,
                    intent_fulfillment_time,
                    frm_routing_algorithm,
                    payout_routing_algorithm,
                    applepay_verified_domains,
                    payment_link_config,
                    session_expiry,
                    authentication_connector_details,
                    payout_link_config,
                    extended_card_info_config,
                    use_billing_as_payment_method_billing,
                    collect_shipping_details_from_wallet_connector,
                    collect_billing_details_from_wallet_connector,
                    is_connector_agnostic_mit_enabled,
                    outgoing_webhook_custom_http_headers,
                    always_collect_billing_details_from_wallet_connector,
                    always_collect_shipping_details_from_wallet_connector,
                    tax_connector_id,
                    is_tax_connector_enabled,
                    is_l2_l3_enabled,
                    dynamic_routing_algorithm,
                    is_network_tokenization_enabled,
                    is_auto_retries_enabled,
                    max_auto_retries_enabled,
                    is_click_to_pay_enabled,
                    authentication_product_ids,
                    card_testing_guard_config,
                    card_testing_secret_key,
                    is_clear_pan_retries_enabled,
                    force_3ds_challenge,
                    is_debit_routing_enabled,
                    merchant_business_country,
                    is_iframe_redirection_enabled,
                    is_pre_network_tokenization_enabled,
                    merchant_category_code,
                    merchant_country_code,
                    dispute_polling_interval,
                    always_request_extended_authorization,
                    is_manual_retry_enabled,
                    always_enable_overcapture,
                    is_external_vault_enabled,
                    external_vault_connector_details,
                    billing_processor_id,
                    network_tokenization_credentials,
                    payment_method_blocking,
                } = *update;

                let is_external_vault_enabled = match is_external_vault_enabled {
                    Some(external_vault_mode) => match external_vault_mode {
                        common_enums::ExternalVaultEnabled::Enable => Some(true),
                        common_enums::ExternalVaultEnabled::Skip => Some(false),
                    },
                    None => Some(false),
                };

                Self {
                    profile_name,
                    modified_at: now,
                    return_url,
                    enable_payment_response_hash,
                    payment_response_hash_key,
                    redirect_to_merchant_with_http_post,
                    webhook_details,
                    metadata,
                    routing_algorithm,
                    intent_fulfillment_time,
                    frm_routing_algorithm,
                    payout_routing_algorithm,
                    is_recon_enabled: None,
                    applepay_verified_domains,
                    payment_link_config,
                    session_expiry,
                    authentication_connector_details,
                    payout_link_config,
                    is_extended_card_info_enabled: None,
                    extended_card_info_config,
                    is_connector_agnostic_mit_enabled,
                    use_billing_as_payment_method_billing,
                    collect_shipping_details_from_wallet_connector,
                    collect_billing_details_from_wallet_connector,
                    outgoing_webhook_custom_http_headers: outgoing_webhook_custom_http_headers
                        .map(Encryption::from),
                    always_collect_billing_details_from_wallet_connector,
                    always_collect_shipping_details_from_wallet_connector,
                    tax_connector_id,
                    is_tax_connector_enabled,
                    is_l2_l3_enabled,
                    dynamic_routing_algorithm,
                    is_network_tokenization_enabled,
                    is_auto_retries_enabled,
                    max_auto_retries_enabled,
                    always_request_extended_authorization,
                    is_click_to_pay_enabled,
                    authentication_product_ids,
                    card_testing_guard_config,
                    card_testing_secret_key: card_testing_secret_key.map(Encryption::from),
                    is_clear_pan_retries_enabled,
                    force_3ds_challenge,
                    is_debit_routing_enabled,
                    merchant_business_country,
                    is_iframe_redirection_enabled,
                    is_pre_network_tokenization_enabled,
                    three_ds_decision_rule_algorithm: None,
                    acquirer_config_map: None,
                    merchant_category_code,
                    merchant_country_code,
                    dispute_polling_interval,
                    is_manual_retry_enabled,
                    always_enable_overcapture,
                    is_external_vault_enabled,
                    external_vault_connector_details,
                    billing_processor_id,
                    network_tokenization_credentials: network_tokenization_credentials
                        .map(Encryption::from),
                    payment_method_blocking,
                    default_fallback_routing: None,
                }
            }
            domain::ProfileUpdate::RoutingAlgorithmUpdate {
                routing_algorithm,
                payout_routing_algorithm,
                three_ds_decision_rule_algorithm,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                routing_algorithm,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm,
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
                three_ds_decision_rule_algorithm,
                acquirer_config_map: None,
                merchant_category_code: None,
                merchant_country_code: None,
                dispute_polling_interval: None,
                is_manual_retry_enabled: None,
                always_enable_overcapture: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                billing_processor_id: None,
                is_l2_l3_enabled: None,
                network_tokenization_credentials: None,
                payment_method_blocking: None,
                default_fallback_routing: None,
            },
            domain::ProfileUpdate::DynamicRoutingAlgorithmUpdate {
                dynamic_routing_algorithm,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
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
                dynamic_routing_algorithm,
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
                is_l2_l3_enabled: None,
                network_tokenization_credentials: None,
                payment_method_blocking: None,
                default_fallback_routing: None,
            },
            domain::ProfileUpdate::ExtendedCardInfoUpdate {
                is_extended_card_info_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
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
                is_extended_card_info_enabled: Some(is_extended_card_info_enabled),
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
                is_l2_l3_enabled: None,
                network_tokenization_credentials: None,
                payment_method_blocking: None,
                default_fallback_routing: None,
            },
            domain::ProfileUpdate::ConnectorAgnosticMitUpdate {
                is_connector_agnostic_mit_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
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
                is_connector_agnostic_mit_enabled: Some(is_connector_agnostic_mit_enabled),
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
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
                is_l2_l3_enabled: None,
                network_tokenization_credentials: None,
                payment_method_blocking: None,
                default_fallback_routing: None,
            },
            domain::ProfileUpdate::NetworkTokenizationUpdate {
                is_network_tokenization_enabled,
                network_tokenization_credentials,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
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
                dynamic_routing_algorithm: None,
                is_network_tokenization_enabled: Some(is_network_tokenization_enabled),
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
                is_l2_l3_enabled: None,
                network_tokenization_credentials: network_tokenization_credentials
                    .map(Encryption::from),
                payment_method_blocking: None,
                default_fallback_routing: None,
            },
            domain::ProfileUpdate::CardTestingSecretKeyUpdate {
                card_testing_secret_key,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
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
                dynamic_routing_algorithm: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                always_request_extended_authorization: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                card_testing_guard_config: None,
                card_testing_secret_key: card_testing_secret_key.map(Encryption::from),
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
                is_l2_l3_enabled: None,
                network_tokenization_credentials: None,
                payment_method_blocking: None,
                default_fallback_routing: None,
            },
            domain::ProfileUpdate::AcquirerConfigMapUpdate {
                acquirer_config_map,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
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
                acquirer_config_map,
                merchant_category_code: None,
                merchant_country_code: None,
                dispute_polling_interval: None,
                is_manual_retry_enabled: None,
                always_enable_overcapture: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                billing_processor_id: None,
                is_l2_l3_enabled: None,
                network_tokenization_credentials: None,
                payment_method_blocking: None,
                default_fallback_routing: None,
            },
            domain::ProfileUpdate::DefaultRoutingFallbackUpdate {
                default_fallback_routing,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
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
                is_l2_l3_enabled: None,
                payment_method_blocking: None,
                default_fallback_routing,
                network_tokenization_credentials: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl Conversion for domain::Profile {
    type DstType = diesel::Profile;
    type NewDstType = diesel::ProfileNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let (is_external_vault_enabled, external_vault_connector_details) =
            self.external_vault_details.clone().into();

        Ok(diesel::Profile {
            profile_id: self.get_id().to_owned(),
            id: Some(self.get_id().to_owned()),
            merchant_id: self.merchant_id,
            profile_name: self.profile_name,
            created_at: self.created_at,
            modified_at: self.modified_at,
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            webhook_details: self.webhook_details,
            metadata: self.metadata,
            routing_algorithm: self.routing_algorithm,
            intent_fulfillment_time: self.intent_fulfillment_time,
            frm_routing_algorithm: self.frm_routing_algorithm,
            payout_routing_algorithm: self.payout_routing_algorithm,
            is_recon_enabled: self.is_recon_enabled,
            applepay_verified_domains: self.applepay_verified_domains,
            payment_link_config: self.payment_link_config,
            session_expiry: self.session_expiry,
            authentication_connector_details: self.authentication_connector_details,
            payout_link_config: self.payout_link_config,
            is_extended_card_info_enabled: self.is_extended_card_info_enabled,
            extended_card_info_config: self.extended_card_info_config,
            is_connector_agnostic_mit_enabled: self.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: self.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: self
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: self
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: self
                .outgoing_webhook_custom_http_headers
                .map(Encryption::from),
            always_collect_billing_details_from_wallet_connector: self
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: self
                .always_collect_shipping_details_from_wallet_connector,
            tax_connector_id: self.tax_connector_id,
            is_tax_connector_enabled: Some(self.is_tax_connector_enabled),
            is_l2_l3_enabled: Some(self.is_l2_l3_enabled),
            version: self.version,
            dynamic_routing_algorithm: self.dynamic_routing_algorithm,
            is_network_tokenization_enabled: self.is_network_tokenization_enabled,
            is_auto_retries_enabled: Some(self.is_auto_retries_enabled),
            max_auto_retries_enabled: self.max_auto_retries_enabled,
            always_request_extended_authorization: self.always_request_extended_authorization,
            is_click_to_pay_enabled: self.is_click_to_pay_enabled,
            authentication_product_ids: self.authentication_product_ids,
            card_testing_guard_config: self.card_testing_guard_config,
            card_testing_secret_key: self.card_testing_secret_key.map(|name| name.into()),
            is_clear_pan_retries_enabled: self.is_clear_pan_retries_enabled,
            force_3ds_challenge: Some(self.force_3ds_challenge),
            is_debit_routing_enabled: self.is_debit_routing_enabled,
            merchant_business_country: self.merchant_business_country,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            is_pre_network_tokenization_enabled: Some(self.is_pre_network_tokenization_enabled),
            three_ds_decision_rule_algorithm: self.three_ds_decision_rule_algorithm,
            acquirer_config_map: self.acquirer_config_map,
            merchant_category_code: self.merchant_category_code,
            merchant_country_code: self.merchant_country_code,
            dispute_polling_interval: self.dispute_polling_interval,
            is_manual_retry_enabled: self.is_manual_retry_enabled,
            always_enable_overcapture: self.always_enable_overcapture,
            is_external_vault_enabled,
            external_vault_connector_details,
            billing_processor_id: self.billing_processor_id,
            network_tokenization_credentials: self
                .network_tokenization_credentials
                .map(|name| name.into()),
            payment_method_blocking: self.payment_method_blocking,
            default_fallback_routing: self.default_fallback_routing,
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        // Decrypt encrypted fields first
        let (
            outgoing_webhook_custom_http_headers,
            card_testing_secret_key,
            network_tokenization_credentials,
        ) = async {
            let outgoing_webhook_custom_http_headers = item
                .outgoing_webhook_custom_http_headers
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let card_testing_secret_key = item
                .card_testing_secret_key
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let network_tokenization_credentials = item
                .network_tokenization_credentials
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            Ok::<_, error_stack::Report<common_utils::errors::CryptoError>>((
                outgoing_webhook_custom_http_headers,
                card_testing_secret_key,
                network_tokenization_credentials,
            ))
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting business profile data".to_string(),
        })?;

        let external_vault_details = domain::ExternalVaultDetails::try_from((
            item.is_external_vault_enabled,
            item.external_vault_connector_details,
        ))?;

        // Construct the domain type
        Ok(domain::ProfileSetter {
            profile_id: item.profile_id,
            merchant_id: item.merchant_id,
            profile_name: item.profile_name,
            created_at: item.created_at,
            modified_at: item.modified_at,
            return_url: item.return_url,
            enable_payment_response_hash: item.enable_payment_response_hash,
            payment_response_hash_key: item.payment_response_hash_key,
            redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
            webhook_details: item.webhook_details,
            metadata: item.metadata,
            routing_algorithm: item.routing_algorithm,
            intent_fulfillment_time: item.intent_fulfillment_time,
            frm_routing_algorithm: item.frm_routing_algorithm,
            payout_routing_algorithm: item.payout_routing_algorithm,
            is_recon_enabled: item.is_recon_enabled,
            applepay_verified_domains: item.applepay_verified_domains,
            payment_link_config: item.payment_link_config,
            session_expiry: item.session_expiry,
            authentication_connector_details: item.authentication_connector_details,
            payout_link_config: item.payout_link_config,
            is_extended_card_info_enabled: item.is_extended_card_info_enabled,
            extended_card_info_config: item.extended_card_info_config,
            is_connector_agnostic_mit_enabled: item.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: item.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: item
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: item
                .collect_billing_details_from_wallet_connector,
            always_collect_billing_details_from_wallet_connector: item
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: item
                .always_collect_shipping_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers,
            tax_connector_id: item.tax_connector_id,
            is_tax_connector_enabled: item.is_tax_connector_enabled.unwrap_or(false),
            is_l2_l3_enabled: item.is_l2_l3_enabled.unwrap_or(false),
            version: item.version,
            dynamic_routing_algorithm: item.dynamic_routing_algorithm,
            is_network_tokenization_enabled: item.is_network_tokenization_enabled,
            is_auto_retries_enabled: item.is_auto_retries_enabled.unwrap_or(false),
            max_auto_retries_enabled: item.max_auto_retries_enabled,
            always_request_extended_authorization: item.always_request_extended_authorization,
            is_click_to_pay_enabled: item.is_click_to_pay_enabled,
            authentication_product_ids: item.authentication_product_ids,
            card_testing_guard_config: item.card_testing_guard_config,
            card_testing_secret_key,
            is_clear_pan_retries_enabled: item.is_clear_pan_retries_enabled,
            force_3ds_challenge: item.force_3ds_challenge.unwrap_or_default(),
            is_debit_routing_enabled: item.is_debit_routing_enabled,
            merchant_business_country: item.merchant_business_country,
            is_iframe_redirection_enabled: item.is_iframe_redirection_enabled,
            is_pre_network_tokenization_enabled: item
                .is_pre_network_tokenization_enabled
                .unwrap_or(false),
            three_ds_decision_rule_algorithm: item.three_ds_decision_rule_algorithm,
            acquirer_config_map: item.acquirer_config_map,
            merchant_category_code: item.merchant_category_code,
            merchant_country_code: item.merchant_country_code,
            dispute_polling_interval: item.dispute_polling_interval,
            is_manual_retry_enabled: item.is_manual_retry_enabled,
            always_enable_overcapture: item.always_enable_overcapture,
            external_vault_details,
            billing_processor_id: item.billing_processor_id,
            network_tokenization_credentials,
            payment_method_blocking: item.payment_method_blocking,
            default_fallback_routing: item.default_fallback_routing,
        }
        .into())
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let (is_external_vault_enabled, external_vault_connector_details) =
            self.external_vault_details.clone().into();

        Ok(diesel::ProfileNew {
            profile_id: self.get_id().clone(),
            id: Some(self.get_id().clone()),
            merchant_id: self.merchant_id,
            profile_name: self.profile_name,
            created_at: self.created_at,
            modified_at: self.modified_at,
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            webhook_details: self.webhook_details,
            metadata: self.metadata,
            routing_algorithm: self.routing_algorithm,
            intent_fulfillment_time: self.intent_fulfillment_time,
            frm_routing_algorithm: self.frm_routing_algorithm,
            payout_routing_algorithm: self.payout_routing_algorithm,
            is_recon_enabled: self.is_recon_enabled,
            applepay_verified_domains: self.applepay_verified_domains,
            payment_link_config: self.payment_link_config,
            session_expiry: self.session_expiry,
            authentication_connector_details: self.authentication_connector_details,
            payout_link_config: self.payout_link_config,
            is_extended_card_info_enabled: self.is_extended_card_info_enabled,
            extended_card_info_config: self.extended_card_info_config,
            is_connector_agnostic_mit_enabled: self.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: self.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: self
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: self
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: self
                .outgoing_webhook_custom_http_headers
                .map(Encryption::from),
            always_collect_billing_details_from_wallet_connector: self
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: self
                .always_collect_shipping_details_from_wallet_connector,
            tax_connector_id: self.tax_connector_id,
            is_tax_connector_enabled: Some(self.is_tax_connector_enabled),
            is_l2_l3_enabled: Some(self.is_l2_l3_enabled),
            version: self.version,
            is_network_tokenization_enabled: self.is_network_tokenization_enabled,
            is_auto_retries_enabled: Some(self.is_auto_retries_enabled),
            max_auto_retries_enabled: self.max_auto_retries_enabled,
            is_click_to_pay_enabled: self.is_click_to_pay_enabled,
            authentication_product_ids: self.authentication_product_ids,
            card_testing_guard_config: self.card_testing_guard_config,
            card_testing_secret_key: self.card_testing_secret_key.map(Encryption::from),
            is_clear_pan_retries_enabled: self.is_clear_pan_retries_enabled,
            force_3ds_challenge: Some(self.force_3ds_challenge),
            is_debit_routing_enabled: self.is_debit_routing_enabled,
            merchant_business_country: self.merchant_business_country,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            is_pre_network_tokenization_enabled: Some(self.is_pre_network_tokenization_enabled),
            merchant_category_code: self.merchant_category_code,
            merchant_country_code: self.merchant_country_code,
            dispute_polling_interval: self.dispute_polling_interval,
            is_manual_retry_enabled: self.is_manual_retry_enabled,
            is_external_vault_enabled,
            external_vault_connector_details,
            billing_processor_id: self.billing_processor_id,
            network_tokenization_credentials: self
                .network_tokenization_credentials
                .map(|name| name.into()),
            payment_method_blocking: self.payment_method_blocking,
            default_fallback_routing: self.default_fallback_routing,
        })
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<domain::ProfileUpdate> for ProfileUpdateInternal {
    fn foreign_from(profile_update: domain::ProfileUpdate) -> Self {
        let now = date_time::now();

        match profile_update {
            domain::ProfileUpdate::Update(update) => {
                let domain::ProfileGeneralUpdate {
                    profile_name,
                    return_url,
                    enable_payment_response_hash,
                    payment_response_hash_key,
                    redirect_to_merchant_with_http_post,
                    webhook_details,
                    metadata,
                    applepay_verified_domains,
                    payment_link_config,
                    session_expiry,
                    authentication_connector_details,
                    payout_link_config,
                    extended_card_info_config,
                    use_billing_as_payment_method_billing,
                    collect_shipping_details_from_wallet_connector,
                    collect_billing_details_from_wallet_connector,
                    is_connector_agnostic_mit_enabled,
                    outgoing_webhook_custom_http_headers,
                    always_collect_billing_details_from_wallet_connector,
                    always_collect_shipping_details_from_wallet_connector,
                    order_fulfillment_time,
                    order_fulfillment_time_origin,
                    is_network_tokenization_enabled,
                    is_click_to_pay_enabled,
                    authentication_product_ids,
                    three_ds_decision_manager_config,
                    card_testing_guard_config,
                    card_testing_secret_key,
                    is_debit_routing_enabled,
                    merchant_business_country,
                    is_iframe_redirection_enabled,
                    is_external_vault_enabled,
                    external_vault_connector_details,
                    merchant_category_code,
                    merchant_country_code,
                    revenue_recovery_retry_algorithm_type,
                    split_txns_enabled,
                    billing_processor_id,
                } = *update;
                Self {
                    profile_name,
                    modified_at: now,
                    return_url,
                    enable_payment_response_hash,
                    payment_response_hash_key,
                    redirect_to_merchant_with_http_post,
                    webhook_details,
                    metadata,
                    is_recon_enabled: None,
                    applepay_verified_domains,
                    payment_link_config,
                    session_expiry,
                    authentication_connector_details,
                    payout_link_config,
                    is_extended_card_info_enabled: None,
                    extended_card_info_config,
                    is_connector_agnostic_mit_enabled,
                    use_billing_as_payment_method_billing,
                    collect_shipping_details_from_wallet_connector,
                    collect_billing_details_from_wallet_connector,
                    outgoing_webhook_custom_http_headers: outgoing_webhook_custom_http_headers
                        .map(Encryption::from),
                    routing_algorithm_id: None,
                    always_collect_billing_details_from_wallet_connector,
                    always_collect_shipping_details_from_wallet_connector,
                    order_fulfillment_time,
                    order_fulfillment_time_origin,
                    frm_routing_algorithm_id: None,
                    payout_routing_algorithm_id: None,
                    default_fallback_routing: None,
                    should_collect_cvv_during_payment: None,
                    tax_connector_id: None,
                    is_tax_connector_enabled: None,
                    is_l2_l3_enabled: None,
                    is_network_tokenization_enabled,
                    is_auto_retries_enabled: None,
                    max_auto_retries_enabled: None,
                    is_click_to_pay_enabled,
                    authentication_product_ids,
                    three_ds_decision_manager_config,
                    card_testing_guard_config,
                    card_testing_secret_key: card_testing_secret_key.map(Encryption::from),
                    is_clear_pan_retries_enabled: None,
                    is_debit_routing_enabled,
                    merchant_business_country,
                    revenue_recovery_retry_algorithm_type,
                    revenue_recovery_retry_algorithm_data: None,
                    is_iframe_redirection_enabled,
                    is_external_vault_enabled,
                    external_vault_connector_details,
                    merchant_category_code,
                    merchant_country_code,
                    split_txns_enabled,
                    billing_processor_id,
                }
            }
            domain::ProfileUpdate::RoutingAlgorithmUpdate {
                routing_algorithm_id,
                payout_routing_algorithm_id,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
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
                routing_algorithm_id,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                payout_routing_algorithm_id,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_l2_l3_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            domain::ProfileUpdate::ExtendedCardInfoUpdate {
                is_extended_card_info_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: Some(is_extended_card_info_enabled),
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_l2_l3_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            domain::ProfileUpdate::ConnectorAgnosticMitUpdate {
                is_connector_agnostic_mit_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                is_l2_l3_enabled: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: Some(is_connector_agnostic_mit_enabled),
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            domain::ProfileUpdate::DefaultRoutingFallbackUpdate {
                default_fallback_routing,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                is_l2_l3_enabled: None,
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
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            domain::ProfileUpdate::NetworkTokenizationUpdate {
                is_network_tokenization_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                is_l2_l3_enabled: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
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
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: Some(is_network_tokenization_enabled),
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            domain::ProfileUpdate::CollectCvvDuringPaymentUpdate {
                should_collect_cvv_during_payment,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
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
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: Some(should_collect_cvv_during_payment),
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                is_l2_l3_enabled: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            domain::ProfileUpdate::DecisionManagerRecordUpdate {
                three_ds_decision_manager_config,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
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
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: Some(three_ds_decision_manager_config),
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_l2_l3_enabled: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            domain::ProfileUpdate::CardTestingSecretKeyUpdate {
                card_testing_secret_key,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
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
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: card_testing_secret_key.map(Encryption::from),
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                is_l2_l3_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            domain::ProfileUpdate::RevenueRecoveryAlgorithmUpdate {
                revenue_recovery_retry_algorithm_type,
                revenue_recovery_retry_algorithm_data,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
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
                routing_algorithm_id: None,
                is_l2_l3_enabled: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: Some(revenue_recovery_retry_algorithm_type),
                revenue_recovery_retry_algorithm_data,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
        }
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl Conversion for domain::Profile {
    type DstType = diesel::Profile;
    type NewDstType = diesel::ProfileNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel::Profile {
            id: self.get_id().to_owned(),
            merchant_id: self.merchant_id,
            profile_name: self.profile_name,
            created_at: self.created_at,
            modified_at: self.modified_at,
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            webhook_details: self.webhook_details,
            metadata: self.metadata,
            is_recon_enabled: self.is_recon_enabled,
            applepay_verified_domains: self.applepay_verified_domains,
            payment_link_config: self.payment_link_config,
            session_expiry: self.session_expiry,
            authentication_connector_details: self.authentication_connector_details,
            payout_link_config: self.payout_link_config,
            is_extended_card_info_enabled: self.is_extended_card_info_enabled,
            extended_card_info_config: self.extended_card_info_config,
            is_connector_agnostic_mit_enabled: self.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: self.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: self
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: self
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: self
                .outgoing_webhook_custom_http_headers
                .map(Encryption::from),
            routing_algorithm_id: self.routing_algorithm_id,
            always_collect_billing_details_from_wallet_connector: self
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: self
                .always_collect_shipping_details_from_wallet_connector,
            payout_routing_algorithm_id: self.payout_routing_algorithm_id,
            order_fulfillment_time: self.order_fulfillment_time,
            order_fulfillment_time_origin: self.order_fulfillment_time_origin,
            frm_routing_algorithm_id: self.frm_routing_algorithm_id,
            default_fallback_routing: self.default_fallback_routing,
            should_collect_cvv_during_payment: self.should_collect_cvv_during_payment,
            tax_connector_id: self.tax_connector_id,
            is_tax_connector_enabled: Some(self.is_tax_connector_enabled),
            version: self.version,
            dynamic_routing_algorithm: None,
            is_network_tokenization_enabled: self.is_network_tokenization_enabled,
            is_auto_retries_enabled: None,
            max_auto_retries_enabled: None,
            always_request_extended_authorization: None,
            is_click_to_pay_enabled: self.is_click_to_pay_enabled,
            authentication_product_ids: self.authentication_product_ids,
            three_ds_decision_manager_config: self.three_ds_decision_manager_config,
            card_testing_guard_config: self.card_testing_guard_config,
            card_testing_secret_key: self.card_testing_secret_key.map(|name| name.into()),
            is_clear_pan_retries_enabled: self.is_clear_pan_retries_enabled,
            force_3ds_challenge: None,
            is_debit_routing_enabled: self.is_debit_routing_enabled,
            merchant_business_country: self.merchant_business_country,
            revenue_recovery_retry_algorithm_type: self.revenue_recovery_retry_algorithm_type,
            revenue_recovery_retry_algorithm_data: self.revenue_recovery_retry_algorithm_data,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            is_external_vault_enabled: self.is_external_vault_enabled,
            external_vault_connector_details: self.external_vault_connector_details,
            three_ds_decision_rule_algorithm: None,
            acquirer_config_map: None,
            merchant_category_code: self.merchant_category_code,
            merchant_country_code: self.merchant_country_code,
            dispute_polling_interval: None,
            split_txns_enabled: Some(self.split_txns_enabled),
            is_manual_retry_enabled: None,
            is_l2_l3_enabled: None,
            always_enable_overcapture: None,
            billing_processor_id: self.billing_processor_id,
            network_tokenization_credentials: None,
            payment_method_blocking: None,
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(
                domain::ProfileSetter {
                    id: item.id,
                    merchant_id: item.merchant_id,
                    profile_name: item.profile_name,
                    created_at: item.created_at,
                    modified_at: item.modified_at,
                    return_url: item.return_url,
                    enable_payment_response_hash: item.enable_payment_response_hash,
                    payment_response_hash_key: item.payment_response_hash_key,
                    redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
                    webhook_details: item.webhook_details,
                    metadata: item.metadata,
                    is_recon_enabled: item.is_recon_enabled,
                    applepay_verified_domains: item.applepay_verified_domains,
                    payment_link_config: item.payment_link_config,
                    session_expiry: item.session_expiry,
                    authentication_connector_details: item.authentication_connector_details,
                    payout_link_config: item.payout_link_config,
                    is_extended_card_info_enabled: item.is_extended_card_info_enabled,
                    extended_card_info_config: item.extended_card_info_config,
                    is_connector_agnostic_mit_enabled: item.is_connector_agnostic_mit_enabled,
                    use_billing_as_payment_method_billing: item
                        .use_billing_as_payment_method_billing,
                    collect_shipping_details_from_wallet_connector: item
                        .collect_shipping_details_from_wallet_connector,
                    collect_billing_details_from_wallet_connector: item
                        .collect_billing_details_from_wallet_connector,
                    outgoing_webhook_custom_http_headers: item
                        .outgoing_webhook_custom_http_headers
                        .async_lift(|inner| async {
                            crypto_operation(
                                state,
                                type_name!(Self::DstType),
                                CryptoOperation::DecryptOptional(inner),
                                key_manager_identifier.clone(),
                                key.peek(),
                            )
                            .await
                            .and_then(|val| val.try_into_optionaloperation())
                        })
                        .await?,
                    routing_algorithm_id: item.routing_algorithm_id,
                    always_collect_billing_details_from_wallet_connector: item
                        .always_collect_billing_details_from_wallet_connector,
                    always_collect_shipping_details_from_wallet_connector: item
                        .always_collect_shipping_details_from_wallet_connector,
                    order_fulfillment_time: item.order_fulfillment_time,
                    order_fulfillment_time_origin: item.order_fulfillment_time_origin,
                    frm_routing_algorithm_id: item.frm_routing_algorithm_id,
                    payout_routing_algorithm_id: item.payout_routing_algorithm_id,
                    default_fallback_routing: item.default_fallback_routing,
                    should_collect_cvv_during_payment: item.should_collect_cvv_during_payment,
                    tax_connector_id: item.tax_connector_id,
                    is_tax_connector_enabled: item.is_tax_connector_enabled.unwrap_or(false),
                    version: item.version,
                    is_network_tokenization_enabled: item.is_network_tokenization_enabled,
                    is_click_to_pay_enabled: item.is_click_to_pay_enabled,
                    authentication_product_ids: item.authentication_product_ids,
                    three_ds_decision_manager_config: item.three_ds_decision_manager_config,
                    card_testing_guard_config: item.card_testing_guard_config,
                    card_testing_secret_key: match item.card_testing_secret_key {
                        Some(encrypted_value) => crypto_operation(
                            state,
                            type_name!(Self::DstType),
                            CryptoOperation::DecryptOptional(Some(encrypted_value)),
                            key_manager_identifier.clone(),
                            key.peek(),
                        )
                        .await
                        .and_then(|val| val.try_into_optionaloperation())
                        .unwrap_or_default(),
                        None => None,
                    },
                    is_clear_pan_retries_enabled: item.is_clear_pan_retries_enabled,
                    is_debit_routing_enabled: item.is_debit_routing_enabled,
                    merchant_business_country: item.merchant_business_country,
                    revenue_recovery_retry_algorithm_type: item
                        .revenue_recovery_retry_algorithm_type,
                    revenue_recovery_retry_algorithm_data: item
                        .revenue_recovery_retry_algorithm_data,
                    is_iframe_redirection_enabled: item.is_iframe_redirection_enabled,
                    is_external_vault_enabled: item.is_external_vault_enabled,
                    external_vault_connector_details: item.external_vault_connector_details,
                    merchant_category_code: item.merchant_category_code,
                    merchant_country_code: item.merchant_country_code,
                    split_txns_enabled: item.split_txns_enabled.unwrap_or_default(),
                    billing_processor_id: item.billing_processor_id,
                }
                .into(),
            )
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting business profile data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel::ProfileNew {
            id: self.get_id().to_owned(),
            merchant_id: self.merchant_id,
            profile_name: self.profile_name,
            created_at: self.created_at,
            modified_at: self.modified_at,
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            webhook_details: self.webhook_details,
            metadata: self.metadata,
            is_recon_enabled: self.is_recon_enabled,
            applepay_verified_domains: self.applepay_verified_domains,
            payment_link_config: self.payment_link_config,
            session_expiry: self.session_expiry,
            authentication_connector_details: self.authentication_connector_details,
            payout_link_config: self.payout_link_config,
            is_extended_card_info_enabled: self.is_extended_card_info_enabled,
            extended_card_info_config: self.extended_card_info_config,
            is_connector_agnostic_mit_enabled: self.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: self.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: self
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: self
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: self
                .outgoing_webhook_custom_http_headers
                .map(Encryption::from),
            routing_algorithm_id: self.routing_algorithm_id,
            always_collect_billing_details_from_wallet_connector: self
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: self
                .always_collect_shipping_details_from_wallet_connector,
            order_fulfillment_time: self.order_fulfillment_time,
            order_fulfillment_time_origin: self.order_fulfillment_time_origin,
            frm_routing_algorithm_id: self.frm_routing_algorithm_id,
            payout_routing_algorithm_id: self.payout_routing_algorithm_id,
            default_fallback_routing: self.default_fallback_routing,
            should_collect_cvv_during_payment: self.should_collect_cvv_during_payment,
            tax_connector_id: self.tax_connector_id,
            is_tax_connector_enabled: Some(self.is_tax_connector_enabled),
            version: self.version,
            is_network_tokenization_enabled: self.is_network_tokenization_enabled,
            is_auto_retries_enabled: None,
            max_auto_retries_enabled: None,
            is_click_to_pay_enabled: self.is_click_to_pay_enabled,
            authentication_product_ids: self.authentication_product_ids,
            three_ds_decision_manager_config: self.three_ds_decision_manager_config,
            card_testing_guard_config: self.card_testing_guard_config,
            card_testing_secret_key: self.card_testing_secret_key.map(Encryption::from),
            is_clear_pan_retries_enabled: Some(self.is_clear_pan_retries_enabled),
            is_debit_routing_enabled: self.is_debit_routing_enabled,
            merchant_business_country: self.merchant_business_country,
            revenue_recovery_retry_algorithm_type: self.revenue_recovery_retry_algorithm_type,
            revenue_recovery_retry_algorithm_data: self.revenue_recovery_retry_algorithm_data,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            is_external_vault_enabled: self.is_external_vault_enabled,
            external_vault_connector_details: self.external_vault_connector_details,
            merchant_category_code: self.merchant_category_code,
            is_l2_l3_enabled: None,
            merchant_country_code: self.merchant_country_code,
            split_txns_enabled: Some(self.split_txns_enabled),
            billing_processor_id: self.billing_processor_id,
            payment_method_blocking: None,
        })
    }
}
