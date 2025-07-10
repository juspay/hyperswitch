pub mod types;
pub mod utils;

use api_models::{
    authentication::{AcquirerDetails, AuthenticationCreateRequest, AuthenticationResponse},
    payments,
};
use diesel_models::authentication::{Authentication, AuthenticationNew};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    payment_method_data,
    router_request_types::{
        authentication::{MessageCategory, PreAuthenticationData},
        unified_authentication_service::{
            AuthenticationInfo, PaymentDetails, ServiceSessionIds, TransactionDetails,
            UasAuthenticationRequestData, UasConfirmationRequestData,
            UasPostAuthenticationRequestData, UasPreAuthenticationRequestData,
        },
        BrowserInformation,
    },
    types::{
        UasAuthenticationRouterData, UasPostAuthenticationRouterData,
        UasPreAuthenticationRouterData,
    },
};

use super::{
    errors::{RouterResponse, RouterResult},
    payments::helpers::MerchantConnectorAccountType,
};
use crate::{
    consts,
    core::{
        errors::utils::StorageErrorExt,
        unified_authentication_service::types::{
            ClickToPay, ExternalAuthentication, UnifiedAuthenticationService,
            UNIFIED_AUTHENTICATION_SERVICE,
        },
        utils as core_utils,
    },
    db::domain,
    routes::SessionState,
    types::transformers::ForeignFrom,
};

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl UnifiedAuthenticationService for ClickToPay {
    fn get_pre_authentication_request_data(
        _payment_method_data: Option<&domain::PaymentMethodData>,
        service_details: Option<payments::CtpServiceDetails>,
        amount: common_utils::types::MinorUnit,
        currency: Option<common_enums::Currency>,
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
    ) -> RouterResult<UasPreAuthenticationRouterData> {
        let pre_authentication_data = Self::get_pre_authentication_request_data(
            payment_method_data,
            service_details,
            amount,
            currency,
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

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            pre_auth_router_data,
        )
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
        _amount: common_utils::types::MinorUnit,
        _currency: Option<common_enums::Currency>,
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
            authentication_info: None,
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
    ) -> RouterResult<UasPreAuthenticationRouterData> {
        let pre_authentication_data = Self::get_pre_authentication_request_data(
            payment_method_data,
            service_details,
            amount,
            currency,
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
        payment_id: Option<common_utils::id_type::PaymentId>,
    ) -> RouterResult<UasAuthenticationRouterData> {
        let authentication_data =
            <Self as UnifiedAuthenticationService>::get_authentication_request_data(
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
        _authentication_id: &common_utils::id_type::AuthenticationId,
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
            authentication.map(|auth| auth.authentication_id.clone()),
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
        req.acquirer_details
            .clone()
            .and_then(|acquirer_details| acquirer_details.bin),
        req.acquirer_details
            .clone()
            .and_then(|acquirer_details| acquirer_details.merchant_id),
        req.acquirer_details
            .clone()
            .and_then(|acquirer_details| acquirer_details.country_code),
        Some(req.amount),
        Some(req.currency),
        req.return_url,
    )
    .await?;

    let acquirer_details = Some(AcquirerDetails {
        bin: new_authentication.acquirer_bin.clone(),
        merchant_id: new_authentication.acquirer_merchant_id.clone(),
        country_code: new_authentication.acquirer_country_code.clone(),
    });

    let amount = new_authentication
        .amount
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("amount failed to get amount from authentication table")?;
    let currency = new_authentication
        .currency
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("currency failed to get currency from authentication table")?;

    let response = AuthenticationResponse::foreign_from((
        new_authentication,
        amount,
        currency,
        profile_id,
        acquirer_details,
    ));

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

impl
    ForeignFrom<(
        Authentication,
        common_utils::types::MinorUnit,
        common_enums::Currency,
        common_utils::id_type::ProfileId,
        Option<AcquirerDetails>,
    )> for AuthenticationResponse
{
    fn foreign_from(
        (authentication, amount, currency, profile_id, acquirer_details): (
            Authentication,
            common_utils::types::MinorUnit,
            common_enums::Currency,
            common_utils::id_type::ProfileId,
            Option<AcquirerDetails>,
        ),
    ) -> Self {
        Self {
            authentication_id: authentication.authentication_id,
            client_secret: authentication
                .authentication_client_secret
                .map(masking::Secret::new),
            amount,
            currency,
            force_3ds_challenge: authentication.force_3ds_challenge,
            merchant_id: authentication.merchant_id,
            status: authentication.authentication_status,
            authentication_connector: authentication.authentication_connector,
            return_url: authentication.return_url,
            created_at: Some(authentication.created_at),
            error_code: authentication.error_code,
            error_message: authentication.error_message,
            profile_id: Some(profile_id),
            psd2_sca_exemption_type: authentication.psd2_sca_exemption_type,
            acquirer_details,
        }
    }
}
