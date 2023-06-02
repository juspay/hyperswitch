use api_models::payments as payment_types;
use async_trait::async_trait;
use common_utils::ext_traits::ByteSliceExt;
use error_stack::{Report, ResultExt};

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    connector,
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, transformers, PaymentData},
    },
    headers, logger,
    routes::{self, metrics},
    services,
    types::{self, api, domain},
    utils::{self, OptionExt},
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
        customer: &Option<domain::Customer>,
    ) -> RouterResult<types::PaymentsSessionRouterData> {
        transformers::construct_payment_router_data::<api::Session, types::PaymentsSessionData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            customer,
        )
        .await
    }
}

#[async_trait]
impl Feature<api::Session, types::PaymentsSessionData> for types::PaymentsSessionRouterData {
    async fn decide_flows<'a>(
        self,
        state: &routes::AppState,
        connector: &api::ConnectorData,
        customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _merchant_account: &domain::MerchantAccount,
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
            customer,
            Some(true),
            call_connector_action,
        )
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
) -> RouterResult<payment_types::ApplepaySessionTokenData> {
    connector_metadata
        .parse_value::<payment_types::ApplepaySessionTokenData>("ApplepaySessionTokenData")
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_metadata".to_string(),
            expected_format: "applepay_metadata_format".to_string(),
        })
}

fn mk_applepay_session_request(
    state: &routes::AppState,
    router_data: &types::PaymentsSessionRouterData,
) -> RouterResult<services::Request> {
    let applepay_metadata = get_applepay_metadata(router_data.connector_meta_data.clone())?;
    let request = payment_types::ApplepaySessionRequest {
        merchant_identifier: applepay_metadata
            .data
            .session_token_data
            .merchant_identifier
            .clone(),
        display_name: applepay_metadata
            .data
            .session_token_data
            .display_name
            .clone(),
        initiative: applepay_metadata.data.session_token_data.initiative.clone(),
        initiative_context: applepay_metadata
            .data
            .session_token_data
            .initiative_context
            .clone(),
    };

    let applepay_session_request =
        utils::Encode::<payment_types::ApplepaySessionRequest>::encode_to_string_of_json(&request)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode ApplePay session request to a string of json")?;

    let mut url = state.conf.connectors.applepay.base_url.to_owned();
    url.push_str("paymentservices/paymentSession");

    let session_request = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(url.as_str())
        .attach_default_headers()
        .headers(vec![(
            headers::CONTENT_TYPE.to_string(),
            "application/json".to_string(),
        )])
        .body(Some(applepay_session_request))
        .add_certificate(Some(
            applepay_metadata
                .data
                .session_token_data
                .certificate
                .clone(),
        ))
        .add_certificate_key(Some(
            applepay_metadata.data.session_token_data.certificate_keys,
        ))
        .build();
    Ok(session_request)
}

