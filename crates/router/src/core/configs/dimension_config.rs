use common_utils::errors::CustomResult;
use external_services::superposition;
use router_env::tracing;

use super::dimension_state::{Dimensions, HasMerchantId};
use crate::{consts::superposition as superposition_consts, core::errors, routes::SessionState};

/// Get requires_cvv config
impl<O, P> Dimensions<HasMerchantId, O, P> {
    pub async fn get_requires_cvv(
        &self,
        state: &SessionState,
    ) -> CustomResult<bool, errors::StorageError> {
        // Try to get merchant_id from dimension state first
        let merchant_id = match self.merchant_id() {
            Ok(mid) => mid.clone(),
            Err(e) => {
                tracing::warn!(
                    error = ?e,
                    "Failed to get merchant_id from dimension_state"
                );
                // Return default value of requires_cvv since we can't construct the DB key without merchant_id
                return Ok(true);
            }
        };

        // DB key format: {merchant_id}_requires_cvv
        let key = format!("{}_requires_cvv", merchant_id.get_string_repr());

        // Try Superposition first, fall back to DB, then default
        let result = crate::core::configs::get_config_bool(
            state,
            superposition_consts::REQUIRES_CVV,
            &key,
            self.to_superposition_context(),
            true, // default value
        )
        .await;
        result
    }
}
