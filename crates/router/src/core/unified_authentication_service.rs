pub mod types;
use std::str::FromStr;

pub mod utils;
#[cfg(feature = "v1")]
use api_models::authentication::{
    AuthenticationEligibilityRequest, AuthenticationEligibilityResponse,
    AuthenticationSyncPostUpdateRequest, AuthenticationSyncRequest, AuthenticationSyncResponse,
};
use api_models::{
    authentication::{
        AcquirerDetails, AuthenticationAuthenticateRequest, AuthenticationAuthenticateResponse,
        AuthenticationCreateRequest, AuthenticationResponse,
    },
    payments,
};
#[cfg(feature = "v1")]
use common_utils::{ext_traits::ValueExt, types::keymanager::ToEncryptable};
use diesel_models::authentication::{Authentication, AuthenticationNew};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
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
        payments::helpers,
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
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
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

        let current_time = common_utils::date_time::now();

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
    ) -> RouterResult<UasPreAuthenticationRequestData> {
        let payment_method_data = payment_method_data
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("payment_method_data is missing")?;
        let payment_details =
            if let payment_method_data::PaymentMethodData::Card(card) = payment_method_data {
                Some(PaymentDetails {
                    pan: card.card_number.clone(),
                    digital_card_id: None,
                    payment_data_type: None,
                    encrypted_src_card_details: None,
                    card_expiry_month: card.card_exp_month.clone(),
                    card_expiry_year: card.card_exp_year.clone(),
                    cardholder_name: card.card_holder_name.clone(),
                    card_token_number: None,
                    account_type: None,
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
    ) -> RouterResult<UasAuthenticationRequestData> {
        Ok(UasAuthenticationRequestData {
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
    merchant_context: domain::MerchantContext,
    req: AuthenticationCreateRequest,
) -> RouterResponse<AuthenticationResponse> {
    let db = &*state.store;
    let merchant_account = merchant_context.get_merchant_account();
    let merchant_id = merchant_account.get_id();
    let key_manager_state = (&state).into();
    let profile_id = core_utils::get_profile_id_from_business_details(
        &key_manager_state,
        None,
        None,
        &merchant_context,
        req.profile_id.as_ref(),
        db,
        true,
    )
    .await?;

    let business_profile = db
        .find_business_profile_by_profile_id(
            &key_manager_state,
            merchant_context.get_merchant_key_store(),
            &profile_id,
        )
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

    let response = AuthenticationResponse::foreign_try_from((
        new_authentication.clone(),
        amount,
        currency,
        profile_id,
        acquirer_details,
        new_authentication.profile_acquirer_id,
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
    )> for AuthenticationResponse
{
    type Error = error_stack::Report<ApiErrorResponse>;
    fn foreign_try_from(
        (authentication, amount, currency, profile_id, acquirer_details, profile_acquirer_id): (
            Authentication,
            common_utils::types::MinorUnit,
            common_enums::Currency,
            common_utils::id_type::ProfileId,
            Option<AcquirerDetails>,
            Option<common_utils::id_type::ProfileAcquirerId>,
        ),
    ) -> Result<Self, Self::Error> {
        let authentication_connector = authentication
            .authentication_connector
            .map(|connector| common_enums::AuthenticationConnectors::from_str(&connector))
            .transpose()
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Incorrect authentication connector stored in table")?;
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
    merchant_context: domain::MerchantContext,
    req: AuthenticationEligibilityRequest,
    authentication_id: common_utils::id_type::AuthenticationId,
) -> RouterResponse<AuthenticationEligibilityResponse> {
    let merchant_account = merchant_context.get_merchant_account();
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
    let key_manager_state = (&state).into();

    let profile_id = core_utils::get_profile_id_from_business_details(
        &key_manager_state,
        None,
        None,
        &merchant_context,
        None,
        db,
        true,
    )
    .await?;

    let business_profile = db
        .find_business_profile_by_profile_id(
            &key_manager_state,
            merchant_context.get_merchant_key_store(),
            &profile_id,
        )
        .await
        .to_not_found_response(ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let (authentication_connector, three_ds_connector_account) =
        auth_utils::get_authentication_connector_data(
            &state,
            merchant_context.get_merchant_key_store(),
            &business_profile,
            authentication.authentication_connector.clone(),
        )
        .await?;

    let notification_url = match authentication_connector {
        common_enums::AuthenticationConnectors::Juspaythreedsserver => {
            Some(url::Url::parse(&format!(
                "{base_url}/authentication/{merchant_id}/{authentication_id}/sync",
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
        endpoint_prefix: metadata.clone().map(|metadata| metadata.endpoint_prefix),
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
            merchant_context
                .get_merchant_key_store()
                .merchant_id
                .clone(),
        ),
        merchant_context.get_merchant_key_store().key.peek(),
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
                    merchant_context
                        .get_merchant_key_store()
                        .merchant_id
                        .clone(),
                ),
                merchant_context.get_merchant_key_store().key.peek(),
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
        merchant_context.get_merchant_key_store(),
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
    merchant_context: domain::MerchantContext,
    req: AuthenticationAuthenticateRequest,
    auth_flow: AuthFlow,
) -> RouterResponse<AuthenticationAuthenticateResponse> {
    let authentication_id = req.authentication_id.clone();
    let merchant_account = merchant_context.get_merchant_account();
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
        .find_business_profile_by_profile_id(
            &key_manager_state,
            merchant_context.get_merchant_key_store(),
            &profile_id,
        )
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
                    merchant_context
                        .get_merchant_key_store()
                        .merchant_id
                        .clone(),
                ),
                merchant_context.get_merchant_key_store().key.peek(),
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
            merchant_context.get_merchant_key_store(),
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
    )
    .await?;

    let authentication = utils::external_authentication_update_trackers(
        &state,
        auth_response,
        authentication.clone(),
        None,
        merchant_context.get_merchant_key_store(),
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
                let tokenised_data = crate::core::payment_methods::vault::get_tokenized_data(
                    &state,
                    authentication_id.get_string_repr(),
                    false,
                    merchant_context.get_merchant_key_store().key.get_inner(),
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
    merchant_context: domain::MerchantContext,
    auth_flow: AuthFlow,
    req: AuthenticationSyncRequest,
) -> RouterResponse<AuthenticationSyncResponse> {
    let authentication_id = req.authentication_id;
    let merchant_account = merchant_context.get_merchant_account();
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
        .find_business_profile_by_profile_id(
            &key_manager_state,
            merchant_context.get_merchant_key_store(),
            &profile_id,
        )
        .await
        .to_not_found_response(ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let (authentication_connector, three_ds_connector_account) =
        auth_utils::get_authentication_connector_data(
            &state,
            merchant_context.get_merchant_key_store(),
            &business_profile,
            authentication.authentication_connector.clone(),
        )
        .await?;

    let updated_authentication = match authentication.trans_status.clone() {
        Some(trans_status) if trans_status.clone().is_pending() => {
            let post_auth_response = ExternalAuthentication::post_authentication(
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

            utils::external_authentication_update_trackers(
                &state,
                post_auth_response,
                authentication.clone(),
                None,
                merchant_context.get_merchant_key_store(),
                None,
                None,
                None,
                None,
            )
            .await?
        }

        _ => authentication,
    };

    let (authentication_value, eci) = match auth_flow {
        AuthFlow::Client => (None, None),
        AuthFlow::Merchant => {
            if let Some(common_enums::TransactionStatus::Success) =
                updated_authentication.trans_status
            {
                let tokenised_data = crate::core::payment_methods::vault::get_tokenized_data(
                    &state,
                    authentication_id.get_string_repr(),
                    false,
                    merchant_context.get_merchant_key_store().key.get_inner(),
                )
                .await
                .inspect_err(|err| router_env::logger::error!(tokenized_data_result=?err))
                .attach_printable("cavv not present after authentication status is success")?;
                (
                    Some(masking::Secret::new(tokenised_data.value1)),
                    updated_authentication.eci.clone(),
                )
            } else {
                (None, None)
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
            merchant_context
                .get_merchant_key_store()
                .merchant_id
                .clone(),
        ),
        merchant_context.get_merchant_key_store().key.peek(),
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
                    merchant_context
                        .get_merchant_key_store()
                        .merchant_id
                        .clone(),
                ),
                merchant_context.get_merchant_key_store().key.peek(),
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
        authentication_value,
        threeds_server_transaction_id: updated_authentication.threeds_server_transaction_id.clone(),
        maximum_supported_3ds_version: updated_authentication.maximum_supported_version.clone(),
        connector_authentication_id: updated_authentication.connector_authentication_id.clone(),
        three_ds_method_data: updated_authentication.three_ds_method_data.clone(),
        three_ds_method_url: updated_authentication.three_ds_method_url.clone(),
        message_version: updated_authentication.message_version.clone(),
        connector_metadata: updated_authentication.connector_metadata.clone(),
        directory_server_id: updated_authentication.directory_server_id.clone(),
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
    merchant_context: domain::MerchantContext,
    req: AuthenticationSyncPostUpdateRequest,
) -> RouterResponse<()> {
    let authentication_id = req.authentication_id;
    let merchant_account = merchant_context.get_merchant_account();
    let merchant_id = merchant_account.get_id();
    let db = &*state.store;
    let authentication = db
        .find_authentication_by_merchant_id_authentication_id(merchant_id, &authentication_id)
        .await
        .to_not_found_response(ApiErrorResponse::AuthenticationNotFound {
            id: authentication_id.get_string_repr().to_owned(),
        })?;
    let key_manager_state = (&state).into();
    let business_profile = db
        .find_business_profile_by_profile_id(
            &key_manager_state,
            merchant_context.get_merchant_key_store(),
            &authentication.profile_id,
        )
        .await
        .to_not_found_response(ApiErrorResponse::ProfileNotFound {
            id: authentication.profile_id.get_string_repr().to_owned(),
        })?;

    let (authentication_connector, three_ds_connector_account) =
        auth_utils::get_authentication_connector_data(
            &state,
            merchant_context.get_merchant_key_store(),
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

    utils::external_authentication_update_trackers(
        &state,
        post_auth_response,
        authentication.clone(),
        None,
        merchant_context.get_merchant_key_store(),
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
        &authentication,
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
        authentication
            .authentication_client_secret
            .clone()
            .map(masking::Secret::new)
            .as_ref(),
        authentication.amount,
    )?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::JsonForRedirection(redirect_response))
}
