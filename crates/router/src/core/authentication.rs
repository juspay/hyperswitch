pub(crate) mod utils;

pub mod transformers;
pub mod types;

use api_models::payments;
use common_enums::Currency;
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use error_stack::ResultExt;
use hyperswitch_domain_models::authentication;
use hyperswitch_masking::ExposeInterface;

use super::errors::StorageErrorExt;
use crate::{
    consts,
    core::{
        errors::ApiErrorResponse, payment_methods::vault, payments as payments_core,
        unified_connector_service,
    },
    routes::SessionState,
    types::{
        self as core_types, api,
        domain::{self},
        storage::PaymentAttemptUpdate,
        transformers::ForeignFrom,
    },
    utils::{check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata, OptionExt},
};

#[allow(clippy::too_many_arguments)]
pub async fn perform_authentication(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    authentication_connector: String,
    payment_method_data: domain::PaymentMethodData,
    payment_method: common_enums::PaymentMethod,
    billing_address: hyperswitch_domain_models::address::Address,
    shipping_address: Option<hyperswitch_domain_models::address::Address>,
    browser_details: Option<core_types::BrowserInformation>,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    amount: Option<common_utils::types::MinorUnit>,
    currency: Option<Currency>,
    message_category: api::authentication::MessageCategory,
    device_channel: payments::DeviceChannel,
    authentication_data: authentication::Authentication,
    return_url: Option<String>,
    sdk_information: Option<payments::SdkInformation>,
    threeds_method_comp_ind: payments::ThreeDsCompletionIndicator,
    email: Option<common_utils::pii::Email>,
    webhook_url: String,
    three_ds_requestor_url: String,
    psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    payment_id: common_utils::id_type::PaymentId,
    force_3ds_challenge: bool,
    merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
    storage_scheme: diesel_models::enums::MerchantStorageScheme,
) -> CustomResult<api::authentication::AuthenticationResponse, ApiErrorResponse> {
    let router_data = transformers::construct_authentication_router_data(
        state,
        merchant_id,
        authentication_connector.clone(),
        payment_method_data,
        payment_method,
        billing_address.clone(),
        shipping_address.clone(),
        browser_details.clone(),
        amount,
        currency,
        message_category,
        device_channel.clone(),
        merchant_connector_account,
        authentication_data.clone(),
        return_url,
        sdk_information.clone(),
        threeds_method_comp_ind,
        email.clone(),
        webhook_url,
        three_ds_requestor_url,
        psd2_sca_exemption_type,
        payment_id,
        force_3ds_challenge,
    )?;
    let response = Box::pin(utils::do_auth_connector_call(
        state,
        authentication_connector.clone(),
        router_data,
    ))
    .await?;

    let authentication_info =
        hyperswitch_domain_models::router_request_types::authentication::AuthenticationInfo {
            billing_address: Some(billing_address),
            shipping_address,
            browser_info: browser_details,
            email,
            device_details: sdk_information
                .and_then(|sdk_information| sdk_information.device_details),
            merchant_category_code: None,
            merchant_country_code: None,
            platform: Some(device_channel),
        };
    let authentication = Box::pin(utils::update_trackers(
        state,
        response.clone(),
        authentication_data,
        None,
        merchant_key_store,
        authentication_info,
        storage_scheme,
    ))
    .await?;
    response
        .response
        .map_err(|err| ApiErrorResponse::ExternalConnectorError {
            code: err.code,
            message: err.message,
            connector: authentication_connector,
            status_code: err.status_code,
            reason: err.reason,
        })?;
    api::authentication::AuthenticationResponse::try_from(authentication)
}

pub async fn perform_post_authentication(
    state: &SessionState,
    processor: &domain::Processor,
    business_profile: domain::Profile,
    authentication_id: common_utils::id_type::AuthenticationId,
    payment_id: &common_utils::id_type::PaymentId,
) -> CustomResult<
    hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore,
    ApiErrorResponse,
> {
    let key_state = &state.into();
    let (authentication_connector, three_ds_connector_account) =
        utils::get_authentication_connector_data(state, processor, &business_profile, None).await?;
    let is_pull_mechanism_enabled =
        check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata(
            three_ds_connector_account
                .get_metadata()
                .map(|metadata| metadata.expose()),
        );
    let authentication = state
        .store
        .find_authentication_by_processor_merchant_id_authentication_id(
            processor.get_account().get_id(),
            &authentication_id,
            processor.get_key_store(),
            key_state,
            processor.get_account().storage_scheme,
        )
        .await
        .to_not_found_response(ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Error while fetching authentication record with authentication_id {}",
                authentication_id.get_string_repr()
            )
        })?;

    let authentication_update = if !authentication.authentication_status.is_terminal_status()
        && is_pull_mechanism_enabled
    {
        // trigger in case of authenticate flow
        let router_data = transformers::construct_post_authentication_router_data(
            state,
            authentication_connector.to_string(),
            business_profile,
            three_ds_connector_account,
            &authentication,
            payment_id,
        )?;
        let router_data = Box::pin(utils::do_auth_connector_call(
            state,
            authentication_connector.to_string(),
            router_data,
        ))
        .await?;

        let authentication_info =
            hyperswitch_domain_models::router_request_types::authentication::AuthenticationInfo {
                billing_address: None,
                shipping_address: None,
                browser_info: None,
                email: None,
                device_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                platform: None,
            };

        utils::update_trackers(
            state,
            router_data,
            authentication,
            None,
            processor.get_key_store(),
            authentication_info,
            processor.get_account().storage_scheme,
        )
        .await?
    } else {
        // trigger in case of webhook flow
        authentication
    };

    // getting authentication value from temp locker before moving ahead with authrisation
    let tokenized_data = vault::get_tokenized_data(
        state,
        authentication_id.get_string_repr(),
        false,
        processor.get_key_store().key.get_inner(),
    )
    .await
    .inspect_err(|err| router_env::logger::error!(tokenized_data_result=?err))
    .attach_printable("cavv not present after authentication flow")
    .ok();

    let authentication_store =
        hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore {
            cavv: tokenized_data.map(|data| hyperswitch_masking::Secret::new(data.value1)),
            authentication: authentication_update,
        };

    Ok(authentication_store)
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_pre_authentication(
    state: &SessionState,
    processor: &domain::Processor,
    provider_merchant_id: common_utils::id_type::MerchantId,
    card: hyperswitch_domain_models::payment_method_data::Card,
    token: String,
    business_profile: &domain::Profile,
    acquirer_details: Option<types::AcquirerDetails>,
    payment_id: common_utils::id_type::PaymentId,
    organization_id: common_utils::id_type::OrganizationId,
    force_3ds_challenge: Option<bool>,
    psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    billing_address: Option<hyperswitch_domain_models::address::Address>,
    shipping_address: Option<hyperswitch_domain_models::address::Address>,
    browser_info: Option<core_types::BrowserInformation>,
    initiator: Option<&domain::Initiator>,
    amount: Option<common_utils::types::MinorUnit>,
    currency: Option<Currency>,
) -> CustomResult<
    hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore,
    ApiErrorResponse,
