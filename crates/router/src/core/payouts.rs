pub mod validator;

use api_models::enums as api_enums;
use diesel_models::enums as storage_enums;
use error_stack::{report, ResultExt};
use masking::Secret;
use router_env::{instrument, logger, tracing};
use serde_json::{self};

use super::errors::StorageErrorExt;
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
        domain, storage,
        transformers::{ForeignFrom, ForeignInto},
    },
    utils::{self, OptionExt},
};

// ********************************************** CORE FLOWS **********************************************

#[instrument(skip_all)]
pub async fn payouts_create_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    // TODO: Remove hardcoded connector
    let connector_name = api_enums::Connector::Adyen;

    // 1. Validate and insert in DB
    let payout_create =
        validate_request_and_form_payout_create(state, &merchant_account, &req, &connector_name)
            .await
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid data passed".to_string(),
            })
            .attach_printable("Failed to validate and form PayoutCreate entry")
            .map_or(storage::payout_create::PayoutCreate::default(), |pc| pc);

    // 2. Form connector data
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name.to_string(),
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    // 3. Eligibility flow
    let payouts = check_payout_eligibility(
        state,
        &merchant_account,
        &key_store,
        &req,
        &connector_data,
        &payout_create,
    )
    .await
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "Eligibility failed".to_string(),
    })
    .attach_printable("Eligibility failed for given Payout request")
    .map_or(storage::payouts::Payouts::default(), |up| up);

    // 4. Payout creation flow
    let _creation_response = create_payout(
        state,
        &merchant_account,
        &key_store,
        &req,
        &connector_data,
        &payout_create,
    )
    .await
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "Payout creation failed".to_string(),
    })
    .attach_printable("Payout creation failed for given Payout request")
    .map_or(storage::payouts::Payouts::default(), |up| up);

    // 5. Auto fulfillment flow
    let _fulfillment_response = fulfill_payout(
        state,
        &merchant_account,
        &key_store,
        &req,
        &connector_data,
        &payout_create,
    )
    .await
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "Payout fulfillment failed".to_string(),
    })
    .attach_printable("Payout fulfillment failed for given Payout request")
    .map_or(storage::payouts::Payouts::default(), |up| up);

    // 6. Send back response
    Ok(services::ApplicationResponse::Json(
        api::PayoutCreateResponse {
            payout_id: payout_create.payout_id,
            merchant_id: merchant_account.merchant_id.clone(),
            amount: payout_create.amount,
            currency: api_enums::Currency::foreign_from(payout_create.destination_currency),
            connector: Some(payouts.connector),
            payout_type: api_enums::PayoutType::foreign_from(payout_create.payout_type),
            billing: req.billing,
            customer_id: payout_create.customer_id,
            auto_fulfilled: req.auto_fulfilled.unwrap_or(false),
            email: req.email,
            name: req.name,
            phone: req.phone,
            phone_country_code: req.phone_country_code,
            client_secret: None, // FIXME: Add client secret
            return_url: payout_create.return_url,
            business_country: req.business_country,
            business_label: req.business_label,
            description: payout_create.description,
            entity_type: api_enums::EntityType::foreign_from(payout_create.entity_type),
            recurring: payout_create.recurring,
            metadata: payout_create.metadata,
            status: api_enums::PayoutStatus::foreign_from(payouts.status),
        },
    ))
}

pub async fn payouts_retrieve_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    req: payouts::PayoutRetrieveRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let (merchant_id, payout_id, payout, payout_create);
    let db = &*state.store;

    payout_id = req.payout_id;
    merchant_id = &merchant_account.merchant_id;
    payout_create = db
        .find_payout_create_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    payout = db
        .find_payout_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;

    Ok(services::ApplicationResponse::Json(
        api::PayoutCreateResponse {
            payout_id: payout_create.payout_id,
            merchant_id: merchant_account.merchant_id.clone(),
            amount: payout_create.amount,
            customer_id: payout_create.customer_id,
            currency: api_enums::Currency::foreign_from(payout_create.destination_currency),
            connector: Some(payout.connector),
            payout_type: api_enums::PayoutType::foreign_from(payout_create.payout_type),
            billing: None, // FIXME: Store + Fetch from DB
            auto_fulfilled: payout_create.auto_fulfill,
            email: Some(Secret::new("".to_string())), // FIXME: Store + Fetch from DB
            name: Some(Secret::new("".to_string())),  // FIXME: Store + Fetch from DB
            phone: Some(Secret::new("".to_string())), // FIXME: Store + Fetch from DB
            phone_country_code: Some("".to_string()), // FIXME: Store + Fetch from DB
            client_secret: None,
            return_url: payout_create.return_url,
            business_country: None, // FIXME: Fetch from MCA
            business_label: None,   // FIXME: Fetch from MCA
            description: payout_create.description,
            entity_type: api_enums::EntityType::foreign_from(payout_create.entity_type),
            recurring: payout_create.recurring,
            metadata: payout_create.metadata,
            status: api_enums::PayoutStatus::foreign_from(payout.status),
        },
    ))
}

// ********************************************** HELPERS **********************************************

