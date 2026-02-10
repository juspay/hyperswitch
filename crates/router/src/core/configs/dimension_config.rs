use external_services::superposition;

use super::{
    dimension_state::{Dimensions, HasMerchantId},
    fetch_db_with_dimensions, DatabaseBackedConfig,
};
use crate::{consts::superposition as superposition_consts, db::StorageInterface};

/// Macro to generate config struct and superposition::Config trait implementation
/// Note: Manually implement `DatabaseBackedConfig` for the config struct:
/// The `fetch_db` method is provided by the default implementation in DatabaseBackedConfig.
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
                
                type TargetingKey: $targeting_type;

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
                    let targeting_key_str = targeting_key.map(|k| k.get_string_repr().to_owned());
                    fetch_db_with_dimensions::<[<$superposition_key:camel>], $requirement, O, P>(
                        storage,
                        superposition_client,
                        self,
                        targeting_key_str,
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
