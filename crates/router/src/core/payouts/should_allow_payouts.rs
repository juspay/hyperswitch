use common_utils::id_type;
use error_stack::report;

pub use crate::core::configs::dimension_config::ShouldAllowPayouts;
use crate::{
    core::{
        configs::{self, dimension_state},
        errors::{self, RouterResult},
    },
    routes::SessionState,
    types::domain,
};

impl ShouldAllowPayouts {
    #[cfg(feature = "v1")]
    pub(super) fn get_resolved_payment_method_type(
        payout_method_data: Option<&crate::types::api::payouts::PayoutMethodData>,
        payment_method: Option<&hyperswitch_domain_models::payment_methods::PaymentMethod>,
        additional_data: Option<&common_utils::payout_method_utils::AdditionalPayoutMethodData>,
    ) -> Option<common_enums::PaymentMethodType> {
        payout_method_data
            .map(From::from)
            .or_else(|| {
                payment_method
                    .and_then(|payment_method| payment_method.get_payment_methods_data())
                    .and_then(|payment_method_data| {
                        payment_method_data
                            .get_additional_payout_method_data()
                            .as_ref()
                            .map(From::from)
                    })
            })
            .or_else(|| {
                payment_method
                    .and_then(|payment_method| payment_method.get_payment_method_subtype())
            })
            .or_else(|| additional_data.map(From::from))
    }

    /// Allow payout mutations only when explicitly enabled for their available OMP and method type dimensions.
    pub async fn ensure_payouts_enabled(
        state: &SessionState,
        platform: &domain::Platform,
        profile_id: Option<&id_type::ProfileId>,
        payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<()> {
        let dimensions = dimension_state::Dimensions::new()
            .with_organization_id(platform.get_processor().get_account().get_org_id().clone())
            .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
            .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id());

        let context = Self::config_context(&dimensions, profile_id, payment_method_type);
        // Payout enablement is configured only in Superposition, with no database fallback.
        let allowed = configs::fetch_db_config::<Self>(
            state.store.as_ref(),
            state.superposition_service.as_ref(),
            vec![None],
            context,
            None,
        )
        .await;

        match allowed {
            true => Ok(()),
            false => Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "Payouts are disabled by configuration".to_string(),
            })),
        }
    }

    fn config_context(
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndOrgId,
        profile_id: Option<&id_type::ProfileId>,
        payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> Option<configs::ConfigContext> {
        match (profile_id, payment_method_type) {
            (Some(profile_id), Some(payment_method_type)) => dimensions
                .with_profile_id(profile_id.clone())
                .with_payment_method_type(payment_method_type)
                .to_superposition_context(),
            (Some(profile_id), None) => dimensions
                .with_profile_id(profile_id.clone())
                .to_superposition_context(),
            (None, Some(payment_method_type)) => dimensions
                .with_payment_method_type(payment_method_type)
                .to_superposition_context(),
            (None, None) => dimensions.to_superposition_context(),
        }
    }
}