> {
    let (authentication_connector, three_ds_connector_account) =
        utils::get_authentication_connector_data(state, processor, business_profile, None).await?;
    let authentication_connector_name = authentication_connector.to_string();
    let authentication = utils::create_new_authentication(
        state,
        provider_merchant_id,
        authentication_connector_name.clone(),
        token,
        business_profile,
        payment_id.clone(),
        three_ds_connector_account
            .get_mca_id()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Error while finding mca_id from merchant_connector_account")?,
        organization_id,
        force_3ds_challenge,
        psd2_sca_exemption_type,
        processor,
        initiator,
        &card,
        browser_info.as_ref(),
        acquirer_details.as_ref(),
        billing_address.as_ref(),
        shipping_address.as_ref(),
        amount,
        currency,
    )
    .await?;

    let authentication = if authentication_connector.is_separate_version_call_required() {
        let router_data: core_types::authentication::PreAuthNVersionCallRouterData =
            transformers::construct_pre_authentication_router_data(
                state,
                authentication_connector_name.clone(),
                card.clone(),
                &three_ds_connector_account,
                business_profile.merchant_id.clone(),
                payment_id.clone(),
            )?;
        let router_data = Box::pin(utils::do_auth_connector_call(
            state,
            authentication_connector_name.clone(),
            router_data,
        ))
        .await?;

        let authentication_info =
            hyperswitch_domain_models::router_request_types::authentication::AuthenticationInfo {
                billing_address: billing_address.clone(),
                shipping_address: shipping_address.clone(),
                browser_info: None,
                email: None,
                device_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                platform: None,
            };

        let updated_authentication = Box::pin(utils::update_trackers(
            state,
            router_data,
            authentication,
            acquirer_details.clone(),
            processor.get_key_store(),
            authentication_info,
            processor.get_account().storage_scheme,
        ))
        .await?;
        // from version call response, we will get to know the maximum supported 3ds version.
        // If the version is not greater than or equal to 3DS 2.0, We should not do the successive pre authentication call.
        if !updated_authentication.is_separate_authn_required() {
            return Ok(hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore{
                authentication: updated_authentication,
                cavv: None, // since cavv wont be present in pre_authentication step
            });
        }
        updated_authentication
    } else {
        authentication
    };

    let router_data: core_types::authentication::PreAuthNRouterData =
        transformers::construct_pre_authentication_router_data(
            state,
            authentication_connector_name.clone(),
            card,
            &three_ds_connector_account,
            business_profile.merchant_id.clone(),
            payment_id,
        )?;
    let router_data = Box::pin(utils::do_auth_connector_call(
        state,
        authentication_connector_name,
        router_data,
    ))
    .await?;

    let authentication_info =
        hyperswitch_domain_models::router_request_types::authentication::AuthenticationInfo {
            billing_address,
            shipping_address,
            browser_info: None,
            email: None,
            device_details: None,
            merchant_category_code: None,
            merchant_country_code: None,
            platform: None,
        };

    let authentication_update = Box::pin(utils::update_trackers(
        state,
        router_data,
        authentication,
        acquirer_details,
        processor.get_key_store(),
        authentication_info,
        processor.get_account().storage_scheme,
    ))
    .await?;

    Ok(
        hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore {
            authentication: authentication_update,
            cavv: None, // since cavv wont be present in pre_authentication step
        },
    )
}

#[cfg(feature = "v1")]
struct ProxyUcsGatewayContext {
    execution_mode: common_enums::ExecutionMode,
    session_state: SessionState,
    platform: domain::Platform,
    external_vault_merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
}

/// Shared by `perform_pre_authentication_proxy` and `perform_post_authentication_proxy`: resolves
/// the UCS gateway/execution mode and vault MCA for the PSP connector, or `None` if UCS isn't
/// enabled for it.
#[cfg(feature = "v1")]
async fn resolve_proxy_ucs_gateway_and_vault_mca<PayF: Clone, RdF: Clone, T, R>(
    state: &SessionState,
    processor: &domain::Processor,
    initiator: Option<&domain::Initiator>,
    business_profile: &domain::Profile,
    payment_data: &payments_core::PaymentData<PayF>,
    psp_router_data: &core_types::RouterData<RdF, T, R>,
) -> CustomResult<Option<ProxyUcsGatewayContext>, ApiErrorResponse>
where
    R: Send + Sync + Clone,
{
    let (execution_path, updated_state) =
        unified_connector_service::should_call_unified_connector_service(
            state,
            processor,
            psp_router_data,
            unified_connector_service::extract_gateway_system_from_payment_intent(payment_data),
            payments_core::CallConnectorAction::Trigger,
            None,
            common_enums::TransactionType::Payment,
        )
        .await?;
    let execution_mode = match execution_path {
        common_enums::ExecutionPath::UnifiedConnectorService => {
            Some(common_enums::ExecutionMode::Primary)
        }
        common_enums::ExecutionPath::ShadowUnifiedConnectorService => {
            Some(common_enums::ExecutionMode::Shadow)
        }
        common_enums::ExecutionPath::Direct => None,
    };

    if let Some(execution_mode) = execution_mode {
        let provider_merchant_id = payment_data.payment_intent.merchant_id.clone();
        let provider_key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &provider_merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(ApiErrorResponse::InternalServerError)
            .attach_printable("Error while fetching the key store for provider merchant")?;
        let provider_account = state
            .store
            .find_merchant_account_by_merchant_id(&provider_merchant_id, &provider_key_store)
            .await
            .to_not_found_response(ApiErrorResponse::InternalServerError)
            .attach_printable("Error while fetching the merchant account for provider")?;
        let platform = domain::Platform::new(
            provider_account,
            provider_key_store,
            processor.get_account().clone(),
            processor.get_key_store().clone(),
            initiator.cloned(),
        );
        let provider_business_profile =
            payments_core::helpers::resolve_provider_profile(state, &platform, business_profile)
                .await?;
        let external_vault_mca_id = provider_business_profile
            .external_vault_details
            .get_vault_connector_id()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("external vault is not enabled for this business profile")?;
        let external_vault_merchant_connector_account =
            payments_core::helpers::MerchantConnectorAccountType::DbVal(Box::new(
                state
                    .store
                    .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                        platform.get_provider().get_account().get_id(),
                        &external_vault_mca_id,
                        platform.get_provider().get_key_store(),
                    )
                    .await
                    .to_not_found_response(ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: external_vault_mca_id.get_string_repr().to_string(),
                    })?,
            ));

        Ok(Some(ProxyUcsGatewayContext {
            execution_mode,
            session_state: updated_state,
            platform,
            external_vault_merchant_connector_account,
        }))
    } else {
        Ok(None)
    }
}

