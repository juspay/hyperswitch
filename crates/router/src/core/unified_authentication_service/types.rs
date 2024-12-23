use crate::{
    core::{
        errors::RouterResult,
        payments::{helpers::MerchantConnectorAccountType, PaymentData},
    },
    db::domain,
    routes::SessionState,
};

pub const CTP_MASTERCARD: &str = "ctp_mastercard";

pub const UNIFIED_AUTHENTICATION_SERVICE: &str = "unified_authentication_service";

pub const IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_attempt_id_in_AUTHENTICATION_flow";

pub const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_AUTHENTICATION_flow";

pub struct ClickToPay;

#[async_trait::async_trait]
pub trait UnifiedAuthenticationService<F: Clone + Sync> {
    #[allow(clippy::too_many_arguments)]
    async fn pre_authentication(
        _state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        _payment_data: &PaymentData<F>,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _connector_name: &str,
        _authentication_id: &str,
        _payment_method: common_enums::PaymentMethod,
    ) -> RouterResult<()>;

    async fn post_authentication(
        _state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        _payment_data: &PaymentData<F>,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _connector_name: &str,
        _payment_method: common_enums::PaymentMethod,
    ) -> RouterResult<hyperswitch_domain_models::types::UasPostAuthenticationRouterData>;

    fn confirmation(
        _state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        _merchant_connector_account: &MerchantConnectorAccountType,
    ) -> RouterResult<()>;
}
