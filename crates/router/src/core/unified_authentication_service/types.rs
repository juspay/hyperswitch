use api_models::payments;
use hyperswitch_domain_models::{
    errors::api_error_response::{self as errors, NotImplementedMessage},
    router_request_types::{
        authentication::MessageCategory,
        unified_authentication_service::{
            UasAuthenticationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        BrowserInformation,
    },
};

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

pub struct ExternalAuthentication;

#[async_trait::async_trait]
pub trait UnifiedAuthenticationService<F: Clone + Sync> {
    fn get_pre_authentication_request_data(
        _payment_data: &PaymentData<F>,
    ) -> RouterResult<UasPreAuthenticationRequestData> {
        Err(errors::ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason(
                "get_pre_authentication_request_data".to_string(),
            ),
        }
        .into())
    }

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
    ) -> RouterResult<hyperswitch_domain_models::types::UasPreAuthenticationRouterData> {
        Err(errors::ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("pre_authentication".to_string()),
        }
        .into())
    }

    #[allow(clippy::too_many_arguments)]
    fn get_authentication_request_data(
        _payment_method_data: domain::PaymentMethodData,
        _billing_address: hyperswitch_domain_models::address::Address,
        _shipping_address: Option<hyperswitch_domain_models::address::Address>,
        _browser_details: Option<BrowserInformation>,
        _amount: Option<common_utils::types::MinorUnit>,
        _currency: Option<common_enums::Currency>,
        _message_category: MessageCategory,
        _device_channel: payments::DeviceChannel,
        _authentication: diesel_models::authentication::Authentication,
        _return_url: Option<String>,
        _sdk_information: Option<payments::SdkInformation>,
        _threeds_method_comp_ind: payments::ThreeDsCompletionIndicator,
        _email: Option<common_utils::pii::Email>,
        _webhook_url: String,
        _three_ds_requestor_url: String,
    ) -> RouterResult<UasAuthenticationRequestData> {
        Err(errors::ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason(
                "get_pre_authentication_request_data".to_string(),
            ),
        }
        .into())
    }

    #[allow(clippy::too_many_arguments)]
    async fn authentication(
        _state: &SessionState,
        _business_profile: &domain::Profile,
        _payment_method: common_enums::PaymentMethod,
        _payment_method_data: domain::PaymentMethodData,
        _billing_address: hyperswitch_domain_models::address::Address,
        _shipping_address: Option<hyperswitch_domain_models::address::Address>,
        _browser_details: Option<BrowserInformation>,
        _amount: Option<common_utils::types::MinorUnit>,
        _currency: Option<common_enums::Currency>,
        _message_category: MessageCategory,
        _device_channel: payments::DeviceChannel,
        _authentication_data: diesel_models::authentication::Authentication,
        _return_url: Option<String>,
        _sdk_information: Option<payments::SdkInformation>,
        _threeds_method_comp_ind: payments::ThreeDsCompletionIndicator,
        _email: Option<common_utils::pii::Email>,
        _webhook_url: String,
        _three_ds_requestor_url: String,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _connector_name: &str,
    ) -> RouterResult<hyperswitch_domain_models::types::UasAuthenticationRouterData> {
        Err(errors::ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("authentication".to_string()),
        }
        .into())
    }

    fn get_post_authentication_request_data(
        _authentication: Option<diesel_models::authentication::Authentication>,
    ) -> RouterResult<UasPostAuthenticationRequestData> {
        Err(errors::ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("post_authentication".to_string()),
        }
        .into())
    }

    #[allow(clippy::too_many_arguments)]
    async fn post_authentication(
        _state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        _payment_data: &PaymentData<F>,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _connector_name: &str,
        _payment_method: common_enums::PaymentMethod,
        _authentication: Option<diesel_models::authentication::Authentication>,
    ) -> RouterResult<hyperswitch_domain_models::types::UasPostAuthenticationRouterData> {
        Err(errors::ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("post_authentication".to_string()),
        }
        .into())
    }

    async fn confirmation(
        _state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        _payment_data: &PaymentData<F>,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _connector_name: &str,
        _payment_method: common_enums::PaymentMethod,
    ) -> RouterResult<()> {
        Err(errors::ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("confirmation".to_string()),
        }
        .into())
    }
}
