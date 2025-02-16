use api_models::{admin as admin_types, payments as payment_types};
use async_trait::async_trait;
use common_utils::{
    ext_traits::ByteSliceExt,
    request::RequestContent,
    types::{AmountConvertor, StringMajorUnitForConnector},
};
use error_stack::{Report, ResultExt};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentIntentData;
use masking::{ExposeInterface, ExposeOptionInterface};

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    consts::PROTOCOL,
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    headers, logger,
    routes::{self, app::settings, metrics},
    services,
    types::{
        self,
        api::{self, enums},
        domain,
    },
    utils::OptionExt,
};

#[cfg(feature = "v2")]
#[async_trait]
impl
    ConstructFlowSpecificData<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for PaymentIntentData<api::Session>
{
    async fn construct_router_data<'a>(
        &self,
        state: &routes::SessionState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccount,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<types::PaymentsSessionRouterData> {
        Box::pin(transformers::construct_payment_router_data_for_sdk_session(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
        ))
        .await
    }

    async fn get_merchant_recipient_data<'a>(
        &self,
        _state: &routes::SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _merchant_connector_account: &helpers::MerchantConnectorAccountType,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        Ok(None)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl
    ConstructFlowSpecificData<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for PaymentData<api::Session>
{
    async fn construct_router_data<'a>(
        &self,
        state: &routes::SessionState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<types::PaymentsSessionRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::Session,
            types::PaymentsSessionData,
        >(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
        ))
        .await
    }

    async fn get_merchant_recipient_data<'a>(
        &self,
        _state: &routes::SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _merchant_connector_account: &helpers::MerchantConnectorAccountType,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        Ok(None)
    }
}

#[async_trait]
impl Feature<api::Session, types::PaymentsSessionData> for types::PaymentsSessionRouterData {
    async fn decide_flows<'a>(
        self,
        state: &routes::SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        _connector_request: Option<services::Request>,
        business_profile: &domain::Profile,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<Self> {
        metrics::SESSION_TOKEN_CREATED.add(
            1,
            router_env::metric_attributes!(("connector", connector.connector_name.to_string())),
        );
        self.decide_flow(
            state,
            connector,
            Some(true),
            call_connector_action,
            business_profile,
            header_payload,
        )
        .await
    }

    async fn add_access_token<'a>(
        &self,
        state: &routes::SessionState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self, creds_identifier)
            .await
    }
}

/// This function checks if for a given connector, payment_method and payment_method_type,
/// the list of required_field_type is present in dynamic fields
#[cfg(feature = "v1")]
fn is_dynamic_fields_required(
    required_fields: &settings::RequiredFields,
    payment_method: enums::PaymentMethod,
    payment_method_type: enums::PaymentMethodType,
    connector: types::Connector,
    required_field_type: Vec<enums::FieldType>,
) -> bool {
    required_fields
        .0
        .get(&payment_method)
        .and_then(|pm_type| pm_type.0.get(&payment_method_type))
        .and_then(|required_fields_for_connector| {
            required_fields_for_connector.fields.get(&connector)
        })
        .map(|required_fields_final| {
            required_fields_final
                .non_mandate
                .iter()
                .any(|(_, val)| required_field_type.contains(&val.field_type))
                || required_fields_final
                    .mandate
                    .iter()
                    .any(|(_, val)| required_field_type.contains(&val.field_type))
                || required_fields_final
                    .common
                    .iter()
                    .any(|(_, val)| required_field_type.contains(&val.field_type))
        })
        .unwrap_or(false)
}

/// This function checks if for a given connector, payment_method and payment_method_type,
/// the list of required_field_type is present in dynamic fields
#[cfg(feature = "v2")]
fn is_dynamic_fields_required(
    required_fields: &settings::RequiredFields,
    payment_method: enums::PaymentMethod,
    payment_method_type: enums::PaymentMethodType,
    connector: types::Connector,
    required_field_type: Vec<enums::FieldType>,
) -> bool {
    required_fields
        .0
        .get(&payment_method)
        .and_then(|pm_type| pm_type.0.get(&payment_method_type))
        .and_then(|required_fields_for_connector| {
            required_fields_for_connector.fields.get(&connector)
        })
        .map(|required_fields_final| {
            required_fields_final
                .non_mandate
                .iter()
                .flatten()
                .any(|field_info| required_field_type.contains(&field_info.field_type))
                || required_fields_final
                    .mandate
                    .iter()
                    .flatten()
                    .any(|field_info| required_field_type.contains(&field_info.field_type))
                || required_fields_final
                    .common
                    .iter()
                    .flatten()
                    .any(|field_info| required_field_type.contains(&field_info.field_type))
        })
        .unwrap_or(false)
}

