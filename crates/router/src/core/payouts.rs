pub mod helpers;
pub mod validator;

use api_models::enums as api_enums;
use common_utils::crypto::Encryptable;
use diesel_models::enums as storage_enums;
use error_stack::{report, ResultExt};
use masking::{PeekInterface, Secret};
use router_env::{instrument, tracing};
use serde_json::{self};

use super::errors::{ConnectorErrorExt, StorageErrorExt};
use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments::{self, helpers as payment_helpers},
        utils as core_utils,
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

    // Validate and insert in DB
    let (payout_create, payouts, billing_address) = validate_request_and_form_payout_create(
        state,
        &merchant_account,
        &key_store,
        &req,
        &connector_name,
    )
    .await
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "Invalid data passed".to_string(),
    })
    .attach_printable("Failed to validate and form PayoutCreate and Payouts entry")
    .map_or(
        (
            storage::payout_create::PayoutCreate::default(),
            storage::payouts::Payouts::default(),
            None,
        ),
        |(pc, p, ba)| (pc, p, ba),
    );

    // Form connector data
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name.to_string(),
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    call_connector_payout(
        state,
        &merchant_account,
        &key_store,
        &req,
        connector_data,
        &payout_create,
        payouts,
        &billing_address,
    )
    .await
}

pub async fn payouts_update_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let db = &*state.store;

    let payout_id = req.payout_id.clone().get_required_value("payout_id")?;
    let merchant_id = &merchant_account.merchant_id;
    let mut payout_create = db
        .find_payout_create_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    let payouts = db
        .find_payout_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;

    let updated_payout_create = storage::PayoutCreateUpdate::Update {
        amount: req.amount.unwrap_or(payout_create.amount.into()).into(),
        destination_currency: req
            .currency
            .unwrap_or(payout_create.destination_currency.foreign_into())
            .foreign_into(),
        source_currency: req
            .currency
            .unwrap_or(payout_create.source_currency.foreign_into())
            .foreign_into(),
        description: req.description.clone().or(payout_create.description),
        recurring: req.recurring.unwrap_or(payout_create.recurring),
        auto_fulfill: req.auto_fulfill.unwrap_or(payout_create.auto_fulfill),
        return_url: req.return_url.clone().or(payout_create.return_url),
        entity_type: req
            .entity_type
            .unwrap_or(payout_create.entity_type.foreign_into())
            .foreign_into(),
        metadata: req.metadata.clone().or(payout_create.metadata),
        last_modified_at: Some(common_utils::date_time::now()),
    };

    payout_create = db
        .update_payout_create_by_merchant_id_payout_id(
            merchant_id,
            &payout_id,
            updated_payout_create,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payout_create")?;

    let billing_address = payment_helpers::get_address_for_payment_request(
        db,
        None,
        Some(&payouts.to_owned().address_id),
        merchant_id,
        Some(&payouts.to_owned().customer_id),
        &key_store,
    )
    .await?;

    // Form connector data
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &payouts.connector,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    call_connector_payout(
        state,
        &merchant_account,
        &key_store,
        &req,
        connector_data,
        &payout_create,
        payouts,
        &billing_address,
    )
    .await
}

#[instrument(skip_all)]
pub async fn payouts_retrieve_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutRetrieveRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let (billing_address, merchant_id, payout_id, payouts, payout_create);
    let db = &*state.store;

    payout_id = req.payout_id.to_owned();
    merchant_id = &merchant_account.merchant_id;
    payout_create = db
        .find_payout_create_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    payouts = db
        .find_payout_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    billing_address = payment_helpers::get_address_for_payment_request(
        db,
        None,
        Some(&payouts.to_owned().address_id),
        merchant_id,
        Some(&payouts.to_owned().customer_id),
        &key_store,
    )
    .await?;

    response_handler(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutRetrieveRequest(req),
        &payout_create,
        &payouts,
        &billing_address,
    )
    .await
}

#[instrument(skip_all)]
pub async fn payouts_cancel_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutActionRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let (billing_address, merchant_id, payout_id, payouts, payout_create);
    let db = &*state.store;

    payout_id = req.payout_id.to_owned();
    merchant_id = &merchant_account.merchant_id;
    payout_create = db
        .find_payout_create_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    payouts = db
        .find_payout_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    billing_address = payment_helpers::get_address_for_payment_request(
        db,
        None,
        Some(&payouts.to_owned().address_id),
        merchant_id,
        Some(&payouts.to_owned().customer_id),
        &key_store,
    )
    .await?;

    // TODO: Add connector integration
    response_handler(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req),
        &payout_create,
        &payouts,
        &billing_address,
    )
    .await
}

