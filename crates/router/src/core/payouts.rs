pub mod helpers;
pub mod validator;

use api_models::enums as api_enums;
use error_stack::{report, ResultExt};
use masking::Secret;
use router_env::{instrument, tracing};
use serde_json::{self};
use storage_models::enums as storage_enums;

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
        storage,
        transformers::{ForeignFrom, ForeignInto},
    },
    utils::{self, OptionExt},
};

// ********************************************** TYPES **********************************************
#[derive(Clone)]
pub struct PayoutData {
    pub billing_address: Option<storage::Address>,
    pub customer_details: Option<storage::Customer>,
    pub payouts: storage::Payouts,
    pub payout_create: storage::PayoutCreate,
    pub payout_method_data: Option<payouts::PayoutMethodData>,
    pub merchant_connector_account: Option<payment_helpers::MerchantConnectorAccountType>,
}

// ********************************************** CORE FLOWS **********************************************

#[instrument(skip_all)]
pub async fn payouts_create_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse>
where
{
    // TODO: Remove hardcoded connector
    let connector_name = api_enums::Connector::Adyen;

    // Validate create request
    let payout_id = validator::validate_create_request(state, &merchant_account, &req)
        .await
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid data passed".to_string(),
        })?;

    // Create DB entries
    let mut payout_data =
        payout_create_db_entries(state, &merchant_account, &req, &payout_id, &connector_name)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

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
        &req,
        connector_data,
        &mut payout_data,
    )
    .await
}

pub async fn payouts_update_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let mut payout_data = make_payout_data(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
    )
    .await?;

    // Update DB with new data
    let payouts = payout_data.payouts.to_owned();
    let updated_payouts = storage::PayoutsUpdate::Update {
        amount: req.amount.unwrap_or(payouts.amount.into()).into(),
        destination_currency: req
            .currency
            .unwrap_or(payouts.destination_currency.foreign_into())
            .foreign_into(),
        source_currency: req
            .currency
            .unwrap_or(payouts.source_currency.foreign_into())
            .foreign_into(),
        description: req.description.clone().or(payouts.description),
        recurring: req.recurring.unwrap_or(payouts.recurring),
        auto_fulfill: req.auto_fulfill.unwrap_or(payouts.auto_fulfill),
        return_url: req.return_url.clone().or(payouts.return_url),
        entity_type: req
            .entity_type
            .unwrap_or(payouts.entity_type.foreign_into())
            .foreign_into(),
        metadata: req.metadata.clone().or(payouts.metadata),
        last_modified_at: Some(common_utils::date_time::now()),
        payout_method_data: None,
    };

    let payout_create = payout_data.payout_create.to_owned();
    let update_payout_create = storage::PayoutCreateUpdate::BusinessUpdate {
        business_country: req.business_country.or(payout_create.business_country),
        business_label: req.business_label.clone().or(payout_create.business_label),
    };

    let db = &*state.store;
    let payout_id = req.payout_id.clone().get_required_value("payout_id")?;
    let merchant_id = &merchant_account.merchant_id;
    payout_data.payouts = db
        .update_payout_by_merchant_id_payout_id(merchant_id, &payout_id, updated_payouts)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payouts")?;

    payout_data.payout_create = db
        .update_payout_create_by_merchant_id_payout_id(
            merchant_id,
            &payout_id,
            update_payout_create,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payout_create")?;

    // Form connector data
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &payout_data.payout_create.connector,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    call_connector_payout(
        state,
        &merchant_account,
        &req,
        connector_data,
        &mut payout_data,
    )
    .await
}

#[instrument(skip_all)]
pub async fn payouts_retrieve_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: payouts::PayoutRetrieveRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_data = make_payout_data(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutRetrieveRequest(req.to_owned()),
    )
    .await?;

    response_handler(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutRetrieveRequest(req.to_owned()),
        &payout_data,
    )
    .await
}

#[instrument(skip_all)]
pub async fn payouts_cancel_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: payouts::PayoutActionRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_data = make_payout_data(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
    )
    .await?;

    // TODO: Add connector integration
    response_handler(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
        &payout_data,
    )
    .await
}