fn build_apple_pay_session_request(
    state: &routes::SessionState,
    request: payment_types::ApplepaySessionRequest,
    apple_pay_merchant_cert: masking::Secret<String>,
    apple_pay_merchant_cert_key: masking::Secret<String>,
) -> RouterResult<services::Request> {
    let mut url = state.conf.connectors.applepay.base_url.to_owned();
    url.push_str("paymentservices/paymentSession");

    let session_request = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(url.as_str())
        .attach_default_headers()
        .headers(vec![(
            headers::CONTENT_TYPE.to_string(),
            "application/json".to_string().into(),
        )])
        .set_body(RequestContent::Json(Box::new(request)))
        .add_certificate(Some(apple_pay_merchant_cert))
        .add_certificate_key(Some(apple_pay_merchant_cert_key))
        .build();
    Ok(session_request)
}

async fn create_applepay_session_token(
    state: &routes::SessionState,
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
    business_profile: &domain::Profile,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
) -> RouterResult<types::PaymentsSessionRouterData> {
    let delayed_response = is_session_response_delayed(state, connector);
    if delayed_response {
        let delayed_response_apple_pay_session =
            Some(payment_types::ApplePaySessionResponse::NoSessionResponse);
        create_apple_pay_session_response(
            router_data,
            delayed_response_apple_pay_session,
            None, // Apple pay payment request will be none for delayed session response
            connector.connector_name.to_string(),
            delayed_response,
            payment_types::NextActionCall::Confirm,
            header_payload,
        )
    } else {
        // Get the apple pay metadata
        let apple_pay_metadata =
            helpers::get_applepay_metadata(router_data.connector_meta_data.clone())
                .attach_printable(
                    "Failed to to fetch apple pay certificates during session call",
                )?;

        // Get payment request data , apple pay session request and merchant keys
        let (
            payment_request_data,
            apple_pay_session_request_optional,
            apple_pay_merchant_cert,
            apple_pay_merchant_cert_key,
            apple_pay_merchant_identifier,
            merchant_business_country,
            merchant_configured_domain_optional,
        ) = match apple_pay_metadata {
            payment_types::ApplepaySessionTokenMetadata::ApplePayCombined(
                apple_pay_combined_metadata,
            ) => match apple_pay_combined_metadata {
                payment_types::ApplePayCombinedMetadata::Simplified {
                    payment_request_data,
                    session_token_data,
                } => {
                    logger::info!("Apple pay simplified flow");

                    let merchant_identifier = state
                        .conf
                        .applepay_merchant_configs
                        .get_inner()
                        .common_merchant_identifier
                        .clone()
                        .expose();

                    let merchant_business_country = session_token_data.merchant_business_country;

                    let apple_pay_session_request = get_session_request_for_simplified_apple_pay(
                        merchant_identifier.clone(),
                        session_token_data.clone(),
                    );

                    let apple_pay_merchant_cert = state
                        .conf
                        .applepay_decrypt_keys
                        .get_inner()
                        .apple_pay_merchant_cert
                        .clone();

                    let apple_pay_merchant_cert_key = state
                        .conf
                        .applepay_decrypt_keys
                        .get_inner()
                        .apple_pay_merchant_cert_key
                        .clone();

                    (
                        payment_request_data,
                        Ok(apple_pay_session_request),
                        apple_pay_merchant_cert,
                        apple_pay_merchant_cert_key,
                        merchant_identifier,
                        merchant_business_country,
                        Some(session_token_data.initiative_context),
                    )
                }
                payment_types::ApplePayCombinedMetadata::Manual {
                    payment_request_data,
                    session_token_data,
                } => {
                    logger::info!("Apple pay manual flow");

                    let apple_pay_session_request = get_session_request_for_manual_apple_pay(
                        session_token_data.clone(),
                        header_payload.x_merchant_domain.clone(),
                    );

                    let merchant_business_country = session_token_data.merchant_business_country;

                    (
                        payment_request_data,
                        apple_pay_session_request,
                        session_token_data.certificate.clone(),
                        session_token_data.certificate_keys,
                        session_token_data.merchant_identifier,
                        merchant_business_country,
                        session_token_data.initiative_context,
                    )
                }
            },
            payment_types::ApplepaySessionTokenMetadata::ApplePay(apple_pay_metadata) => {
                logger::info!("Apple pay manual flow");

                let apple_pay_session_request = get_session_request_for_manual_apple_pay(
                    apple_pay_metadata.session_token_data.clone(),
                    header_payload.x_merchant_domain.clone(),
                );

                let merchant_business_country = apple_pay_metadata
                    .session_token_data
                    .merchant_business_country;
                (
                    apple_pay_metadata.payment_request_data,
                    apple_pay_session_request,
                    apple_pay_metadata.session_token_data.certificate.clone(),
                    apple_pay_metadata
                        .session_token_data
                        .certificate_keys
                        .clone(),
                    apple_pay_metadata.session_token_data.merchant_identifier,
                    merchant_business_country,
                    apple_pay_metadata.session_token_data.initiative_context,
                )
            }
        };

        // Get amount info for apple pay
        let amount_info = get_apple_pay_amount_info(
            payment_request_data.label.as_str(),
            router_data.request.to_owned(),
        )?;

        let required_billing_contact_fields = if business_profile
            .always_collect_billing_details_from_wallet_connector
            .unwrap_or(false)
        {
            Some(payment_types::ApplePayBillingContactFields(vec![
                payment_types::ApplePayAddressParameters::PostalAddress,
            ]))
        } else if business_profile
            .collect_billing_details_from_wallet_connector
            .unwrap_or(false)
        {
            let billing_variants = enums::FieldType::get_billing_variants();
            is_dynamic_fields_required(
                &state.conf.required_fields,
                enums::PaymentMethod::Wallet,
                enums::PaymentMethodType::ApplePay,
                connector.connector_name,
                billing_variants,
            )
            .then_some(payment_types::ApplePayBillingContactFields(vec![
                payment_types::ApplePayAddressParameters::PostalAddress,
            ]))
        } else {
            None
        };

        let required_shipping_contact_fields = if business_profile
            .always_collect_shipping_details_from_wallet_connector
            .unwrap_or(false)
        {
            Some(payment_types::ApplePayShippingContactFields(vec![
                payment_types::ApplePayAddressParameters::PostalAddress,
                payment_types::ApplePayAddressParameters::Phone,
                payment_types::ApplePayAddressParameters::Email,
            ]))
        } else if business_profile
            .collect_shipping_details_from_wallet_connector
            .unwrap_or(false)
        {
            let shipping_variants = enums::FieldType::get_shipping_variants();
            is_dynamic_fields_required(
                &state.conf.required_fields,
                enums::PaymentMethod::Wallet,
                enums::PaymentMethodType::ApplePay,
                connector.connector_name,
                shipping_variants,
            )
            .then_some(payment_types::ApplePayShippingContactFields(vec![
                payment_types::ApplePayAddressParameters::PostalAddress,
                payment_types::ApplePayAddressParameters::Phone,
                payment_types::ApplePayAddressParameters::Email,
            ]))
        } else {
            None
        };

        // If collect_shipping_details_from_wallet_connector is false, we check if
        // collect_billing_details_from_wallet_connector is true. If it is, then we pass the Email and Phone in
        // ApplePayShippingContactFields as it is a required parameter and ApplePayBillingContactFields
        // does not contain Email and Phone.
        let required_shipping_contact_fields_updated = if required_billing_contact_fields.is_some()
            && required_shipping_contact_fields.is_none()
        {
            Some(payment_types::ApplePayShippingContactFields(vec![
                payment_types::ApplePayAddressParameters::Phone,
                payment_types::ApplePayAddressParameters::Email,
            ]))
        } else {
            required_shipping_contact_fields
        };

        // Get apple pay payment request
        let applepay_payment_request = get_apple_pay_payment_request(
            amount_info,
            payment_request_data,
            router_data.request.to_owned(),
            apple_pay_merchant_identifier.as_str(),
            merchant_business_country,
            required_billing_contact_fields,
            required_shipping_contact_fields_updated,
        )?;

        let apple_pay_session_response = match (
            header_payload.browser_name.clone(),
            header_payload.x_client_platform.clone(),
        ) {
            (Some(common_enums::BrowserName::Safari), Some(common_enums::ClientPlatform::Web))
            | (None, None) => {
                let apple_pay_session_request = apple_pay_session_request_optional
                    .attach_printable("Failed to obtain apple pay session request")?;
                let applepay_session_request = build_apple_pay_session_request(
                    state,
                    apple_pay_session_request.clone(),
                    apple_pay_merchant_cert.clone(),
                    apple_pay_merchant_cert_key.clone(),
                )?;

                let response = services::call_connector_api(
                    state,
                    applepay_session_request,
                    "create_apple_pay_session_token",
                )
                .await;

                let updated_response = match (
                    response.as_ref().ok(),
                    header_payload.x_merchant_domain.clone(),
                ) {
                    (Some(Err(error)), Some(_)) => {
                        logger::error!(
                            "Retry apple pay session call with the merchant configured domain {error:?}"
                        );
                        let merchant_configured_domain = merchant_configured_domain_optional
                            .get_required_value("apple pay domain")
                            .attach_printable("Failed to get domain for apple pay session call")?;
                        let apple_pay_retry_session_request =
                            payment_types::ApplepaySessionRequest {
                                initiative_context: merchant_configured_domain,
                                ..apple_pay_session_request
                            };
                        let applepay_retry_session_request = build_apple_pay_session_request(
                            state,
                            apple_pay_retry_session_request,
                            apple_pay_merchant_cert,
                            apple_pay_merchant_cert_key,
                        )?;
                        services::call_connector_api(
                            state,
                            applepay_retry_session_request,
                            "create_apple_pay_session_token",
                        )
                        .await
                    }
                    _ => response,
                };

                // logging the error if present in session call response
                log_session_response_if_error(&updated_response);
                updated_response
                    .ok()
                    .and_then(|apple_pay_res| {
                        apple_pay_res
                            .map(|res| {
                                let response: Result<
                                    payment_types::NoThirdPartySdkSessionResponse,
                                    Report<common_utils::errors::ParsingError>,
                                > = res.response.parse_struct("NoThirdPartySdkSessionResponse");

                                // logging the parsing failed error
                                if let Err(error) = response.as_ref() {
                                    logger::error!(?error);
                                };

                                response.ok()
                            })
                            .ok()
                    })
                    .flatten()
            }
            _ => {
                logger::debug!("Skipping apple pay session call based on the browser name");
                None
            }
        };

        let session_response =
            apple_pay_session_response.map(payment_types::ApplePaySessionResponse::NoThirdPartySdk);

        create_apple_pay_session_response(
            router_data,
            session_response,
            Some(applepay_payment_request),
            connector.connector_name.to_string(),
            delayed_response,
            payment_types::NextActionCall::Confirm,
            header_payload,
        )
    }
}

