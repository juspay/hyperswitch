use api_models::enums;
use common_utils::ext_traits::AsyncExt;
use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, RouterResponse},
        payments, utils as core_utils,
    },
    routes::AppState,
    services,
    types::{
        self,
        api::{self, payouts},
        storage,
    },
};

#[instrument(skip_all)]
pub async fn payout_create_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_create = storage::PayoutCreateNew::default();
    let pc = db
        .insert_payout_create(payout_create)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error inserting payment_create in db")?;
    //if eligible
    let connector_name = enums::Connector::Adyen;
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_name.to_string(),
        &merchant_account,
        &pc,
        None,
        &req,
    )
    .await?;

    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name.to_string(),
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector")?;

    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::Payout,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector.connector.get_connector_integration();

    services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await?
}