/// Drives the live UCS pre-authenticate call. Returns `Ok(Err(ErrorResponse))` (not a hard `Err`)
/// for a UCS/connector-level failure, so the caller can persist it via a single `update_trackers` call.
#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
async fn call_ucs_pre_authenticate_proxy(
    pre_authenticate_router_data: &core_types::RouterData<
        api::PreAuthenticate,
        core_types::PaymentsPreAuthenticateData,
        core_types::PaymentsResponseData,
    >,
    external_vault_pmd: &domain::ExternalVaultPaymentMethodData,
    state: &SessionState,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    lineage_ids: external_services::grpc_client::LineageIds,
    auth_merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    external_vault_merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    processor: &domain::Processor,
    connector_enum: common_enums::connector_enums::Connector,
    execution_mode: common_enums::ExecutionMode,
) -> CustomResult<
    Result<core_types::authentication::AuthenticationResponseData, core_types::ErrorResponse>,
    ApiErrorResponse,
> {
    let pre_authenticate_router_data = Box::pin(
        payments_core::flows::authorize_flow::call_unified_connector_service_pre_authenticate_proxy(
            pre_authenticate_router_data,
            external_vault_pmd,
            state,
            header_payload,
            lineage_ids,
            auth_merchant_connector_account,
            external_vault_merchant_connector_account,
            processor,
            connector_enum,
            execution_mode,
        ),
    )
    .await;

    let pre_authenticate_result = match pre_authenticate_router_data {
        Ok(pre_authenticate_router_data) => match pre_authenticate_router_data.response {
            Ok(core_types::PaymentsResponseData::TransactionResponse {
                authentication_data,
                redirection_data,
                ..
            }) => Ok((authentication_data.map(|boxed| *boxed), *redirection_data)),
            Ok(_) => Ok((None, None)),
            Err(err) => Err(err),
        },
        Err(err) => Err(core_types::ErrorResponse {
            message: format!("UCS pre-authenticate call failed: {err}"),
            ..Default::default()
        }),
    };
    match pre_authenticate_result {
        Err(err) => Ok(Err(err)),
        Ok((ucs_authentication_data, ddc_redirection_data)) => {
            let (three_ds_method_data, three_ds_method_url) = match &ddc_redirection_data {
                Some(hyperswitch_domain_models::router_response_types::RedirectForm::Form {
                    form_fields,
                    ..
                }) => (
                    form_fields.get(consts::UCS_DDC_METHOD_DATA_KEY).cloned(),
                    form_fields.get(consts::UCS_DDC_METHOD_URL_KEY).cloned(),
                ),
                _ => (None, None),
            };

            let message_version = ucs_authentication_data
                .as_ref()
                .and_then(|data| data.message_version.clone())
                .unwrap_or_else(|| common_utils::types::SemanticVersion::new(2, 2, 0));
            let threeds_server_transaction_id = ucs_authentication_data
                .as_ref()
                .and_then(|data| data.threeds_server_transaction_id.clone())
                .ok_or(ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "UCS pre-authenticate response missing threeds_server_transaction_id",
                )?;
            let directory_server_id = ucs_authentication_data
                .as_ref()
                .and_then(|data| data.ds_trans_id.clone());

            Ok(Ok(
                core_types::authentication::AuthenticationResponseData::PreAuthNResponse {
                    connector_authentication_id: threeds_server_transaction_id.clone(),
                    threeds_server_transaction_id,
                    maximum_supported_3ds_version: message_version.clone(),
                    three_ds_method_data,
                    three_ds_method_url,
                    message_version,
                    connector_metadata: None,
                    directory_server_id,
                    scheme_id: None,
                },
            ))
        }
    }
}

/// Proxy analogue of `perform_pre_authentication`, driving the auth connector over UCS. Fails
/// closed (hard `Err`) if UCS isn't enabled for the connector, since the merchant explicitly
/// requested external 3DS for this payment.
#[cfg(feature = "v1")]
pub async fn perform_pre_authentication_proxy<F: Clone>(
    state: &SessionState,
    processor: &domain::Processor,
    initiator: Option<&domain::Initiator>,
    business_profile: &domain::Profile,
    payment_data: &payments_core::PaymentData<F>,
) -> CustomResult<
    (
        Option<
            hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore,
        >,
        Option<String>,
    ),
    ApiErrorResponse,