fn create_paze_session_token(
    router_data: &types::PaymentsSessionRouterData,
    _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
) -> RouterResult<types::PaymentsSessionRouterData> {
    let paze_wallet_details = router_data
        .connector_wallets_details
        .clone()
        .parse_value::<payment_types::PazeSessionTokenData>("PazeSessionTokenData")
        .change_context(errors::ConnectorError::NoConnectorWalletDetails)
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_wallets_details".to_string(),
            expected_format: "paze_metadata_format".to_string(),
        })?;
    let required_amount_type = StringMajorUnitForConnector;
    let transaction_currency_code = router_data.request.currency;
    let transaction_amount = required_amount_type
        .convert(router_data.request.minor_amount, transaction_currency_code)
        .change_context(errors::ApiErrorResponse::PreconditionFailed {
            message: "Failed to convert amount to string major unit for paze".to_string(),
        })?;
    Ok(types::PaymentsSessionRouterData {
        response: Ok(types::PaymentsResponseData::SessionResponse {
            session_token: payment_types::SessionToken::Paze(Box::new(
                payment_types::PazeSessionTokenResponse {
                    client_id: paze_wallet_details.data.client_id,
                    client_name: paze_wallet_details.data.client_name,
                    client_profile_id: paze_wallet_details.data.client_profile_id,
                    transaction_currency_code,
                    transaction_amount,
                    email_address: router_data.request.email.clone(),
                },
            )),
        }),
        ..router_data.clone()
    })
}

