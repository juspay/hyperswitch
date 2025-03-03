pub mod admin;
pub mod analytics;
pub mod api_keys;
pub mod apple_pay_certificates_migration;
pub mod blocklist;
pub mod cards_info;
pub mod conditional_configs;
pub mod connector_enums;
pub mod connector_onboarding;
pub mod consts;
pub mod currency;
pub mod customers;
pub mod disputes;
pub mod enums;
pub mod ephemeral_key;
#[cfg(feature = "errors")]
pub mod errors;
pub mod events;
pub mod external_service_auth;
pub mod feature_matrix;
pub mod files;
pub mod gsm;
pub mod health_check;
pub mod locker_migration;
pub mod mandates;
pub mod organization;
pub mod payment_methods;
pub mod payments;
#[cfg(feature = "payouts")]
pub mod payouts;
pub mod pm_auth;
pub mod poll;
#[cfg(feature = "recon")]
pub mod recon;
pub mod refunds;
pub mod relay;
pub mod routing;
pub mod surcharge_decision_configs;
pub mod user;
pub mod user_role;
pub mod verifications;
pub mod verify_connector;
pub mod webhook_events;
pub mod webhooks;

pub trait ValidateFieldAndGet<Request> {
    fn validate_field_and_get(
        &self,
        request: &Request,
    ) -> common_utils::errors::CustomResult<Self, common_utils::errors::ValidationError>
    where
        Self: Sized;
}