#[instrument(skip_all)]
pub async fn payouts_fulfill_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: payouts::PayoutActionRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let mut payout_data = make_payout_data(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
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
    payout_data = fulfill_payout(
        state,
        &merchant_account,
        &payouts::PayoutRequest::PayoutActionRequest(req.to_owned()),
        &connector_data,
        &mut payout_data,
        &pmd,
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
        &payout_data,
    )
    .await
}

// ********************************************** HELPERS **********************************************
pub async fn call_connector_payout(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    connector_data: api::ConnectorData,
    payout_data: &mut PayoutData,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_create = &payout_data.payout_create.to_owned();
    let payout_method_data = helpers::make_payout_method_data(state, req, payout_create).await?;
    let payouts: &storage_models::payouts::Payouts = &payout_data.payouts.to_owned();
    if let Some(true) = req.create_payout {
        let pmd = payout_method_data
            .clone()
            .get_required_value("payout_method_data")?;

        // Eligibility flow
        if payouts.payout_type == storage_enums::PayoutType::Card {
            *payout_data = check_payout_eligibility(
                state,
                merchant_account,
                req,
                &connector_data,
                payout_data,
                &pmd,
            )
            .await
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Eligibility failed".to_string(),
            })
            .attach_printable("Eligibility failed for given Payout request")?;
        }

        // Payout creation flow
        utils::when(!payout_create.is_eligible.unwrap_or(true), || {
            Err(report!(errors::ApiErrorResponse::PayoutFailed {
                data: Some(serde_json::json!({
                    "message": "Payout method data is invalid"
                }))
            })
            .attach_printable("Payout data provided is invalid"))
        })?;
        *payout_data = create_payout(
            state,
            merchant_account,
            req,
            &connector_data,
            payout_data,
            &pmd,
        )
        .await
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Payout creation failed".to_string(),
        })
        .attach_printable("Payout creation failed for given Payout request")?;
    };

    // Auto fulfillment flow
    if payouts.auto_fulfill {
        let pmd = payout_method_data.get_required_value("payout_method_data")?;
        *payout_data = fulfill_payout(
            state,
            merchant_account,
            &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
            &connector_data,
            payout_data,
            &pmd,
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
        payout_data,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn check_payout_eligibility(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
    payout_method_data: &api::PayoutMethodData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        payout_data,
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
    )
    .await
    .map_err(|error| error.to_payout_failed_response())?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_id = &payout_data.payouts.payout_id;
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let updated_payout_create = storage::payout_create::PayoutCreateUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
            };
            payout_data.payout_create = db
                .update_payout_create_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payout_create,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        }
        Err(err) => {
            let updated_payout_create = storage::payout_create::PayoutCreateUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
            };
            payout_data.payout_create = db
                .update_payout_create_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payout_create,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        }
    };

    Ok(payout_data.clone())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_payout(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
    payout_method_data: &api::PayoutMethodData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        &payouts::PayoutRequest::PayoutCreateRequest(req.to_owned()),
        payout_data,
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
    )
    .await
    .map_err(|error| error.to_payout_failed_response())?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_id = &payout_data.payouts.payout_id;
    match router_data_resp.response {
        Ok(payout_response_data) => {
            let updated_payout_create = storage::payout_create::PayoutCreateUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
            };
            payout_data.payout_create = db
                .update_payout_create_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payout_create,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        }
        Err(err) => {
            let updated_payout_create = storage::payout_create::PayoutCreateUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
            };
            payout_data.payout_create = db
                .update_payout_create_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payout_create,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?;
        }
    };

    Ok(payout_data.clone())
}