fn create_samsung_pay_session_token(
    router_data: &types::PaymentsSessionRouterData,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
) -> RouterResult<types::PaymentsSessionRouterData> {
    let samsung_pay_session_token_data = router_data
        .connector_wallets_details
        .clone()
        .parse_value::<payment_types::SamsungPaySessionTokenData>("SamsungPaySessionTokenData")
        .change_context(errors::ConnectorError::NoConnectorWalletDetails)
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_wallets_details".to_string(),
            expected_format: "samsung_pay_metadata_format".to_string(),
        })?;

    let required_amount_type = StringMajorUnitForConnector;
    let samsung_pay_amount = required_amount_type
        .convert(
            router_data.request.minor_amount,
            router_data.request.currency,
        )
        .change_context(errors::ApiErrorResponse::PreconditionFailed {
            message: "Failed to convert amount to string major unit for Samsung Pay".to_string(),
        })?;

    let merchant_domain = match header_payload.x_client_platform {
        Some(common_enums::ClientPlatform::Web) => Some(
            header_payload
                .x_merchant_domain
                .get_required_value("samsung pay domain")
                .attach_printable("Failed to get domain for samsung pay session call")?,
        ),
        _ => None,
    };

    let samsung_pay_wallet_details = match samsung_pay_session_token_data.data {
        payment_types::SamsungPayCombinedMetadata::MerchantCredentials(
            samsung_pay_merchant_credentials,
        ) => samsung_pay_merchant_credentials,
        payment_types::SamsungPayCombinedMetadata::ApplicationCredentials(
            _samsung_pay_application_credentials,
        ) => Err(errors::ApiErrorResponse::NotSupported {
            message: "Samsung Pay decryption flow with application credentials is not implemented"
                .to_owned(),
        })?,
    };

    let formatted_payment_id = router_data.payment_id.replace("_", "-");

    Ok(types::PaymentsSessionRouterData {
        response: Ok(types::PaymentsResponseData::SessionResponse {
            session_token: payment_types::SessionToken::SamsungPay(Box::new(
                payment_types::SamsungPaySessionTokenResponse {
                    version: "2".to_string(),
                    service_id: samsung_pay_wallet_details.service_id,
                    order_number: formatted_payment_id,
                    merchant_payment_information:
                        payment_types::SamsungPayMerchantPaymentInformation {
                            name: samsung_pay_wallet_details.merchant_display_name,
                            url: merchant_domain,
                            country_code: samsung_pay_wallet_details.merchant_business_country,
                        },
                    amount: payment_types::SamsungPayAmountDetails {
                        amount_format: payment_types::SamsungPayAmountFormat::FormatTotalPriceOnly,
                        currency_code: router_data.request.currency,
                        total_amount: samsung_pay_amount,
                    },
                    protocol: payment_types::SamsungPayProtocolType::Protocol3ds,
                    allowed_brands: samsung_pay_wallet_details.allowed_brands,
                },
            )),
        }),
        ..router_data.clone()
    })
}

