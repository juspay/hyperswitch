//! Contains the id type for merchant account
//!
//! Ids for merchant account are derived from the merchant name
//! If there are any special characters, they are removed

use std::fmt::Display;

use crate::{
    date_time,
    errors::{CustomResult, ValidationError},
    generate_id_with_default_len,
    id_type::{AlphaNumericId, LengthId},
    new_type::MerchantName,
    types::keymanager,
};

crate::id_type!(
    MerchantId,
    "A type for merchant_id that can be used for merchant ids"
);
crate::impl_id_type_methods!(MerchantId, "merchant_id");

// This is to display the `MerchantId` as MerchantId(abcd)
crate::impl_debug_id_type!(MerchantId);
crate::impl_default_id_type!(MerchantId, "mer");
crate::impl_try_from_cow_str_id_type!(MerchantId, "merchant_id");

crate::impl_generate_id_id_type!(MerchantId, "mer");
crate::impl_serializable_secret_id_type!(MerchantId);
crate::impl_queryable_id_type!(MerchantId);
crate::impl_to_sql_from_sql_id_type!(MerchantId);

// This is implemented so that we can use merchant id directly as attribute in metrics
#[cfg(feature = "metrics")]
impl From<MerchantId> for router_env::opentelemetry::Value {
    fn from(val: MerchantId) -> Self {
        Self::from(val.0 .0 .0)
    }
}

impl MerchantId {
    /// Create a Merchant id from MerchantName
    pub fn from_merchant_name(merchant_name: MerchantName) -> Self {
        let merchant_name_string = merchant_name.into_inner();

        let merchant_id_prefix = merchant_name_string.trim().to_lowercase().replace(' ', "");

        let alphanumeric_id =
            AlphaNumericId::new_unchecked(generate_id_with_default_len(&merchant_id_prefix));
        let length_id = LengthId::new_unchecked(alphanumeric_id);

        Self(length_id)
    }

    /// Get a merchant id with the const value of `MERCHANT_ID_NOT_FOUND`
    pub fn get_merchant_id_not_found() -> Self {
        let alphanumeric_id = AlphaNumericId::new_unchecked("MERCHANT_ID_NOT_FOUND".to_string());
        let length_id = LengthId::new_unchecked(alphanumeric_id);
        Self(length_id)
    }

    /// Get a merchant id for internal use only
    pub fn get_internal_user_merchant_id(merchant_id: &str) -> Self {
        let alphanumeric_id = AlphaNumericId::new_unchecked(merchant_id.to_string());
        let length_id = LengthId::new_unchecked(alphanumeric_id);
        Self(length_id)
    }

    /// Create a new merchant_id from unix timestamp, of the format `merchant_{timestamp}`
    pub fn new_from_unix_timestamp() -> Self {
        let merchant_id = format!("merchant_{}", date_time::now_unix_timestamp());

        let alphanumeric_id = AlphaNumericId::new_unchecked(merchant_id);
        let length_id = LengthId::new_unchecked(alphanumeric_id);

        Self(length_id)
    }

    /// Get a merchant id with a value of `irrelevant_merchant_id`
    pub fn get_irrelevant_merchant_id() -> Self {
        let alphanumeric_id = AlphaNumericId::new_unchecked("irrelevant_merchant_id".to_string());
        let length_id = LengthId::new_unchecked(alphanumeric_id);
        Self(length_id)
    }

    /// Get a merchant id from String
    pub fn wrap(merchant_id: String) -> CustomResult<Self, ValidationError> {
        Self::try_from(std::borrow::Cow::from(merchant_id))
    }
}

impl From<MerchantId> for keymanager::Identifier {
    fn from(value: MerchantId) -> Self {
        Self::Merchant(value)
    }
}

/// All the keys that can be formed from merchant id
impl MerchantId {
    /// get step up enabled key
    pub fn get_step_up_enabled_key(&self) -> String {
        format!("step_up_enabled_{}", self.get_string_repr())
    }

