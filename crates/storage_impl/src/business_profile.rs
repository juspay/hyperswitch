use hyperswitch_domain_models::business_profile::Profile;
use common_utils::errors::CustomResult;
use common_utils::errors::ValidationError;
use common_utils::encryption::Encryption;
use common_utils::types::keymanager;
use masking::Secret;
use hyperswitch_domain_models::type_encryption::crypto_operation;
use common_utils::type_name;
use hyperswitch_domain_models::type_encryption::CryptoOperation;
use masking::PeekInterface;
use hyperswitch_domain_models::{business_profile::ProfileSetter, type_encryption::AsyncLift};
use error_stack::ResultExt;


#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl super::behaviour::Conversion for Profile {
    type DstType = diesel_models::business_profile::Profile;
    type NewDstType = diesel_models::business_profile::ProfileNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::business_profile::Profile {
            id: self.get_id().clone(),
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

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(ProfileSetter {
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
                use_billing_as_payment_method_billing: item.use_billing_as_payment_method_billing,
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
                // version: item.version,
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
                revenue_recovery_retry_algorithm_type: item.revenue_recovery_retry_algorithm_type,
                revenue_recovery_retry_algorithm_data: item.revenue_recovery_retry_algorithm_data,
                is_iframe_redirection_enabled: item.is_iframe_redirection_enabled,
                is_external_vault_enabled: item.is_external_vault_enabled,
                external_vault_connector_details: item.external_vault_connector_details,
                merchant_category_code: item.merchant_category_code,
            }.into())
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting business profile data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::business_profile::ProfileNew {
            id: self.get_id().clone(),
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
        })
    }
}