fn get_session_request_for_simplified_apple_pay(
    apple_pay_merchant_identifier: String,
    session_token_data: payment_types::SessionTokenForSimplifiedApplePay,
) -> payment_types::ApplepaySessionRequest {
    payment_types::ApplepaySessionRequest {
        merchant_identifier: apple_pay_merchant_identifier,
        display_name: "Apple pay".to_string(),
        initiative: "web".to_string(),
        initiative_context: session_token_data.initiative_context,
    }
}

fn get_session_request_for_manual_apple_pay(
    session_token_data: payment_types::SessionTokenInfo,
    merchant_domain: Option<String>,
) -> RouterResult<payment_types::ApplepaySessionRequest> {
    let initiative_context = merchant_domain
        .or_else(|| session_token_data.initiative_context.clone())
        .get_required_value("apple pay domain")
        .attach_printable("Failed to get domain for apple pay session call")?;

    Ok(payment_types::ApplepaySessionRequest {
        merchant_identifier: session_token_data.merchant_identifier.clone(),
        display_name: session_token_data.display_name.clone(),
        initiative: session_token_data.initiative.to_string(),
        initiative_context,
    })
}

fn get_apple_pay_amount_info(
    label: &str,
    session_data: types::PaymentsSessionData,
) -> RouterResult<payment_types::AmountInfo> {
    let required_amount_type = StringMajorUnitForConnector;
    let apple_pay_amount = required_amount_type
        .convert(session_data.minor_amount, session_data.currency)
        .change_context(errors::ApiErrorResponse::PreconditionFailed {
            message: "Failed to convert amount to string major unit for applePay".to_string(),
        })?;
    let amount_info = payment_types::AmountInfo {
        label: label.to_string(),
        total_type: Some("final".to_string()),
        amount: apple_pay_amount,
    };

    Ok(amount_info)
}

fn get_apple_pay_payment_request(
    amount_info: payment_types::AmountInfo,
    payment_request_data: payment_types::PaymentRequestMetadata,
    session_data: types::PaymentsSessionData,
    merchant_identifier: &str,
    merchant_business_country: Option<api_models::enums::CountryAlpha2>,
    required_billing_contact_fields: Option<payment_types::ApplePayBillingContactFields>,
    required_shipping_contact_fields: Option<payment_types::ApplePayShippingContactFields>,
) -> RouterResult<payment_types::ApplePayPaymentRequest> {
    let applepay_payment_request = payment_types::ApplePayPaymentRequest {
        country_code: merchant_business_country.or(session_data.country).ok_or(
            errors::ApiErrorResponse::MissingRequiredField {
                field_name: "country_code",
            },
        )?,
        currency_code: session_data.currency,
        total: amount_info,
        merchant_capabilities: Some(payment_request_data.merchant_capabilities),
        supported_networks: Some(payment_request_data.supported_networks),
        merchant_identifier: Some(merchant_identifier.to_string()),
        required_billing_contact_fields,
        required_shipping_contact_fields,
        recurring_payment_request: session_data.apple_pay_recurring_details,
    };
    Ok(applepay_payment_request)
}

fn create_apple_pay_session_response(
    router_data: &types::PaymentsSessionRouterData,
    session_response: Option<payment_types::ApplePaySessionResponse>,
    apple_pay_payment_request: Option<payment_types::ApplePayPaymentRequest>,
    connector_name: String,
    delayed_response: bool,
    next_action: payment_types::NextActionCall,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
) -> RouterResult<types::PaymentsSessionRouterData> {
    match session_response {
        Some(response) => Ok(types::PaymentsSessionRouterData {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: payment_types::SessionToken::ApplePay(Box::new(
                    payment_types::ApplepaySessionTokenResponse {
                        session_token_data: Some(response),
                        payment_request_data: apple_pay_payment_request,
                        connector: connector_name,
                        delayed_session_token: delayed_response,
                        sdk_next_action: { payment_types::SdkNextAction { next_action } },
                        connector_reference_id: None,
                        connector_sdk_public_key: None,
                        connector_merchant_id: None,
                    },
                )),
            }),
            ..router_data.clone()
        }),
        None => {
            match (
                header_payload.browser_name,
                header_payload.x_client_platform,
            ) {
                (
                    Some(common_enums::BrowserName::Safari),
                    Some(common_enums::ClientPlatform::Web),
                )
                | (None, None) => Ok(types::PaymentsSessionRouterData {
                    response: Ok(types::PaymentsResponseData::SessionResponse {
                        session_token: payment_types::SessionToken::NoSessionTokenReceived,
                    }),
                    ..router_data.clone()
                }),
                _ => Ok(types::PaymentsSessionRouterData {
                    response: Ok(types::PaymentsResponseData::SessionResponse {
                        session_token: payment_types::SessionToken::ApplePay(Box::new(
                            payment_types::ApplepaySessionTokenResponse {
                                session_token_data: None,
                                payment_request_data: apple_pay_payment_request,
                                connector: connector_name,
                                delayed_session_token: delayed_response,
                                sdk_next_action: { payment_types::SdkNextAction { next_action } },
                                connector_reference_id: None,
                                connector_sdk_public_key: None,
                                connector_merchant_id: None,
                            },
                        )),
                    }),
                    ..router_data.clone()
                }),
            }
        }
    }
}