#[instrument(skip_all)]
pub async fn payouts_fulfill_core(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutActionRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let (merchant_id, payout_id, mut payouts, payout_create, billing_address);
    let db = &*state.store;

    payout_id = req.payout_id.to_owned();
    merchant_id = &merchant_account.merchant_id;

    // Fetch details from DB
    payout_create = db
        .find_payout_create_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    payouts = db
        .find_payout_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    billing_address = payment_helpers::get_address_for_payment_request(
        db,
        None,
        Some(&payouts.to_owned().address_id),
        merchant_id,
        Some(&payouts.to_owned().customer_id),
        &key_store,
    )
    .await?;

    // Verify if fulfillment can be triggered
    // TODO: Add function for determining terminal state

    // TODO: Remove hardcoded connector
    let connector_name = api_enums::Connector::Adyen;

    // Form connector data
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name.to_string(),
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    // Trigger fulfillment
    let pmd = payouts::PayoutMethodData::default(); // TODO: Fetch from locker
    payouts = fulfill_payout(
        state,
        &merchant_account,
        &key_store,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
        &connector_data,
        &payout_create,
        &payouts,
        &pmd,
        &billing_address,
    )
    .await
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "Payout fulfillment failed".to_string(),
    })
    .attach_printable("Payout fulfillment failed for given Payout request")?;

    response_handler(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
        &payout_create,
        &payouts,
        &billing_address,
    )
    .await
}

// ********************************************** HELPERS **********************************************
pub async fn call_connector_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: api::ConnectorData,
    payout_create: &storage::PayoutCreate,
    mut payouts: storage::Payouts,
    billing_address: &Option<domain::Address>,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_method_data = helpers::make_payout_data(state, req, payout_create).await?;
    payouts = match req.create_payout {
        Some(true) => {
            let pmd = payout_method_data
                .clone()
                .get_required_value("payout_method_data")?;

            // Eligibility flow
            if payout_create.payout_type == storage_enums::PayoutType::Card {
                payouts = check_payout_eligibility(
                    state,
                    merchant_account,
                    key_store,
                    req,
                    &connector_data,
                    payout_create,
                    &payouts,
                    &pmd,
                    billing_address,
                )
                .await
                .change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Eligibility failed".to_string(),
                })
                .attach_printable("Eligibility failed for given Payout request")?;
            }

            // Payout creation flow
            utils::when(!payouts.is_eligible.unwrap_or(true), || {
                Err(report!(errors::ApiErrorResponse::PayoutFailed {
                    data: Some(serde_json::json!({
                        "message": "Payout method data is invalid"
                    }))
                })
                .attach_printable("Payout data provided is invalid"))
            })?;
            create_payout(
                state,
                merchant_account,
                key_store,
                req,
                &connector_data,
                payout_create,
                &payouts,
                &pmd,
                billing_address,
            )
            .await
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Payout creation failed".to_string(),
            })
            .attach_printable("Payout creation failed for given Payout request")?
        }
        _ => payouts,
    };

    // Auto fulfillment flow
    if payout_create.auto_fulfill {
        let pmd = payout_method_data.get_required_value("payout_method_data")?;
        payouts = fulfill_payout(
            state,
            merchant_account,
            key_store,
            &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
            &connector_data,
            payout_create,
            &payouts,
            &pmd,
            billing_address,
        )
        .await
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Payout fulfillment failed".to_string(),
        })
        .attach_printable("Payout fulfillment failed for given Payout request")?;
    }

    response_handler(
        state,
        merchant_account,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        payout_create,
        &payouts,
        billing_address,
    )
    .await
}

