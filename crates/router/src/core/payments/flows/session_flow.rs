use api_models::payments as payment_types;
use async_trait::async_trait;
use common_utils::{ext_traits::ByteSliceExt, request::RequestContent};
use error_stack::{Report, ResultExt};
use masking::ExposeInterface;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
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
        domain, storage,
    },
    utils::OptionExt,
};

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
        ))
        .await
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
        business_profile: &storage::business_profile::BusinessProfile,
    ) -> RouterResult<Self> {
        metrics::SESSION_TOKEN_CREATED.add(
            &metrics::CONTEXT,
            1,
            &[metrics::request::add_attributes(
                "connector",
                connector.connector_name.to_string(),
            )],
        );
        self.decide_flow(
            state,
            connector,
            Some(true),
            call_connector_action,
            business_profile,
        )
        .await
    }

    async fn add_access_token<'a>(
        &self,
        state: &routes::SessionState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }
}

/// This function checks if for a given connector, payment_method and payment_method_type,
/// the list of required_field_type is present in dynamic fields
fn is_dynamic_fields_required(
    required_fields: &settings::RequiredFields,
    payment_method: enums::PaymentMethod,
    payment_method_type: enums::PaymentMethodType,
    connector: &types::Connector,
    required_field_type: Vec<enums::FieldType>,
) -> bool {
    required_fields
        .0
        .get(&payment_method)
        .and_then(|pm_type| pm_type.0.get(&payment_method_type))
        .and_then(|required_fields_for_connector| {
            required_fields_for_connector.fields.get(connector)
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
    business_profile: &storage::business_profile::BusinessProfile,
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
        )
    } else {
        // Get the apple pay metadata
        let apple_pay_metadata =
            helpers::get_applepay_metadata(router_data.connector_meta_data.clone())?;

        // Get payment request data , apple pay session request and merchant keys
        let (
            payment_request_data,
            apple_pay_session_request,
            apple_pay_merchant_cert,
            apple_pay_merchant_cert_key,
            merchant_business_country,
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
                        merchant_identifier,
                        session_token_data,
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
                        apple_pay_session_request,
                        apple_pay_merchant_cert,
                        apple_pay_merchant_cert_key,
                        merchant_business_country,
                    )
                }
                payment_types::ApplePayCombinedMetadata::Manual {
                    payment_request_data,
                    session_token_data,
                } => {
                    logger::info!("Apple pay manual flow");

                    let apple_pay_session_request =
                        get_session_request_for_manual_apple_pay(session_token_data.clone());

                    let merchant_business_country = session_token_data.merchant_business_country;

                    (
                        payment_request_data,
                        apple_pay_session_request,
                        session_token_data.certificate.clone(),
                        session_token_data.certificate_keys,
                        merchant_business_country,
                    )
                }
            },
            payment_types::ApplepaySessionTokenMetadata::ApplePay(apple_pay_metadata) => {
                logger::info!("Apple pay manual flow");

                let apple_pay_session_request = get_session_request_for_manual_apple_pay(
                    apple_pay_metadata.session_token_data.clone(),
                );

                let merchant_business_country = apple_pay_metadata
                    .session_token_data
                    .merchant_business_country;
                (
                    apple_pay_metadata.payment_request_data,
                    apple_pay_session_request,
                    apple_pay_metadata.session_token_data.certificate.clone(),
                    apple_pay_metadata.session_token_data.certificate_keys,
                    merchant_business_country,
                )
            }
        };

        // Get amount info for apple pay
        let amount_info = get_apple_pay_amount_info(
            payment_request_data.label.as_str(),
            router_data.request.to_owned(),
        )?;

        let billing_variants = enums::FieldType::get_billing_variants();

        let required_billing_contact_fields = is_dynamic_fields_required(
            &state.conf.required_fields,
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::ApplePay,
            &connector.connector_name,
            billing_variants,
        )
        .then_some(payment_types::ApplePayBillingContactFields(vec![
            payment_types::ApplePayAddressParameters::PostalAddress,
        ]));

        let required_shipping_contact_fields =
            if business_profile.collect_shipping_details_from_wallet_connector == Some(true) {
                let shipping_variants = enums::FieldType::get_shipping_variants();

                is_dynamic_fields_required(
                    &state.conf.required_fields,
                    enums::PaymentMethod::Wallet,
                    enums::PaymentMethodType::ApplePay,
                    &connector.connector_name,
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

        // Get apple pay payment request
        let applepay_payment_request = get_apple_pay_payment_request(
            amount_info,
            payment_request_data,
            router_data.request.to_owned(),
            apple_pay_session_request.merchant_identifier.as_str(),
            merchant_business_country,
            required_billing_contact_fields,
            required_shipping_contact_fields,
        )?;

        let applepay_session_request = build_apple_pay_session_request(
            state,
            apple_pay_session_request,
            apple_pay_merchant_cert,
            apple_pay_merchant_cert_key,
        )?;
        let response = services::call_connector_api(
            state,
            applepay_session_request,
            "create_apple_pay_session_token",
        )
        .await;

        // logging the error if present in session call response
        log_session_response_if_error(&response);

        let apple_pay_session_response = response
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
            .flatten();

        let session_response =
            apple_pay_session_response.map(payment_types::ApplePaySessionResponse::NoThirdPartySdk);

        create_apple_pay_session_response(
            router_data,
            session_response,
            Some(applepay_payment_request),
            connector.connector_name.to_string(),
            delayed_response,
            payment_types::NextActionCall::Confirm,
        )
    }
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
) -> payment_types::ApplepaySessionRequest {
    payment_types::ApplepaySessionRequest {
        merchant_identifier: session_token_data.merchant_identifier.clone(),
        display_name: session_token_data.display_name.clone(),
        initiative: session_token_data.initiative.clone(),
        initiative_context: session_token_data.initiative_context,
    }
}