fn create_gpay_session_token(
    state: &routes::SessionState,
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
    business_profile: &domain::Profile,
) -> RouterResult<types::PaymentsSessionRouterData> {
    // connector_wallet_details is being parse into admin types to check specifically if google_pay field is present
    // this is being done because apple_pay details from metadata is also being filled into connector_wallets_details
    let connector_wallets_details = router_data
        .connector_wallets_details
        .clone()
        .parse_value::<admin_types::ConnectorWalletDetails>("ConnectorWalletDetails")
        .change_context(errors::ConnectorError::NoConnectorWalletDetails)
        .attach_printable(format!(
            "cannot parse connector_wallets_details from the given value {:?}",
            router_data.connector_wallets_details
        ))
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_wallets_details".to_string(),
            expected_format: "admin_types_connector_wallets_details_format".to_string(),
        })?;
    let connector_metadata = router_data.connector_meta_data.clone();
    let delayed_response = is_session_response_delayed(state, connector);

    if delayed_response {
        Ok(types::PaymentsSessionRouterData {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: payment_types::SessionToken::GooglePay(Box::new(
                    payment_types::GpaySessionTokenResponse::ThirdPartyResponse(
                        payment_types::GooglePayThirdPartySdk {
                            delayed_session_token: true,
                            connector: connector.connector_name.to_string(),
                            sdk_next_action: payment_types::SdkNextAction {
                                next_action: payment_types::NextActionCall::Confirm,
                            },
                        },
                    ),
                )),
            }),
            ..router_data.clone()
        })
    } else {
        let always_collect_billing_details_from_wallet_connector = business_profile
            .always_collect_billing_details_from_wallet_connector
            .unwrap_or(false);

        let is_billing_details_required = if always_collect_billing_details_from_wallet_connector {
            always_collect_billing_details_from_wallet_connector
        } else if business_profile
            .collect_billing_details_from_wallet_connector
            .unwrap_or(false)
        {
            let billing_variants = enums::FieldType::get_billing_variants();
            is_dynamic_fields_required(
                &state.conf.required_fields,
                enums::PaymentMethod::Wallet,
                enums::PaymentMethodType::GooglePay,
                connector.connector_name,
                billing_variants,
            )
        } else {
            false
        };

        let required_amount_type = StringMajorUnitForConnector;
        let google_pay_amount = required_amount_type
            .convert(
                router_data.request.minor_amount,
                router_data.request.currency,
            )
            .change_context(errors::ApiErrorResponse::PreconditionFailed {
                message: "Failed to convert amount to string major unit for googlePay".to_string(),
            })?;
        let session_data = router_data.request.clone();
        let transaction_info = payment_types::GpayTransactionInfo {
            country_code: session_data.country.unwrap_or_default(),
            currency_code: router_data.request.currency,
            total_price_status: "Final".to_string(),
            total_price: google_pay_amount,
        };

        let always_collect_shipping_details_from_wallet_connector = business_profile
            .always_collect_shipping_details_from_wallet_connector
            .unwrap_or(false);

        let required_shipping_contact_fields =
            if always_collect_shipping_details_from_wallet_connector {
                true
            } else if business_profile
                .collect_shipping_details_from_wallet_connector
                .unwrap_or(false)
            {
                let shipping_variants = enums::FieldType::get_shipping_variants();

                is_dynamic_fields_required(
                    &state.conf.required_fields,
                    enums::PaymentMethod::Wallet,
                    enums::PaymentMethodType::GooglePay,
                    connector.connector_name,
                    shipping_variants,
                )
            } else {
                false
            };

        if connector_wallets_details.google_pay.is_some() {
            let gpay_data = router_data
                .connector_wallets_details
                .clone()
                .parse_value::<payment_types::GooglePayWalletDetails>("GooglePayWalletDetails")
                .change_context(errors::ConnectorError::NoConnectorWalletDetails)
                .attach_printable(format!(
                    "cannot parse gpay connector_wallets_details from the given value {:?}",
                    router_data.connector_wallets_details
                ))
                .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "connector_wallets_details".to_string(),
                    expected_format: "gpay_connector_wallets_details_format".to_string(),
                })?;

            let payment_types::GooglePayProviderDetails::GooglePayMerchantDetails(gpay_info) =
                gpay_data.google_pay.provider_details.clone();

            let gpay_allowed_payment_methods = get_allowed_payment_methods_from_cards(
                gpay_data,
                &gpay_info.merchant_info.tokenization_specification,
                is_billing_details_required,
            )?;

            Ok(types::PaymentsSessionRouterData {
                response: Ok(types::PaymentsResponseData::SessionResponse {
                    session_token: payment_types::SessionToken::GooglePay(Box::new(
                        payment_types::GpaySessionTokenResponse::GooglePaySession(
                            payment_types::GooglePaySessionResponse {
                                merchant_info: payment_types::GpayMerchantInfo {
                                    merchant_name: gpay_info.merchant_info.merchant_name,
                                    merchant_id: gpay_info.merchant_info.merchant_id,
                                },
                                allowed_payment_methods: vec![gpay_allowed_payment_methods],
                                transaction_info,
                                connector: connector.connector_name.to_string(),
                                sdk_next_action: payment_types::SdkNextAction {
                                    next_action: payment_types::NextActionCall::Confirm,
                                },
                                delayed_session_token: false,
                                secrets: None,
                                shipping_address_required: required_shipping_contact_fields,
                                // We pass Email as a required field irrespective of
                                // collect_billing_details_from_wallet_connector or
                                // collect_shipping_details_from_wallet_connector as it is common to both.
                                email_required: required_shipping_contact_fields
                                    || is_billing_details_required,
                                shipping_address_parameters:
                                    api_models::payments::GpayShippingAddressParameters {
                                        phone_number_required: required_shipping_contact_fields,
                                    },
                            },
                        ),
                    )),
                }),
                ..router_data.clone()
            })
        } else {
            let billing_address_parameters = is_billing_details_required.then_some(
                payment_types::GpayBillingAddressParameters {
                    phone_number_required: is_billing_details_required,
                    format: payment_types::GpayBillingAddressFormat::FULL,
                },
            );

            let gpay_data = connector_metadata
                .clone()
                .parse_value::<payment_types::GpaySessionTokenData>("GpaySessionTokenData")
                .change_context(errors::ConnectorError::NoConnectorMetaData)
                .attach_printable(format!(
                    "cannot parse gpay metadata from the given value {connector_metadata:?}"
                ))
                .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "connector_metadata".to_string(),
                    expected_format: "gpay_metadata_format".to_string(),
                })?;

            let gpay_allowed_payment_methods = gpay_data
                .data
                .allowed_payment_methods
                .into_iter()
                .map(
                    |allowed_payment_methods| payment_types::GpayAllowedPaymentMethods {
                        parameters: payment_types::GpayAllowedMethodsParameters {
                            billing_address_required: Some(is_billing_details_required),
                            billing_address_parameters: billing_address_parameters.clone(),
                            ..allowed_payment_methods.parameters
                        },
                        ..allowed_payment_methods
                    },
                )
                .collect();

            Ok(types::PaymentsSessionRouterData {
                response: Ok(types::PaymentsResponseData::SessionResponse {
                    session_token: payment_types::SessionToken::GooglePay(Box::new(
                        payment_types::GpaySessionTokenResponse::GooglePaySession(
                            payment_types::GooglePaySessionResponse {
                                merchant_info: gpay_data.data.merchant_info,
                                allowed_payment_methods: gpay_allowed_payment_methods,
                                transaction_info,
                                connector: connector.connector_name.to_string(),
                                sdk_next_action: payment_types::SdkNextAction {
                                    next_action: payment_types::NextActionCall::Confirm,
                                },
                                delayed_session_token: false,
                                secrets: None,
                                shipping_address_required: required_shipping_contact_fields,
                                // We pass Email as a required field irrespective of
                                // collect_billing_details_from_wallet_connector or
                                // collect_shipping_details_from_wallet_connector as it is common to both.
                                email_required: required_shipping_contact_fields
                                    || is_billing_details_required,
                                shipping_address_parameters:
                                    api_models::payments::GpayShippingAddressParameters {
                                        phone_number_required: required_shipping_contact_fields,
                                    },
                            },
                        ),
                    )),
                }),
                ..router_data.clone()
            })
        }
    }
}