pub async fn validate_request_and_form_payout_create(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_name: &api_enums::Connector,
) -> RouterResult<(
    storage::payout_create::PayoutCreate,
    storage::payouts::Payouts,
    Option<domain::Address>,
)> {
    let db = &*state.store;
    let (
        billing_address,
        customer_id,
        payout_id,
        currency,
        payout_create_req,
        payouts_req,
        payouts,
    );

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
        Some(payout) => {
            payouts = db
                .find_payout_by_merchant_id_payout_id(
                    &merchant_account.merchant_id,
                    &payout_id.to_owned(),
                )
                .await
                .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout {
                    payout_id: payout_id.to_owned(),
                })
                .attach_printable("Error finding payouts in db")?;
            billing_address = payment_helpers::get_address_for_payment_request(
                db,
                req.billing.as_ref(),
                None,
                &merchant_account.merchant_id,
                Some(&payouts.customer_id.to_owned()),
                key_store,
            )
            .await?;
            payout
        }
        None => {
            currency = req
                .currency
                .to_owned()
                .map(ForeignInto::foreign_into)
                .get_required_value("currency")?;
            let status = if req.payout_method_data.is_some() {
                storage_enums::PayoutStatus::RequiresFulfillment
            } else {
                storage_enums::PayoutStatus::RequirePayoutMethodData
            };
            billing_address = payment_helpers::get_address_for_payment_request(
                db,
                req.billing.as_ref(),
                None,
                &merchant_account.merchant_id,
                req.customer_id.as_ref(),
                key_store,
            )
            .await?;
            let address_id = billing_address
                .to_owned()
                .map_or("".to_string(), |b| b.address_id);
            payouts_req = storage::PayoutsNew::default()
                .set_payout_id(payout_id.to_owned())
                .set_customer_id(customer_id.to_owned())
                .set_merchant_id(merchant_account.merchant_id.to_owned())
                .set_address_id(address_id.to_owned())
                .set_connector_payout_id(String::default())
                .set_connector(connector_name.to_string())
                .set_status(status)
                .to_owned();
            payout_create_req = storage::PayoutCreateNew::default()
                .set_payout_id(payout_id.to_owned())
                .set_merchant_id(merchant_account.merchant_id.to_owned())
                .set_customer_id(customer_id.to_owned())
                .set_address_id(address_id.to_owned())
                .set_payout_type(req.payout_type.foreign_into())
                .set_amount(req.amount.unwrap_or(api::Amount::Zero).into())
                .set_destination_currency(currency)
                .set_source_currency(currency)
                .set_description(req.description.to_owned().unwrap_or("".to_string()))
                .set_recurring(req.recurring.unwrap_or(false))
                .set_auto_fulfill(req.auto_fulfill.unwrap_or(false))
                .set_return_url(req.return_url.to_owned())
                .set_entity_type(storage_enums::EntityType::foreign_from(
                    req.entity_type.unwrap_or(api_enums::EntityType::default()),
                ))
                .set_metadata(req.metadata.to_owned())
                .set_created_at(Some(common_utils::date_time::now()))
                .set_last_modified_at(Some(common_utils::date_time::now()))
                .to_owned();

            payouts = db
                .insert_payout(payouts_req)
                .await
                .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout {
                    payout_id: payout_id.to_owned(),
                })
                .attach_printable("Error inserting payouts in db")?;

            db.insert_payout_create(payout_create_req)
                .await
                .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout {
                    payout_id: payout_id.to_owned(),
                })
                .attach_printable("Error inserting payout_create in db")?
        }
    };

    Ok((payout, payouts, billing_address))
}

#[allow(clippy::too_many_arguments)]
pub async fn check_payout_eligibility(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::ConnectorData,
    payout_create: &storage::payout_create::PayoutCreate,
    payouts: &storage::payouts::Payouts,
    payout_method_data: &api::PayoutMethodData,
    billing_address: &Option<domain::Address>,
) -> RouterResult<storage::payouts::Payouts> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        payout_create,
        payouts,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        billing_address,
        payout_method_data,
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
    .to_payout_failed_response()?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let updated_payouts = match router_data_resp.response {
        Ok(payout_response_data) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
                payout_method_id: payouts.payout_method_id.to_owned(),
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in db")?
        }
        Err(err) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
                payout_method_id: payouts.payout_method_id.to_owned(),
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in db")?
        }
    };

    Ok(updated_payouts)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::ConnectorData,
    payout_create: &storage::payout_create::PayoutCreate,
    payouts: &storage::payouts::Payouts,
    payout_method_data: &api::PayoutMethodData,
    billing_address: &Option<domain::Address>,
) -> RouterResult<storage::payouts::Payouts> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        payout_create,
        payouts,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        billing_address,
        payout_method_data,
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
    .to_payout_failed_response()?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let updated_payouts = match router_data_resp.response {
        Ok(payout_response_data) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
                payout_method_id: payouts.payout_method_id.to_owned(),
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in db")?
        }
        Err(err) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
                payout_method_id: payouts.payout_method_id.to_owned(),
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in db")?
        }
    };

    Ok(updated_payouts)
}

