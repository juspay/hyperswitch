use api_models::payments as payment_types;
use async_trait::async_trait;
use common_utils::ext_traits::ByteSliceExt;
use error_stack::{report, ResultExt};

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    connector,
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, transformers, PaymentData},
    },
    headers,
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

fn mk_applepay_session_request(
    state: &routes::AppState,
    router_data: &types::PaymentsSessionRouterData,
) -> RouterResult<(services::Request, payment_types::ApplepaySessionTokenData)> {
    let connector_metadata = router_data.connector_meta_data.clone();

    let applepay_metadata = connector_metadata
        .parse_value::<payment_types::ApplepaySessionTokenData>("ApplepaySessionTokenData")
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_metadata".to_string(),
            expected_format: "applepay_metadata_format".to_string(),
        })?;
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
            "application/json".to_string().into(),
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
            applepay_metadata
                .data
                .session_token_data
                .certificate_keys
                .clone(),
        ))
        .build();
    Ok((session_request, applepay_metadata))
}

async fn create_applepay_session_token(
    state: &routes::AppState,
    router_data: &types::PaymentsSessionRouterData,
    connector: &api::ConnectorData,
) -> RouterResult<types::PaymentsSessionRouterData> {
    let (applepay_session_request, applepay_metadata) =
        mk_applepay_session_request(state, router_data)?;
    let response = services::call_connector_api(state, applepay_session_request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failure in calling connector api")?;
    let session_response: payment_types::ApplePaySessionResponse = match response {
        Ok(resp) => resp
            .response
            .parse_struct("ApplePaySessionResponse")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse ApplePaySessionResponse struct"),
        Err(err) => {
            let error_response: payment_types::ApplepayErrorResponse = err
                .response
                .parse_struct("ApplepayErrorResponse")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse ApplepayErrorResponse struct")?;
            Err(
                report!(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
                    "Failed with {} status code and the error response is {:?}",
                    err.status_code, error_response
                )),
            )
        }
    }?;

    let amount_info = payment_types::AmountInfo {
        label: applepay_metadata.data.payment_request_data.label,
        total_type: "final".to_string(),
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
        merchant_identifier: applepay_metadata
            .data
            .session_token_data
            .merchant_identifier,
    };

    let response_router_data = types::PaymentsSessionRouterData {
        response: Ok(types::PaymentsResponseData::SessionResponse {
            session_token: payment_types::SessionToken::ApplePay(Box::new(
                payment_types::ApplepaySessionTokenResponse {
                    session_token_data: session_response,
                    payment_request_data: applepay_payment_request,
                    connector: connector.connector_name.to_string(),
                },
            )),
        }),
        ..router_data.clone()
    };

    Ok(response_router_data)
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
        }),
        ..router_data.clone()
    };

    Ok(response_router_data)
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
