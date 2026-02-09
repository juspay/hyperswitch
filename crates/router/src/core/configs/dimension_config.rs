use external_services::superposition;
use hyperswitch_domain_models::configs::ConfigInterface;

use super::{
    dimension_state::{Dimensions, HasMerchantId},
    fetch_db_with_dimensions, DatabaseBackedConfig,
};
use crate::consts::superposition as superposition_consts;

/// Macro to generate config struct and superposition::Config trait implementation
/// Note: Manually implement `DatabaseBackedConfig` for the config struct:
/// The `fetch_db` method is provided by the default implementation in DatabaseBackedConfig.
#[macro_export]
macro_rules! config {
    (
        $config:ident => $output:ty,
        superposition_key = $superposition_key:expr,
        default = $default:expr,
        requires = $requirement:ty,
        method = $method:ident
    ) => {
        /// Config definition
        pub struct $config;

        impl superposition::Config for $config {
            type Output = $output;

            const SUPERPOSITION_KEY: &'static str = $superposition_key;

            const DEFAULT_VALUE: $output = $default;
        }

        /// Get $config - ONLY available when Dimensions has required state
        impl<O, P> Dimensions<$requirement, O, P>
        where
            O: Send + Sync,
            P: Send + Sync,
        {
            pub async fn $method(
                &self,
                storage: &(dyn ConfigInterface<Error = storage_impl::errors::StorageError>
                      + Send
                      + Sync),
                superposition_client: Option<&superposition::SuperpositionClient>,
            ) -> $output {
                fetch_db_with_dimensions::<$config, $requirement, O, P>(
                    storage,
                    superposition_client,
                    self,
                )
                .await
            }
        }
    };
}

config! {
    RequiresCvv => bool,
    superposition_key = superposition_consts::REQUIRES_CVV,
    default = true,
    requires = HasMerchantId,
    method = get_requires_cvv
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
    ImplicitCustomerUpdate => bool,
    superposition_key = superposition_consts::IMPLICIT_CUSTOMER_UPDATE,
    default = false,
    requires = HasMerchantId,
    method = get_implicit_customer_update
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