> {
    let external_vault_pmd = payment_data
        .external_vault_pmd
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("external_vault_pmd is required for the pre-authentication proxy flow")?;

    let psp_connector_name = payment_data
        .payment_attempt
        .connector
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("connector missing on attempt for external vault 3DS pre-authenticate")?;
    let psp_merchant_connector_account = payments_core::helpers::get_merchant_connector_account(
        state,
        processor,
        None,
        business_profile.get_id(),
        psp_connector_name.as_str(),
        payment_data.payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let payment_method = payment_data
        .payment_attempt
        .payment_method
        .unwrap_or(common_enums::PaymentMethod::Card);

    let pre_authenticate_request_data = core_types::PaymentsPreAuthenticateData {
        payment_method_data: domain::PaymentMethodData::Card(domain::Card::default()),
        amount: payment_data
            .payment_attempt
            .get_total_amount()
            .get_amount_as_i64(),
        minor_amount: payment_data.payment_attempt.get_total_amount(),
        email: None,
        capture_method: payment_data.payment_attempt.capture_method,
        currency: payment_data.payment_intent.currency,
        payment_method_type: payment_data.payment_attempt.payment_method_type,
        router_return_url: payment_data.payment_intent.return_url.clone(),
        complete_authorize_url: None,
        browser_info: None,
        enrolled_for_3ds: true,
        customer_name: None,
        metadata: None,
        webhook_url: None,
    };

    let psp_router_data: core_types::RouterData<
        api::PreAuthenticate,
        core_types::PaymentsPreAuthenticateData,
        core_types::PaymentsResponseData,
    > = transformers::construct_router_data(
        state,
        psp_connector_name,
        payment_method,
        business_profile.merchant_id.clone(),
        payment_data.address.clone(),
        pre_authenticate_request_data,
        &psp_merchant_connector_account,
        payment_data.payment_intent.psd2_sca_exemption_type,
        payment_data.payment_attempt.payment_id.clone(),
    )?;

    let ProxyUcsGatewayContext {
        execution_mode,
        session_state: updated_state,
        platform: _,
        external_vault_merchant_connector_account,
    } = resolve_proxy_ucs_gateway_and_vault_mca(
        state,
        processor,
        initiator,
        business_profile,
        payment_data,
        &psp_router_data,
    )
    .await?
    .ok_or_else(|| {
        error_stack::report!(ApiErrorResponse::PreconditionFailed {
            message: "External 3DS authentication was requested for this payment, but UCS is \
                      not enabled for this connector; cannot proceed without performing 3DS"
                .to_string(),
        })
    })?;

    let (authentication_connector, auth_merchant_connector_account) =
        utils::get_authentication_connector_data(state, processor, business_profile, None).await?;
    let connector_enum = authentication_connector
        .to_string()
        .parse::<common_enums::connector_enums::Connector>()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Invalid authentication connector name for UCS external vault 3DS auth leg",
        )?;

    let merchant_connector_id = auth_merchant_connector_account
        .get_mca_id()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while finding mca_id from merchant_connector_account")?;

    let auth_webhook_url = auth_merchant_connector_account.get_mca_id().map(|mca_id| {
        payments_core::helpers::create_webhook_url(
            &state.base_url,
            &business_profile.merchant_id,
            mca_id.get_string_repr(),
        )
    });

    let browser_info: Option<core_types::BrowserInformation> = payment_data
        .payment_attempt
        .browser_info
        .clone()
        .map(|browser_info| browser_info.parse_value("BrowserInformation"))
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse browser_info from payment_attempt")?;

    let authentication_connector_name = authentication_connector.to_string();
    let authentication = utils::create_new_authentication(
        state,
        business_profile.merchant_id.clone(),
        authentication_connector_name,
        common_utils::generate_id(consts::ID_LENGTH, "authn"),
        business_profile,
        payment_data.payment_attempt.payment_id.clone(),
        merchant_connector_id,
        payment_data.payment_attempt.organization_id.clone(),
        payment_data.payment_intent.force_3ds_challenge,
        payment_data.payment_intent.psd2_sca_exemption_type,
        processor,
        initiator,
        &domain::Card::default(),
        browser_info.as_ref(),
        None,
        payment_data.address.get_payment_method_billing(),
        payment_data.address.get_shipping(),
        Some(payment_data.payment_intent.amount),
        payment_data.payment_intent.currency,
    )
    .await?;

    let mut pre_authenticate_router_data = psp_router_data;
    pre_authenticate_router_data.connector = connector_enum.to_string();
    pre_authenticate_router_data.request.webhook_url = auth_webhook_url;

    let lineage_ids = external_services::grpc_client::LineageIds::new(
        business_profile.merchant_id.clone(),
        business_profile.get_id().clone(),
    );
    let header_payload = hyperswitch_domain_models::payments::HeaderPayload::default();

    let pre_authenticate_response_data = call_ucs_pre_authenticate_proxy(
        &pre_authenticate_router_data,
        &external_vault_pmd,
        &updated_state,
        &header_payload,
        lineage_ids,
        auth_merchant_connector_account.clone(),
        external_vault_merchant_connector_account,
        processor,
        connector_enum,
        execution_mode,
    )
    .await?;

    let mut pre_authenticate_router_data: core_types::RouterData<
        api::PreAuthentication,
        (),
        core_types::authentication::AuthenticationResponseData,
    > = transformers::construct_router_data(
        state,
        authentication_connector.to_string(),
        payment_method,
        business_profile.merchant_id.clone(),
        payment_data.address.clone(),
        (),
        &auth_merchant_connector_account,
        payment_data.payment_intent.psd2_sca_exemption_type,
        payment_data.payment_attempt.payment_id.clone(),
    )?;
    pre_authenticate_router_data.response = pre_authenticate_response_data;

    let authentication_info =
        hyperswitch_domain_models::router_request_types::authentication::AuthenticationInfo {
            billing_address: payment_data.address.get_payment_method_billing().cloned(),
            shipping_address: payment_data.address.get_shipping().cloned(),
            browser_info,
            email: None,
            device_details: None,
            merchant_category_code: None,
            merchant_country_code: None,
            platform: None,
        };

    let authentication = Box::pin(utils::update_trackers(
        state,
        pre_authenticate_router_data,
        authentication,
        None,
        processor.get_key_store(),
        authentication_info,
        processor.get_account().storage_scheme,
    ))
    .await?;

    let alias_token = payment_data
        .token
        .clone()
        .get_required_value("token")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "payment_data.token should not be None while making external vault pre authentication call",
        )?;

    Ok((
        Some(
            hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore {
                cavv: None,
                authentication,
            },
        ),
        Some(alias_token),
    ))
}

/// Drives the live UCS post-authenticate call. Returns `Ok(Err(ErrorResponse))` (not a hard `Err`)
/// for any 3DS-outcome failure, so the caller can persist it via a single `update_trackers` call.
#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
async fn call_ucs_post_authenticate_proxy<F: Clone>(
    state: &SessionState,
    processor: &domain::Processor,
    initiator: Option<&domain::Initiator>,
    business_profile: &domain::Profile,
    payment_data: &payments_core::PaymentData<F>,
    authentication: &authentication::Authentication,
    auth_connector_enum: common_enums::connector_enums::Connector,
    auth_merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    payment_method: common_enums::PaymentMethod,
) -> CustomResult<
    Result<core_types::authentication::AuthenticationResponseData, core_types::ErrorResponse>,
    ApiErrorResponse,
