use std::marker::PhantomData;

#[cfg(feature = "v1")]
use api_models::payments::BrowserInformation;
use common_enums::enums::PaymentMethod;
use common_utils::{
    ext_traits::{AsyncExt, ValueExt},
    types::keymanager::ToEncryptable,
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    business_profile,
    errors::api_error_response::ApiErrorResponse,
    ext_traits::OptionExt,
    payment_address::PaymentAddress,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_data_v2::UasFlowData,
    router_request_types::unified_authentication_service::{
        PostAuthenticationDetails, UasAuthenticationResponseData,
    },
    type_encryption::AsyncLift,
};
use masking::{ExposeInterface, PeekInterface};

use super::types::{
    IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW,
    IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW,
};
use crate::{
    consts::DEFAULT_SESSION_EXPIRY,
    core::{
        errors::{
            utils::{ConnectorErrorExt, StorageErrorExt},
            RouterResult,
        },
        payment_methods, payments,
    },
    db::domain,
    services::{self, execute_connector_processing_step},
    types::{api, transformers::ForeignFrom},
    SessionState,
};

pub async fn do_auth_connector_call<F, Req, Res>(
    state: &SessionState,
    authentication_connector_name: String,
    router_data: RouterData<F, Req, Res>,
) -> RouterResult<RouterData<F, Req, Res>>
where
    Req: std::fmt::Debug + Clone + 'static,
    Res: std::fmt::Debug + Clone + 'static,
    F: std::fmt::Debug + Clone + 'static,
    dyn api::Connector + Sync: services::api::ConnectorIntegration<F, Req, Res>,
    dyn api::ConnectorV2 + Sync: services::api::ConnectorIntegrationV2<F, UasFlowData, Req, Res>,
{
    let connector_data =
        api::AuthenticationConnectorData::get_connector_by_name(&authentication_connector_name)?;
    let connector_integration: services::BoxedUnifiedAuthenticationServiceInterface<F, Req, Res> =
        connector_data.connector.get_connector_integration();
    let router_data = execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_payment_failed_response()?;
    Ok(router_data)
}

