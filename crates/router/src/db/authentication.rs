use common_utils::{ext_traits::AsyncExt, types::keymanager::KeyManagerState};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store::MerchantKeyStore,
};
use router_env::{instrument, tracing};
use storage_impl::StorageError;

use super::{MockDb, Store};
use crate::{connection, core::errors::CustomResult, types::storage};

#[async_trait::async_trait]
pub trait AuthenticationInterface {
    async fn insert_authentication(
        &self,
        state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        authentication: hyperswitch_domain_models::authentication::Authentication,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError>;

    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        merchant_key_store: &MerchantKeyStore,
        state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError>;

    async fn find_authentication_by_merchant_id_connector_authentication_id(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
        connector_authentication_id: String,
        merchant_key_store: &MerchantKeyStore,
        state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError>;

    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: hyperswitch_domain_models::authentication::Authentication,
        authentication_update: hyperswitch_domain_models::authentication::AuthenticationUpdate,
        merchant_key_store: &MerchantKeyStore,
        state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError>;
}

#[async_trait::async_trait]
impl AuthenticationInterface for Store {
    #[instrument(skip_all)]
    async fn insert_authentication(
        &self,
        state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        authentication: hyperswitch_domain_models::authentication::Authentication,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        authentication
            .construct_new()
            .await
            .change_context(StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))?
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        merchant_key_store: &MerchantKeyStore,
        state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Authentication::find_by_merchant_id_authentication_id(
            &conn,
            merchant_id,
            authentication_id,
        )
        .await
        .map_err(|error| report!(StorageError::from(error)))
        .async_and_then(|authn| async {
            authn
                .convert(
                    state,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn find_authentication_by_merchant_id_connector_authentication_id(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
        connector_authentication_id: String,
        merchant_key_store: &MerchantKeyStore,
        state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Authentication::find_authentication_by_merchant_id_connector_authentication_id(
            &conn,
            &merchant_id,
            &connector_authentication_id,
        )
        .await
        .map_err(|error| report!(StorageError::from(error)))
        .async_and_then(|authn| async {
            authn
                .convert(
                    state,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: hyperswitch_domain_models::authentication::Authentication,
        authentication_update: hyperswitch_domain_models::authentication::AuthenticationUpdate,
        merchant_key_store: &MerchantKeyStore,
        state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Authentication::update_by_merchant_id_authentication_id(
            &conn,
            previous_state.merchant_id,
            previous_state.authentication_id,
            authentication_update.into(),
        )
        .await
        .map_err(|error| report!(StorageError::from(error)))
        .async_and_then(|authn| async {
            authn
                .convert(
                    state,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)
        })
        .await
    }
}

#[async_trait::async_trait]
impl AuthenticationInterface for MockDb {
    async fn insert_authentication(
        &self,
        state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        authentication: hyperswitch_domain_models::authentication::Authentication,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let mut authentications = self.authentications.lock().await;
        if authentications.iter().any(|authentication_inner| {
            authentication_inner.authentication_id == authentication.authentication_id
        }) {
            Err(StorageError::DuplicateValue {
                entity: "authentication_id",
                key: Some(
                    authentication
                        .authentication_id
                        .get_string_repr()
                        .to_string(),
                ),
            })?
        }
        let authentication_new = storage::Authentication {
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            authentication_id: authentication.authentication_id,
            merchant_id: authentication.merchant_id,
            authentication_status: authentication.authentication_status,
            authentication_connector: authentication.authentication_connector,
            connector_authentication_id: authentication.connector_authentication_id,
            authentication_data: None,
            payment_method_id: authentication.payment_method_id,
            authentication_type: authentication.authentication_type,
            authentication_lifecycle_status: authentication.authentication_lifecycle_status,
            error_code: authentication.error_code,
            error_message: authentication.error_message,
            connector_metadata: authentication.connector_metadata,
            maximum_supported_version: authentication.maximum_supported_version,
            threeds_server_transaction_id: authentication.threeds_server_transaction_id,
            cavv: authentication.cavv,
            authentication_flow_type: authentication.authentication_flow_type,
            message_version: authentication.message_version,
            eci: authentication.eci,
            trans_status: authentication.trans_status,
            acquirer_bin: authentication.acquirer_bin,
            acquirer_merchant_id: authentication.acquirer_merchant_id,
            three_ds_method_data: authentication.three_ds_method_data,
            three_ds_method_url: authentication.three_ds_method_url,
            acs_url: authentication.acs_url,
            challenge_request: authentication.challenge_request,
            challenge_request_key: authentication.challenge_request_key,
            acs_reference_number: authentication.acs_reference_number,
            acs_trans_id: authentication.acs_trans_id,
            acs_signed_content: authentication.acs_signed_content,
            profile_id: authentication.profile_id,
            payment_id: authentication.payment_id,
            merchant_connector_id: authentication.merchant_connector_id,
            ds_trans_id: authentication.ds_trans_id,
            directory_server_id: authentication.directory_server_id,
            acquirer_country_code: authentication.acquirer_country_code,
            service_details: authentication.service_details,
            organization_id: authentication.organization_id,
            authentication_client_secret: authentication.authentication_client_secret,
            force_3ds_challenge: authentication.force_3ds_challenge,
            psd2_sca_exemption_type: authentication.psd2_sca_exemption_type,
            return_url: authentication.return_url,
            amount: authentication.amount,
            currency: authentication.currency,
            billing_address: None,
            shipping_address: None,
            browser_info: authentication.browser_info,
            email: None,
            profile_acquirer_id: authentication.profile_acquirer_id,
            challenge_code: authentication.challenge_code,
            challenge_cancel: authentication.challenge_cancel,
            challenge_code_reason: authentication.challenge_code_reason,
            message_extension: authentication.message_extension,
            customer_details: authentication.customer_details,
            earliest_supported_version: authentication.earliest_supported_version,
            latest_supported_version: authentication.latest_supported_version,
            mcc: authentication.mcc,
            platform: authentication.platform.map(|platform| platform.to_string()),
            device_type: authentication.device_type,
            device_brand: authentication.device_brand,
            device_os: authentication.device_os,
            device_display: authentication.device_display,
            browser_name: authentication.browser_name,
            browser_version: authentication.browser_version,
            scheme_name: authentication.scheme_name,
            exemption_requested: authentication.exemption_requested,
            exemption_accepted: authentication.exemption_accepted,
            issuer_id: authentication.issuer_id,
            issuer_country: authentication.issuer_country,
            merchant_country_code: authentication.merchant_country_code,
            billing_country: authentication.billing_country,
            shipping_country: authentication.shipping_country,
        };

        let authentication: hyperswitch_domain_models::authentication::Authentication =
            authentication_new
                .convert(
                    state,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::EncryptionError)?;

        authentications.push(authentication.clone());
        Ok(authentication)
    }

    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        _merchant_key_store: &MerchantKeyStore,
        _state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let authentications = self.authentications.lock().await;

        authentications
            .iter()
            .find(|auth| {
                auth.merchant_id == *merchant_id && auth.authentication_id == *authentication_id
            })
            .cloned()
            .ok_or(
                StorageError::ValueNotFound(format!(
                    "Authentication not found for merchant_id: {} and authentication_id: {}",
                    merchant_id.get_string_repr(),
                    authentication_id.get_string_repr()
                ))
                .into(),
            )
    }

    async fn find_authentication_by_merchant_id_connector_authentication_id(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
        connector_authentication_id: String,
        _merchant_key_store: &MerchantKeyStore,
        _state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let authentications = self.authentications.lock().await;

        authentications
            .iter()
            .find(|auth| {
                auth.merchant_id == merchant_id
                    && auth.connector_authentication_id.as_ref()
                        == Some(&connector_authentication_id)
            })
            .cloned()
            .ok_or(
                StorageError::ValueNotFound(format!(
                "Authentication not found for merchant_id: {} and connector_authentication_id: {}",
                merchant_id.get_string_repr(),
                connector_authentication_id
            ))
                .into(),
            )
    }

    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: hyperswitch_domain_models::authentication::Authentication,
        authentication_update: hyperswitch_domain_models::authentication::AuthenticationUpdate,
        _merchant_key_store: &MerchantKeyStore,
        _state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let mut authentications = self.authentications.lock().await;

        let auth_to_update = authentications
            .iter_mut()
            .find(|auth| {
                auth.merchant_id == previous_state.merchant_id
                    && auth.authentication_id == previous_state.authentication_id
            })
            .ok_or(StorageError::ValueNotFound(format!(
                "Authentication not found for merchant_id: {} and authentication_id: {}",
                previous_state.merchant_id.get_string_repr(),
                previous_state.authentication_id.get_string_repr()
            )))?;

        // Apply the update based on the variant
        match authentication_update {
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PreAuthenticationVersionCallUpdate {
                maximum_supported_3ds_version,
                message_version,
            } => {
                auth_to_update.maximum_supported_version = Some(maximum_supported_3ds_version);
                auth_to_update.message_version = Some(message_version);
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PreAuthenticationThreeDsMethodCall {
                threeds_server_transaction_id,
                three_ds_method_data,
                three_ds_method_url,
                acquirer_bin,
                acquirer_merchant_id,
                connector_metadata,
            } => {
                auth_to_update.threeds_server_transaction_id = Some(threeds_server_transaction_id);
                auth_to_update.three_ds_method_data = three_ds_method_data;
                auth_to_update.three_ds_method_url = three_ds_method_url;
                auth_to_update.acquirer_bin = acquirer_bin;
                auth_to_update.acquirer_merchant_id = acquirer_merchant_id;
                auth_to_update.connector_metadata = connector_metadata;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PreAuthenticationUpdate {
                threeds_server_transaction_id,
                maximum_supported_3ds_version,
                connector_authentication_id,
                three_ds_method_data,
                three_ds_method_url,
                message_version,
                connector_metadata,
                authentication_status,
                acquirer_bin,
                acquirer_merchant_id,
                directory_server_id,
                acquirer_country_code,
                billing_address,
                shipping_address,
                browser_info,
                email,
                scheme_id,
                merchant_category_code,
                merchant_country_code,
                billing_country,
                shipping_country,
                earliest_supported_version,
                latest_supported_version,
            } => {
                auth_to_update.threeds_server_transaction_id = Some(threeds_server_transaction_id);
                auth_to_update.maximum_supported_version = Some(maximum_supported_3ds_version);
                auth_to_update.connector_authentication_id = Some(connector_authentication_id);
                auth_to_update.three_ds_method_data = three_ds_method_data;
                auth_to_update.three_ds_method_url = three_ds_method_url;
                auth_to_update.message_version = Some(message_version);
                auth_to_update.connector_metadata = connector_metadata;
                auth_to_update.authentication_status = authentication_status;
                auth_to_update.acquirer_bin = acquirer_bin;
                auth_to_update.acquirer_merchant_id = acquirer_merchant_id;
                auth_to_update.directory_server_id = directory_server_id;
                auth_to_update.acquirer_country_code = acquirer_country_code;
                auth_to_update.billing_address = *billing_address;
                auth_to_update.shipping_address = *shipping_address;
                auth_to_update.browser_info = *browser_info;
                auth_to_update.email = email;
                auth_to_update.scheme_name = scheme_id;
                auth_to_update.mcc = merchant_category_code;
                auth_to_update.merchant_country_code = merchant_country_code;
                auth_to_update.billing_country = billing_country;
                auth_to_update.shipping_country = shipping_country;
                auth_to_update.earliest_supported_version = earliest_supported_version;
                auth_to_update.latest_supported_version = latest_supported_version;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::AuthenticationUpdate {
                trans_status,
                authentication_type,
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                acs_signed_content,
                connector_metadata,
                authentication_status,
                ds_trans_id,
                eci,
                challenge_code,
                challenge_cancel,
                challenge_code_reason,
                message_extension,
                challenge_request_key,
                device_type,
                device_brand,
                device_os,
                device_display,
            } => {
                auth_to_update.trans_status = Some(trans_status);
                auth_to_update.authentication_type = Some(authentication_type);
                auth_to_update.acs_url = acs_url;
                auth_to_update.challenge_request = challenge_request;
                auth_to_update.acs_reference_number = acs_reference_number;
                auth_to_update.acs_trans_id = acs_trans_id;
                auth_to_update.acs_signed_content = acs_signed_content;
                auth_to_update.connector_metadata = connector_metadata;
                auth_to_update.authentication_status = authentication_status;
                auth_to_update.ds_trans_id = ds_trans_id;
                auth_to_update.eci = eci;
                auth_to_update.challenge_code = challenge_code;
                auth_to_update.challenge_cancel = challenge_cancel;
                auth_to_update.challenge_code_reason = challenge_code_reason;
                auth_to_update.message_extension = message_extension;
                auth_to_update.challenge_request_key = challenge_request_key;
                auth_to_update.device_type = device_type;
                auth_to_update.device_brand = device_brand;
                auth_to_update.device_os = device_os;
                auth_to_update.device_display = device_display;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PostAuthenticationUpdate {
                trans_status,
                eci,
                authentication_status,
                challenge_cancel,
                challenge_code_reason,
            } => {
                auth_to_update.trans_status = Some(trans_status);
                auth_to_update.eci = eci;
                auth_to_update.authentication_status = authentication_status;
                auth_to_update.challenge_cancel = challenge_cancel;
                auth_to_update.challenge_code_reason = challenge_code_reason;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::ErrorUpdate {
                error_message,
                error_code,
                authentication_status,
                connector_authentication_id,
            } => {
                auth_to_update.error_message = error_message;
                auth_to_update.error_code = error_code;
                auth_to_update.authentication_status = authentication_status;
                auth_to_update.connector_authentication_id = connector_authentication_id;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PostAuthorizationUpdate {
                authentication_lifecycle_status,
            } => {
                auth_to_update.authentication_lifecycle_status = authentication_lifecycle_status;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::AuthenticationStatusUpdate {
                trans_status,
                authentication_status,
            } => {
                auth_to_update.trans_status = Some(trans_status);
                auth_to_update.authentication_status = authentication_status;
            }
        }

        auth_to_update.modified_at = common_utils::date_time::now();

        Ok(auth_to_update.clone())
    }
}