> {
    let psp_connector_name = payment_data
        .payment_attempt
        .connector
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "connector missing on attempt for external vault 3DS post-authenticate",
        )?;
    let psp_merchant_connector_account = payments_core::helpers::get_merchant_connector_account(
        state,
        processor,
        None,
        business_profile.get_id(),
        psp_connector_name.as_str(),
        payment_data.payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let browser_info: Option<core_types::BrowserInformation> = payment_data
        .payment_attempt
        .browser_info
        .clone()
        .map(|browser_information| browser_information.parse_value("BrowserInformation"))
        .transpose()
        .change_context(ApiErrorResponse::InvalidDataValue {
            field_name: "browser_info",
        })?;

    let amount = payment_data.payment_attempt.get_total_amount();
    let post_authenticate_request_data = core_types::PaymentsPostAuthenticateData {
        payment_method_data: None,
        payment_method_type: payment_data.payment_attempt.payment_method_type,
        amount: Some(amount.get_amount_as_i64()),
        minor_amount: Some(amount),
        email: None,
        currency: payment_data.payment_intent.currency,
        capture_method: payment_data.payment_attempt.capture_method,
        browser_info,
        connector_transaction_id: authentication.threeds_server_transaction_id.clone(),
        redirect_response: authentication
            .threeds_server_transaction_id
            .clone()
            .map(|tds| {
                hyperswitch_domain_models::router_request_types::CompleteAuthorizeRedirectResponse {
                    params: Some(hyperswitch_masking::Secret::new(tds)),
                    payload: None,
                }
            }),
        metadata: None,
        complete_authorize_url: None,
    };

    let psp_router_data: core_types::RouterData<
        api::PostAuthenticate,
        core_types::PaymentsPostAuthenticateData,
        core_types::PaymentsResponseData,
    > = transformers::construct_router_data(
        state,
        psp_connector_name,
        payment_method,
        business_profile.merchant_id.clone(),
        payment_data.address.clone(),
        post_authenticate_request_data,
        &psp_merchant_connector_account,
        payment_data.payment_intent.psd2_sca_exemption_type,
        payment_data.payment_attempt.payment_id.clone(),
    )?;

    let Some(ProxyUcsGatewayContext {
        execution_mode,
        session_state: updated_state,
        platform,
        external_vault_merchant_connector_account,
    }) = resolve_proxy_ucs_gateway_and_vault_mca(
        state,
        processor,
        initiator,
        business_profile,
        payment_data,
        &psp_router_data,
    )
    .await?
    else {
        return Ok(Err(core_types::ErrorResponse {
            message: "UCS is not enabled for this connector; external vault 3DS post-authenticate over UCS is unavailable".to_string(),
            ..Default::default()
        }));
    };

    let mut post_authenticate_router_data = psp_router_data;
    post_authenticate_router_data.connector = auth_connector_enum.to_string();

    let lineage_ids = external_services::grpc_client::LineageIds::new(
        business_profile.merchant_id.clone(),
        business_profile.get_id().clone(),
    );

    let post_auth_payment_token = payment_data
        .payment_attempt
        .payment_token
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "payment_token missing on attempt for external vault 3DS post-authenticate",
        )?;
    let post_auth_payment_method = payment_data
        .payment_attempt
        .payment_method
        .unwrap_or(common_enums::PaymentMethod::Card);
    let external_vault_pmd = payments_core::resolve_external_vault_alias_from_payment_token(
        state,
        &platform,
        post_auth_payment_token,
        post_auth_payment_method,
    )
    .await?;

    let header_payload = hyperswitch_domain_models::payments::HeaderPayload::default();

    let post_authenticate_router_data = Box::pin(
        payments_core::flows::complete_authorize_flow::call_unified_connector_service_post_authenticate_proxy(
            &post_authenticate_router_data,
            &external_vault_pmd,
            &updated_state,
            &header_payload,
            lineage_ids,
            auth_merchant_connector_account,
            external_vault_merchant_connector_account,
            processor,
            execution_mode,
        ),
    )
    .await;

    let post_authenticate_router_data = match post_authenticate_router_data {
        Ok(router_data) => router_data,
        Err(err) => {
            return Ok(Err(core_types::ErrorResponse {
                message: format!("UCS post-authenticate call failed: {err}"),
                ..Default::default()
            }));
        }
    };

    let post_auth_authentication_data = match post_authenticate_router_data.response {
        Ok(core_types::PaymentsResponseData::TransactionResponse {
            authentication_data,
            ..
        }) => authentication_data.map(|boxed| *boxed),
        Ok(_) => None,
        Err(err) => return Ok(Err(err)),
    };

    let trans_status = post_auth_authentication_data
        .as_ref()
        .and_then(|data| data.trans_status.clone())
        .or_else(|| authentication.trans_status.clone())
        .unwrap_or(common_enums::TransactionStatus::VerificationNotPerformed);
    let eci = post_auth_authentication_data
        .as_ref()
        .and_then(|data| data.eci.clone())
        .or_else(|| authentication.eci.clone());
    let cavv = post_auth_authentication_data
        .as_ref()
        .and_then(|data| data.cavv.clone());

    let authentication_status =
        common_enums::AuthenticationStatus::foreign_from(trans_status.clone());

    if authentication_status != common_enums::AuthenticationStatus::Success || cavv.is_none() {
        return Ok(Err(core_types::ErrorResponse {
            message: "External vault 3DS post-authenticate did not succeed".to_string(),
            reason: Some(format!("trans_status: {trans_status:?}")),
            connector_transaction_id: authentication.threeds_server_transaction_id.clone(),
            ..Default::default()
        }));
    }

    Ok(Ok(
        core_types::authentication::AuthenticationResponseData::PostAuthNResponse {
            trans_status,
            authentication_value: cavv,
            eci,
            challenge_cancel: post_auth_authentication_data
                .as_ref()
                .and_then(|data| data.challenge_cancel.clone()),
            challenge_code_reason: post_auth_authentication_data
                .as_ref()
                .and_then(|data| data.challenge_code_reason.clone()),
        },
    ))
}

/// Proxy analogue of `perform_post_authentication`, driving the auth connector over UCS instead
/// of directly. Mirrors its `!is_terminal_status() && is_pull_mechanism_enabled` gating shape.
#[cfg(feature = "v1")]
pub async fn perform_post_authentication_proxy<F: Clone>(
    state: &SessionState,
    processor: &domain::Processor,
    initiator: Option<&domain::Initiator>,
    business_profile: &domain::Profile,
    payment_data: &payments_core::PaymentData<F>,
    authentication_id: common_utils::id_type::AuthenticationId,
) -> CustomResult<
    hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore,
    ApiErrorResponse,
> {
    let key_store = processor.get_key_store();
    let key_manager_state = state.into();
    let storage_scheme = processor.get_account().storage_scheme;

    let authentication = state
        .store
        .find_authentication_by_processor_merchant_id_authentication_id(
            processor.get_account().get_id(),
            &authentication_id,
            key_store,
            &key_manager_state,
            storage_scheme,
        )
        .await
        .to_not_found_response(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Error while fetching external vault authentication record for post-authenticate resume",
        )?;

    let (authentication_connector, auth_merchant_connector_account) =
        utils::get_authentication_connector_data(state, processor, business_profile, None).await?;
    let auth_connector_enum = authentication_connector
        .to_string()
        .parse::<common_enums::connector_enums::Connector>()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Invalid authentication connector name for UCS external vault 3DS auth leg",
        )?;

    let is_pull_mechanism_enabled =
        check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata(
            auth_merchant_connector_account
                .get_metadata()
                .map(|metadata| metadata.expose()),
        );

    let payment_method = payment_data
        .payment_attempt
        .payment_method
        .unwrap_or(common_enums::PaymentMethod::Card);

    let authentication = if !authentication.authentication_status.is_terminal_status()
        && is_pull_mechanism_enabled
    {
        // trigger in case of authenticate flow
        let post_authenticate_response_data = Box::pin(call_ucs_post_authenticate_proxy(
            state,
            processor,
            initiator,
            business_profile,
            payment_data,
            &authentication,
            auth_connector_enum,
            auth_merchant_connector_account.clone(),
            payment_method,
        ))
        .await?;

        let mut post_authenticate_tracker_router_data: core_types::RouterData<
            api::PostAuthentication,
            (),
            core_types::authentication::AuthenticationResponseData,
        > = transformers::construct_router_data(
            state,
            authentication_connector.to_string(),
            payment_method,
            business_profile.merchant_id.clone(),
            payment_data.address.clone(),
            (),
            &auth_merchant_connector_account,
            payment_data.payment_intent.psd2_sca_exemption_type,
            payment_data.payment_attempt.payment_id.clone(),
        )?;
        post_authenticate_tracker_router_data.response = post_authenticate_response_data;

        let authentication_info =
            hyperswitch_domain_models::router_request_types::authentication::AuthenticationInfo {
                billing_address: None,
                shipping_address: None,
                browser_info: None,
                email: None,
                device_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                platform: None,
            };

        Box::pin(utils::update_trackers(
            state,
            post_authenticate_tracker_router_data,
            authentication,
            None,
            key_store,
            authentication_info,
            storage_scheme,
        ))
        .await?
    } else {
        authentication
    };

    let tokenized_data = vault::get_tokenized_data(
        state,
        authentication_id.get_string_repr(),
        false,
        key_store.key.get_inner(),
    )
    .await
    .inspect_err(|err| router_env::logger::error!(external_vault_cavv_vault_lookup_error=?err))
    .attach_printable("cavv not present after authentication flow")
    .ok();

    Ok(
        hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore {
            cavv: tokenized_data.map(|data| hyperswitch_masking::Secret::new(data.value1)),
            authentication,
        },
    )
}