async fn create_applepay_session_token(
    state: &routes::AppState,
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
) -> RouterResult<types::PaymentsSessionRouterData> {
    let delayed_response = &state
        .conf
        .delayed_session_response
        .connectors_with_delayed_session_response;

    let connector_name = connector.connector_name;
    let applepay_metadata = get_applepay_metadata(router_data.connector_meta_data.clone())?;

    let amount_info = payment_types::AmountInfo {
        label: applepay_metadata.data.payment_request_data.label,
        total_type: Some("final".to_string()),
        amount: connector::utils::to_currency_base_unit(
            router_data.request.amount,
            router_data.request.currency,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert currency to base unit")?,
    };

    let applepay_payment_request = payment_types::ApplePayPaymentRequest {
        country_code: router_data
            .request
            .country
            .to_owned()
            .get_required_value("country_code")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "country_code",
            })?,
        currency_code: router_data.request.currency.to_string(),
        total: amount_info,
        merchant_capabilities: applepay_metadata
            .data
            .payment_request_data
            .merchant_capabilities,
        supported_networks: applepay_metadata
            .data
            .payment_request_data
            .supported_networks,
        merchant_identifier: Some(
            applepay_metadata
                .data
                .session_token_data
                .merchant_identifier,
        ),
    };

    let delayed_response = delayed_response.contains(&connector_name);

    if delayed_response {
        let delayed_response_apple_pay_session =
            payment_types::ApplePaySessionResponse::NoSessionResponse;
        create_apple_pay_session_response(
            router_data,
            delayed_response_apple_pay_session,
            None,
            connector_name.to_string(),
            delayed_response,
            payment_types::NextActionCall::SessionToken,
            None,
        )
    } else {
        let applepay_session_request = mk_applepay_session_request(state, router_data)?;
        let response = services::call_connector_api(state, applepay_session_request).await;
        log_session_response_if_error(&response);

        let session_response = response
            .ok()
            .and_then(|apple_pay_res| {
                apple_pay_res
                    .map(|res| {
                        let response: Result<
                            payment_types::NoThirdPartySdkSessionResponse,
                            Report<common_utils::errors::ParsingError>,
                        > = res.response.parse_struct("NoThirdPartySdkSessionResponse");
                        response.ok()
                    })
                    .ok()
            })
            .flatten();

        create_apple_pay_session_response(
            router_data,
            payment_types::ApplePaySessionResponse::NoThirdPartySdk(session_response),
            Some(applepay_payment_request),
            connector_name.to_string(),
            delayed_response,
            payment_types::NextActionCall::Confirm,
            None,
        )
    }
}

fn create_apple_pay_session_response(
    router_data: &types::PaymentsSessionRouterData,
    session_response: payment_types::ApplePaySessionResponse,
    apple_pay_payment_request: Option<payment_types::ApplePayPaymentRequest>,
    connector_name: String,
    delayed_response: bool,
    next_action: payment_types::NextActionCall,
    response_id: Option<String>,
) -> RouterResult<types::PaymentsSessionRouterData> {
    Ok(types::PaymentsSessionRouterData {
        response: Ok(types::PaymentsResponseData::SessionResponse {
            session_token: payment_types::SessionToken::ApplePay(Box::new(
                payment_types::ApplepaySessionTokenResponse {
                    session_token_data: session_response,
                    payment_request_data: apple_pay_payment_request,
                    connector: connector_name,
                    delayed_session_token: delayed_response,
                    sdk_next_action: { payment_types::SdkNextAction { next_action } },
                },
            )),
            response_id,
        }),
        ..router_data.clone()
    })
}

fn create_gpay_session_token(
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
) -> RouterResult<types::PaymentsSessionRouterData> {
    let connector_metadata = router_data.connector_meta_data.clone();

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
        currency_code: router_data.request.currency.to_string(),
        total_price_status: "Final".to_string(),
        total_price: utils::to_currency_base_unit(
            router_data.request.amount,
            router_data.request.currency,
        )
        .attach_printable("Cannot convert given amount to base currency denomination".to_string())
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "amount",
        })?,
    };

    let response_router_data = types::PaymentsSessionRouterData {
        response: Ok(types::PaymentsResponseData::SessionResponse {
            session_token: payment_types::SessionToken::GooglePay(Box::new(
                payment_types::GpaySessionTokenResponse {
                    merchant_info: gpay_data.data.merchant_info,
                    allowed_payment_methods: gpay_data.data.allowed_payment_methods,
                    transaction_info,
                    connector: connector.connector_name.to_string(),
                },
            )),
            response_id: None,
        }),
        ..router_data.clone()
    };

    Ok(response_router_data)
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
        _customer: &Option<domain::Customer>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<Self> {
        match connector.get_token {
            api::GetToken::GpayMetadata => create_gpay_session_token(self, connector),
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
                )
                .await
                .map_err(|error| error.to_payment_failed_response())?;

                Ok(resp)
            }
        }
    }
}
