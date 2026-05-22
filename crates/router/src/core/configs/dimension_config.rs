use external_services::superposition;
use scheduler::consumer::types::process_data::RetryMapping;

use super::{
    dimension_state::{DimensionsWithMerchantId, DimensionsWithMerchantIdAndProfileId},
    fetch_db_config_for_dimensions, DatabaseBackedConfig,
};
use crate::{
    consts::superposition as superposition_consts,
    core::configs::dimension_state::DimensionsWithMerchantIdAndConnector, db::StorageInterface,
    utils::id_type,
};

/// Macro to generate config struct and superposition::Config trait implementation.
/// Note: Manually implement `DatabaseBackedConfig` for the config struct:
/// The `fetch_db` method is provided by the default implementation in DatabaseBackedConfig.
///
/// # Targeting Key
///
/// In Superposition, a **targeting key** is the identifier used to assign a user/entity
/// to a specific experiment variant during traffic splitting. When an experiment is running
/// (e.g., testing a new config value vs the old one), Superposition uses the targeting key
/// to deterministically decide which variant a given entity sees.
///
/// The same targeting key value will always resolve to the same variant, ensuring
/// consistent behavior for a given entity across requests.
///
/// ## How to Select a Targeting Key
///
/// The targeting key should be the **most granular entity that should have a consistent
/// experiment experience**:
///
/// - Use **`CustomerId`** when the config affects customer-facing behavior.
///   This ensures the same customer always sees the same variant
///   across multiple payments and sessions.
///
/// - Use **`PaymentId`** when there is no customer context involved, or when the config
///   decision is per-payment (e.g., proxy payment flows). Each payment will independently
///   resolve to a variant.
///
/// As a rule of thumb: pick the entity whose experience should remain stable throughout
/// the experiment.
macro_rules! config {
    // Object config variant (with object = true)
    (
        superposition_key = $key:ident,
        output = $output:ty,
        default = $default:expr,
        object = true,
        requires = $requirement:ty,
        targeting_key = $targeting_type:ty
    ) => {
        paste::paste! {
            pub struct [<$key:camel>];

            impl superposition::Config for [<$key:camel>] {
                type Output = serde_json::Value;
                type TargetingKey = $targeting_type;
                const SUPERPOSITION_KEY: &'static str = superposition_consts::$key;
                fn default_value() -> Self::Output {
                    serde_json::to_value(&$default).expect("Failed to serialize default")
                }
            }

            impl $requirement {
                pub async fn [<get_ $key:lower>](
                    &self,
                    storage: &dyn StorageInterface,
                    superposition_client: Option<&superposition::SuperpositionClient>,
                    targeting_key: Option<&$targeting_type>,
                ) -> $output {
                    // Fetch JSON and convert to $output using the conversion function
                    crate::core::configs::fetch_db_config_for_objects::<[<$key:camel>], $output>(
                        storage, superposition_client, self, targeting_key
                    ).await
                }
            }

            impl DatabaseBackedConfig for [<$key:camel>] {
                const KEY: &'static str = stringify!([<$key:snake>]);
                fn db_key(_dimensions: &impl super::dimension_state::DimensionsBase) -> Option<String> {
                    None
                }
            }
        }
    };

    // Primitive config variant (no helper function - use get_{{key_name}}() directly on Dimensions)
    (
        superposition_key = $key:ident,
        output = $output:ty,
        default = $default:expr,
        requires = $requirement:ty,
        targeting_key = $targeting_type:ty
    ) => {
        paste::paste! {
            pub struct [<$key:camel>];

            impl superposition::Config for [<$key:camel>] {
                type Output = $output;
                type TargetingKey = $targeting_type;
                const SUPERPOSITION_KEY: &'static str = superposition_consts::$key;
                fn default_value() -> Self::Output {
                    $default
                }
            }

            impl $requirement {
                pub async fn [<get_ $key:lower>](
                    &self,
                    storage: &dyn StorageInterface,
                    superposition_client: Option<&superposition::SuperpositionClient>,
                    targeting_key: Option<&$targeting_type>,
                ) -> $output {
                    fetch_db_config_for_dimensions::<[<$key:camel>]>(storage, superposition_client, self, targeting_key).await
                }
            }
        }
    };
}

config! {
    superposition_key = REQUIRES_CVV,
    output = bool,
    default = true,
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for RequiresCvv {
    const KEY: &'static str = "requires_cvv";
    fn db_key(dimensions: &impl super::dimension_state::DimensionsBase) -> Option<String> {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        Some(format!("{}_{}", merchant_id, Self::KEY))
    }
}

config! {
    superposition_key = IMPLICIT_CUSTOMER_UPDATE,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantIdAndProfileId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for ImplicitCustomerUpdate {
    const KEY: &'static str = "implicit_customer_update";
    fn db_key(dimensions: &impl super::dimension_state::DimensionsBase) -> Option<String> {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        Some(format!("{}_{}", merchant_id, Self::KEY))
    }
}

config! {
    superposition_key = PAYOUT_TRACKER_MAPPING,
    output = RetryMapping,
    default = RetryMapping::default(),
    object = true,
    requires = DimensionsWithMerchantIdAndConnector,
    targeting_key = id_type::PayoutId
}

config! {
    superposition_key = CLIENT_SESSION_VALIDATION_ENABLED,
    output = bool,
    default = true,
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::PaymentId
}

impl DatabaseBackedConfig for ClientSessionValidationEnabled {
    const KEY: &'static str = "client_session_validation_enabled";
    fn db_key(_dimensions: &impl super::dimension_state::DimensionsBase) -> Option<String> {
        None
    }
}