#[allow(clippy::too_many_arguments)]
pub async fn fulfill_payout(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &payouts::PayoutRequest,
    connector_data: &api::ConnectorData,
    payout_create: &storage::payout_create::PayoutCreate,
    payouts: &storage::payouts::Payouts,
    payout_method_data: &api::PayoutMethodData,
    billing_address: &Option<domain::Address>,
) -> RouterResult<storage::payouts::Payouts> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        key_store,
        payout_create,
        payouts,
        req,
        billing_address,
        payout_method_data,
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
    .to_payout_failed_response()?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let updated_payouts = match router_data_resp.response {
        Ok(payout_response_data) => {
            let payment_method_id = helpers::save_payout_data_to_locker(
                state,
                payout_create,
                payout_method_data,
                merchant_account,
            )
            .await?;
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
                payout_method_id: payment_method_id.or(payouts.payout_method_id.to_owned()),
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in db")?
        }
        Err(err) => {
            let updated_payouts = storage::payouts::PayoutsUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
                payout_method_id: None,
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_create.payout_id,
                updated_payouts,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in db")?
        }
    };

    Ok(updated_payouts)
}

pub async fn response_handler(
    _state: &AppState,
    merchant_account: &domain::MerchantAccount,
    _req: &payouts::PayoutRequest,
    payout_create: &storage::payout_create::PayoutCreate,
    payouts: &storage::payouts::Payouts,
    billing_address: &Option<domain::Address>,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let status = api_enums::PayoutStatus::foreign_from(payouts.status.to_owned());
    let currency = api_enums::Currency::foreign_from(payout_create.destination_currency.to_owned());
    let entity_type = api_enums::EntityType::foreign_from(payout_create.entity_type.to_owned());
    let payout_type = api_enums::PayoutType::foreign_from(payout_create.payout_type.to_owned());

    let (name, phone, phone_country_code, address) =
        billing_address
            .as_ref()
            .map_or((None, None, None, None), |a| {
                let name = format!(
                    "{} {}",
                    a.first_name
                        .to_owned()
                        .map_or("".to_string(), |f| f.peek().to_string()),
                    a.last_name
                        .to_owned()
                        .map_or("".to_string(), |l| l.peek().to_string()),
                );
                let phone = a
                    .phone_number
                    .to_owned()
                    .map_or("".to_string(), |p| p.peek().to_string());

                let phone_details = api_models::payments::PhoneDetails {
                    number: a.phone_number.to_owned().map(Encryptable::into_inner),
                    country_code: a.country_code.to_owned(),
                };
                let address_details = api_models::payments::AddressDetails {
                    city: a.city.to_owned(),
                    country: a.country.to_owned(),
                    line1: a.line1.to_owned().map(Encryptable::into_inner),
                    line2: a.line2.to_owned().map(Encryptable::into_inner),
                    line3: a.line3.to_owned().map(Encryptable::into_inner),
                    zip: a.zip.to_owned().map(Encryptable::into_inner),
                    first_name: a.first_name.to_owned().map(Encryptable::into_inner),
                    last_name: a.last_name.to_owned().map(Encryptable::into_inner),
                    state: a.state.to_owned().map(Encryptable::into_inner),
                };
                (
                    Some(Secret::new(name)),
                    Some(Secret::new(phone)),
                    a.country_code.to_owned(),
                    Some(api::payments::Address {
                        phone: Some(phone_details),
                        address: Some(address_details),
                    }),
                )
            });

    let response = api::PayoutCreateResponse {
        payout_id: payout_create.payout_id.to_owned(),
        merchant_id: merchant_account.merchant_id.to_owned(),
        amount: payout_create.amount.to_owned(),
        currency,
        connector: Some(payouts.connector.to_owned()),
        payout_type,
        billing: address,
        customer_id: payout_create.customer_id.to_owned(),
        auto_fulfill: payout_create.auto_fulfill,
        email: Some(Secret::new("".to_string())), // FIXME: Store + Fetch from DB
        name,
        phone,
        phone_country_code,
        client_secret: None,
        return_url: payout_create.return_url.to_owned(),
        business_country: None, // FIXME: Fetch from MCA
        business_label: None,   // FIXME: Fetch from MCA
        description: payout_create.description.to_owned(),
        entity_type,
        recurring: payout_create.recurring,
        metadata: payout_create.metadata.to_owned(),
        status,
        error_message: payouts.error_message.to_owned(),
        error_code: payouts.error_code.to_owned(),
    };
    Ok(services::ApplicationResponse::Json(response))
}