#[cfg(feature = "v1")]
struct AuthenticateProxyContext {
    authenticate_router_data: core_types::RouterData<
        api::Authenticate,
        core_types::PaymentsAuthenticateData,
        core_types::PaymentsResponseData,
    >,
    authentication: authentication::Authentication,
    authentication_id: common_utils::id_type::AuthenticationId,
    auth_connector_enum: common_enums::connector_enums::Connector,
    auth_merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    payment_method: common_enums::PaymentMethod,
    browser_info: Option<core_types::BrowserInformation>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AcquirerMetadata {
    acquirer_bin: Option<String>,
    acquirer_merchant_id: Option<String>,
    acquirer_country_code: Option<String>,
}

impl AcquirerMetadata {
    fn is_empty(&self) -> bool {
        self.acquirer_bin.is_none()
            && self.acquirer_merchant_id.is_none()
            && self.acquirer_country_code.is_none()
    }
}

/// Drives the live UCS authenticate (AReq) call; response interpretation is
/// `parse_ucs_authenticate_response`'s job.
#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
async fn call_ucs_authenticate_proxy(
    state: &SessionState,
    platform: &domain::Platform,
    payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    business_profile: &domain::Profile,
    amount: common_utils::types::MinorUnit,
    currency: Currency,
    billing_address: Option<hyperswitch_domain_models::address::Address>,
    optional_email: Option<common_utils::pii::Email>,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    device_channel: payments::DeviceChannel,
    sdk_information: Option<payments::SdkInformation>,
) -> CustomResult<AuthenticateProxyContext, ApiErrorResponse> {
    let processor = platform.get_processor();
    let key_store = processor.get_key_store();
    let key_manager_state = state.into();
    let storage_scheme = processor.get_account().storage_scheme;
    let processor_merchant_id = processor.get_account().get_id();

    let payment_token = payment_attempt
        .payment_token
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("payment_token missing on attempt for external vault 3DS authenticate")?;
    let payment_method = payment_attempt
        .payment_method
        .unwrap_or(common_enums::PaymentMethod::Card);
    let external_vault_pmd = payments_core::resolve_external_vault_alias_from_payment_token(
        state,
        platform,
        payment_token,
        payment_method,
    )
    .await?;

    let provider_business_profile =
        payments_core::helpers::resolve_provider_profile(state, platform, business_profile).await?;
    let external_vault_mca_id = provider_business_profile
        .external_vault_details
        .get_vault_connector_id()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("external vault is not enabled for this business profile")?;
    let external_vault_merchant_connector_account =
        payments_core::helpers::MerchantConnectorAccountType::DbVal(Box::new(
            state
                .store
                .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    platform.get_provider().get_account().get_id(),
                    &external_vault_mca_id,
                    platform.get_provider().get_key_store(),
                )
                .await
                .to_not_found_response(ApiErrorResponse::MerchantConnectorAccountNotFound {
                    id: external_vault_mca_id.get_string_repr().to_string(),
                })?,
        ));

    let profile_id = business_profile.get_id();
    let psp_connector_name = payment_attempt
        .connector
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("connector missing on attempt for external vault 3DS authenticate")?;
    let psp_merchant_connector_account = payments_core::helpers::get_merchant_connector_account(
        state,
        processor,
        None,
        profile_id,
        psp_connector_name.as_str(),
        payment_attempt.merchant_connector_id.as_ref(),
    )
    .await?;

