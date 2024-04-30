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
    routes::{self, metrics},
    services,
    types::{self, api, domain},
    utils::OptionExt,
};

#[async_trait]
impl
    ConstructFlowSpecificData<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for PaymentData<api::Session>
{
    async fn construct_router_data<'a>(
        &self,
        state: &routes::AppState,
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
        state: &routes::AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        _connector_request: Option<services::Request>,
    ) -> RouterResult<Self> {
        metrics::SESSION_TOKEN_CREATED.add(
            &metrics::CONTEXT,
            1,
            &[metrics::request::add_attributes(
                "connector",
                connector.connector_name.to_string(),
            )],
        );
        self.decide_flow(state, connector, Some(true), call_connector_action)
            .await
    }

    async fn add_access_token<'a>(
        &self,
        state: &routes::AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }
}

fn get_applepay_metadata(
    connector_metadata: Option<common_utils::pii::SecretSerdeValue>,
) -> RouterResult<payment_types::ApplepaySessionTokenMetadata> {
    connector_metadata
        .clone()
        .parse_value::<api_models::payments::ApplepayCombinedSessionTokenData>(
            "ApplepayCombinedSessionTokenData",
        )
        .map(|combined_metadata| {
            api_models::payments::ApplepaySessionTokenMetadata::ApplePayCombined(
                combined_metadata.apple_pay_combined,
            )
        })
        .or_else(|_| {
            connector_metadata
                .parse_value::<api_models::payments::ApplepaySessionTokenData>(
                    "ApplepaySessionTokenData",
                )
                .map(|old_metadata| {
                    api_models::payments::ApplepaySessionTokenMetadata::ApplePay(
                        old_metadata.apple_pay,
                    )
                })
        })
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_metadata".to_string(),
            expected_format: "applepay_metadata_format".to_string(),
        })
}

fn build_apple_pay_session_request(
    state: &routes::AppState,
    request: payment_types::ApplepaySessionRequest,
    apple_pay_merchant_cert: String,
    apple_pay_merchant_cert_key: String,
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
    state: &routes::AppState,
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
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
        let apple_pay_metadata = get_applepay_metadata(router_data.connector_meta_data.clone())?;

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
                        .clone()
                        .expose();

                    let apple_pay_merchant_cert_key = state
                        .conf
                        .applepay_decrypt_keys
                        .get_inner()
                        .apple_pay_merchant_cert_key
                        .clone()
                        .expose();

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

        // Get apple pay payment request
        let applepay_payment_request = get_apple_pay_payment_request(
            amount_info,
            payment_request_data,
            router_data.request.to_owned(),
            apple_pay_session_request.merchant_identifier.as_str(),
            merchant_business_country,
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
    state: &routes::AppState,
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
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

        Ok(types::PaymentsSessionRouterData {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: payment_types::SessionToken::GooglePay(Box::new(
                    payment_types::GpaySessionTokenResponse::GooglePaySession(
                        payment_types::GooglePaySessionResponse {
                            merchant_info: gpay_data.data.merchant_info,
                            allowed_payment_methods: gpay_data.data.allowed_payment_methods,
                            transaction_info,
                            connector: connector.connector_name.to_string(),
                            sdk_next_action: payment_types::SdkNextAction {
                                next_action: payment_types::NextActionCall::Confirm,
                            },
                            delayed_session_token: false,
                            secrets: None,
                        },
                    ),
                )),
            }),
            ..router_data.clone()
        })
    }
}

fn is_session_response_delayed(state: &routes::AppState, connector: &api::ConnectorData) -> bool {
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

impl types::PaymentsSessionRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a routes::AppState,
        connector: &api::ConnectorData,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<Self> {
        match connector.get_token {
            api::GetToken::GpayMetadata => create_gpay_session_token(state, self, connector),
            api::GetToken::ApplePayMetadata => {
                create_applepay_session_token(state, self, connector).await
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
