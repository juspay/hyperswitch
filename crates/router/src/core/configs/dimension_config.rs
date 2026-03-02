use external_services::superposition;

use super::{
    dimension_state::{
        Dimensions, DimensionsWithMerchantAndOrgId, DimensionsWithMerchantAndPaymentMethodType,
        DimensionsWithMerchantAndPayoutRetryType, DimensionsWithMerchantId,
        DimensionsWithMerchantPaymentMethodAndPaymentMethodType, DimensionsWithOrgId,
        DimensionsWithProfileId,
    },
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
            impl $requirement {
                pub async fn [<get_ $superposition_key:lower>](
                    &self,
                    storage: &dyn StorageInterface,
                    superposition_client: Option<&superposition::SuperpositionClient>,
                    targeting_key: Option<&$targeting_type>,
                ) -> $output {
                    fetch_db_with_dimensions::<[<$superposition_key:camel>], _, _, _, _, _, _>(
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
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for RequiresCvv {
    const KEY: &'static str = "requires_cvv";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
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
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for ImplicitCustomerUpdate {
    const KEY: &'static str = "implicit_customer_update";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
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
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for BlocklistGuardEnabled {
    const KEY: &'static str = "blocklist_guard_enabled";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
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
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for ShouldCallGsm {
    const KEY: &'static str = "should_call_gsm";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
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
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::PaymentId
}

impl DatabaseBackedConfig for PaymentUpdateEnabledForClientAuth {
    const KEY: &'static str = "payment_update_enabled_for_client_auth";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
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
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for UcsEnabled {
    const KEY: &'static str = "ucs_enabled";

    fn db_key<M, O, P, R, T, PM>(_dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        Self::KEY.to_string()
    }
}

config! {
    superposition_key = AUTHENTICATION_SERVICE_ELIGIBLE,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantAndOrgId,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for AuthenticationServiceEligible {
    const KEY: &'static str = "authentication_service_eligible";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", Self::KEY, merchant_id)
    }
}

config! {
    superposition_key = SHOULD_DISABLE_AUTH_TOKENIZATION,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for ShouldDisableAuthTokenization {
    const KEY: &'static str = "should_disable_auth_tokenization";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", Self::KEY, merchant_id)
    }
}

config! {
    superposition_key = SHOULD_ENABLE_MIT_WITH_LIMITED_CARD_DATA,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::PaymentId
}

impl DatabaseBackedConfig for ShouldEnableMitWithLimitedCardData {
    const KEY: &'static str = "should_enable_mit_with_limited_card_data";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", Self::KEY, merchant_id)
    }
}

config! {
    superposition_key = SHOULD_PERFORM_ELIGIBILITY,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::CustomerId
}

impl DatabaseBackedConfig for ShouldPerformEligibility {
    const KEY: &'static str = "should_perform_eligibility";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", Self::KEY, merchant_id)
    }
}

config! {
    superposition_key = SHOULD_STORE_ELIGIBILITY_CHECK_DATA_FOR_AUTHENTICATION,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for ShouldStoreEligibilityCheckDataForAuthentication {
    const KEY: &'static str = "should_store_eligibility_check_data_for_authentication";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", Self::KEY, merchant_id)
    }
}

config! {
    superposition_key = SHOULD_RETURN_RAW_PAYMENT_METHOD_DETAILS,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantId,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for ShouldReturnRawPaymentMethodDetails {
    const KEY: &'static str = "should_return_raw_payment_method_details";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", Self::KEY, merchant_id)
    }
}

config! {
    superposition_key = SHOULD_CALL_PM_MODULAR_SERVICE,
    output = bool,
    default = false,
    requires = DimensionsWithOrgId,
    targeting_key = id_type::OrganizationId
}

impl DatabaseBackedConfig for ShouldCallPmModularService {
    const KEY: &'static str = "should_call_pm_modular_service";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let organization_id = dimensions
            .get_organization_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", Self::KEY, organization_id)
    }
}

config! {
    superposition_key = ENABLE_EXTENDED_CARD_BIN,
    output = bool,
    default = false,
    requires = DimensionsWithProfileId,
    targeting_key = id_type::PaymentId
}

impl DatabaseBackedConfig for EnableExtendedCardBin {
    const KEY: &'static str = "enable_extended_card_bin";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let profile_id = dimensions
            .get_profile_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", profile_id, Self::KEY)
    }
}

config! {
    superposition_key = SHOULD_CALL_GSM_PAYOUT,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantAndPayoutRetryType,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for ShouldCallGsmPayout {
    const KEY: &'static str = "should_call_gsm_payout";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        match dimensions.get_payout_retry_type() {
            Some(common_enums::PayoutRetryType::SingleConnector) => {
                format!("should_call_gsm_single_connector_payout_{}", merchant_id)
            }
            Some(common_enums::PayoutRetryType::MultiConnector) => {
                format!("should_call_gsm_multiple_connector_payout_{}", merchant_id)
            }
            None => format!("{}_{}", Self::KEY, merchant_id),
        }
    }
}

config! {
    superposition_key = SKIP_SAVING_WALLET_AT_CONNECTOR,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantAndPaymentMethodType,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for SkipSavingWalletAtConnector {
    const KEY: &'static str = "skip_saving_wallet_at_connector";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("{}_{}", Self::KEY, merchant_id)
    }
}

config! {
    superposition_key = PRE_ROUTING_DISABLED_PM_PMT,
    output = bool,
    default = false,
    requires = DimensionsWithMerchantPaymentMethodAndPaymentMethodType,
    targeting_key = id_type::MerchantId
}

impl DatabaseBackedConfig for PreRoutingDisabledPmPmt {
    const KEY: &'static str = "pre_routing_disabled_pm_pmt";

    fn db_key<M, O, P, R, T, PM>(dimensions: &Dimensions<M, O, P, R, T, PM>) -> String {
        let merchant_id = dimensions
            .get_merchant_id()
            .map(|id| id.get_string_repr())
            .unwrap_or_default();
        format!("pre_routing_disabled_pm_pmt_for_{}", merchant_id)
    }
}