    let authentication_id = payment_attempt
        .authentication_id
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "missing authentication_id on attempt for external vault 3DS authenticate",
        )?;
    let authentication = state
        .store
        .find_authentication_by_processor_merchant_id_authentication_id(
            processor_merchant_id,
            &authentication_id,
            key_store,
            &key_manager_state,
            storage_scheme,
        )
        .await
        .to_not_found_response(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while fetching external vault authentication record")?;

    let ucs_authentication_data =
        hyperswitch_domain_models::router_request_types::UcsAuthenticationData {
            eci: authentication.eci.clone(),
            cavv: None,
            threeds_server_transaction_id: authentication.threeds_server_transaction_id.clone(),
            message_version: authentication.message_version.clone(),
            ds_trans_id: authentication.ds_trans_id.clone(),
            acs_trans_id: authentication.acs_trans_id.clone(),
            trans_status: authentication.trans_status.clone(),
            transaction_id: authentication.connector_authentication_id.clone(),
            ucaf_collection_indicator: None,
            challenge_code: authentication.challenge_code.clone(),
            challenge_cancel: authentication.challenge_cancel.clone(),
            challenge_code_reason: authentication.challenge_code_reason.clone(),
            message_extension: authentication.message_extension.clone(),
        };

    let browser_info: Option<core_types::BrowserInformation> = authentication
        .browser_info
        .clone()
        .map(|browser_information| browser_information.parse_value("BrowserInformation"))
        .transpose()
        .change_context(ApiErrorResponse::InvalidDataValue {
            field_name: "browser_info",
        })?;

    let authenticate_request_data = core_types::PaymentsAuthenticateData {
        payment_method_data: None,
        payment_method_type: payment_attempt.payment_method_type,
        amount: Some(amount.get_amount_as_i64()),
        minor_amount: Some(amount),
        email: optional_email.clone(),
        currency: Some(currency),
        complete_authorize_url: None,
        browser_info: browser_info.clone(),
        redirect_response: None,
        capture_method: payment_attempt.capture_method,
        authentication_data: Some(ucs_authentication_data),
        sdk_information: sdk_information.clone(),
        device_channel: Some(device_channel.clone()),
        webhook_url: None,
    };

    let payment_address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
        None,
        billing_address.clone(),
        None,
        None,
    );

    let (authentication_connector, auth_merchant_connector_account) =
        utils::get_authentication_connector_data(state, processor, business_profile, None).await?;
    let auth_connector_enum = authentication_connector
        .to_string()
        .parse::<common_enums::connector_enums::Connector>()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Invalid authentication connector name for UCS external vault 3DS auth leg",
        )?;

    let mut authenticate_router_data: core_types::RouterData<
        api::Authenticate,
        core_types::PaymentsAuthenticateData,
        core_types::PaymentsResponseData,
    > = transformers::construct_router_data(
        state,
        auth_connector_enum.to_string(),
        payment_method,
        business_profile.merchant_id.clone(),
        payment_address,
        authenticate_request_data,
        &auth_merchant_connector_account,
        payment_intent.psd2_sca_exemption_type,
        payment_intent.payment_id.clone(),
    )?;
    authenticate_router_data.request.webhook_url =
        auth_merchant_connector_account.get_mca_id().map(|mca_id| {
            payments_core::helpers::create_webhook_url(
                &state.base_url,
                &business_profile.merchant_id,
                mca_id.get_string_repr(),
            )
        });

    let (execution_path, updated_state) =
        unified_connector_service::should_call_unified_connector_service(
            state,
            processor,
            &authenticate_router_data,
            None,
            payments_core::CallConnectorAction::Trigger,
            None,
            common_enums::TransactionType::Payment,
        )
        .await?;
    let execution_mode = match execution_path {
        common_enums::ExecutionPath::UnifiedConnectorService => {
            Some(common_enums::ExecutionMode::Primary)
        }
        common_enums::ExecutionPath::ShadowUnifiedConnectorService => {
            Some(common_enums::ExecutionMode::Shadow)
        }
        common_enums::ExecutionPath::Direct => None,
    }
    .ok_or(ApiErrorResponse::InternalServerError)
    .attach_printable(
        "UCS is not enabled for this connector; external vault 3DS authenticate over UCS is unavailable",
    )?;

    let lineage_ids = external_services::grpc_client::LineageIds::new(
        business_profile.merchant_id.clone(),
        business_profile.get_id().clone(),
    );

    let notification_url = Some(common_utils::types::Url::wrap(
        url::Url::parse(&payments_core::helpers::create_authorize_url(
            &state.base_url,
            payment_attempt,
            &psp_connector_name,
        ))
        .change_context(ApiErrorResponse::InternalServerError)?,
    ));

    // Prefer acquirer details set directly on the PSP connector's own metadata; fall back to the
    // profile's card-network-keyed acquirer config the same way
    // `get_payment_external_authentication_flow_during_confirm` does for the direct flow — a
    // profile-acquirer-id-specific bucket first, then the profile's network default.
    let acquirer_metadata = psp_merchant_connector_account
        .get_metadata()
        .and_then(|metadata| serde_json::from_value::<AcquirerMetadata>(metadata.expose()).ok())
        .filter(|metadata| !metadata.is_empty())
        .or_else(|| {
            let card_network = match &external_vault_pmd {
                domain::ExternalVaultPaymentMethodData::Card(card) => card.card_network.clone(),
                domain::ExternalVaultPaymentMethodData::VaultToken(_) => None,
            }?;
            let acquirer_config = payment_intent
                .profile_acquirer_id
                .as_ref()
                .and_then(|profile_acquirer_id| {
                    business_profile.get_acquirer_details_for_profile_acquirer(
                        profile_acquirer_id,
                        card_network.clone(),
                    )
                })
                .or_else(|| {
                    business_profile.get_default_acquirer_details_from_network(card_network)
                })?;
            Some(AcquirerMetadata {
                acquirer_bin: acquirer_config.acquirer_bin,
                acquirer_merchant_id: acquirer_config.acquirer_assigned_merchant_id,
                acquirer_country_code: acquirer_config.acquirer_country_code,
            })
        })
        .and_then(|metadata| serde_json::to_value(metadata).ok());

    let authenticate_router_data = Box::pin(
        payments_core::flows::complete_authorize_flow::call_unified_connector_service_authenticate_proxy(
            &authenticate_router_data,
            &external_vault_pmd,
            &updated_state,
            header_payload,
            lineage_ids,
            auth_merchant_connector_account.clone(),
            external_vault_merchant_connector_account,
            processor,
            execution_mode,
            Some(
                payment_intent
                    .force_3ds_challenge
                    .unwrap_or(business_profile.force_3ds_challenge),
            ),
            notification_url,
            acquirer_metadata,
        ),
    )
    .await
    .change_context(ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to call UCS authenticate for external vault proxy")?;

    Ok(AuthenticateProxyContext {
        authenticate_router_data,
        authentication,
        authentication_id,
        auth_connector_enum,
        auth_merchant_connector_account,
        payment_method,
        browser_info,
    })
}

#[cfg(feature = "v1")]
struct ParsedAuthenticateResponse {
    authenticate_response_data: core_types::authentication::AuthenticationResponseData,
    authentication_type: common_enums::DecoupledAuthenticationType,
    trans_status: common_enums::TransactionStatus,
    acs_url: Option<url::Url>,
    challenge_request: Option<String>,
    acs_reference_number: Option<String>,
    acs_trans_id: Option<String>,
    three_ds_server_transaction_id: Option<String>,
    acs_signed_content: Option<String>,
}

#[cfg(feature = "v1")]
#[derive(serde::Deserialize)]
struct AppChallengeAcsMetadata {
    acs_signed_content: Option<String>,
    acs_reference_number: Option<String>,
    acs_trans_id: Option<String>,
}