fn get_apple_pay_amount_info(
    label: &str,
    session_data: types::PaymentsSessionData,
) -> RouterResult<payment_types::AmountInfo> {
    let amount_info = payment_types::AmountInfo {
        label: label.to_string(),
        total_type: Some("final".to_string()),
        amount: session_data
            .currency
            .to_currency_base_unit(session_data.amount)
            .change_context(errors::ApiErrorResponse::PreconditionFailed {
                message: "Failed to convert currency to base unit".to_string(),
            })?,
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
) -> RouterResult<types::PaymentsSessionRouterData> {
    match session_response {
        Some(response) => Ok(types::PaymentsSessionRouterData {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: payment_types::SessionToken::ApplePay(Box::new(
                    payment_types::ApplepaySessionTokenResponse {
                        session_token_data: response,
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
        None => Ok(types::PaymentsSessionRouterData {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: payment_types::SessionToken::NoSessionTokenReceived,
            }),
            ..router_data.clone()
        }),
    }
}

fn create_gpay_session_token(
    state: &routes::SessionState,
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
    business_profile: &storage::business_profile::BusinessProfile,
) -> RouterResult<types::PaymentsSessionRouterData> {
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

        let billing_variants = enums::FieldType::get_billing_variants();

        let is_billing_details_required = is_dynamic_fields_required(
            &state.conf.required_fields,
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::GooglePay,
            &connector.connector_name,
            billing_variants,
        );

        let billing_address_parameters =
            is_billing_details_required.then_some(payment_types::GpayBillingAddressParameters {
                phone_number_required: is_billing_details_required,
                format: payment_types::GpayBillingAddressFormat::FULL,
            });

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

        let session_data = router_data.request.clone();
        let transaction_info = payment_types::GpayTransactionInfo {
            country_code: session_data.country.unwrap_or_default(),
            currency_code: router_data.request.currency,
            total_price_status: "Final".to_string(),
            total_price: router_data
                .request
                .currency
                .to_currency_base_unit(router_data.request.amount)
                .attach_printable(
                    "Cannot convert given amount to base currency denomination".to_string(),
                )
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "amount",
                })?,
        };

        let required_shipping_contact_fields =
            if business_profile.collect_shipping_details_from_wallet_connector == Some(true) {
                let shipping_variants = enums::FieldType::get_shipping_variants();

                is_dynamic_fields_required(
                    &state.conf.required_fields,
                    enums::PaymentMethod::Wallet,
                    enums::PaymentMethodType::GooglePay,
                    &connector.connector_name,
                    shipping_variants,
                )
            } else {
                false
            };

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
                            email_required: required_shipping_contact_fields,
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
        business_profile: &storage::business_profile::BusinessProfile,
    ) -> RouterResult<Self>;
}

fn create_paypal_sdk_session_token(
    _state: &routes::SessionState,
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
    _business_profile: &storage::business_profile::BusinessProfile,
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
                        next_action: payment_types::NextActionCall::Confirm,
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
        business_profile: &storage::business_profile::BusinessProfile,
    ) -> RouterResult<Self> {
        match connector.get_token {
            api::GetToken::GpayMetadata => {
                create_gpay_session_token(state, self, connector, business_profile)
            }
            api::GetToken::ApplePayMetadata => {
                create_applepay_session_token(state, self, connector, business_profile).await
            }
            api::GetToken::PaypalSdkMetadata => {
                create_paypal_sdk_session_token(state, self, connector, business_profile)
            }
            api::GetToken::Connector => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
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
