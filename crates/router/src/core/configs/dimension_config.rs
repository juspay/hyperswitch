use super::{Config, dimension_state::{Dimensions, HasMerchantId}};
use crate::{
    consts::superposition as superposition_consts,
    routes::SessionState,
};


/// Config definition for requiring CVV
pub struct RequiresCvv;

impl Config for RequiresCvv {
    type Output = bool;

    const SUPERPOSITION_KEY: &'static str = superposition_consts::REQUIRES_CVV;

    const KEY: &'static str = "requires_cvv";

    const DEFAULT_VALUE: bool = true;
}

impl RequiresCvv {
    /// Generate the database key for this config from dimensions
    /// Returns Some(db_key) if merchant_id is available, None otherwise
    pub fn db_key<O, P>(dimensions: &Dimensions<HasMerchantId, O, P>) -> Option<String> {
        dimensions.merchant_id().ok().map(|merchant_id| {
            format!("{}_{}", merchant_id.get_string_repr(), Self::KEY)
        })
    }
}

/// Get requires_cvv config
impl<O, P> Dimensions<HasMerchantId, O, P> {
    pub async fn get_requires_cvv(
        &self,
        state: &SessionState,
    ) -> bool {
        // Generate db_key, return default if merchant_id unavailable
        let db_key = match RequiresCvv::db_key(self) {
            Some(key) => key,
            None => return RequiresCvv::DEFAULT_VALUE,
        };

        // Fetch the value using the db_key
        RequiresCvv::fetch(state, &db_key, self.to_superposition_context()).await
    }
}
