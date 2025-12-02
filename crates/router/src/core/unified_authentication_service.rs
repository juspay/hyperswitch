pub mod types;
use std::str::FromStr;

use common_utils::ext_traits::StringExt;

pub mod utils;
#[cfg(feature = "v1")]
use api_models::authentication::{
    AuthenticationEligibilityCheckData, AuthenticationEligibilityCheckRequest,
    AuthenticationEligibilityCheckResponse, AuthenticationEligibilityCheckResponseData,
    AuthenticationEligibilityRequest, AuthenticationEligibilityResponse,
    AuthenticationRetrieveEligibilityCheckRequest, AuthenticationRetrieveEligibilityCheckResponse,
    AuthenticationSyncPostUpdateRequest, AuthenticationSyncRequest, AuthenticationSyncResponse,
    ClickToPayEligibilityCheckResponseData,
};
use api_models::{
    authentication::{
        AcquirerDetails, AuthenticationAuthenticateRequest, AuthenticationAuthenticateResponse,
        AuthenticationCreateRequest, AuthenticationResponse, AuthenticationSdkNextAction,
        AuthenticationSessionTokenRequest,
    },
    payments::{self, CustomerDetails},
};
#[cfg(feature = "v1")]
use common_utils::{
    errors::CustomResult, ext_traits::ValueExt, types::keymanager::ToEncryptable,
    types::AmountConvertor,
};
use diesel_models::authentication::{Authentication, AuthenticationNew};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    ext_traits::OptionExt,
    payment_method_data,
    router_request_types::{
        authentication::{MessageCategory, PreAuthenticationData},
        unified_authentication_service::{
            AuthenticationInfo, PaymentDetails, ServiceSessionIds, ThreeDsMetaData,
            TransactionDetails, UasAuthenticationRequestData, UasConfirmationRequestData,
            UasPostAuthenticationRequestData, UasPreAuthenticationRequestData,
        },
        BrowserInformation,
    },
    types::{
        UasAuthenticationRouterData, UasPostAuthenticationRouterData,
        UasPreAuthenticationRouterData,
    },
};
use masking::{ExposeInterface, PeekInterface};