/// Card Type for Google Pay Allowerd Payment Methods
pub(crate) const CARD: &str = "CARD";

fn get_allowed_payment_methods_from_cards(
    gpay_info: payment_types::GooglePayWalletDetails,
    gpay_token_specific_data: &payment_types::GooglePayTokenizationSpecification,
    is_billing_details_required: bool,
) -> RouterResult<payment_types::GpayAllowedPaymentMethods> {
    let billing_address_parameters =
        is_billing_details_required.then_some(payment_types::GpayBillingAddressParameters {
            phone_number_required: is_billing_details_required,
            format: payment_types::GpayBillingAddressFormat::FULL,
        });

    let protocol_version: Option<String> = gpay_token_specific_data
        .parameters
        .public_key
        .as_ref()
        .map(|_| PROTOCOL.to_string());

    Ok(payment_types::GpayAllowedPaymentMethods {
        parameters: payment_types::GpayAllowedMethodsParameters {
            billing_address_required: Some(is_billing_details_required),
            billing_address_parameters: billing_address_parameters.clone(),
            ..gpay_info.google_pay.cards
        },
        payment_method_type: CARD.to_string(),
        tokenization_specification: payment_types::GpayTokenizationSpecification {
            token_specification_type: gpay_token_specific_data.tokenization_type.to_string(),
            parameters: payment_types::GpayTokenParameters {
                protocol_version,
                public_key: gpay_token_specific_data.parameters.public_key.clone(),
                gateway: gpay_token_specific_data.parameters.gateway.clone(),
                gateway_merchant_id: gpay_token_specific_data
                    .parameters
                    .gateway_merchant_id
                    .clone()
                    .expose_option(),
                stripe_publishable_key: gpay_token_specific_data
                    .parameters
                    .stripe_publishable_key
                    .clone()
                    .expose_option(),
                stripe_version: gpay_token_specific_data
                    .parameters
                    .stripe_version
                    .clone()
                    .expose_option(),
            },
        },
    })
}