/// Interprets a successful UCS authenticate response into `AuthenticationResponseData::AuthNResponse`
/// plus the SDK-facing challenge/ARes fields; `AppChallengeAcsMetadata` reads ACS fields back out
/// of the JSON-stuffed `connector_metadata` since UCS has no typed slots for them. A connector-level
/// error is the caller's responsibility to hard-fail on (mirroring `perform_authentication`'s
/// `response.response.map_err(...)?`) before ever reaching this function.
#[cfg(feature = "v1")]
fn parse_ucs_authenticate_response(
    response: &core_types::PaymentsResponseData,
) -> CustomResult<ParsedAuthenticateResponse, ApiErrorResponse> {
    let (areq_authentication_data, redirection_data, connector_metadata) = match response {
        core_types::PaymentsResponseData::TransactionResponse {
            authentication_data,
            redirection_data,
            connector_metadata,
            ..
        } => (
            authentication_data.clone().map(|boxed| *boxed),
            (**redirection_data).clone(),
            connector_metadata.clone(),
        ),
        _ => (None, None, None),
    };

    let app_acs = connector_metadata.as_ref().and_then(|metadata| {
        serde_json::from_value::<AppChallengeAcsMetadata>(metadata.clone()).ok()
    });

    let (acs_url, challenge_request) = match &redirection_data {
        Some(hyperswitch_domain_models::router_response_types::RedirectForm::Form {
            endpoint,
            form_fields,
            ..
        }) => (
            Some(endpoint.clone()),
            form_fields.get(consts::CREQ_CHALLENGE_REQUEST_KEY).cloned(),
        ),
        _ => (None, None),
    };
    let acs_signed_content = app_acs.as_ref().and_then(|m| m.acs_signed_content.clone());
    let acs_reference_number = app_acs
        .as_ref()
        .and_then(|m| m.acs_reference_number.clone());

    let trans_status = areq_authentication_data
        .as_ref()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("UCS authenticate response missing authentication_data")?
        .trans_status
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("UCS authenticate response missing trans_status")?;
    let acs_trans_id = areq_authentication_data
        .as_ref()
        .and_then(|data| data.acs_trans_id.clone())
        .or_else(|| app_acs.as_ref().and_then(|m| m.acs_trans_id.clone()));
    let eci = areq_authentication_data
        .as_ref()
        .and_then(|data| data.eci.clone());
    let ds_trans_id = areq_authentication_data
        .as_ref()
        .and_then(|data| data.ds_trans_id.clone());
    let three_ds_server_transaction_id = areq_authentication_data
        .as_ref()
        .and_then(|data| data.threeds_server_transaction_id.clone());

    let areq_cavv = areq_authentication_data
        .as_ref()
        .and_then(|data| data.cavv.clone());

    let authentication_type = common_enums::DecoupledAuthenticationType::from(trans_status.clone());

    let acs_url = acs_url
        .map(|url| url::Url::parse(&url))
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("acs_url returned by UCS authenticate is not a valid URL")?;

    let authn_flow_type = match authentication_type {
        common_enums::DecoupledAuthenticationType::Challenge => {
            hyperswitch_domain_models::router_request_types::authentication::AuthNFlowType::Challenge(Box::new(
                hyperswitch_domain_models::router_request_types::authentication::ChallengeParams {
                    acs_url: acs_url.clone(),
                    challenge_request: challenge_request.clone(),
                    challenge_request_key: None,
                    acs_reference_number: acs_reference_number.clone(),
                    acs_trans_id: acs_trans_id.clone(),
                    three_dsserver_trans_id: three_ds_server_transaction_id.clone(),
                    acs_signed_content: acs_signed_content.clone(),
                },
            ))
        }
        common_enums::DecoupledAuthenticationType::Frictionless => {
            hyperswitch_domain_models::router_request_types::authentication::AuthNFlowType::Frictionless
        }
    };

    let authenticate_response_data =
        core_types::authentication::AuthenticationResponseData::AuthNResponse {
            authn_flow_type,
            authentication_value: areq_cavv,
            trans_status: trans_status.clone(),
            connector_metadata: None,
            ds_trans_id,
            eci,
            challenge_code: areq_authentication_data
                .as_ref()
                .and_then(|data| data.challenge_code.clone()),
            challenge_cancel: areq_authentication_data
                .as_ref()
                .and_then(|data| data.challenge_cancel.clone()),
            challenge_code_reason: areq_authentication_data
                .as_ref()
                .and_then(|data| data.challenge_code_reason.clone()),
            message_extension: areq_authentication_data
                .as_ref()
                .and_then(|data| data.message_extension.clone()),
        };

    Ok(ParsedAuthenticateResponse {
        authenticate_response_data,
        authentication_type,
        trans_status,
        acs_url,
        challenge_request,
        acs_reference_number,
        acs_trans_id,
        three_ds_server_transaction_id,
        acs_signed_content,
    })
}

/// Proxy analogue of `perform_authentication` (the AReq step), called from the
/// `/payments/{id}/3ds/authentication` handler for external-vault-proxy payments.
#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn perform_authentication_proxy(
    state: &SessionState,
    platform: &domain::Platform,
    payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    business_profile: &domain::Profile,
    amount: common_utils::types::MinorUnit,
    currency: Currency,
    billing_address: Option<hyperswitch_domain_models::address::Address>,
    shipping_address: Option<hyperswitch_domain_models::address::Address>,
    optional_email: Option<common_utils::pii::Email>,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    device_channel: payments::DeviceChannel,
    sdk_information: Option<payments::SdkInformation>,
) -> CustomResult<api::authentication::AuthenticationResponse, ApiErrorResponse> {
    let processor = platform.get_processor();
    let key_store = processor.get_key_store();
    let storage_scheme = processor.get_account().storage_scheme;

    let AuthenticateProxyContext {
        authenticate_router_data,
        authentication,
        authentication_id,
        auth_connector_enum,
        auth_merchant_connector_account,
        payment_method,
        browser_info,
    } = Box::pin(call_ucs_authenticate_proxy(
        state,
        platform,
        payment_intent,
        payment_attempt,
        business_profile,
        amount,
        currency,
        billing_address.clone(),
        optional_email.clone(),
        &header_payload,
        device_channel.clone(),
        sdk_information.clone(),
    ))
    .await?;

    let authenticate_response_data_ref =
        authenticate_router_data.response.as_ref().map_err(|err| {
            error_stack::report!(ApiErrorResponse::ExternalConnectorError {
                code: err.code.clone(),
                message: err.message.clone(),
                connector: auth_connector_enum.to_string(),
                status_code: err.status_code,
                reason: err.reason.clone(),
            })
        })?;

    let ParsedAuthenticateResponse {
        authenticate_response_data,
        authentication_type,
        trans_status,
        acs_url,
        challenge_request,
        acs_reference_number,
        acs_trans_id,
        three_ds_server_transaction_id,
        acs_signed_content,
    } = parse_ucs_authenticate_response(authenticate_response_data_ref)?;

    let mut authenticate_tracker_router_data: core_types::RouterData<
        api::Authentication,
        (),
        core_types::authentication::AuthenticationResponseData,
    > = transformers::construct_router_data(
        state,
        auth_connector_enum.to_string(),
        payment_method,
        business_profile.merchant_id.clone(),
        hyperswitch_domain_models::payment_address::PaymentAddress::new(
            None,
            billing_address.clone(),
            None,
            None,
        ),
        (),
        &auth_merchant_connector_account,
        payment_intent.psd2_sca_exemption_type,
        payment_intent.payment_id.clone(),
    )?;
    authenticate_tracker_router_data.response = Ok(authenticate_response_data);

    let authentication_info =
        hyperswitch_domain_models::router_request_types::authentication::AuthenticationInfo {
            billing_address,
            shipping_address,
            browser_info,
            email: optional_email,
            device_details: sdk_information
                .as_ref()
                .and_then(|sdk_information| sdk_information.device_details.clone()),
            merchant_category_code: None,
            merchant_country_code: None,
            platform: Some(device_channel),
        };

    let authentication = Box::pin(utils::update_trackers(
        state,
        authenticate_tracker_router_data,
        authentication,
        None,
        key_store,
        authentication_info,
        storage_scheme,
    ))
    .await?;

    let attempt_update = PaymentAttemptUpdate::AuthenticationUpdate {
        status: payment_attempt.status,
        external_three_ds_authentication_attempted: Some(true),
        external_threeds_authentication_type: Some(authentication_type),
        authentication_connector: authentication.authentication_connector.clone(),
        authentication_id: Some(authentication_id),
        updated_by: storage_scheme.to_string(),
    };
    state
        .store
        .update_payment_attempt_with_attempt_id(
            payment_attempt.clone(),
            attempt_update,
            storage_scheme,
            key_store,
        )
        .await
        .to_not_found_response(ApiErrorResponse::PaymentNotFound)
        .attach_printable(
            "Error while updating the payment_attempt for external vault authenticate",
        )?;

    Ok(api::authentication::AuthenticationResponse {
        trans_status,
        acs_url,
        challenge_request,
        acs_reference_number,
        acs_trans_id,
        three_dsserver_trans_id: three_ds_server_transaction_id,
        acs_signed_content,
        challenge_request_key: None,
        // A connector-level error is now a hard `Err` above, not a value ending up here.
        error_message: None,
    })
}
