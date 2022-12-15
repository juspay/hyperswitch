use api_models::payments as payment_types;
use async_trait::async_trait;
use error_stack::ResultExt;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        payments::{self, transformers, PaymentData},
    },
    routes, services,
    types::{
        self, api,
        storage::{self, enums},
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
        state: &routes::AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::PaymentsSessionRouterData> {
        transformers::construct_payment_router_data::<api::Session, types::PaymentsSessionData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
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
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _storage_schema: enums::MerchantStorageScheme,
    ) -> RouterResult<Self> {
        self.decide_flow(
            state,
            connector,
            customer,
            Some(true),
            call_connector_action,
        )
        .await
    }
}

fn create_gpay_session_token(
    router_data: &types::PaymentsSessionRouterData,
) -> RouterResult<types::PaymentsSessionRouterData> {
    let connector_metadata = router_data.connector_meta_data.clone();

    let gpay_data = error_stack::ResultExt::change_context(
        connector_metadata
            .clone()
            .parse_value::<payment_types::GpaySessionTokenData>("GpaySessionTokenData"),
        errors::ConnectorError::NoConnectorMetaData,
    )
    .change_context(errors::ApiErrorResponse::InvalidDataFormat {
        field_name: "connector_metadata".to_string(),
        expected_format: "gpay metadata format".to_string(),
    })
    .attach_printable(format!(
        "cannnot parse gpay metadata from the given value {:?}",
        connector_metadata
    ))?;

    let session_data = router_data.request.clone();
    let transaction_info = payment_types::GpayTransactionInfo {
        country_code: session_data.country.unwrap_or_else(|| "US".to_string()),
        currency_code: router_data.request.currency.to_string(),
        total_price_status: "Final".to_string(),
        total_price: router_data.request.amount,
    };

    let response_router_data = types::PaymentsSessionRouterData {
        response: Ok(types::PaymentsResponseData::SessionResponse {
            session_token: payment_types::SessionToken::Gpay {
                allowed_payment_methods: gpay_data.gpay.allowed_payment_methods,
                transaction_info,
            },
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
        _customer: &Option<storage::Customer>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<types::PaymentsSessionRouterData> {
        match connector.connector_type {
            api::ConnectorType::SessionTokenFromMetadata => create_gpay_session_token(self),
            api::ConnectorType::NormalFlow => {
                let connector_integration: services::BoxedConnectorIntegration<
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