#[allow(clippy::too_many_arguments)]
pub async fn fulfill_payout(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    req: &payouts::PayoutRequest,
    connector_data: &api::ConnectorData,
    payout_data: &mut PayoutData,
    payout_method_data: &api::PayoutMethodData,
) -> RouterResult<PayoutData> {
    // 1. Form Router data
    let router_data = core_utils::construct_payout_router_data(
        state,
        &connector_data.connector_name.to_string(),
        merchant_account,
        req,
        payout_data,
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
    )
    .await
    .map_err(|error| error.to_payout_failed_response())?;

    // 4. Process data returned by the connector
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_id = &payout_data.payouts.payout_id;
    let payout_create = &payout_data.payout_create;
    match router_data_resp.response {
        Ok(payout_response_data) => {
            if payout_data.payouts.recurring {
                helpers::save_payout_data_to_locker(
                    state,
                    payout_create,
                    payout_method_data,
                    merchant_account,
                )
                .await?;
            }

            let updated_payouts = storage::payout_create::PayoutCreateUpdate::StatusUpdate {
                connector_payout_id: payout_response_data.connector_payout_id,
                status: payout_response_data.status,
                error_code: None,
                error_message: None,
                is_eligible: payout_response_data.payout_eligible,
            };
            payout_data.payout_create = db
                .update_payout_create_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payouts,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?
        }
        Err(err) => {
            let updated_payouts = storage::payout_create::PayoutCreateUpdate::StatusUpdate {
                connector_payout_id: String::default(),
                status: storage_enums::PayoutStatus::Failed,
                error_code: Some(err.code),
                error_message: Some(err.message),
                is_eligible: None,
            };
            payout_data.payout_create = db
                .update_payout_create_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
                    updated_payouts,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating payouts in db")?
        }
    };

    Ok(payout_data.clone())
}

pub async fn response_handler(
    _state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    _req: &payouts::PayoutRequest,
    payout_data: &PayoutData,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let payout_create = payout_data.payout_create.to_owned();
    let payouts = payout_data.payouts.to_owned();
    let billing_address = payout_data.billing_address.to_owned();
    let customer_details = payout_data.customer_details.to_owned();

    let status = api_enums::PayoutStatus::foreign_from(payout_create.status.to_owned());
    let currency = api_enums::Currency::foreign_from(payouts.destination_currency.to_owned());
    let entity_type = api_enums::EntityType::foreign_from(payouts.entity_type.to_owned());
    let payout_type = api_enums::PayoutType::foreign_from(payouts.payout_type.to_owned());

    let customer_id = payouts.customer_id;

    let (email, name, phone, phone_country_code) =
        customer_details.map_or((None, None, None, None), |c| {
            (
                c.email,
                Some(Secret::new(c.name.unwrap_or_default())),
                c.phone,
                c.phone_country_code,
            )
        });

    let address = billing_address.as_ref().map(|a| {
        let phone_details = api_models::payments::PhoneDetails {
            number: a.phone_number.to_owned(),
            country_code: a.country_code.to_owned(),
        };
        let address_details = api_models::payments::AddressDetails {
            city: a.city.to_owned(),
            country: a.country.to_owned(),
            line1: a.line1.to_owned(),
            line2: a.line2.to_owned(),
            line3: a.line3.to_owned(),
            zip: a.zip.to_owned(),
            first_name: a.first_name.to_owned(),
            last_name: a.last_name.to_owned(),
            state: a.state.to_owned(),
        };
        api::payments::Address {
            phone: Some(phone_details),
            address: Some(address_details),
        }
    });

    let response = api::PayoutCreateResponse {
        payout_id: payouts.payout_id.to_owned(),
        merchant_id: merchant_account.merchant_id.to_owned(),
        amount: payouts.amount.to_owned(),
        currency,
        connector: Some(payout_create.connector.to_owned()),
        payout_type,
        billing: address,
        customer_id,
        auto_fulfill: payouts.auto_fulfill,
        email,
        name,
        phone,
        phone_country_code,
        client_secret: None,
        return_url: payouts.return_url.to_owned(),
        business_country: None, // FIXME: Fetch from MCA
        business_label: None,   // FIXME: Fetch from MCA
        description: payouts.description.to_owned(),
        entity_type,
        recurring: payouts.recurring,
        metadata: payouts.metadata,
        status,
        error_message: payout_create.error_message.to_owned(),
        error_code: payout_create.error_code,
    };
    Ok(services::ApplicationResponse::Json(response))
}

