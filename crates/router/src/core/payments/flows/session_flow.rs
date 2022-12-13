use async_trait::async_trait;
use error_stack::ResultExt;
use masking::{Deserialize, Serialize};

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
        let router = transformers::construct_payment_router_data::<
            api::Session,
            types::PaymentsSessionData,
        >(state, self.clone(), connector_id, merchant_account)
        .await;

        let mut router_info = router?;
        if connector_id == "applepay" {
            let cert = "applepay_cert";
            let cert_value = state
                .store
                .get_key(cert)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to obtain certificate")?;

            let cert_key = "applepay_key";
            let cert_key_value = state
                .store
                .get_key(cert_key)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to obtain certificate key")?;

            let applepay_session = "applepay_session";
            let session_value: SessionObject = crate::db::get_and_deserialize_key(
                &*state.store,
                applepay_session,
                "SessionObject",
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

            let initiative_context = session_value
                .initiative_context
                .get_required_value("initiative_context")?;

            let display_name = session_value
                .display_name
                .get_required_value("display_name")?;

            let merchant_identifier = session_value
                .merchant_identifier
                .get_required_value("merchant_identifier")?;

            let initiative = session_value.initiative.get_required_value("initiative")?;

            router_info.request.certificate = Some(cert_value);
            router_info.request.certificate_keys = Some(cert_key_value);
            router_info.request.requestor_domain = Some(initiative_context);
            router_info.request.display_name = Some(display_name);
            router_info.request.merchant_identifier = Some(merchant_identifier);
            router_info.request.initiative = Some(initiative);
        };

        Ok(router_info)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionObject {
    pub initiative_context: Option<String>,
    pub merchant_identifier: Option<String>,
    pub display_name: Option<String>,
    pub initiative: Option<String>,
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

impl types::PaymentsSessionRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a routes::AppState,
        connector: &api::ConnectorData,
        _customer: &Option<storage::Customer>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<types::PaymentsSessionRouterData> {
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
