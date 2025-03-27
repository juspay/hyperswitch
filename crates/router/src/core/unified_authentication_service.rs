pub mod types;
pub mod utils;

use api_models::payments;
use diesel_models::authentication::{Authentication, AuthenticationNew};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    payment_method_data,
    router_request_types::{
        authentication::{MessageCategory, PreAuthenticationData},
        unified_authentication_service::{
            PaymentDetails, ServiceSessionIds, TransactionDetails, UasAuthenticationRequestData,
            UasConfirmationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        BrowserInformation,
    },
    types::{
        UasAuthenticationRouterData, UasPostAuthenticationRouterData,
        UasPreAuthenticationRouterData,
    },
};
use masking::ExposeInterface;

use super::{errors::RouterResult, payments::helpers::MerchantConnectorAccountType};
use crate::{
    core::{
        errors::utils::StorageErrorExt,
        payments::PaymentData,
        unified_authentication_service::types::{
            ClickToPay, ExternalAuthentication, UnifiedAuthenticationService,
            UNIFIED_AUTHENTICATION_SERVICE,
        },
    },
    db::domain,
    routes::SessionState,
};

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl<F: Clone + Sync> UnifiedAuthenticationService<F> for ClickToPay {
    fn get_pre_authentication_request_data(
        payment_data: &PaymentData<F>,
    ) -> RouterResult<UasPreAuthenticationRequestData> {
        let service_details = hyperswitch_domain_models::router_request_types::unified_authentication_service::CtpServiceDetails {
            service_session_ids: Some(ServiceSessionIds {
                merchant_transaction_id: payment_data
                    .service_details
                    .as_ref()
                    .and_then(|details| details.merchant_transaction_id.clone()),
                correlation_id: payment_data
                    .service_details
                    .as_ref()
                    .and_then(|details| details.correlation_id.clone()),
                x_src_flow_id: payment_data
                    .service_details
                    .as_ref()
                    .and_then(|details| details.x_src_flow_id.clone()),
            }),
            payment_details: None,
        };
        let currency = payment_data.payment_attempt.currency.ok_or(
            ApiErrorResponse::MissingRequiredField {
                field_name: "currency",
            },
        )?;

        let amount = payment_data.payment_attempt.net_amount.get_order_amount();
        let transaction_details = TransactionDetails {
            amount: Some(amount),
            currency: Some(currency),
            device_channel: None,
            message_category: None,
        };

        Ok(UasPreAuthenticationRequestData {
            service_details: Some(service_details),
            transaction_details: Some(transaction_details),
            payment_details: None,
        })
    }

    async fn pre_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        payment_data: &PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        authentication_id: &str,
        payment_method: common_enums::PaymentMethod,
    ) -> RouterResult<UasPreAuthenticationRouterData> {
        let pre_authentication_data = Self::get_pre_authentication_request_data(payment_data)?;

        let pre_auth_router_data: UasPreAuthenticationRouterData =
            utils::construct_uas_router_data(
                state,
                connector_name.to_string(),
                payment_method,
                payment_data.payment_attempt.merchant_id.clone(),
                None,
                pre_authentication_data,
                merchant_connector_account,
                Some(authentication_id.to_owned()),
            )?;

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            pre_auth_router_data,
        )
        .await
    }

    async fn post_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        payment_data: &PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        payment_method: common_enums::PaymentMethod,
        _authentication: Option<Authentication>,
    ) -> RouterResult<UasPostAuthenticationRouterData> {
        let authentication_id = payment_data
            .payment_attempt
            .authentication_id
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing authentication id in payment attempt")?;

        let post_authentication_data = UasPostAuthenticationRequestData {
            threeds_server_transaction_id: None,
        };

        let post_auth_router_data: UasPostAuthenticationRouterData =
            utils::construct_uas_router_data(
                state,
                connector_name.to_string(),
                payment_method,
                payment_data.payment_attempt.merchant_id.clone(),
                None,
                post_authentication_data,
                merchant_connector_account,
                Some(authentication_id.clone()),
            )?;

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            post_auth_router_data,
        )
        .await
    }

    async fn confirmation(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        payment_data: &PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        payment_method: common_enums::PaymentMethod,
    ) -> RouterResult<()> {
        let authentication_id = payment_data
            .payment_attempt
            .authentication_id
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing authentication id in payment attempt")?;

        let currency = payment_data.payment_attempt.currency.ok_or(
            ApiErrorResponse::MissingRequiredField {
                field_name: "currency",
            },
        )?;

        let current_time = common_utils::date_time::now();

        let payment_attempt_status = payment_data.payment_attempt.status;

        let (checkout_event_status, confirmation_reason) =
            utils::get_checkout_event_status_and_reason(payment_attempt_status);

        let click_to_pay_details = payment_data.service_details.clone();

        let authentication_confirmation_data = UasConfirmationRequestData {
            x_src_flow_id: payment_data
                .service_details
                .as_ref()
                .and_then(|details| details.x_src_flow_id.clone()),
            transaction_amount: payment_data.payment_attempt.net_amount.get_order_amount(),
            transaction_currency: currency,
            checkout_event_type: Some("01".to_string()), // hardcoded to '01' since only authorise flow is implemented
            checkout_event_status: checkout_event_status.clone(),
            confirmation_status: checkout_event_status.clone(),
            confirmation_reason,
            confirmation_timestamp: Some(current_time),
            network_authorization_code: Some("01".to_string()), // hardcoded to '01' since only authorise flow is implemented
            network_transaction_identifier: Some("01".to_string()), // hardcoded to '01' since only authorise flow is implemented
            correlation_id: click_to_pay_details
                .clone()
                .and_then(|details| details.correlation_id),
            merchant_transaction_id: click_to_pay_details
                .and_then(|details| details.merchant_transaction_id),
        };

        let authentication_confirmation_router_data : hyperswitch_domain_models::types::UasAuthenticationConfirmationRouterData = utils::construct_uas_router_data(
            state,
            connector_name.to_string(),
            payment_method,
            payment_data.payment_attempt.merchant_id.clone(),
            None,
            authentication_confirmation_data,
            merchant_connector_account,
            Some(authentication_id.clone()),
        )?;

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            authentication_confirmation_router_data,
        )
        .await?;

        Ok(())
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl<F: Clone + Sync> UnifiedAuthenticationService<F> for ExternalAuthentication {
    fn get_pre_authentication_request_data(
        payment_data: &PaymentData<F>,
    ) -> RouterResult<UasPreAuthenticationRequestData> {
        let payment_method_data = payment_data
            .payment_method_data
            .as_ref()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("payment_data.payment_method_data is missing")?;
        let payment_details =
            if let payment_method_data::PaymentMethodData::Card(card) = payment_method_data {
                Some(PaymentDetails {
                    pan: card.card_number.clone(),
                    digital_card_id: None,
                    payment_data_type: None,
                    encrypted_src_card_details: None,
                    card_expiry_date: card.card_exp_year.clone(),
                    cardholder_name: card.card_holder_name.clone(),
                    card_token_number: card.card_cvc.clone(),
                    account_type: card.card_network.clone(),
                })
            } else {
                None
            };
        Ok(UasPreAuthenticationRequestData {
            service_details: None,
            transaction_details: None,
            payment_details,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn pre_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        payment_data: &PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        authentication_id: &str,
        payment_method: common_enums::PaymentMethod,
    ) -> RouterResult<UasPreAuthenticationRouterData> {
        let pre_authentication_data = Self::get_pre_authentication_request_data(payment_data)?;

        let pre_auth_router_data: UasPreAuthenticationRouterData =
            utils::construct_uas_router_data(
                state,
                connector_name.to_string(),
                payment_method,
                payment_data.payment_attempt.merchant_id.clone(),
                None,
                pre_authentication_data,
                merchant_connector_account,
                Some(authentication_id.to_owned()),
            )?;

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            pre_auth_router_data,
        )
        .await
    }

    fn get_authentication_request_data(
        payment_method_data: domain::PaymentMethodData,
        billing_address: hyperswitch_domain_models::address::Address,
        shipping_address: Option<hyperswitch_domain_models::address::Address>,
        browser_details: Option<BrowserInformation>,
        amount: Option<common_utils::types::MinorUnit>,
        currency: Option<common_enums::Currency>,
        message_category: MessageCategory,
        device_channel: payments::DeviceChannel,
        authentication: Authentication,
        return_url: Option<String>,
        sdk_information: Option<payments::SdkInformation>,
        threeds_method_comp_ind: payments::ThreeDsCompletionIndicator,
        email: Option<common_utils::pii::Email>,
        webhook_url: String,
        three_ds_requestor_url: String,
    ) -> RouterResult<UasAuthenticationRequestData> {
        Ok(UasAuthenticationRequestData {
            payment_method_data,
            billing_address,
            shipping_address,
            browser_details,
            transaction_details: TransactionDetails {
                amount,
                currency,
                device_channel: Some(device_channel),
                message_category: Some(message_category),
            },
            pre_authentication_data: PreAuthenticationData {
                threeds_server_transaction_id: authentication.threeds_server_transaction_id.ok_or(
                    ApiErrorResponse::MissingRequiredField {
                        field_name: "authentication.threeds_server_transaction_id",
                    },
                )?,
                message_version: authentication.message_version.ok_or(
                    ApiErrorResponse::MissingRequiredField {
                        field_name: "authentication.message_version",
                    },
                )?,
                acquirer_bin: authentication.acquirer_bin,
                acquirer_merchant_id: authentication.acquirer_merchant_id,
                acquirer_country_code: authentication.acquirer_country_code,
                connector_metadata: authentication.connector_metadata,
            },
            return_url,
            sdk_information,
            email,
            threeds_method_comp_ind,
            three_ds_requestor_url,
            webhook_url,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn authentication(
        state: &SessionState,
        business_profile: &domain::Profile,
        payment_method: common_enums::PaymentMethod,
        payment_method_data: domain::PaymentMethodData,
        billing_address: hyperswitch_domain_models::address::Address,
        shipping_address: Option<hyperswitch_domain_models::address::Address>,
        browser_details: Option<BrowserInformation>,
        amount: Option<common_utils::types::MinorUnit>,
        currency: Option<common_enums::Currency>,
        message_category: MessageCategory,
        device_channel: payments::DeviceChannel,
        authentication: Authentication,
        return_url: Option<String>,
        sdk_information: Option<payments::SdkInformation>,
        threeds_method_comp_ind: payments::ThreeDsCompletionIndicator,
        email: Option<common_utils::pii::Email>,
        webhook_url: String,
        three_ds_requestor_url: String,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
    ) -> RouterResult<UasAuthenticationRouterData> {
        let authentication_data =
            <Self as UnifiedAuthenticationService<F>>::get_authentication_request_data(
                payment_method_data,
                billing_address,
                shipping_address,
                browser_details,
                amount,
                currency,
                message_category,
                device_channel,
                authentication.clone(),
                return_url,
                sdk_information,
                threeds_method_comp_ind,
                email,
                webhook_url,
                three_ds_requestor_url,
            )?;
        let auth_router_data: UasAuthenticationRouterData = utils::construct_uas_router_data(
            state,
            connector_name.to_string(),
            payment_method,
            business_profile.merchant_id.clone(),
            None,
            authentication_data,
            merchant_connector_account,
            Some(authentication.authentication_id.to_owned()),
        )?;

        Box::pin(utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            auth_router_data,
        ))
        .await
    }

    fn get_post_authentication_request_data(
        authentication: Option<Authentication>,
    ) -> RouterResult<UasPostAuthenticationRequestData> {
        Ok(UasPostAuthenticationRequestData {
            // authentication.threeds_server_transaction_id is mandatory for post-authentication in ExternalAuthentication
            threeds_server_transaction_id: Some(
                authentication
                    .and_then(|auth| auth.threeds_server_transaction_id)
                    .ok_or(ApiErrorResponse::MissingRequiredField {
                        field_name: "authentication.threeds_server_transaction_id",
                    })?,
            ),
        })
    }

    async fn post_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        business_profile: &domain::Profile,
        _payment_data: &PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        payment_method: common_enums::PaymentMethod,
        authentication: Option<Authentication>,
    ) -> RouterResult<UasPostAuthenticationRouterData> {
        let authentication_data =
            <Self as UnifiedAuthenticationService<F>>::get_post_authentication_request_data(
                authentication.clone(),
            )?;
        let auth_router_data: UasPostAuthenticationRouterData = utils::construct_uas_router_data(
            state,
            connector_name.to_string(),
            payment_method,
            business_profile.merchant_id.clone(),
            None,
            authentication_data,
            merchant_connector_account,
            authentication.map(|auth| auth.authentication_id),
        )?;

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            auth_router_data,
        )
        .await
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[allow(clippy::too_many_arguments)]
pub async fn create_new_authentication(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    authentication_connector: String,
    profile_id: common_utils::id_type::ProfileId,
    payment_id: Option<common_utils::id_type::PaymentId>,
    merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    authentication_id: &str,
    service_details: Option<payments::CtpServiceDetails>,
    authentication_status: common_enums::AuthenticationStatus,
    network_token: Option<payment_method_data::NetworkTokenData>,
    organization_id: common_utils::id_type::OrganizationId,
) -> RouterResult<Authentication> {
    let service_details_value = service_details
        .map(serde_json::to_value)
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "unable to parse service details into json value while inserting to DB",
        )?;
    let new_authorization = AuthenticationNew {
        authentication_id: authentication_id.to_owned(),
        merchant_id,
        authentication_connector,
        connector_authentication_id: None,
        payment_method_id: "".to_string(),
        authentication_type: None,
        authentication_status,
        authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus::Unused,
        error_message: None,
        error_code: None,
        connector_metadata: None,
        maximum_supported_version: None,
        threeds_server_transaction_id: None,
        cavv: network_token
            .clone()
            .and_then(|data| data.token_cryptogram.map(|cavv| cavv.expose())),
        authentication_flow_type: None,
        message_version: None,
        eci: network_token.and_then(|data| data.eci),
        trans_status: None,
        acquirer_bin: None,
        acquirer_merchant_id: None,
        three_ds_method_data: None,
        three_ds_method_url: None,
        acs_url: None,
        challenge_request: None,
        acs_reference_number: None,
        acs_trans_id: None,
        acs_signed_content: None,
        profile_id,
        payment_id,
        merchant_connector_id,
        ds_trans_id: None,
        directory_server_id: None,
        acquirer_country_code: None,
        service_details: service_details_value,
        organization_id,
    };
    state
        .store
        .insert_authentication(new_authorization)
        .await
        .to_duplicate_response(ApiErrorResponse::GenericDuplicateError {
            message: format!(
                "Authentication with authentication_id {} already exists",
                authentication_id
            ),
        })
}