use super::{
    errors::{RouterResponse, RouterResult},
    payments::helpers::MerchantConnectorAccountType,
};
use crate::{
    consts,
    core::{
        authentication::utils as auth_utils,
        errors::utils::StorageErrorExt,
        payment_methods,
        payments::{helpers, validate_customer_details_for_click_to_pay},
        unified_authentication_service::types::{
            ClickToPay, ExternalAuthentication, UnifiedAuthenticationService,
            UNIFIED_AUTHENTICATION_SERVICE,
        },
        utils as core_utils,
    },
    db::domain,
    routes::SessionState,
    services::AuthFlow,
    types::{domain::types::AsyncLift, transformers::ForeignTryFrom},
};
#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl UnifiedAuthenticationService for ClickToPay {
    fn get_pre_authentication_request_data(
        _payment_method_data: Option<&domain::PaymentMethodData>,
        service_details: Option<payments::CtpServiceDetails>,
        amount: common_utils::types::MinorUnit,
        currency: Option<common_enums::Currency>,
        merchant_details: Option<&hyperswitch_domain_models::router_request_types::unified_authentication_service::MerchantDetails>,
        billing_address: Option<&hyperswitch_domain_models::address::Address>,
        acquirer_bin: Option<String>,
        acquirer_merchant_id: Option<String>,
        _payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<UasPreAuthenticationRequestData> {
        let domain_service_details = hyperswitch_domain_models::router_request_types::unified_authentication_service::CtpServiceDetails {
            service_session_ids: Some(ServiceSessionIds {
                merchant_transaction_id: service_details
                    .as_ref()
                    .and_then(|details| details.merchant_transaction_id.clone()),
                correlation_id: service_details
                    .as_ref()
                    .and_then(|details| details.correlation_id.clone()),
                x_src_flow_id: service_details
                    .as_ref()
                    .and_then(|details| details.x_src_flow_id.clone()),
            }),
            payment_details: None,
        };

        let transaction_details = TransactionDetails {
            amount: Some(amount),
            currency,
            device_channel: None,
            message_category: None,
            force_3ds_challenge: None,
            psd2_sca_exemption_type: None,
        };

        let authentication_info = Some(AuthenticationInfo {
            authentication_type: None,
            authentication_reasons: None,
            consent_received: false, // This is not relevant in this flow so keeping it as false
            is_authenticated: false, // This is not relevant in this flow so keeping it as false
            locale: None,
            supported_card_brands: None,
            encrypted_payload: service_details
                .as_ref()
                .and_then(|details| details.encrypted_payload.clone()),
        });
        Ok(UasPreAuthenticationRequestData {
            service_details: Some(domain_service_details),
            transaction_details: Some(transaction_details),
            payment_details: None,
            authentication_info,
            merchant_details: merchant_details.cloned(),
            billing_address: billing_address.cloned(),
            acquirer_bin,
            acquirer_merchant_id,
        })
    }

    async fn pre_authentication(
        state: &SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: Option<&common_utils::id_type::PaymentId>,
        payment_method_data: Option<&domain::PaymentMethodData>,
        payment_method_type: Option<common_enums::PaymentMethodType>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        authentication_id: &common_utils::id_type::AuthenticationId,
        payment_method: common_enums::PaymentMethod,
        amount: common_utils::types::MinorUnit,
        currency: Option<common_enums::Currency>,
        service_details: Option<payments::CtpServiceDetails>,
        merchant_details: Option<&hyperswitch_domain_models::router_request_types::unified_authentication_service::MerchantDetails>,
        billing_address: Option<&hyperswitch_domain_models::address::Address>,
        acquirer_bin: Option<String>,
        acquirer_merchant_id: Option<String>,
    ) -> RouterResult<UasPreAuthenticationRouterData> {
        let pre_authentication_data = Self::get_pre_authentication_request_data(
            payment_method_data,
            service_details,
            amount,
            currency,
            merchant_details,
            billing_address,
            acquirer_bin,
            acquirer_merchant_id,
            payment_method_type,
        )?;

        let pre_auth_router_data: UasPreAuthenticationRouterData =
            utils::construct_uas_router_data(
                state,
                connector_name.to_string(),
                payment_method,
                merchant_id.clone(),
                None,
                pre_authentication_data,
                merchant_connector_account,
                Some(authentication_id.to_owned()),
                payment_id.cloned(),
            )?;

        Box::pin(utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            pre_auth_router_data,
        ))
        .await
    }

    async fn post_authentication(
        state: &SessionState,
        _business_profile: &domain::Profile,
        payment_id: Option<&common_utils::id_type::PaymentId>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        authentication_id: &common_utils::id_type::AuthenticationId,
        payment_method: common_enums::PaymentMethod,
        merchant_id: &common_utils::id_type::MerchantId,
        _authentication: Option<&Authentication>,
    ) -> RouterResult<UasPostAuthenticationRouterData> {
        let post_authentication_data = UasPostAuthenticationRequestData {
            threeds_server_transaction_id: None,
        };

        let post_auth_router_data: UasPostAuthenticationRouterData =
            utils::construct_uas_router_data(
                state,
                connector_name.to_string(),
                payment_method,
                merchant_id.clone(),
                None,
                post_authentication_data,
                merchant_connector_account,
                Some(authentication_id.to_owned()),
                payment_id.cloned(),
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
        authentication_id: Option<&common_utils::id_type::AuthenticationId>,
        currency: Option<common_enums::Currency>,
        status: common_enums::AttemptStatus,
        service_details: Option<payments::CtpServiceDetails>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        payment_method: common_enums::PaymentMethod,
        net_amount: common_utils::types::MinorUnit,
        payment_id: Option<&common_utils::id_type::PaymentId>,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> RouterResult<()> {
        let authentication_id = authentication_id
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing authentication id in tracker")?;

        let currency = currency.ok_or(ApiErrorResponse::MissingRequiredField {
            field_name: "currency",
        })?;

        let current_time = common_utils::date_time::date_as_yyyymmddthhmmssmmmz()
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get current time")?;

        let payment_attempt_status = status;

        let (checkout_event_status, confirmation_reason) =
            utils::get_checkout_event_status_and_reason(payment_attempt_status);

        let click_to_pay_details = service_details.clone();

        let authentication_confirmation_data = UasConfirmationRequestData {
            x_src_flow_id: click_to_pay_details
                .as_ref()
                .and_then(|details| details.x_src_flow_id.clone()),
            transaction_amount: net_amount,
            transaction_currency: currency,
            checkout_event_type: Some("01".to_string()), // hardcoded to '01' since only authorise flow is implemented
            checkout_event_status: checkout_event_status.clone(),
            confirmation_status: checkout_event_status.clone(),
            confirmation_reason,
            confirmation_timestamp: Some(current_time),
            network_authorization_code: Some("01".to_string()), // hardcoded to '01' since only authorise flow is implemented
            network_transaction_identifier: Some("mastercard".to_string()), // hardcoded to 'mastercard' since only mastercard has confirmation flow requirement
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
            merchant_id.clone(),
            None,
            authentication_confirmation_data,
            merchant_connector_account,
            Some(authentication_id.to_owned()),
            payment_id.cloned(),
        )?;

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            authentication_confirmation_router_data,
        )
        .await
        .ok(); // marking this as .ok() since this is not a required step at our end for completing the transaction

        Ok(())
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl UnifiedAuthenticationService for ExternalAuthentication {
    fn get_pre_authentication_request_data(
        payment_method_data: Option<&domain::PaymentMethodData>,
        _service_details: Option<payments::CtpServiceDetails>,
        amount: common_utils::types::MinorUnit,
        currency: Option<common_enums::Currency>,
        merchant_details: Option<&hyperswitch_domain_models::router_request_types::unified_authentication_service::MerchantDetails>,
        billing_address: Option<&hyperswitch_domain_models::address::Address>,
        acquirer_bin: Option<String>,
        acquirer_merchant_id: Option<String>,
        payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<UasPreAuthenticationRequestData> {
        let payment_method_data = payment_method_data
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("payment_method_data is missing")?;
        let payment_details =
            if let payment_method_data::PaymentMethodData::Card(card) = payment_method_data {
                Some(PaymentDetails {
                    pan: card.card_number.clone(),
                    digital_card_id: None,
                    payment_data_type: payment_method_type,
                    encrypted_src_card_details: None,
                    card_expiry_month: card.card_exp_month.clone(),
                    card_expiry_year: card.card_exp_year.clone(),
                    cardholder_name: card.card_holder_name.clone(),
                    card_token_number: None,
                    account_type: payment_method_type,
                    card_cvc: Some(card.card_cvc.clone()),
                })
            } else {
                None
            };
        let transaction_details = TransactionDetails {
            amount: Some(amount),
            currency,
            device_channel: None,
            message_category: None,
            force_3ds_challenge: None,
            psd2_sca_exemption_type: None,
        };
        Ok(UasPreAuthenticationRequestData {
            service_details: None,
            transaction_details: Some(transaction_details),
            payment_details,
            authentication_info: None,
            merchant_details: merchant_details.cloned(),
            billing_address: billing_address.cloned(),
            acquirer_bin,
            acquirer_merchant_id,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn pre_authentication(
        state: &SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: Option<&common_utils::id_type::PaymentId>,
        payment_method_data: Option<&domain::PaymentMethodData>,
        payment_method_type: Option<common_enums::PaymentMethodType>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        authentication_id: &common_utils::id_type::AuthenticationId,
        payment_method: common_enums::PaymentMethod,
        amount: common_utils::types::MinorUnit,
        currency: Option<common_enums::Currency>,
        service_details: Option<payments::CtpServiceDetails>,
        merchant_details: Option<&hyperswitch_domain_models::router_request_types::unified_authentication_service::MerchantDetails>,
        billing_address: Option<&hyperswitch_domain_models::address::Address>,
        acquirer_bin: Option<String>,
        acquirer_merchant_id: Option<String>,
    ) -> RouterResult<UasPreAuthenticationRouterData> {
        let pre_authentication_data = Self::get_pre_authentication_request_data(
            payment_method_data,
            service_details,
            amount,
            currency,
            merchant_details,
            billing_address,
            acquirer_bin,
            acquirer_merchant_id,
            payment_method_type,
        )?;

        let pre_auth_router_data: UasPreAuthenticationRouterData =
            utils::construct_uas_router_data(
                state,
                connector_name.to_string(),
                payment_method,
                merchant_id.clone(),
                None,
                pre_authentication_data,
                merchant_connector_account,
                Some(authentication_id.to_owned()),
                payment_id.cloned(),
            )?;

        Box::pin(utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            pre_auth_router_data,
        ))
        .await
    }

    fn get_authentication_request_data(
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
        force_3ds_challenge: Option<bool>,
        psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    ) -> RouterResult<UasAuthenticationRequestData> {
        Ok(UasAuthenticationRequestData {
            browser_details,
            transaction_details: TransactionDetails {
                amount,
                currency,
                device_channel: Some(device_channel),
                message_category: Some(message_category),
                force_3ds_challenge,
                psd2_sca_exemption_type,
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
            webhook_url,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn authentication(
        state: &SessionState,
        business_profile: &domain::Profile,
        payment_method: &common_enums::PaymentMethod,
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
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        payment_id: Option<common_utils::id_type::PaymentId>,
        force_3ds_challenge: Option<bool>,
        psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    ) -> RouterResult<UasAuthenticationRouterData> {
        let authentication_data =
            <Self as UnifiedAuthenticationService>::get_authentication_request_data(
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
                force_3ds_challenge,
                psd2_sca_exemption_type,
            )?;
        let auth_router_data: UasAuthenticationRouterData = utils::construct_uas_router_data(
            state,
            connector_name.to_string(),
            payment_method.to_owned(),
            business_profile.merchant_id.clone(),
            None,
            authentication_data,
            merchant_connector_account,
            Some(authentication.authentication_id.to_owned()),
            payment_id,
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
        business_profile: &domain::Profile,
        payment_id: Option<&common_utils::id_type::PaymentId>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
        authentication_id: &common_utils::id_type::AuthenticationId,
        payment_method: common_enums::PaymentMethod,
        _merchant_id: &common_utils::id_type::MerchantId,
        authentication: Option<&Authentication>,
    ) -> RouterResult<UasPostAuthenticationRouterData> {
        let authentication_data =
            <Self as UnifiedAuthenticationService>::get_post_authentication_request_data(
                authentication.cloned(),
            )?;
        let auth_router_data: UasPostAuthenticationRouterData = utils::construct_uas_router_data(
            state,
            connector_name.to_string(),
            payment_method,
            business_profile.merchant_id.clone(),
            None,
            authentication_data,
            merchant_connector_account,
            Some(authentication_id.clone()),
            payment_id.cloned(),
        )?;

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            auth_router_data,
        )
        .await
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn create_new_authentication(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    authentication_connector: Option<String>,
    profile_id: common_utils::id_type::ProfileId,
    payment_id: Option<common_utils::id_type::PaymentId>,
    merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    authentication_id: &common_utils::id_type::AuthenticationId,
    service_details: Option<payments::CtpServiceDetails>,
    authentication_status: common_enums::AuthenticationStatus,
    network_token: Option<payment_method_data::NetworkTokenData>,
    organization_id: common_utils::id_type::OrganizationId,
    force_3ds_challenge: Option<bool>,
    psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    acquirer_bin: Option<String>,
    acquirer_merchant_id: Option<String>,
    acquirer_country_code: Option<String>,
    amount: Option<common_utils::types::MinorUnit>,
    currency: Option<common_enums::Currency>,
    return_url: Option<String>,
    profile_acquirer_id: Option<common_utils::id_type::ProfileAcquirerId>,
    customer_details: Option<common_utils::encryption::Encryption>,
) -> RouterResult<Authentication> {
    let service_details_value = service_details
        .map(serde_json::to_value)
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "unable to parse service details into json value while inserting to DB",
        )?;
    let authentication_client_secret = Some(common_utils::generate_id_with_default_len(&format!(
        "{}_secret",
        authentication_id.get_string_repr()
    )));
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
        cavv: None,
        authentication_flow_type: None,
        message_version: None,
        eci: network_token.and_then(|data| data.eci),
        trans_status: None,
        acquirer_bin,
        acquirer_merchant_id,
        three_ds_method_data: None,
        three_ds_method_url: None,
        acs_url: None,
        challenge_request: None,
        challenge_request_key: None,
        acs_reference_number: None,
        acs_trans_id: None,
        acs_signed_content: None,
        profile_id,
        payment_id,
        merchant_connector_id,
        ds_trans_id: None,
        directory_server_id: None,
        acquirer_country_code,
        service_details: service_details_value,
        organization_id,
        authentication_client_secret,
        force_3ds_challenge,
        psd2_sca_exemption_type,
        return_url,
        amount,
        currency,
        billing_address: None,
        shipping_address: None,
        browser_info: None,
        email: None,
        profile_acquirer_id,
        challenge_code: None,
        challenge_cancel: None,
        challenge_code_reason: None,
        message_extension: None,
        customer_details,
    };
    state
        .store
        .insert_authentication(new_authorization)
        .await
        .to_duplicate_response(ApiErrorResponse::GenericDuplicateError {
            message: format!(
                "Authentication with authentication_id {} already exists",
                authentication_id.get_string_repr()
            ),
        })
}

// Modular authentication
#[cfg(feature = "v1")]
pub async fn authentication_create_core(
    state: SessionState,
    platform: domain::Platform,
    req: AuthenticationCreateRequest,
) -> RouterResponse<AuthenticationResponse> {
    let db = &*state.store;
    let merchant_account = platform.get_processor().get_account();
    let merchant_id = merchant_account.get_id();
    let key_manager_state = (&state).into();
    let profile_id = core_utils::get_profile_id_from_business_details(
        None,
        None,
        &platform,
        req.profile_id.as_ref(),
        db,
        true,
    )
    .await?;

    let business_profile = db
        .find_business_profile_by_profile_id(platform.get_processor().get_key_store(), &profile_id)
        .await
        .to_not_found_response(ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;
    let organization_id = merchant_account.organization_id.clone();
    let authentication_id = common_utils::id_type::AuthenticationId::generate_authentication_id(
        consts::AUTHENTICATION_ID_PREFIX,
    );

    let force_3ds_challenge = Some(
        req.force_3ds_challenge
            .unwrap_or(business_profile.force_3ds_challenge),
    );

    // Priority logic: First check req.acquirer_details, then fallback to profile_acquirer_id lookup
    let (acquirer_bin, acquirer_merchant_id, acquirer_country_code) =
        if let Some(acquirer_details) = &req.acquirer_details {
            // Priority 1: Use acquirer_details from request if present
            (
                acquirer_details.acquirer_bin.clone(),
                acquirer_details.acquirer_merchant_id.clone(),
                acquirer_details.merchant_country_code.clone(),
            )
        } else {
            // Priority 2: Fallback to profile_acquirer_id lookup
            let acquirer_details = req.profile_acquirer_id.clone().and_then(|acquirer_id| {
                business_profile
                    .acquirer_config_map
                    .and_then(|acquirer_config_map| {
                        acquirer_config_map.0.get(&acquirer_id).cloned()
                    })
            });

            acquirer_details
                .as_ref()
                .map(|details| {
                    (
                        Some(details.acquirer_bin.clone()),
                        Some(details.acquirer_assigned_merchant_id.clone()),
                        business_profile
                            .merchant_country_code
                            .map(|code| code.get_country_code().to_owned()),
                    )
                })
                .unwrap_or((None, None, None))
        };

    let customer_details = req
        .customer_details
        .clone()
        .async_lift(|customer_details| async {
            domain::types::crypto_operation(
                &key_manager_state,
                common_utils::type_name!(Authentication),
                domain::types::CryptoOperation::EncryptOptional(
                    customer_details
                        .map(|details| {
                            common_utils::ext_traits::Encode::encode_to_value(&details)
                                .map(masking::Secret::<serde_json::Value>::new)
                                .change_context(ApiErrorResponse::InternalServerError)
                                .attach_printable(
                                    "Unable to encode customer details to serde_json::Value",
                                )
                        })
                        .transpose()?,
                ),
                common_utils::types::keymanager::Identifier::Merchant(
                    platform.get_processor().get_key_store().merchant_id.clone(),
                ),
                platform.get_processor().get_key_store().key.peek(),
            )
            .await
            .and_then(|val| val.try_into_optionaloperation())
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt customer details")
        })
        .await?;

    let new_authentication = create_new_authentication(
        &state,
        merchant_id.clone(),
        req.authentication_connector
            .map(|connector| connector.to_string()),
        profile_id.clone(),
        None,
        None,
        &authentication_id,
        None,
        common_enums::AuthenticationStatus::Started,
        None,
        organization_id,
        force_3ds_challenge,
        req.psd2_sca_exemption_type,
        acquirer_bin,
        acquirer_merchant_id,
        acquirer_country_code,
        Some(req.amount),
        Some(req.currency),
        req.return_url,
        req.profile_acquirer_id.clone(),
        customer_details
            .clone()
            .map(common_utils::encryption::Encryption::from),
    )
    .await?;

    let acquirer_details = Some(AcquirerDetails {
        acquirer_bin: new_authentication.acquirer_bin.clone(),
        acquirer_merchant_id: new_authentication.acquirer_merchant_id.clone(),
        merchant_country_code: new_authentication.acquirer_country_code.clone(),
    });

    let amount = new_authentication
        .amount
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("amount failed to get amount from authentication table")?;
    let currency = new_authentication
        .currency
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("currency failed to get currency from authentication table")?;

    let customer_details_decrypted = new_authentication
        .customer_details
        .clone()
        .async_lift(|inner| async {
            domain::types::crypto_operation::<serde_json::Value, masking::WithType>(
                &key_manager_state,
                common_utils::type_name!(Authentication),
                domain::types::CryptoOperation::DecryptOptional(inner),
                common_utils::types::keymanager::Identifier::Merchant(
                    platform.get_processor().get_key_store().merchant_id.clone(),
                ),
                platform.get_processor().get_key_store().key.peek(),
            )
            .await
            .and_then(|val| val.try_into_optionaloperation())
        })
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to decrypt email from authentication table")?;

    let customer_details = customer_details_decrypted
        .map(|inner| inner.parse_value("CustomerData"))
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing customer data from authentication table")?;

    let response = AuthenticationResponse::foreign_try_from((
        new_authentication.clone(),
        amount,
        currency,
        profile_id,
        acquirer_details,
        new_authentication.profile_acquirer_id,
        customer_details,
        req.customer_details.map(|details| details.id.clone()),
    ))?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

impl
    ForeignTryFrom<(
        Authentication,
        common_utils::types::MinorUnit,
        common_enums::Currency,
        common_utils::id_type::ProfileId,
        Option<AcquirerDetails>,
        Option<common_utils::id_type::ProfileAcquirerId>,
        Option<hyperswitch_domain_models::payments::payment_intent::CustomerData>,
        Option<common_utils::id_type::CustomerId>,
    )> for AuthenticationResponse
{
    type Error = error_stack::Report<ApiErrorResponse>;
    fn foreign_try_from(
        (
            authentication,
            amount,
            currency,
            profile_id,
            acquirer_details,
            profile_acquirer_id,
            customer_data,
            customer_id,
        ): (
            Authentication,
            common_utils::types::MinorUnit,
            common_enums::Currency,
            common_utils::id_type::ProfileId,
            Option<AcquirerDetails>,
            Option<common_utils::id_type::ProfileAcquirerId>,
            Option<hyperswitch_domain_models::payments::payment_intent::CustomerData>,
            Option<common_utils::id_type::CustomerId>,
        ),
    ) -> Result<Self, Self::Error> {
        let authentication_connector = authentication
            .authentication_connector
            .map(|connector| common_enums::AuthenticationConnectors::from_str(&connector))
            .transpose()
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Incorrect authentication connector stored in table")?;

        let customer_details = if let Some(details) = customer_data {
            let customer_id = customer_id
                .as_ref()
                .ok_or(ApiErrorResponse::InternalServerError)
                .attach_printable("Customer id not found in authentication create request")?;
            Some(CustomerDetails {
                id: customer_id.clone(),
                name: details.name,
                email: details.email,
                phone: details.phone,
                phone_country_code: details.phone_country_code,
                tax_registration_id: details.tax_registration_id,
            })
        } else {
            None
        };

        Ok(Self {
            authentication_id: authentication.authentication_id,
            client_secret: authentication
                .authentication_client_secret
                .map(masking::Secret::new),
            amount,
            currency,
            force_3ds_challenge: authentication.force_3ds_challenge,
            merchant_id: authentication.merchant_id,
            status: authentication.authentication_status,
            authentication_connector,
            return_url: authentication.return_url,
            created_at: Some(authentication.created_at),
            error_code: authentication.error_code,
            error_message: authentication.error_message,
            profile_id: Some(profile_id),
            psd2_sca_exemption_type: authentication.psd2_sca_exemption_type,
            acquirer_details,
            profile_acquirer_id,
            customer_details,
        })
    }
}

#[cfg(feature = "v1")]
impl
    ForeignTryFrom<(
        Authentication,
        api_models::authentication::NextAction,
        common_utils::id_type::ProfileId,
        Option<payments::Address>,
        Option<payments::Address>,
        Option<payments::BrowserInformation>,
        common_utils::crypto::OptionalEncryptableEmail,
    )> for AuthenticationEligibilityResponse
{
    type Error = error_stack::Report<ApiErrorResponse>;
    fn foreign_try_from(
        (authentication, next_action, profile_id, billing, shipping, browser_information, email): (
            Authentication,
            api_models::authentication::NextAction,
            common_utils::id_type::ProfileId,
            Option<payments::Address>,
            Option<payments::Address>,
            Option<payments::BrowserInformation>,
            common_utils::crypto::OptionalEncryptableEmail,
        ),
    ) -> Result<Self, Self::Error> {
        let authentication_connector = authentication
            .authentication_connector
            .map(|connector| common_enums::AuthenticationConnectors::from_str(&connector))
            .transpose()
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Incorrect authentication connector stored in table")?;
        let three_ds_method_url = authentication
            .three_ds_method_url
            .map(|url| url::Url::parse(&url))
            .transpose()
            .map_err(error_stack::Report::from)
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse three_ds_method_url")?;

        let three_ds_data = Some(api_models::authentication::ThreeDsData {
            three_ds_server_transaction_id: authentication.threeds_server_transaction_id,
            maximum_supported_3ds_version: authentication.maximum_supported_version,
            connector_authentication_id: authentication.connector_authentication_id,
            three_ds_method_data: authentication.three_ds_method_data,
            three_ds_method_url,
            message_version: authentication.message_version,
            directory_server_id: authentication.directory_server_id,
        });
        let acquirer_details = AcquirerDetails {
            acquirer_bin: authentication.acquirer_bin,
            acquirer_merchant_id: authentication.acquirer_merchant_id,
            merchant_country_code: authentication.acquirer_country_code,
        };
        Ok(Self {
            authentication_id: authentication.authentication_id,
            next_action,
            status: authentication.authentication_status,
            eligibility_response_params: three_ds_data
                .map(api_models::authentication::EligibilityResponseParams::ThreeDsData),
            connector_metadata: authentication.connector_metadata,
            profile_id,
            error_message: authentication.error_message,
            error_code: authentication.error_code,
            billing,
            shipping,
            authentication_connector,
            browser_information,
            email,
            acquirer_details: Some(acquirer_details),
        })
    }
}

#[cfg(feature = "v1")]
pub async fn authentication_eligibility_core(
    state: SessionState,
    platform: domain::Platform,
    req: AuthenticationEligibilityRequest,
    authentication_id: common_utils::id_type::AuthenticationId,
) -> RouterResponse<AuthenticationEligibilityResponse> {
    let merchant_account = platform.get_processor().get_account();
    let merchant_id = merchant_account.get_id();
    let db = &*state.store;
    let authentication = db
        .find_authentication_by_merchant_id_authentication_id(merchant_id, &authentication_id)
        .await
        .to_not_found_response(ApiErrorResponse::AuthenticationNotFound {
            id: authentication_id.get_string_repr().to_owned(),
        })?;

    req.client_secret
        .clone()
        .map(|client_secret| {
            utils::authenticate_authentication_client_secret_and_check_expiry(
                client_secret.peek(),
                &authentication,
            )
        })
        .transpose()?;

    ensure_not_terminal_status(authentication.trans_status.clone())?;

    let key_manager_state = (&state).into();

    let profile_id = core_utils::get_profile_id_from_business_details(
        None,
        None,
        &platform,
        req.profile_id.as_ref(),
        db,
        true,
    )
    .await?;

    let business_profile = db
        .find_business_profile_by_profile_id(platform.get_processor().get_key_store(), &profile_id)
        .await
        .to_not_found_response(ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let (authentication_connector, three_ds_connector_account) =
        auth_utils::get_authentication_connector_data(
            &state,
            platform.get_processor().get_key_store(),
            &business_profile,
            authentication.authentication_connector.clone(),
        )
        .await?;

    let notification_url = match authentication_connector {
        common_enums::AuthenticationConnectors::Juspaythreedsserver => {
            Some(url::Url::parse(&format!(
                "{base_url}/authentication/{merchant_id}/{authentication_id}/redirect",
                base_url = state.base_url,
                merchant_id = merchant_id.get_string_repr(),
                authentication_id = authentication_id.get_string_repr()
            )))
            .transpose()
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse notification url")?
        }
        _ => authentication
            .return_url
            .as_ref()
            .map(|url| url::Url::parse(url))
            .transpose()
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse return url")?,
    };

    let authentication_connector_name = authentication_connector.to_string();

    let payment_method_data = domain::PaymentMethodData::from(req.payment_method_data.clone());

    let amount = authentication
        .amount
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("no amount found in authentication table")?;

    let acquirer_details = authentication
        .profile_acquirer_id
        .clone()
        .and_then(|acquirer_id| {
            business_profile
                .acquirer_config_map
                .and_then(|acquirer_config_map| acquirer_config_map.0.get(&acquirer_id).cloned())
        });

    let metadata: Option<ThreeDsMetaData> = three_ds_connector_account
        .get_metadata()
        .map(|metadata| {
            metadata.expose().parse_value("ThreeDsMetaData").inspect_err(|err| {
            router_env::logger::warn!(parsing_error=?err,"Error while parsing ThreeDsMetaData");
        })
        })
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)?;

    let merchant_country_code = authentication.acquirer_country_code.clone();

    let merchant_details = Some(hyperswitch_domain_models::router_request_types::unified_authentication_service::MerchantDetails {
        merchant_id: Some(authentication.merchant_id.get_string_repr().to_string()),
        merchant_name: acquirer_details.clone().map(|detail| detail.merchant_name.clone()).or(metadata.clone().and_then(|metadata| metadata.merchant_name)),
        merchant_category_code: business_profile.merchant_category_code.or(metadata.clone().and_then(|metadata| metadata.merchant_category_code)),
        endpoint_prefix: metadata.clone().and_then(|metadata| metadata.endpoint_prefix),
        three_ds_requestor_url: business_profile.authentication_connector_details.map(|details| details.three_ds_requestor_url),
        three_ds_requestor_id: metadata.clone().and_then(|metadata| metadata.three_ds_requestor_id),
        three_ds_requestor_name: metadata.clone().and_then(|metadata| metadata.three_ds_requestor_name),
        merchant_country_code: merchant_country_code.map(common_types::payments::MerchantCountryCode::new),
        notification_url,
    });

    let domain_address = req
        .billing
        .clone()
        .map(hyperswitch_domain_models::address::Address::from);

    let pre_auth_response =
        <ExternalAuthentication as UnifiedAuthenticationService>::pre_authentication(
            &state,
            merchant_id,
            None,
            Some(&payment_method_data),
            req.payment_method_type,
            &three_ds_connector_account,
            &authentication_connector_name,
            &authentication_id,
            req.payment_method,
            amount,
            authentication.currency,
            None,
            merchant_details.as_ref(),
            domain_address.as_ref(),
            authentication.acquirer_bin.clone(),
            authentication.acquirer_merchant_id.clone(),
        )
        .await?;

    let billing_details_encoded = req
        .billing
        .clone()
        .map(|billing| {
            common_utils::ext_traits::Encode::encode_to_value(&billing)
                .map(masking::Secret::<serde_json::Value>::new)
        })
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encode billing details to serde_json::Value")?;

    let shipping_details_encoded = req
        .shipping
        .clone()
        .map(|shipping| {
            common_utils::ext_traits::Encode::encode_to_value(&shipping)
                .map(masking::Secret::<serde_json::Value>::new)
        })
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encode shipping details to serde_json::Value")?;

    let encrypted_data = domain::types::crypto_operation(
        &key_manager_state,
        common_utils::type_name!(hyperswitch_domain_models::authentication::Authentication),
        domain::types::CryptoOperation::BatchEncrypt(
            hyperswitch_domain_models::authentication::UpdateEncryptableAuthentication::to_encryptable(
                hyperswitch_domain_models::authentication::UpdateEncryptableAuthentication {
                    billing_address: billing_details_encoded,
                    shipping_address: shipping_details_encoded,
                },
            ),
        ),
        common_utils::types::keymanager::Identifier::Merchant(
            platform
                .get_processor().get_key_store()
                .merchant_id
                .clone(),
        ),
        platform.get_processor().get_key_store().key.peek(),
    )
    .await
    .and_then(|val| val.try_into_batchoperation())
    .change_context(ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to encrypt authentication data".to_string())?;

    let encrypted_data = hyperswitch_domain_models::authentication::FromRequestEncryptableAuthentication::from_encryptable(encrypted_data)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to get encrypted data for authentication after encryption")?;

    let email_encrypted = req
        .email
        .clone()
        .async_lift(|inner| async {
            domain::types::crypto_operation(
                &key_manager_state,
                common_utils::type_name!(Authentication),
                domain::types::CryptoOperation::EncryptOptional(inner.map(|inner| inner.expose())),
                common_utils::types::keymanager::Identifier::Merchant(
                    platform.get_processor().get_key_store().merchant_id.clone(),
                ),
                platform.get_processor().get_key_store().key.peek(),
            )
            .await
            .and_then(|val| val.try_into_optionaloperation())
        })
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt email")?;

    let browser_info = req
        .browser_information
        .as_ref()
        .map(common_utils::ext_traits::Encode::encode_to_value)
        .transpose()
        .change_context(ApiErrorResponse::InvalidDataValue {
            field_name: "browser_information",
        })?;

    let updated_authentication = utils::external_authentication_update_trackers(
        &state,
        pre_auth_response,
        authentication.clone(),
        None,
        platform.get_processor().get_key_store(),
        encrypted_data
            .billing_address
            .map(common_utils::encryption::Encryption::from),
        encrypted_data
            .shipping_address
            .map(common_utils::encryption::Encryption::from),
        email_encrypted
            .clone()
            .map(common_utils::encryption::Encryption::from),
        browser_info,
    )
    .await?;

    let response = AuthenticationEligibilityResponse::foreign_try_from((
        updated_authentication,
        req.get_next_action_api(
            state.base_url,
            authentication_id.get_string_repr().to_string(),
        )
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to get next action api")?,
        profile_id,
        req.get_billing_address(),
        req.get_shipping_address(),
        req.get_browser_information(),
        email_encrypted,
    ))?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

#[cfg(feature = "v1")]
pub async fn authentication_authenticate_core(
    state: SessionState,
    platform: domain::Platform,
    req: AuthenticationAuthenticateRequest,
    auth_flow: AuthFlow,
) -> RouterResponse<AuthenticationAuthenticateResponse> {
    let authentication_id = req.authentication_id.clone();
    let merchant_account = platform.get_processor().get_account();
    let merchant_id = merchant_account.get_id();
    let db = &*state.store;
    let authentication = db
        .find_authentication_by_merchant_id_authentication_id(merchant_id, &authentication_id)
        .await
        .to_not_found_response(ApiErrorResponse::AuthenticationNotFound {
            id: authentication_id.get_string_repr().to_owned(),
        })?;

    req.client_secret
        .map(|client_secret| {
            utils::authenticate_authentication_client_secret_and_check_expiry(
                client_secret.peek(),
                &authentication,
            )
        })
        .transpose()?;

    ensure_not_terminal_status(authentication.trans_status.clone())?;

    let key_manager_state = (&state).into();

    let profile_id = authentication.profile_id.clone();

    let business_profile = db
        .find_business_profile_by_profile_id(platform.get_processor().get_key_store(), &profile_id)
        .await
        .to_not_found_response(ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let email_encrypted = authentication
        .email
        .clone()
        .async_lift(|inner| async {
            domain::types::crypto_operation(
                &key_manager_state,
                common_utils::type_name!(Authentication),
                domain::types::CryptoOperation::DecryptOptional(inner),
                common_utils::types::keymanager::Identifier::Merchant(
                    platform.get_processor().get_key_store().merchant_id.clone(),
                ),
                platform.get_processor().get_key_store().key.peek(),
            )
            .await
            .and_then(|val| val.try_into_optionaloperation())
        })
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to decrypt email from authentication table")?;

    let browser_info = authentication
        .browser_info
        .clone()
        .map(|browser_info| browser_info.parse_value::<BrowserInformation>("BrowserInformation"))
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse browser information from authentication table")?;

    let (authentication_connector, three_ds_connector_account) =
        auth_utils::get_authentication_connector_data(
            &state,
            platform.get_processor().get_key_store(),
            &business_profile,
            authentication.authentication_connector.clone(),
        )
        .await?;

    let authentication_details = business_profile
        .authentication_connector_details
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("authentication_connector_details not configured by the merchant")?;

    let connector_name_string = authentication_connector.to_string();
    let mca_id_option = three_ds_connector_account.get_mca_id();
    let merchant_connector_account_id_or_connector_name = mca_id_option
        .as_ref()
        .map(|mca_id| mca_id.get_string_repr())
        .unwrap_or(&connector_name_string);

    let webhook_url = helpers::create_webhook_url(
        &state.base_url,
        merchant_id,
        merchant_connector_account_id_or_connector_name,
    );

    let auth_response = <ExternalAuthentication as UnifiedAuthenticationService>::authentication(
        &state,
        &business_profile,
        &common_enums::PaymentMethod::Card,
        browser_info,
        authentication.amount,
        authentication.currency,
        MessageCategory::Payment,
        req.device_channel,
        authentication.clone(),
        None,
        req.sdk_information,
        req.threeds_method_comp_ind,
        email_encrypted.map(common_utils::pii::Email::from),
        webhook_url,
        &three_ds_connector_account,
        &authentication_connector.to_string(),
        None,
        authentication.force_3ds_challenge,
        authentication.psd2_sca_exemption_type,
    )
    .await?;

    let authentication = utils::external_authentication_update_trackers(
        &state,
        auth_response,
        authentication.clone(),
        None,
        platform.get_processor().get_key_store(),
        None,
        None,
        None,
        None,
    )
    .await?;

    let (authentication_value, eci) = match auth_flow {
        AuthFlow::Client => (None, None),
        AuthFlow::Merchant => {
            if let Some(common_enums::TransactionStatus::Success) = authentication.trans_status {
                let tokenised_data = payment_methods::vault::get_tokenized_data(
                    &state,
                    authentication_id.get_string_repr(),
                    false,
                    platform.get_processor().get_key_store().key.get_inner(),
                )
                .await
                .inspect_err(|err| router_env::logger::error!(tokenized_data_result=?err))
                .attach_printable("cavv not present after authentication status is success")?;
                (
                    Some(masking::Secret::new(tokenised_data.value1)),
                    authentication.eci.clone(),
                )
            } else {
                (None, None)
            }
        }
    };

    let response = AuthenticationAuthenticateResponse::foreign_try_from((
        &authentication,
        authentication_value,
        eci,
        authentication_details,
    ))?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

// Trait for Eligibility Checks
#[cfg(feature = "v1")]
#[async_trait::async_trait]
trait EligibilityCheck {
    type Output;

    // Determine if the check should be run based on the runtime checks
    async fn should_run(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
    ) -> CustomResult<bool, ApiErrorResponse>;

    // Run the actual check and return the SDK Next Action if applicable
    async fn execute_check(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        authentication_eligibility_check_request: &AuthenticationEligibilityCheckRequest,
    ) -> CustomResult<Self::Output, ApiErrorResponse>;

    fn transform(output: Self::Output) -> Option<AuthenticationSdkNextAction>;
}

// Result of an Eligibility Check
#[cfg(feature = "v1")]
#[derive(Debug, Clone)]
pub enum CheckResult {
    Allow,
    Deny { message: String },
    Await,
}

#[cfg(feature = "v1")]
impl From<CheckResult> for Option<AuthenticationSdkNextAction> {
    fn from(result: CheckResult) -> Self {
        match result {
            CheckResult::Allow => None,
            CheckResult::Deny { message } => Some(AuthenticationSdkNextAction::Deny { message }),
            CheckResult::Await => Some(AuthenticationSdkNextAction::AwaitMerchantCallback),
        }
    }
}

// Perform StoreEligibilityCheckData for the authentication
#[cfg(feature = "v1")]
struct StoreEligibilityCheckData;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl EligibilityCheck for StoreEligibilityCheckData {
    type Output = CheckResult;

    async fn should_run(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
    ) -> CustomResult<bool, ApiErrorResponse> {
        let merchant_id = platform.get_processor().get_account().get_id();
        let should_store_eligibility_check_data_key =
            merchant_id.get_should_store_eligibility_check_data_for_authentication();
        let should_store_eligibility_check_data = state
            .store
            .find_config_by_key_unwrap_or(
                &should_store_eligibility_check_data_key,
                Some("false".to_string()),
            )
            .await;

        Ok(match should_store_eligibility_check_data {
            Ok(config) => serde_json::from_str(&config.config).unwrap_or(false),

            // If it is not present in db we are defaulting it to false
            Err(inner) => {
                if !inner.current_context().is_db_not_found() {
                    router_env::logger::error!(
                        "Error fetching should store eligibility check data enabled config {:?}",
                        inner
                    );
                }
                false
            }
        })
    }

    async fn execute_check(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        authentication_eligibility_check_request: &AuthenticationEligibilityCheckRequest,
    ) -> CustomResult<CheckResult, ApiErrorResponse> {
        let redis = &state
            .store
            .get_redis_conn()
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;
        let key = format!(
            "{}_{}_{}",
            consts::AUTHENTICATION_ELIGIBILITY_CHECK_DATA_KEY,
            platform
                .get_processor()
                .get_account()
                .get_id()
                .get_string_repr(),
            authentication_eligibility_check_request
                .authentication_id
                .get_string_repr()
        );
        redis
            .serialize_and_set_key_with_expiry(
                &key.as_str().into(),
                &authentication_eligibility_check_request.eligibility_check_data,
                consts::AUTHENTICATION_ELIGIBILITY_CHECK_DATA_TTL,
            )
            .await
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to set key in redis")?;
        Ok(CheckResult::Await)
    }

    fn transform(output: CheckResult) -> Option<AuthenticationSdkNextAction> {
        output.into()
    }
}

// Eligibility handler to run all the eligibility checks
#[cfg(feature = "v1")]
pub struct EligibilityHandler {
    state: SessionState,
    platform: domain::Platform,
    authentication_eligibility_check_request: AuthenticationEligibilityCheckRequest,
}

#[cfg(feature = "v1")]
impl EligibilityHandler {
    fn new(
        state: SessionState,
        platform: domain::Platform,
        authentication_eligibility_check_request: AuthenticationEligibilityCheckRequest,
    ) -> Self {
        Self {
            state,
            platform,
            authentication_eligibility_check_request,
        }
    }

    async fn run_check<C: EligibilityCheck>(
        &self,
        check: C,
    ) -> CustomResult<Option<AuthenticationSdkNextAction>, ApiErrorResponse> {
        let should_run = check.should_run(&self.state, &self.platform).await?;
        Ok(match should_run {
            true => check
                .execute_check(
                    &self.state,
                    &self.platform,
                    &self.authentication_eligibility_check_request,
                )
                .await
                .map(C::transform)?,
            false => None,
        })
    }
}

#[cfg(feature = "v1")]
pub async fn authentication_eligibility_check_core(
    state: SessionState,
    platform: domain::Platform,
    req: AuthenticationEligibilityCheckRequest,
    _auth_flow: AuthFlow,
) -> RouterResponse<AuthenticationEligibilityCheckResponse> {
    let authentication_id = req.authentication_id.clone();
    let eligibility_handler = EligibilityHandler::new(state, platform, req);
    // Run the checks in sequence, short-circuiting on the first that returns a next action
    let sdk_next_action = eligibility_handler
        .run_check(StoreEligibilityCheckData)
        .await?
        .unwrap_or(AuthenticationSdkNextAction::Proceed);
    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        AuthenticationEligibilityCheckResponse {
            authentication_id,
            sdk_next_action,
        },
    ))
}

#[cfg(feature = "v1")]
pub async fn authentication_retrieve_eligibility_check_core(
    state: SessionState,
    platform: domain::Platform,
    req: AuthenticationRetrieveEligibilityCheckRequest,
) -> RouterResponse<AuthenticationRetrieveEligibilityCheckResponse> {
    let redis = &state
        .store
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;
    let key = format!(
        "{}_{}_{}",
        consts::AUTHENTICATION_ELIGIBILITY_CHECK_DATA_KEY,
        platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
        req.authentication_id.get_string_repr()
    );
    let eligibility_check_data: AuthenticationEligibilityCheckData = redis
        .get_key::<String>(&key.as_str().into())
        .await
        .change_context(ApiErrorResponse::GenericNotFoundError {
            message: format!(
                "Eligibility check data not found for authentication id: {}",
                req.authentication_id.get_string_repr()
            ),
        })
        .attach_printable("Failed to get key from redis")?
        .parse_struct("PaymentTokenData")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("failed to deserialize eligibility check data")?;
    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        AuthenticationRetrieveEligibilityCheckResponse {
            authentication_id: req.authentication_id,
            eligibility_check_data:
                AuthenticationEligibilityCheckResponseData::ClickToPayEnrollmentStatus(
                    ClickToPayEligibilityCheckResponseData {
                        visa: eligibility_check_data
                            .get_click_to_pay_data()
                            .and_then(|data| data.visa.clone().map(|visa| visa.consumer_present)),
                        mastercard: eligibility_check_data.get_click_to_pay_data().and_then(
                            |data| {
                                data.mastercard
                                    .clone()
                                    .map(|mastercard| mastercard.consumer_present)
                            },
                        ),
                    },
                ),
        },
    ))
}

impl
    ForeignTryFrom<(
        &Authentication,
        Option<masking::Secret<String>>,
        Option<String>,
        diesel_models::business_profile::AuthenticationConnectorDetails,
    )> for AuthenticationAuthenticateResponse
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn foreign_try_from(
        (authentication, authentication_value, eci, authentication_details): (
            &Authentication,
            Option<masking::Secret<String>>,
            Option<String>,
            diesel_models::business_profile::AuthenticationConnectorDetails,
        ),
    ) -> Result<Self, Self::Error> {
        let authentication_connector = authentication
            .authentication_connector
            .as_ref()
            .map(|connector| common_enums::AuthenticationConnectors::from_str(connector))
            .transpose()
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Incorrect authentication connector stored in table")?;
        let acs_url = authentication
            .acs_url
            .clone()
            .map(|acs_url| url::Url::parse(&acs_url))
            .transpose()
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to parse the url with param")?;
        let acquirer_details = AcquirerDetails {
            acquirer_bin: authentication.acquirer_bin.clone(),
            acquirer_merchant_id: authentication.acquirer_merchant_id.clone(),
            merchant_country_code: authentication.acquirer_country_code.clone(),
        };
        Ok(Self {
            transaction_status: authentication.trans_status.clone(),
            acs_url,
            challenge_request: authentication.challenge_request.clone(),
            acs_reference_number: authentication.acs_reference_number.clone(),
            acs_trans_id: authentication.acs_trans_id.clone(),
            three_ds_server_transaction_id: authentication.threeds_server_transaction_id.clone(),
            acs_signed_content: authentication.acs_signed_content.clone(),
            three_ds_requestor_url: authentication_details.three_ds_requestor_url.clone(),
            three_ds_requestor_app_url: authentication_details.three_ds_requestor_app_url.clone(),
            error_code: None,
            error_message: authentication.error_message.clone(),
            authentication_value,
            status: authentication.authentication_status,
            authentication_connector,
            eci,
            authentication_id: authentication.authentication_id.clone(),
            acquirer_details: Some(acquirer_details),
        })
    }
}

#[cfg(feature = "v1")]
pub async fn authentication_sync_core(
    state: SessionState,
    platform: domain::Platform,
    auth_flow: AuthFlow,
    req: AuthenticationSyncRequest,
) -> RouterResponse<AuthenticationSyncResponse> {
    let authentication_id = req.authentication_id;
    let merchant_account = platform.get_processor().get_account();
    let merchant_id = merchant_account.get_id();
    let db = &*state.store;
    let authentication = db
        .find_authentication_by_merchant_id_authentication_id(merchant_id, &authentication_id)
        .await
        .to_not_found_response(ApiErrorResponse::AuthenticationNotFound {
            id: authentication_id.get_string_repr().to_owned(),
        })?;

    req.client_secret
        .map(|client_secret| {
            utils::authenticate_authentication_client_secret_and_check_expiry(
                client_secret.peek(),
                &authentication,
            )
        })
        .transpose()?;

    let key_manager_state = (&state).into();

    let profile_id = authentication.profile_id.clone();

    let business_profile = db
        .find_business_profile_by_profile_id(platform.get_processor().get_key_store(), &profile_id)
        .await
        .to_not_found_response(ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let (authentication_connector, three_ds_connector_account) =
        auth_utils::get_authentication_connector_data(
            &state,
            platform.get_processor().get_key_store(),
            &business_profile,
            authentication.authentication_connector.clone(),
        )
        .await?;

    if authentication_connector.is_pre_auth_required_in_post_authn_flow() {
        let service_details = req.payment_method_details.and_then(|details| {
            details
                .payment_method_data
                .get_click_to_pay_details()
                .cloned()
        });

        let domain_service_details = hyperswitch_domain_models::router_request_types::unified_authentication_service::CtpServiceDetails {
            service_session_ids: Some(ServiceSessionIds {
                merchant_transaction_id: service_details
                    .as_ref()
                    .and_then(|details| details.merchant_transaction_id.clone()),
                correlation_id: service_details
                    .as_ref()
                    .and_then(|details| details.correlation_id.clone()),
                x_src_flow_id: service_details
                    .as_ref()
                    .and_then(|details| details.x_src_flow_id.clone()),
            }),
            payment_details: None,
        };

        let transaction_details = TransactionDetails {
            amount: authentication.amount,
            currency: authentication.currency,
            device_channel: None,
            message_category: None,
            force_3ds_challenge: authentication.force_3ds_challenge,
            psd2_sca_exemption_type: authentication.psd2_sca_exemption_type,
        };

        let authentication_info = Some(AuthenticationInfo {
            authentication_type: None,
            authentication_reasons: None,
            consent_received: false, // This is not relevant in this flow so keeping it as false
            is_authenticated: false, // This is not relevant in this flow so keeping it as false
            locale: None,
            supported_card_brands: None,
            encrypted_payload: service_details
                .as_ref()
                .and_then(|details| details.encrypted_payload.clone()),
        });
        let pre_authentication_request_data = UasPreAuthenticationRequestData {
            service_details: Some(domain_service_details),
            transaction_details: Some(transaction_details),
            payment_details: None,
            authentication_info,
            merchant_details: None,
            billing_address: None,
            acquirer_bin: None,
            acquirer_merchant_id: None,
        };
        // call pre-auth
        let pre_auth_router_data: UasPreAuthenticationRouterData =
            utils::construct_uas_router_data(
                &state,
                authentication_connector.to_string(),
                common_enums::PaymentMethod::Card,
                authentication.merchant_id.clone(),
                None,
                pre_authentication_request_data,
                &three_ds_connector_account,
                Some(authentication.authentication_id.to_owned()),
                None,
            )?;

        let _pre_auth_response = Box::pin(utils::do_auth_connector_call(
            &state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            pre_auth_router_data,
        ))
        .await?;
    }

    let (updated_authentication, payment_method_data, vault_token_data) =
        if !authentication.authentication_status.is_terminal_status() {
            let post_auth_response = if authentication_connector.is_click_to_pay() {
                ClickToPay::post_authentication(
                    &state,
                    &business_profile,
                    None,
                    &three_ds_connector_account.clone(),
                    &authentication_connector.to_string(),
                    &authentication_id,
                    common_enums::PaymentMethod::Card,
                    merchant_id,
                    None,
                )
                .await?
            } else {
                ExternalAuthentication::post_authentication(
                    &state,
                    &business_profile,
                    None,
                    &three_ds_connector_account,
                    &authentication_connector.to_string(),
                    &authentication_id,
                    common_enums::PaymentMethod::Card,
                    merchant_id,
                    Some(&authentication),
                )
                .await?
            };

            let config = db
                .find_config_by_key_unwrap_or(
                    &merchant_id.get_should_disable_auth_tokenization(),
                    Some("false".to_string()),
                )
                .await;
            let should_disable_auth_tokenization = match config {
                Ok(conf) => conf.config == "true",
                Err(error) => {
                    router_env::logger::error!(?error);
                    false
                }
            };

            let vault_token_data = if should_disable_auth_tokenization {
                // Do not tokenize if the disable flag is present in the config
                None
            } else {
                Box::pin(utils::get_auth_multi_token_from_external_vault(
                    &state,
                    &platform,
                    &business_profile,
                    &post_auth_response,
                ))
                .await?
            };

            let payment_method_data =
                utils::get_authentication_payment_method_data(&post_auth_response);

            let auth_update_response = utils::external_authentication_update_trackers(
                &state,
                post_auth_response,
                authentication.clone(),
                None,
                platform.get_processor().get_key_store(),
                None,
                None,
                None,
                None,
            )
            .await?;

            (auth_update_response, payment_method_data, vault_token_data)
        } else {
            (authentication, None, None)
        };

    let eci = match auth_flow {
        AuthFlow::Client => None,
        AuthFlow::Merchant => {
            if let Some(common_enums::TransactionStatus::Success) =
                updated_authentication.trans_status
            {
                updated_authentication.eci.clone()
            } else {
                None
            }
        }
    };

    let acquirer_details = Some(AcquirerDetails {
        acquirer_bin: updated_authentication.acquirer_bin.clone(),
        acquirer_merchant_id: updated_authentication.acquirer_merchant_id.clone(),
        merchant_country_code: updated_authentication.acquirer_country_code.clone(),
    });

    let encrypted_data = domain::types::crypto_operation(
        &key_manager_state,
        common_utils::type_name!(hyperswitch_domain_models::authentication::Authentication),
        domain::types::CryptoOperation::BatchDecrypt(
            hyperswitch_domain_models::authentication::EncryptedAuthentication::to_encryptable(
                hyperswitch_domain_models::authentication::EncryptedAuthentication {
                    billing_address: updated_authentication.billing_address,
                    shipping_address: updated_authentication.shipping_address,
                },
            ),
        ),
        common_utils::types::keymanager::Identifier::Merchant(
            platform.get_processor().get_key_store().merchant_id.clone(),
        ),
        platform.get_processor().get_key_store().key.peek(),
    )
    .await
    .and_then(|val| val.try_into_batchoperation())
    .change_context(ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to encrypt authentication data".to_string())?;

    let encrypted_data = hyperswitch_domain_models::authentication::FromRequestEncryptableAuthentication::from_encryptable(encrypted_data)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to get encrypted data for authentication after encryption")?;

    let email_decrypted = updated_authentication
        .email
        .clone()
        .async_lift(|inner| async {
            domain::types::crypto_operation(
                &key_manager_state,
                common_utils::type_name!(Authentication),
                domain::types::CryptoOperation::DecryptOptional(inner),
                common_utils::types::keymanager::Identifier::Merchant(
                    platform.get_processor().get_key_store().merchant_id.clone(),
                ),
                platform.get_processor().get_key_store().key.peek(),
            )
            .await
            .and_then(|val| val.try_into_optionaloperation())
        })
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt email")?;

    let browser_info = updated_authentication
        .browser_info
        .clone()
        .map(|browser_info| {
            browser_info.parse_value::<payments::BrowserInformation>("BrowserInformation")
        })
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)?;

    let amount = updated_authentication
        .amount
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("amount failed to get amount from authentication table")?;
    let currency = updated_authentication
        .currency
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("currency failed to get currency from authentication table")?;

    let authentication_connector = updated_authentication
        .authentication_connector
        .map(|connector| common_enums::AuthenticationConnectors::from_str(&connector))
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Incorrect authentication connector stored in table")?;

    let billing = encrypted_data
        .billing_address
        .map(|billing| {
            billing
                .into_inner()
                .expose()
                .parse_value::<payments::Address>("Address")
        })
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse billing address")?;

    let shipping = encrypted_data
        .shipping_address
        .map(|shipping| {
            shipping
                .into_inner()
                .expose()
                .parse_value::<payments::Address>("Address")
        })
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse shipping address")?;

    let response = AuthenticationSyncResponse {
        authentication_id: authentication_id.clone(),
        merchant_id: merchant_id.clone(),
        status: updated_authentication.authentication_status,
        client_secret: updated_authentication
            .authentication_client_secret
            .map(masking::Secret::new),
        amount,
        currency,
        authentication_connector,
        force_3ds_challenge: updated_authentication.force_3ds_challenge,
        return_url: updated_authentication.return_url.clone(),
        created_at: updated_authentication.created_at,
        profile_id: updated_authentication.profile_id.clone(),
        psd2_sca_exemption_type: updated_authentication.psd2_sca_exemption_type,
        acquirer_details,
        error_message: updated_authentication.error_message.clone(),
        error_code: updated_authentication.error_code.clone(),
        threeds_server_transaction_id: updated_authentication.threeds_server_transaction_id.clone(),
        maximum_supported_3ds_version: updated_authentication.maximum_supported_version.clone(),
        connector_authentication_id: updated_authentication.connector_authentication_id.clone(),
        three_ds_method_data: updated_authentication.three_ds_method_data.clone(),
        three_ds_method_url: updated_authentication.three_ds_method_url.clone(),
        message_version: updated_authentication.message_version.clone(),
        connector_metadata: updated_authentication.connector_metadata.clone(),
        directory_server_id: updated_authentication.directory_server_id.clone(),
        payment_method_data,
        vault_token_data,
        billing,
        shipping,
        browser_information: browser_info,
        email: email_decrypted,
        transaction_status: updated_authentication.trans_status.clone(),
        acs_url: updated_authentication.acs_url.clone(),
        challenge_request: updated_authentication.challenge_request.clone(),
        acs_reference_number: updated_authentication.acs_reference_number.clone(),
        acs_trans_id: updated_authentication.acs_trans_id.clone(),
        acs_signed_content: updated_authentication.acs_signed_content,
        three_ds_requestor_url: business_profile
            .authentication_connector_details
            .clone()
            .map(|details| details.three_ds_requestor_url),
        three_ds_requestor_app_url: business_profile
            .authentication_connector_details
            .and_then(|details| details.three_ds_requestor_app_url),
        profile_acquirer_id: updated_authentication.profile_acquirer_id.clone(),
        eci,
    };
    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

#[cfg(feature = "v1")]
pub async fn authentication_post_sync_core(
    state: SessionState,
    platform: domain::Platform,
    req: AuthenticationSyncPostUpdateRequest,
) -> RouterResponse<()> {
    let authentication_id = req.authentication_id;
    let merchant_account = platform.get_processor().get_account();
    let merchant_id = merchant_account.get_id();
    let db = &*state.store;
    let authentication = db
        .find_authentication_by_merchant_id_authentication_id(merchant_id, &authentication_id)
        .await
        .to_not_found_response(ApiErrorResponse::AuthenticationNotFound {
            id: authentication_id.get_string_repr().to_owned(),
        })?;

    ensure_not_terminal_status(authentication.trans_status.clone())?;

    let business_profile = db
        .find_business_profile_by_profile_id(
            platform.get_processor().get_key_store(),
            &authentication.profile_id,
        )
        .await
        .to_not_found_response(ApiErrorResponse::ProfileNotFound {
            id: authentication.profile_id.get_string_repr().to_owned(),
        })?;

    let (authentication_connector, three_ds_connector_account) =
        auth_utils::get_authentication_connector_data(
            &state,
            platform.get_processor().get_key_store(),
            &business_profile,
            authentication.authentication_connector.clone(),
        )
        .await?;

    let post_auth_response =
        <ExternalAuthentication as UnifiedAuthenticationService>::post_authentication(
            &state,
            &business_profile,
            None,
            &three_ds_connector_account,
            &authentication_connector.to_string(),
            &authentication_id,
            common_enums::PaymentMethod::Card,
            merchant_id,
            Some(&authentication),
        )
        .await?;

    let updated_authentication = utils::external_authentication_update_trackers(
        &state,
        post_auth_response,
        authentication.clone(),
        None,
        platform.get_processor().get_key_store(),
        None,
        None,
        None,
        None,
    )
    .await?;

    let authentication_details = business_profile
        .authentication_connector_details
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("authentication_connector_details not configured by the merchant")?;

    let authentication_response = AuthenticationAuthenticateResponse::foreign_try_from((
        &updated_authentication,
        None,
        None,
        authentication_details,
    ))?;

    let redirect_response = helpers::get_handle_response_url_for_modular_authentication(
        authentication_id,
        &business_profile,
        &authentication_response,
        authentication_connector.to_string(),
        authentication.return_url,
        updated_authentication
            .authentication_client_secret
            .clone()
            .map(masking::Secret::new)
            .as_ref(),
        updated_authentication.amount,
    )?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::JsonForRedirection(redirect_response))
}

fn ensure_not_terminal_status(
    status: Option<common_enums::TransactionStatus>,
) -> Result<(), error_stack::Report<ApiErrorResponse>> {
    status
        .filter(|s| s.clone().is_terminal_state())
        .map(|s| {
            Err(error_stack::Report::new(
                ApiErrorResponse::UnprocessableEntity {
                    message: format!(
                        "authentication status for the given authentication_id is already in {s}"
                    ),
                },
            ))
        })
        .unwrap_or(Ok(()))
}

#[cfg(feature = "v1")]
pub async fn authentication_session_core(
    state: SessionState,
    platform: domain::Platform,
    req: AuthenticationSessionTokenRequest,
) -> RouterResponse<api_models::authentication::AuthenticationSessionResponse> {
    let merchant_account = platform.get_processor().get_account();
    let merchant_id = merchant_account.get_id();

    let authentication_id = req.authentication_id;
    let authentication = state
        .store
        .find_authentication_by_merchant_id_authentication_id(merchant_id, &authentication_id)
        .await
        .to_not_found_response(ApiErrorResponse::AuthenticationNotFound {
            id: authentication_id.get_string_repr().to_owned(),
        })?;

    let mut session_tokens = Vec::new();

    let business_profile = state
        .store
        .find_business_profile_by_profile_id(
            platform.get_processor().get_key_store(),
            &authentication.profile_id,
        )
        .await
        .to_not_found_response(ApiErrorResponse::ProfileNotFound {
            id: authentication.profile_id.get_string_repr().to_owned(),
        })?;

    if business_profile.is_click_to_pay_enabled {
        if let Some(value) = business_profile.authentication_product_ids.clone() {
            let session_token = get_session_token_for_click_to_pay(
                &state,
                platform.get_processor().get_account().get_id(),
                &platform,
                value,
                &authentication,
            )
            .await?;
            session_tokens.push(session_token);
        }
    }

    let response = api_models::authentication::AuthenticationSessionResponse {
        authentication_id,
        session_token: session_tokens,
    };

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

#[cfg(feature = "v1")]
pub async fn get_session_token_for_click_to_pay(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    platform: &domain::Platform,
    authentication_product_ids: common_types::payments::AuthenticationConnectorAccountMap,
    authentication: &Authentication,
) -> RouterResult<api_models::authentication::AuthenticationSessionToken> {
    let click_to_pay_mca_id = authentication_product_ids
        .get_click_to_pay_connector_account_id()
        .change_context(ApiErrorResponse::MissingRequiredField {
            field_name: "authentication_product_ids",
        })?;
    let key_manager_state = &(state).into();

    let merchant_connector_account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            &click_to_pay_mca_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: click_to_pay_mca_id.get_string_repr().to_string(),
        })?;

    let click_to_pay_metadata: hyperswitch_domain_models::payments::ClickToPayMetaData =
        merchant_connector_account
            .metadata
            .clone()
            .parse_value("ClickToPayMetaData")
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Error while parsing ClickToPayMetaData")?;
    let transaction_currency = authentication
        .currency
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("currency is not present in payment_data.payment_intent")?;
    let required_amount_type = common_utils::types::StringMajorUnitForConnector;
    let amount = authentication
        .amount
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("amount is not present in authentication")?;
    let transaction_amount = required_amount_type
        .convert(amount, transaction_currency)
        .change_context(ApiErrorResponse::AmountConversionFailed {
            amount_type: "string major unit",
        })?;

    let customer_details_decrypted = authentication
        .customer_details
        .clone()
        .async_lift(|inner| async {
            domain::types::crypto_operation::<serde_json::Value, masking::WithType>(
                key_manager_state,
                common_utils::type_name!(Authentication),
                domain::types::CryptoOperation::DecryptOptional(inner),
                common_utils::types::keymanager::Identifier::Merchant(
                    platform.get_processor().get_key_store().merchant_id.clone(),
                ),
                platform.get_processor().get_key_store().key.peek(),
            )
            .await
            .and_then(|val| val.try_into_optionaloperation())
        })
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to decrypt email from authentication table")?;

    let customer_details = customer_details_decrypted
        .parse_value("CustomerData")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing customer data from authentication table")?;

    validate_customer_details_for_click_to_pay(&customer_details)?;

    let provider = merchant_connector_account
        .get_ctp_service_provider()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Error while parsing ctp service provider from merchant connector account",
        )?;

    let card_brands = [
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::Visa,
    ]
    .iter()
    .cloned()
    .collect::<std::collections::HashSet<_>>();

    Ok(
        api_models::authentication::AuthenticationSessionToken::ClickToPay(Box::new(
            payments::ClickToPaySessionResponse {
                dpa_id: click_to_pay_metadata.dpa_id,
                dpa_name: click_to_pay_metadata.dpa_name,
                locale: click_to_pay_metadata.locale,
                card_brands,
                acquirer_bin: click_to_pay_metadata.acquirer_bin,
                acquirer_merchant_id: click_to_pay_metadata.acquirer_merchant_id,
                merchant_category_code: click_to_pay_metadata.merchant_category_code,
                merchant_country_code: click_to_pay_metadata.merchant_country_code,
                transaction_amount,
                transaction_currency_code: transaction_currency,
                phone_number: customer_details.phone.clone(),
                email: customer_details.email.clone(),
                phone_country_code: customer_details.phone_country_code.clone(),
                provider,
                dpa_client_id: click_to_pay_metadata.dpa_client_id.clone(),
            },
        )),
    )
}