// DB entries
pub async fn payout_create_db_entries(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    payout_id: &String,
    connector_name: &api_enums::Connector,
) -> RouterResult<PayoutData> {
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;

    // Get or create customer
    let customer_details = payments::CustomerDetails {
        customer_id: req.customer_id.to_owned(),
        name: req.name.to_owned(),
        email: req.email.to_owned(),
        phone: req.phone.to_owned(),
        phone_country_code: req.phone_country_code.to_owned(),
    };
    let customer =
        helpers::get_or_create_customer_details(state, &customer_details, merchant_account).await?;
    let customer_id = customer
        .to_owned()
        .map_or("".to_string(), |c| c.customer_id);

    // Get or create address
    let billing_address = payment_helpers::get_address_for_payment_request(
        db,
        req.billing.as_ref(),
        None,
        merchant_id,
        &Some(customer_id.to_owned()),
    )
    .await?;
    let address_id = billing_address
        .to_owned()
        .map_or("".to_string(), |b| b.address_id);

    // Make payouts entry
    let currency = req
        .currency
        .to_owned()
        .map(ForeignInto::foreign_into)
        .get_required_value("currency")?;
    let payout_type = req
        .payout_type
        .to_owned()
        .get_required_value("payout_type")?;

    let payouts_req = storage::PayoutsNew::default()
        .set_payout_id(payout_id.to_owned())
        .set_merchant_id(merchant_id.to_owned())
        .set_customer_id(customer_id.to_owned())
        .set_address_id(address_id.to_owned())
        .set_payout_type(payout_type.foreign_into())
        .set_amount(req.amount.unwrap_or(api::Amount::Zero).into())
        .set_destination_currency(currency)
        .set_source_currency(currency)
        .set_description(req.description.to_owned())
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
    let payouts = db
        .insert_payout(payouts_req)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.to_owned(),
        })
        .attach_printable("Error inserting payouts in db")?;

    // Make payout_create entry
    let status = if req.payout_method_data.is_some() {
        storage_enums::PayoutStatus::RequiresFulfillment
    } else {
        storage_enums::PayoutStatus::RequiresPayoutMethodData
    };
    let payout_create_req = storage::PayoutCreateNew::default()
        .set_payout_id(payout_id.to_owned())
        .set_customer_id(customer_id.to_owned())
        .set_merchant_id(merchant_id.to_owned())
        .set_address_id(address_id.to_owned())
        .set_connector(connector_name.to_string())
        .set_status(status)
        .set_business_country(req.business_country.to_owned())
        .set_business_label(req.business_label.to_owned())
        .to_owned();
    let payout_create = db
        .insert_payout_create(payout_create_req)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.to_owned(),
        })
        .attach_printable("Error inserting payout_create in db")?;

    // Make PayoutData
    Ok(PayoutData {
        billing_address,
        customer_details: customer,
        payouts,
        payout_create,
        payout_method_data: None,
        merchant_connector_account: None,
    })
}

pub async fn make_payout_data(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    req: &payouts::PayoutRequest,
) -> RouterResult<PayoutData> {
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_id = match req {
        payouts::PayoutRequest::PayoutActionRequest(r) => r.payout_id.clone(),
        payouts::PayoutRequest::PayoutCreateRequest(r) => r.payout_id.clone().unwrap_or_default(),
        payouts::PayoutRequest::PayoutRetrieveRequest(r) => r.payout_id.clone(),
    };

    let payouts = db
        .find_payout_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;

    let payout_create = db
        .find_payout_create_by_merchant_id_payout_id(merchant_id, &payout_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;

    let billing_address = payment_helpers::get_address_for_payment_request(
        db,
        None,
        Some(&payouts.address_id.to_owned()),
        merchant_id,
        &Some(payouts.customer_id.to_owned()),
    )
    .await?;

    let customer_details = db
        .find_customer_optional_by_customer_id_merchant_id(
            &payouts.customer_id.to_owned(),
            merchant_id,
        )
        .await
        .map_or(None, |c| c);

    Ok(PayoutData {
        billing_address,
        customer_details,
        payouts,
        payout_create,
        payout_method_data: None,
        merchant_connector_account: None,
    })
}
