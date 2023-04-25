pub mod validator;

use api_models::enums as api_enums;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
use storage_models::enums as storage_enums;

use crate::{
    consts,
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments, utils as core_utils,
    },
    routes::AppState,
    services,
    types::{
        self,
        api::{self, payouts},
        storage,
        transformers::{ForeignFrom, ForeignInto},
    },
    utils::{self, OptionExt},
};

use super::errors::StorageErrorExt;

#[instrument(skip_all)]
pub async fn payouts_create_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    // Validate and insert in DB
    let payout_create = validate_and_form_payout_create(state, &merchant_account, &req)
        .await
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid data passed".to_string(),
        })
        .attach_printable("Failed to validate and form PayoutCreate entry")
        .map_or(storage::payout_create::PayoutCreate::default(), |pc| pc);

    //eligibility flow
    let connector_name = api_enums::Connector::Adyen;
    //move to diff fn
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_name.to_string(),
        &merchant_account,
        &payout_create,
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

    let db = &*state.store;
    let updated_payout_create = match router_data_resp.response {
        Ok(payout_response_data) => {
            let payout = storage::PayoutsNew {
                connector_payout_id: payout_response_data.connector_payout_id.unwrap_or_default(),
                ..Default::default()
            };
            db.insert_payout(payout)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error inserting payout in db")?;
            let updated_payout_create =
                storage::payout_create::PayoutCreateUpdate::CreationUpdate {
                    status: payout_response_data.status,
                    error_code: None,
                    error_message: None,
                };
            db.update_payout_create_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payout_create,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payment_create in db")?
        }
        Err(err) => {
            let updated_payout_create =
                storage::payout_create::PayoutCreateUpdate::CreationUpdate {
                    status: storage_enums::PayoutStatus::Failed,
                    error_code: Some(err.code),
                    error_message: Some(err.message),
                };
            db.update_payout_create_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payout_create,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payment_create in db")?
        }
    };

    // Response
    Ok(services::ApplicationResponse::Json(
        api::PayoutCreateResponse {
            payout_id: updated_payout_create.payout_id,
            merchant_id: merchant_account.merchant_id.clone(),
            amount: req.amount,
            currency: req.currency,
            connector: Some(updated_payout_create.connector),
            payout_type: req.payout_type,
            billing: req.billing,
            customer_id: Some(updated_payout_create.customer_id),
            auto_fulfilled: Some(req.auto_fulfilled.unwrap_or(false)),
            email: req.email,
            name: req.name,
            phone: req.phone,
            phone_country_code: req.phone_country_code,
            client_secret: None,
            return_url: req.return_url,
            business_country: req.business_country,
            business_label: req.business_label,
            description: req.description,
            entity_type: req.entity_type,
            recurring: req.recurring,
            metadata: req.metadata,
            status: api_enums::PayoutStatus::foreign_from(updated_payout_create.status),
        },
    ))
}

pub async fn validate_and_form_payout_create(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
) -> RouterResult<storage::payout_create::PayoutCreate> {
    let db = &*state.store;
    let (payout_id, currency, payout_create_req, payout);

    // Create payout_id if not passed in request
    payout_id = core_utils::get_or_generate_id("payout_id", &req.payout_id, "payout")?;

    let predicate = req
        .merchant_id
        .as_ref()
        .map(|merchant_id| merchant_id != &merchant_account.merchant_id);

    utils::when(predicate.unwrap_or(false), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "merchant_id".to_string(),
            expected_format: "merchant_id from merchant account".to_string()
        })
        .attach_printable("invalid merchant_id in request"))
    })?;

    let payout = match validator::validate_uniqueness_of_payout_id_against_merchant_id(
        db,
        &payout_id,
        &merchant_account.merchant_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "Unique violation while checking payout_id: {} against merchant_id: {}",
            payout_id.clone(),
            &merchant_account.merchant_id
        )
    })? {
        Some(payout) => payout,
        None => {
            currency = req
                .currency
                .clone()
                .map(ForeignInto::foreign_into)
                .get_required_value("currency")?;
            payout_create_req = storage::PayoutCreateNew::default()
                .set_payout_id(payout_id.clone())
                .set_merchant_id(merchant_account.merchant_id.clone())
                .set_customer_id(
                    req.customer_id
                        .clone()
                        .unwrap_or(utils::generate_id(consts::ID_LENGTH, "cust")),
                )
                .set_address_id(utils::generate_id(consts::ID_LENGTH, "addr"))
                .set_payout_type(req.payout_type.clone().foreign_into())
                .set_amount(req.amount.clone().unwrap().into())
                .set_destination_currency(currency)
                .set_source_currency(currency)
                .set_description(req.description.clone().unwrap_or("".to_string()))
                .set_created_at(Some(common_utils::date_time::now()))
                .set_modified_at(Some(common_utils::date_time::now()))
                .set_metadata(req.metadata.clone())
                .set_recurring(req.recurring.clone().unwrap_or(false))
                .to_owned();

            payout = db
                .insert_payout_create(payout_create_req)
                .await
                .to_duplicate_response(errors::ApiErrorResponse::DuplicateRefundRequest)?;
            payout

            // TODO: add function to trigger gateway eligibility
        }
    };

    Ok(payout)
}
