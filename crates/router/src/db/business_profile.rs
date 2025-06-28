use common_utils::{ext_traits::AsyncExt, types::keymanager::KeyManagerState};
use error_stack::{report, ResultExt};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::business_profile::ProfileUpdate;
use router_env::{instrument, tracing};

use super::Store;
#[cfg(feature = "v2")]
use crate::types::transformers::ForeignFrom;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage,
    },
};

use common_utils::encryption::Encryption;
use hyperswitch_domain_models::business_profile::ProfileGeneralUpdate;

#[async_trait::async_trait]
pub trait ProfileInterface
where
    domain::Profile: Conversion<DstType = storage::Profile, NewDstType = storage::ProfileNew>,
{
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, errors::StorageError>;
}

#[async_trait::async_trait]
impl ProfileInterface for Store {
    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_accounts_connection_write(self).await?;
        business_profile
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        storage::Profile::find_by_profile_id(&conn, profile_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        storage::Profile::find_by_merchant_id_profile_id(&conn, merchant_id, profile_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        storage::Profile::find_by_profile_name_merchant_id(&conn, profile_name, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_accounts_connection_write(self).await?;
        Conversion::convert(current_state)
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update_by_profile_id(&conn, storage::ProfileUpdateInternal::foreign_from(profile_update))
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_accounts_connection_write(self).await?;
        storage::Profile::delete_by_profile_id_merchant_id(&conn, profile_id, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, errors::StorageError> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        storage::Profile::list_profile_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|business_profiles| async {
                let mut domain_business_profiles = Vec::with_capacity(business_profiles.len());
                for business_profile in business_profiles.into_iter() {
                    domain_business_profiles.push(
                        business_profile
                            .convert(
                                key_manager_state,
                                merchant_key_store.key.get_inner(),
                                merchant_key_store.merchant_id.clone().into(),
                            )
                            .await
                            .change_context(errors::StorageError::DecryptionError)?,
                    );
                }
                Ok(domain_business_profiles)
            })
            .await
    }
}

#[async_trait::async_trait]
impl ProfileInterface for MockDb {
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let stored_business_profile = Conversion::convert(business_profile)
            .await
            .change_context(errors::StorageError::EncryptionError)?;

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
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
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
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No business profile found for profile_id = {profile_id:?}"
                ))
                .into(),
            )
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
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
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No business profile found for merchant_id = {merchant_id:?} and profile_id = {profile_id:?}"
                ))
                .into(),
            )
    }

    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let profile_id = current_state.get_id().to_owned();
        self.business_profiles
            .lock()
            .await
            .iter_mut()
            .find(|business_profile| business_profile.get_id() == current_state.get_id())
            .async_map(|business_profile| async {
                let profile_updated = storage::ProfileUpdateInternal::foreign_from(profile_update)
                    .apply_changeset(
                        Conversion::convert(current_state)
                            .await
                            .change_context(errors::StorageError::EncryptionError)?,
                    );
                *business_profile = profile_updated.clone();

                profile_updated
                    .convert(
                        key_manager_state,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No business profile found for profile_id = {profile_id:?}",
                ))
                .into(),
            )
    }

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut business_profiles = self.business_profiles.lock().await;
        let index = business_profiles
            .iter()
            .position(|business_profile| {
                business_profile.get_id() == profile_id
                    && business_profile.merchant_id == *merchant_id
            })
            .ok_or::<errors::StorageError>(errors::StorageError::ValueNotFound(format!(
                "No business profile found for profile_id = {profile_id:?} and merchant_id = {merchant_id:?}"
            )))?;
        business_profiles.remove(index);
        Ok(true)
    }

    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, errors::StorageError> {
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
                .change_context(errors::StorageError::DecryptionError)?;
            domain_business_profiles.push(domain_profile);
        }

        Ok(domain_business_profiles)
    }

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
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
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No business profile found for profile_name = {profile_name} and merchant_id = {merchant_id:?}"

                ))
                .into(),
            )
    }
}


#[cfg(feature = "v2")]
impl ForeignFrom<ProfileUpdate> for diesel_models::business_profile::ProfileUpdateInternal {
    fn foreign_from(profile_update: ProfileUpdate) -> Self {
        use common_utils::date_time;

        let now = date_time::now();

        match profile_update {
            ProfileUpdate::Update(update) => {


                let ProfileGeneralUpdate {
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
                    revenue_recovery_retry_algorithm_type: None,
                    revenue_recovery_retry_algorithm_data: None,
                    is_iframe_redirection_enabled,
                    is_external_vault_enabled,
                    external_vault_connector_details,
                    merchant_category_code,
                }
            }
            ProfileUpdate::RoutingAlgorithmUpdate {
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
            },
            ProfileUpdate::ExtendedCardInfoUpdate {
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
            },
            ProfileUpdate::ConnectorAgnosticMitUpdate {
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
            },
            ProfileUpdate::DefaultRoutingFallbackUpdate {
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
            },
            ProfileUpdate::NetworkTokenizationUpdate {
                is_network_tokenization_enabled,
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
            },
            ProfileUpdate::CollectCvvDuringPaymentUpdate {
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
            },
            ProfileUpdate::DecisionManagerRecordUpdate {
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
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
            },
            ProfileUpdate::CardTestingSecretKeyUpdate {
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
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
            },
            ProfileUpdate::RevenueRecoveryAlgorithmUpdate {
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
            },
        }
    }
}