pub async fn validate_request_and_form_payout_create(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    connector_name: &api_enums::Connector,
) -> RouterResult<storage::payout_create::PayoutCreate> {
    let db = &*state.store;
    let (address_id, customer_id, payout_id, currency, payout_create_req, payouts_req);

    // Create address_id FIXME: Handle addresses
    address_id = utils::generate_id(consts::ID_LENGTH, "addr");

    // Create customer_id if not passed in request
    customer_id = core_utils::get_or_generate_id("customer_id", &req.customer_id, "cust")?;

    // Create payout_id if not passed in request
    payout_id = core_utils::get_or_generate_id("payout_id", &req.payout_id, "payout")?;

    let predicate = req
        .merchant_id
        .as_ref()
        .map(|merchant_id| merchant_id != &merchant_account.merchant_id);

    utils::when(predicate.unwrap_or(false), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "merchant_id".to_string(),
            expected_format: "merchant_id from merchant account".to_string(),
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
            payout_id.to_owned(),
            &merchant_account.merchant_id
        )
    })? {
        Some(payout) => payout,
        None => {
            currency = req
                .currency
                .to_owned()
                .map(ForeignInto::foreign_into)
                .get_required_value("currency")?;
            payouts_req = storage::PayoutsNew::default()
                .set_payout_id(payout_id.to_owned())
                .set_customer_id(customer_id.to_owned())
                .set_merchant_id(merchant_account.merchant_id.to_owned())
                .set_address_id(address_id.to_owned())
                .set_connector(connector_name.to_string())
                .set_payout_method_data(Some(
                    serde_json::to_value(&req.payout_method_data)
                        .unwrap_or(serde_json::Value::Null),
                ))
                .to_owned();
            payout_create_req = storage::PayoutCreateNew::default()
                .set_payout_id(payout_id.to_owned())
                .set_merchant_id(merchant_account.merchant_id.to_owned())
                .set_customer_id(customer_id.to_owned())
                .set_address_id(address_id.to_owned())
                .set_payout_type(req.payout_type.foreign_into())
                .set_payout_method_data(Some(
                    serde_json::to_value(&req.payout_method_data)
                        .unwrap_or(serde_json::Value::Null),
                ))
                .set_amount(req.amount.unwrap_or(api::Amount::Zero).into())
                .set_destination_currency(currency)
                .set_source_currency(currency)
                .set_description(req.description.to_owned().unwrap_or("".to_string()))
                .set_recurring(req.recurring.unwrap_or(false))
                .set_auto_fulfill(req.auto_fulfilled.unwrap_or(false))
                .set_return_url(req.return_url.to_owned())
                .set_entity_type(storage_enums::EntityType::foreign_from(
                    req.entity_type.unwrap_or(api_enums::EntityType::default()),
                ))
                .set_metadata(req.metadata.to_owned())
                .set_created_at(Some(common_utils::date_time::now()))
                .set_last_modified_at(Some(common_utils::date_time::now()))
                .to_owned();

            db.insert_payout(payouts_req)
                .await
                .to_duplicate_response(errors::ApiErrorResponse::DuplicateRefundRequest)
                .attach_printable("Error inserting payouts in db")?;

            db.insert_payout_create(payout_create_req)
                .await
                .to_duplicate_response(errors::ApiErrorResponse::DuplicateRefundRequest)
                .attach_printable("Error inserting payout_create in db")?
        }
    };

    Ok(payout)
}

pub async fn check_payout_eligibility(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::ConnectorData,
    payout_create: &storage::payout_create::PayoutCreate,
) -> RouterResult<storage::payouts::Payouts> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        payout_create,
        None,
        req,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PEligibility,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get connector response")?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let updated_payouts = match router_data_resp.response {
        Ok(payout_response_data) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id.unwrap_or_default(),
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payment_create in db")?
        }
        Err(err) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
            };
            logger::debug!("PAYOUT ERR {:?}", updated_payouts);
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payment_create in db")?
        }
    };

    Ok(updated_payouts)
}

pub async fn create_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::ConnectorData,
    payout_create: &storage::payout_create::PayoutCreate,
) -> RouterResult<storage::payouts::Payouts> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        payout_create,
        None,
        req,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PCreate,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get connector response")?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let updated_payouts = match router_data_resp.response {
        Ok(payout_response_data) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id.unwrap_or_default(),
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payment_create in db")?
        }
        Err(err) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
            };
            logger::debug!("PAYOUT ERR {:?}", updated_payouts);
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payment_create in db")?
        }
    };

    Ok(updated_payouts)
}

pub async fn fulfill_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::ConnectorData,
    payout_create: &storage::payout_create::PayoutCreate,
) -> RouterResult<storage::payouts::Payouts> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        payout_create,
        None,
        req,
    )
    .await?;

    // 2. Fetch connector integration details
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PFulfill,
        types::PayoutsData,
        types::PayoutsResponseData,
    > = connector_data.connector.get_connector_integration();

    // 3. Call connector service
    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get connector response")?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let updated_payouts = match router_data_resp.response {
        Ok(payout_response_data) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id.unwrap_or_default(),
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payment_create in db")?
        }
        Err(err) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
            };
            logger::debug!("PAYOUT ERR {:?}", updated_payouts);
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payment_create in db")?
        }
    };

    Ok(updated_payouts)
}