fn is_session_response_delayed(
    state: &routes::SessionState,
    connector: &api::ConnectorData,
) -> bool {
    let connectors_with_delayed_response = &state
        .conf
        .delayed_session_response
        .connectors_with_delayed_session_response;

    connectors_with_delayed_response.contains(&connector.connector_name)
}

fn log_session_response_if_error(
    response: &Result<Result<types::Response, types::Response>, Report<errors::ApiClientError>>,
) {
    if let Err(error) = response.as_ref() {
        logger::error!(?error);
    };
    response
        .as_ref()
        .ok()
        .map(|res| res.as_ref().map_err(|error| logger::error!(?error)));
}

#[async_trait]
pub trait RouterDataSession
where
    Self: Sized,
{
    async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a routes::SessionState,
        connector: &api::ConnectorData,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        business_profile: &domain::Profile,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<Self>;
}

fn create_paypal_sdk_session_token(
    _state: &routes::SessionState,
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
    _business_profile: &domain::Profile,
) -> RouterResult<types::PaymentsSessionRouterData> {
    let connector_metadata = router_data.connector_meta_data.clone();

    let paypal_sdk_data = connector_metadata
        .clone()
        .parse_value::<payment_types::PaypalSdkSessionTokenData>("PaypalSdkSessionTokenData")
        .change_context(errors::ConnectorError::NoConnectorMetaData)
        .attach_printable(format!(
            "cannot parse paypal_sdk metadata from the given value {connector_metadata:?}"
        ))
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_metadata".to_string(),
            expected_format: "paypal_sdk_metadata_format".to_string(),
        })?;

    Ok(types::PaymentsSessionRouterData {
        response: Ok(types::PaymentsResponseData::SessionResponse {
            session_token: payment_types::SessionToken::Paypal(Box::new(
                payment_types::PaypalSessionTokenResponse {
                    connector: connector.connector_name.to_string(),
                    session_token: paypal_sdk_data.data.client_id,
                    sdk_next_action: payment_types::SdkNextAction {
                        next_action: payment_types::NextActionCall::PostSessionTokens,
                    },
                },
            )),
        }),
        ..router_data.clone()
    })
}

#[async_trait]
impl RouterDataSession for types::PaymentsSessionRouterData {
    async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a routes::SessionState,
        connector: &api::ConnectorData,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        business_profile: &domain::Profile,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<Self> {
        match connector.get_token {
            api::GetToken::GpayMetadata => {
                create_gpay_session_token(state, self, connector, business_profile)
            }
            api::GetToken::SamsungPayMetadata => {
                create_samsung_pay_session_token(self, header_payload)
            }
            api::GetToken::ApplePayMetadata => {
                create_applepay_session_token(
                    state,
                    self,
                    connector,
                    business_profile,
                    header_payload,
                )
                .await
            }
            api::GetToken::PaypalSdkMetadata => {
                create_paypal_sdk_session_token(state, self, connector, business_profile)
            }
            api::GetToken::PazeMetadata => create_paze_session_token(self, header_payload),
            api::GetToken::Connector => {
                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                    api::Session,
                    types::PaymentsSessionData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();
                let resp = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    self,
                    call_connector_action,
                    None,
                )
                .await
                .to_payment_failed_response()?;

                Ok(resp)
            }
        }
    }
}