#[allow(clippy::too_many_arguments)]
pub fn construct_uas_router_data<F: Clone, Req, Res>(
    state: &SessionState,
    authentication_connector_name: String,
    payment_method: PaymentMethod,
    merchant_id: common_utils::id_type::MerchantId,
    address: Option<PaymentAddress>,
    request_data: Req,
    merchant_connector_account: &payments::helpers::MerchantConnectorAccountType,
    authentication_id: Option<common_utils::id_type::AuthenticationId>,
    payment_id: Option<common_utils::id_type::PaymentId>,
) -> RouterResult<RouterData<F, Req, Res>> {
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing ConnectorAuthType")?;
    Ok(RouterData {
        flow: PhantomData,
        merchant_id,
        customer_id: None,
        connector_customer: None,
        connector: authentication_connector_name,
        payment_id: payment_id
            .map(|id| id.get_string_repr().to_owned())
            .unwrap_or_default(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method,
        payment_method_type: None,
        connector_auth_type: auth_type,
        description: None,
        address: address.unwrap_or_default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata().clone(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request: request_data,
        response: Err(ErrorResponse::default()),
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: None,
        refund_id: None,
        payout_id: None,
        payment_method_status: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id,
        psd2_sca_exemption_type: None,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
        customer_document_number: None,
    })
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v1")]
pub async fn external_authentication_update_trackers<F: Clone, Req>(
    state: &SessionState,
    router_data: RouterData<F, Req, UasAuthenticationResponseData>,
    authentication: hyperswitch_domain_models::authentication::Authentication,
    acquirer_details: Option<
        hyperswitch_domain_models::router_request_types::authentication::AcquirerDetails,
    >,
    merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
    billing_address: Option<hyperswitch_domain_models::address::Address>,
    shipping_address: Option<hyperswitch_domain_models::address::Address>,
    email: Option<common_utils::pii::Email>,
    browser_info: Option<BrowserInformation>,
    device_details: Option<api_models::payments::DeviceDetails>,
    merchant_category_code: Option<common_enums::MerchantCategoryCode>,
    merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
) -> RouterResult<hyperswitch_domain_models::authentication::Authentication> {
    let key_state = state.into();
    let authentication_update = match router_data.response {
        Ok(response) => match response {
            UasAuthenticationResponseData::PreAuthentication {
                authentication_details,
            } => {
                let billing_details_encoded = billing_address
                    .clone()
                    .map(|billing| {
                        common_utils::ext_traits::Encode::encode_to_value(&billing)
                            .map(masking::Secret::<serde_json::Value>::new)
                    })
                    .transpose()
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encode billing details to serde_json::Value")?;

                let shipping_details_encoded = shipping_address
                    .clone()
                    .map(|shipping| {
                        common_utils::ext_traits::Encode::encode_to_value(&shipping)
                            .map(masking::Secret::<serde_json::Value>::new)
                    })
                    .transpose()
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encode shipping details to serde_json::Value")?;

                let encrypted_data = domain::types::crypto_operation(
        &key_state,
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
            merchant_key_store
                .merchant_id
                .clone(),
        ),
        merchant_key_store.key.peek(),
    )
    .await
    .and_then(|val| val.try_into_batchoperation())
    .change_context(ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to encrypt authentication data".to_string())?;

                let encrypted_data = hyperswitch_domain_models::authentication::FromRequestEncryptableAuthentication::from_encryptable(encrypted_data)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to get encrypted data for authentication after encryption")?;

                let email_encrypted = email
                    .clone()
                    .async_lift(|inner| async {
                        domain::types::crypto_operation(
                            &key_state,
                            common_utils::type_name!(
                                hyperswitch_domain_models::authentication::Authentication
                            ),
                            domain::types::CryptoOperation::EncryptOptional(
                                inner.map(|inner| inner.expose()),
                            ),
                            common_utils::types::keymanager::Identifier::Merchant(
                                merchant_key_store.merchant_id.clone(),
                            ),
                            merchant_key_store.key.peek(),
                        )
                        .await
                        .and_then(|val| val.try_into_optionaloperation())
                    })
                    .await
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encrypt email")?;

                let browser_info = browser_info
                    .as_ref()
                    .map(common_utils::ext_traits::Encode::encode_to_value)
                    .transpose()
                    .change_context(ApiErrorResponse::InvalidDataValue {
                        field_name: "browser_information",
                    })?;
                Ok(
                    hyperswitch_domain_models::authentication::AuthenticationUpdate::PreAuthenticationUpdate {
                        earliest_supported_version:authentication_details.maximum_supported_3ds_version.clone(),
                        latest_supported_version: authentication_details.maximum_supported_3ds_version.clone(),
                        threeds_server_transaction_id: authentication_details
                            .threeds_server_transaction_id
                            .ok_or(ApiErrorResponse::InternalServerError)
                            .attach_printable(
                                "missing threeds_server_transaction_id in PreAuthentication Details",
                            )?,
                        maximum_supported_3ds_version: authentication_details
                            .maximum_supported_3ds_version
                            .clone()
                            .ok_or(ApiErrorResponse::InternalServerError)
                            .attach_printable(
                                "missing maximum_supported_3ds_version in PreAuthentication Details",
                            )?,
                        connector_authentication_id: authentication_details
                            .connector_authentication_id
                            .ok_or(ApiErrorResponse::InternalServerError)
                            .attach_printable(
                                "missing connector_authentication_id in PreAuthentication Details",
                            )?,
                        three_ds_method_data: authentication_details.three_ds_method_data,
                        three_ds_method_url: authentication_details.three_ds_method_url,
                        message_version: authentication_details
                            .message_version
                            .ok_or(ApiErrorResponse::InternalServerError)
                            .attach_printable("missing message_version in PreAuthentication Details")?,
                        connector_metadata: authentication_details.connector_metadata.clone(),
                        authentication_status: common_enums::AuthenticationStatus::Pending,
                        acquirer_bin: acquirer_details
                            .as_ref()
                            .map(|acquirer_details| acquirer_details.acquirer_bin.clone()),
                        acquirer_merchant_id: acquirer_details
                            .as_ref()
                            .map(|acquirer_details| acquirer_details.acquirer_merchant_id.clone()),
                        acquirer_country_code: acquirer_details
                            .and_then(|acquirer_details| acquirer_details.acquirer_country_code),
                        directory_server_id: authentication_details.directory_server_id.clone(),
                        browser_info: Box::new(browser_info),
                        email:email_encrypted,
                        billing_address: Box::new(encrypted_data.billing_address),
                        shipping_address: Box::new(encrypted_data.shipping_address),
                        scheme_id: authentication_details.scheme_id.clone(),
                        merchant_category_code,
                        merchant_country_code: merchant_country_code.map(|merchant_country_code| merchant_country_code.get_country_code()),
                        billing_country: billing_address
                            .clone()
                            .and_then(|billing| billing.address.clone().and_then(|address| address.country.map(|country| country.to_string()))),
                        shipping_country: shipping_address
                            .clone()
                            .and_then(|shipping| shipping.address.clone().and_then(|address| address.country.map(|country| country.to_string()))),
                    },
                )
            }
            UasAuthenticationResponseData::Authentication {
                authentication_details,
            } => {
                let authentication_status = common_enums::AuthenticationStatus::foreign_from(
                    authentication_details.trans_status.clone(),
                );
                authentication_details
                    .authentication_value
                    .async_map(|auth_val| {
                        payment_methods::vault::create_tokenize(
                            state,
                            auth_val.expose(),
                            None,
                            authentication
                                .authentication_id
                                .get_string_repr()
                                .to_string(),
                            merchant_key_store.key.get_inner(),
                        )
                    })
                    .await
                    .transpose()?;
                Ok(
                    hyperswitch_domain_models::authentication::AuthenticationUpdate::AuthenticationUpdate {
                        trans_status: authentication_details.trans_status,
                        acs_url: authentication_details.authn_flow_type.get_acs_url(),
                        challenge_request: authentication_details
                            .authn_flow_type
                            .get_challenge_request(),
                        challenge_request_key: authentication_details
                            .authn_flow_type
                            .get_challenge_request_key(),
                        acs_reference_number: authentication_details
                            .authn_flow_type
                            .get_acs_reference_number(),
                        acs_trans_id: authentication_details.authn_flow_type.get_acs_trans_id(),
                        acs_signed_content: authentication_details
                            .authn_flow_type
                            .get_acs_signed_content(),
                        authentication_type: authentication_details
                            .authn_flow_type
                            .get_decoupled_authentication_type(),
                        authentication_status,
                        connector_metadata: authentication_details.connector_metadata,
                        ds_trans_id: authentication_details.ds_trans_id,
                        eci: authentication_details.eci,
                        challenge_code: authentication_details.challenge_code,
                        challenge_cancel: authentication_details.challenge_cancel,
                        challenge_code_reason: authentication_details.challenge_code_reason,
                        message_extension: authentication_details.message_extension,
                        device_type: device_details
                            .as_ref()
                            .and_then(|details| details.device_type.clone()),
                        device_brand: device_details
                            .as_ref()
                            .and_then(|details| details.device_brand.clone()),
                        device_os: device_details
                            .as_ref()
                            .and_then(|details| details.device_os.clone()),
                        device_display: device_details
                            .as_ref()
                            .and_then(|details| details.device_display.clone()),
                    },
                )
            }
            UasAuthenticationResponseData::PostAuthentication {
                authentication_details,
            } => {
                let trans_status = authentication_details
                    .trans_status
                    .ok_or(ApiErrorResponse::InternalServerError)
                    .attach_printable("missing trans_status in PostAuthentication Details")?;

                authentication_details
                    .dynamic_data_details
                    .and_then(|details| details.dynamic_data_value)
                    .map(ExposeInterface::expose)
                    .async_map(|auth_val| {
                        payment_methods::vault::create_tokenize(
                            state,
                            auth_val,
                            None,
                            authentication
                                .authentication_id
                                .get_string_repr()
                                .to_string(),
                            merchant_key_store.key.get_inner(),
                        )
                    })
                    .await
                    .transpose()?;
                Ok(
                    hyperswitch_domain_models::authentication::AuthenticationUpdate::PostAuthenticationUpdate {
                        authentication_status: common_enums::AuthenticationStatus::foreign_from(
                            trans_status.clone(),
                        ),
                        trans_status,
                        eci: authentication_details.eci,
                        challenge_cancel: authentication_details.challenge_cancel,
                        challenge_code_reason: authentication_details.challenge_code_reason,
                    },
                )
            }

            UasAuthenticationResponseData::Confirmation { .. } => Err(
                ApiErrorResponse::InternalServerError,
            )
            .attach_printable("unexpected api confirmation in external authentication flow."),
        },
        Err(error) => Ok(
            hyperswitch_domain_models::authentication::AuthenticationUpdate::ErrorUpdate {
                connector_authentication_id: error.connector_transaction_id,
                authentication_status: common_enums::AuthenticationStatus::Failed,
                error_message: error
                    .reason
                    .map(|reason| format!("message: {}, reason: {}", error.message, reason))
                    .or(Some(error.message)),
                error_code: Some(error.code),
            },
        ),
    }?;

    state
        .store
        .update_authentication_by_merchant_id_authentication_id(
            authentication,
            authentication_update,
            merchant_key_store,
            &key_state,
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while updating authentication")
}

pub fn get_checkout_event_status_and_reason(
    attempt_status: common_enums::AttemptStatus,
) -> (Option<String>, Option<String>) {
    match attempt_status {
        common_enums::AttemptStatus::Charged | common_enums::AttemptStatus::Authorized => (
            Some("01".to_string()),
            Some("The payment was successful".to_string()),
        ),
        _ => (
            Some("03".to_string()),
            Some("The payment was not successful".to_string()),
        ),
    }
}

pub fn authenticate_authentication_client_secret_and_check_expiry(
    req_client_secret: &String,
    authentication: &hyperswitch_domain_models::authentication::Authentication,
) -> RouterResult<()> {
    let stored_client_secret = authentication
        .authentication_client_secret
        .clone()
        .get_required_value("authentication_client_secret")
        .change_context(ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret",
        })
        .attach_printable("client secret not found in db")?;

    if req_client_secret != &stored_client_secret {
        Err(report!(ApiErrorResponse::ClientSecretInvalid))
    } else {
        let current_timestamp = common_utils::date_time::now();
        let session_expiry = authentication
            .created_at
            .saturating_add(time::Duration::seconds(DEFAULT_SESSION_EXPIRY));

        if current_timestamp > session_expiry {
            Err(report!(ApiErrorResponse::ClientSecretExpired))
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "v1")]
pub async fn get_auth_multi_token_from_external_vault<F, Req>(
    state: &SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    router_data: &RouterData<F, Req, UasAuthenticationResponseData>,
) -> RouterResult<Option<api_models::authentication::AuthenticationVaultTokenData>> {
    if let Ok(UasAuthenticationResponseData::PostAuthentication {
        authentication_details,
    }) = router_data.response.clone()
    {
        match business_profile.external_vault_details.clone() {
            business_profile::ExternalVaultDetails::ExternalVaultEnabled(
                external_vault_details,
            ) => {
                let external_vault_mca_id = external_vault_details.vault_connector_id.clone();

                let merchant_connector_account_details = state
                    .store
                    .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                        platform.get_processor().get_account().get_id(),
                        &external_vault_mca_id,
                        platform.get_processor().get_key_store(),
                    )
                    .await
                    .to_not_found_response(ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: external_vault_mca_id.get_string_repr().to_string(),
                    })?;

                let vault_data = get_vault_details(authentication_details)?;

                // decide which fields to tokenize in vault
                let vault_custom_data = payment_methods::get_payment_method_custom_data(
                    vault_data,
                    external_vault_details.vault_token_selector,
                )?;

                let external_vault_response =
                    Box::pin(payment_methods::vault_payment_method_external_v1(
                        state,
                        &vault_custom_data,
                        platform.get_processor().get_account(),
                        merchant_connector_account_details,
                        Some(true),
                    ))
                    .await?;

                let vault_token_data = external_vault_response
                    .vault_id
                    .get_auth_vault_token_data()?;

                Ok(Some(vault_token_data))
            }
            business_profile::ExternalVaultDetails::Skip => {
                Err(ApiErrorResponse::InternalServerError)
                    .attach_printable("External Vault Details are missing")
            }
        }
    } else {
        Ok(None)
    }
}

#[cfg(feature = "v1")]
fn get_vault_details(
    auth_details: PostAuthenticationDetails,
) -> RouterResult<hyperswitch_domain_models::vault::PaymentMethodVaultingData> {
    // if raw card details are present, it have the highest priority
    // if network_token details are present, then it is returned if raw card details are not present
    // throw error if both are not present
    match (auth_details.raw_card_details, auth_details.token_details) {
        (Some(card_details), _) => Ok(
            hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(
                api_models::payment_methods::CardDetail {
                    card_number: card_details.pan.clone(),
                    card_exp_month: card_details.expiration_month,
                    card_exp_year: card_details.expiration_year,
                    card_cvc: card_details.card_security_code,
                    card_issuer: None,
                    card_network: None,
                    card_type: None,
                    card_issuing_country: None,
                    card_issuing_country_code: None,
                    card_holder_name: None,
                    nick_name: None,
                },
            ),
        ),
        (None, Some(token_data)) => {
            let cryptogram = auth_details
                .dynamic_data_details
                .and_then(|dynamic_data| dynamic_data.dynamic_data_value);

            Ok(
                hyperswitch_domain_models::vault::PaymentMethodVaultingData::NetworkToken(
                    hyperswitch_domain_models::payment_method_data::NetworkTokenDetails {
                        network_token: token_data.payment_token.clone(),
                        network_token_exp_month: token_data.token_expiration_month.clone(),
                        network_token_exp_year: token_data.token_expiration_year.clone(),
                        cryptogram,
                        card_issuer: None,
                        card_network: None,
                        card_type: None,
                        card_issuing_country: None,
                        card_holder_name: None,
                        nick_name: None,
                    },
                ),
            )
        }
        (None, None) => Err(ApiErrorResponse::MissingRequiredField {
            field_name: "Either Card or Network Token details",
        }
        .into()),
    }
}

#[cfg(feature = "v1")]
pub fn get_authentication_payment_method_data<F, Req>(
    router_data: &RouterData<F, Req, UasAuthenticationResponseData>,
) -> Option<api_models::authentication::AuthenticationPaymentMethodDataResponse> {
    if let Ok(UasAuthenticationResponseData::PostAuthentication {
        authentication_details,
    }) = router_data.response.clone()
    {
        authentication_details.into()
    } else {
        None
    }
}
