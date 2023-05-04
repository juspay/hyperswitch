use api_models::enums;
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use storage_models::enums as storage_enums;

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
        transformers::ForeignFrom,
    },
};

#[instrument(skip_all)]
pub async fn payout_create_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let db = &*state.store;
    let _merchant_id = &merchant_account.merchant_id;
    let payout_create = storage::PayoutCreateNew::default();
    let pc = db
        .insert_payout_create(payout_create)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error inserting payment_create in db")?;
    //eligibility flow
    let connector_name = enums::Connector::Adyen;
    //move to diff fn
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

    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get connector response")?;

    let updated_payout_create = match router_data_resp.response {
        Ok(payout_response_data) => {
            let payout = storage::PayoutsNew {
                connector_payout_id: payout_response_data.connector_payout_id.unwrap_or_default(),
                ..Default::default()
            };
            db.insert_payouts(payout)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error inserting payout in db")?;
            let updated_payout = storage::PayoutCreateUpdate::Update {
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
            };
            db.update_payout_create_by_payout_id(pc, updated_payout)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payment_create in db")?
        }
        Err(err) => {
            let updated_payout = storage::PayoutCreateUpdate::Update {
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
            };
            db.update_payout_create_by_payout_id(pc, updated_payout)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payment_create in db")?
        }
    };

    Ok(services::ApplicationResponse::Json(
        payouts::PayoutCreateResponse {
            payout_id: updated_payout_create.payout_id,
            amount: req.amount,
            currency: None,
            connector: Some(updated_payout_create.connector),
            status: enums::PayoutStatus::foreign_from(updated_payout_create.status),
            created: Some(updated_payout_create.created_at),
            customer_id: Some(updated_payout_create.customer_id),
            billing: req.billing,
            email: req.email,
            name: req.name,
            phone_country_code: req.phone_country_code,
            phone: req.phone,
            client_secret: None,
            return_url: req.return_url,
        },
    ))
}