    /// get_max_auto_retries_enabled key
    pub fn get_max_auto_retries_enabled(&self) -> String {
        format!("max_auto_retries_enabled_{}", self.get_string_repr())
    }

    /// get_requires_cvv_key
    pub fn get_requires_cvv_key(&self) -> String {
        format!("{}_requires_cvv", self.get_string_repr())
    }

    /// get_pm_filters_cgraph_key
    pub fn get_pm_filters_cgraph_key(&self) -> String {
        format!("pm_filters_cgraph_{}", self.get_string_repr())
    }

    /// get_blocklist_enabled_key
    pub fn get_blocklist_guard_key(&self) -> String {
        format!("guard_blocklist_for_{}", self.get_string_repr())
    }

    /// get_merchant_fingerprint_secret_key
    pub fn get_merchant_fingerprint_secret_key(&self) -> String {
        format!("fingerprint_secret_{}", self.get_string_repr())
    }

    /// get_surcharge_dsk_key
    pub fn get_surcharge_dsk_key(&self) -> String {
        format!("surcharge_dsl_{}", self.get_string_repr())
    }

    /// get_dsk_key
    pub fn get_dsl_config(&self) -> String {
        format!("dsl_{}", self.get_string_repr())
    }

    /// get_creds_identifier_key
    pub fn get_creds_identifier_key(&self, creds_identifier: &str) -> String {
        format!("mcd_{}_{creds_identifier}", self.get_string_repr())
    }

    /// get_poll_id
    pub fn get_poll_id(&self, unique_id: &str) -> String {
        format!("poll_{}_{unique_id}", self.get_string_repr())
    }

    /// get_access_token_key
    pub fn get_access_token_key(
        &self,
        merchant_connector_id_or_connector_name: impl Display,
    ) -> String {
        format!(
            "access_token_{}_{merchant_connector_id_or_connector_name}",
            self.get_string_repr()
        )
    }

    /// get_skip_saving_wallet_at_connector_key
    pub fn get_skip_saving_wallet_at_connector_key(&self) -> String {
        format!("skip_saving_wallet_at_connector_{}", self.get_string_repr())
    }

    /// get_payment_config_routing_id
    pub fn get_payment_config_routing_id(&self) -> String {
        format!("payment_config_id_{}", self.get_string_repr())
    }

    /// get_payment_method_surcharge_routing_id
    pub fn get_payment_method_surcharge_routing_id(&self) -> String {
        format!("payment_method_surcharge_id_{}", self.get_string_repr())
    }

    /// get_webhook_config_disabled_events_key
    pub fn get_webhook_config_disabled_events_key(&self, connector_id: &str) -> String {
        format!(
            "whconf_disabled_events_{}_{connector_id}",
            self.get_string_repr()
        )
    }

    /// get_should_call_gsm_payout_key
    pub fn get_should_call_gsm_payout_key(
        &self,
        payout_retry_type: common_enums::PayoutRetryType,
    ) -> String {
        match payout_retry_type {
            common_enums::PayoutRetryType::SingleConnector => format!(
                "should_call_gsm_single_connector_payout_{}",
                self.get_string_repr()
            ),
            common_enums::PayoutRetryType::MultiConnector => format!(
                "should_call_gsm_multiple_connector_payout_{}",
                self.get_string_repr()
            ),
        }
    }

    /// Get should call gsm key for payment
    pub fn get_should_call_gsm_key(&self) -> String {
        format!("should_call_gsm_{}", self.get_string_repr())
    }

    /// get_max_auto_single_connector_payout_retries_enabled_
    pub fn get_max_auto_single_connector_payout_retries_enabled(
        &self,
        payout_retry_type: common_enums::PayoutRetryType,
    ) -> String {
        match payout_retry_type {
            common_enums::PayoutRetryType::SingleConnector => format!(
                "max_auto_single_connector_payout_retries_enabled_{}",
                self.get_string_repr()
            ),
            common_enums::PayoutRetryType::MultiConnector => format!(
                "max_auto_multiple_connector_payout_retries_enabled_{}",
                self.get_string_repr()
            ),
        }
    }
}
