use external_services::superposition;

use super::{
    dimension_state::{Dimensions, HasMerchantId},
    fetch_db_with_dimensions, DatabaseBackedConfig,
};
use crate::{consts::superposition as superposition_consts, db::StorageInterface, utils::id_type};

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
#[macro_export]
macro_rules! config {
    (
        superposition_key = $superposition_key:ident,
        output = $output:ty,
        default = $default:expr,
        requires = $requirement:ty,
        targeting_key = $targeting_type:ty
    ) => {
        paste::paste! {
            /// Config definition
            pub struct [<$superposition_key:camel>];

            impl superposition::Config for [<$superposition_key:camel>] {
                type Output = $output;
                type TargetingKey = $targeting_type;

                const SUPERPOSITION_KEY: &'static str =
                    superposition_consts::$superposition_key;

                const DEFAULT_VALUE: $output = $default;
            }

            /// Get [<$superposition_key:camel>] - ONLY available when Dimensions has required state
            impl<O, P> Dimensions<$requirement, O, P>
            where
                O: Send + Sync,
                P: Send + Sync,
            {
                pub async fn [<get_ $superposition_key:lower>](
                    &self,
                    storage: &dyn StorageInterface,
                    superposition_client: Option<&superposition::SuperpositionClient>,
                    targeting_key: Option<&$targeting_type>,
                ) -> $output {
                    fetch_db_with_dimensions::<[<$superposition_key:camel>], $requirement, O, P>(
                        storage,
                        superposition_client,
                        self,
                        targeting_key,
                    )
                    .await
                }
            }
        }
    };
}

config! {
    superposition_key = REQUIRES_CVV,
    output = bool,
    default = true,
    requires = HasMerchantId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for RequiresCvv {
    const KEY: &'static str = "requires_cvv";

    fn db_key<M, O, P>(dimensions: &Dimensions<M, O, P>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", merchant_id, Self::KEY)
    }
}

config! {
    superposition_key = IMPLICIT_CUSTOMER_UPDATE,
    output = bool,
    default = false,
    requires = HasMerchantId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for ImplicitCustomerUpdate {
    const KEY: &'static str = "implicit_customer_update";

    fn db_key<M, O, P>(dimensions: &Dimensions<M, O, P>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", merchant_id, Self::KEY)
    }
}

config! {
    superposition_key = BLOCKLIST_GUARD_ENABLED,
    output = bool,
    default = false,
    requires = HasMerchantId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for BlocklistGuardEnabled {
    const KEY: &'static str = "blocklist_guard_enabled";

    fn db_key<M, O, P>(dimensions: &Dimensions<M, O, P>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", merchant_id, Self::KEY)
    }
}

config! {
    superposition_key = SHOULD_CALL_GSM,
    output = bool,
    default = false,
    requires = HasMerchantId,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for ShouldCallGsm {
    const KEY: &'static str = "should_call_gsm";

    fn db_key<M, O, P>(dimensions: &Dimensions<M, O, P>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", merchant_id, Self::KEY)
    }
}

config! {
    superposition_key = PAYMENT_UPDATE_ENABLED_FOR_CLIENT_AUTH,
    output = bool,
    default = false,
    requires = HasMerchantId,
    targeting_key = id_type::PaymentId
}

impl DatabaseBackedConfig for PaymentUpdateEnabledForClientAuth {
    const KEY: &'static str = "payment_update_enabled_for_client_auth";

    fn db_key<M, O, P>(dimensions: &Dimensions<M, O, P>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", merchant_id, Self::KEY)
    }
}

config! {
    superposition_key = UCS_ENABLED,
    output = bool,
    default = false,
    requires = HasMerchantId,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for UcsEnabled {
    const KEY: &'static str = "ucs_enabled";

    fn db_key<M, O, P>(_dimensions: &Dimensions<M, O, P>) -> String {
        Self::KEY.to_string()
    }
}

config! {
    superposition_key = AUTHENTICATION_SERVICE_ELIGIBLE,
    output = bool,
    default = false,
    requires = HasMerchantId,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for AuthenticationServiceEligible {
    const KEY: &'static str = "authentication_service_eligible";

    fn db_key<M, O, P>(dimensions: &Dimensions<M, O, P>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", Self::KEY, merchant_id)
    }
}
