use std::str::FromStr;

use ::cards::CardNumber;
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, ValueExt},
};
use diesel_models::encryption::Encryption;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};

use crate::{
    core::{
        errors::{self, RouterResult},
        payment_methods::{cards, vault},
        payments::{customers::get_connector_customer_details_if_present, CustomerDetails},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        api::{self, enums as api_enums},
        domain::{
            self,
            types::{self as domain_types, AsyncLift},
        },
        storage,
    },
    utils::{self, OptionExt},
};

pub async fn make_payout_method_data<'a>(
    state: &'a AppState,
    key_store: &domain::MerchantKeyStore,
    payout_method_data: Option<&api::PayoutMethodData>,
    payout_attempt: &storage::PayoutAttempt,
) -> RouterResult<Option<api::PayoutMethodData>> {
    let db = &*state.store;
    match (
        payout_method_data.to_owned(),
        payout_attempt.payout_token.to_owned(),
    ) {
        // Get operation
        (None, Some(payout_token)) => {
            let pm = vault::Vault::get_payout_method_data_from_temporary_locker(
                state,
                &payout_token,
            )
            .await
            .attach_printable(
                "Payout method for given token not found or there was a problem fetching it",
            );
            match pm {
                Ok((pm, supplementary_data)) => {
                    utils::when(
                        supplementary_data
                            .customer_id
                            .ne(&Some(payout_attempt.customer_id.to_owned())),
                        || {
                            Err(errors::ApiErrorResponse::PreconditionFailed { message: "customer associated with payout method and customer passed in payout are not same".into() })
                        },
                    )?;
                    Ok(pm)
                }
                Err(_e) => {
                    let pm = cards::get_payment_method_from_hs_locker(
                        state,
                        key_store,
                        &payout_attempt.customer_id,
                        &payout_attempt.merchant_id,
                        &payout_token,
                    )
                    .await
                    .attach_printable("Failed to fetch payout method details from basilisk")
                    .change_context(errors::ApiErrorResponse::PayoutNotFound)?;
                    let pm_parsed: api::PayoutMethodData = pm
                        .peek()
                        .as_bytes()
                        .to_vec()
                        .parse_struct("PayoutMethodData")
                        .change_context(errors::ApiErrorResponse::InternalServerError)?;

                    Ok(Some(pm_parsed))
                }
            }
        }

        // Create / Update operation
        (Some(payout_method), payout_token) => {
            let lookup_key = vault::Vault::store_payout_method_data_in_locker(
                state,
                payout_token.to_owned(),
                payout_method,
                Some(payout_attempt.customer_id.to_owned()),
            )
            .await?;

            // Update payout_token in payout_attempt table
            if payout_token.is_none() {
                let payout_update = storage::PayoutAttemptUpdate::PayoutTokenUpdate {
                    payout_token: lookup_key,
                    status: payout_attempt.status,
                    last_modified_at: Some(common_utils::date_time::now()),
                };
                db.update_payout_attempt_by_merchant_id_payout_id(
                    &payout_attempt.merchant_id,
                    &payout_attempt.payout_id,
                    payout_update,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating token in payout attempt")?;
            }
            Ok(Some(payout_method.clone()))
        }

        // Ignore if nothing is passed
        _ => Ok(None),
    }
}

pub async fn save_payout_data_to_locker(
    state: &AppState,
    payout_attempt: &storage::payout_attempt::PayoutAttempt,
    payout_method_data: &api::PayoutMethodData,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<()> {
    match payout_method_data {
        api_models::payouts::PayoutMethodData::Card(card) => {
            let card_details = api::CardDetail {
                card_number: card.card_number.to_owned(),
                card_exp_month: card.expiry_month.to_owned(),
                card_exp_year: card.expiry_year.to_owned(),
                card_holder_name: Some(card.card_holder_name.to_owned()),
                nick_name: None,
            };
            let stored_card_resp = cards::call_to_card_hs(
                state,
                &card_details,
                None,
                &payout_attempt.customer_id,
                merchant_account,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
            let db = &*state.store;
            let update_payout = storage::PayoutsUpdate::PayoutMethodIdUpdate {
                payout_method_id: Some(stored_card_resp.card_reference),
                last_modified_at: Some(common_utils::date_time::now()),
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id.clone(),
                &payout_attempt.payout_id,
                update_payout,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in saved payout method")?;
            Ok(())
        }
        api_models::payouts::PayoutMethodData::Bank(_) => {
            let db = &*state.store;
            let key = key_store.key.get_inner().peek();
            let enc_str = async {
                serde_json::to_value(payout_method_data.to_owned())
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encode payout method data")
                    .ok()
                    .map(|v| {
                        let secret: Secret<String> = Secret::new(v.to_string());
                        secret
                    })
                    .async_lift(|inner| domain_types::encrypt_optional(inner, key))
                    .await
            }
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encrypt payout method data")?
            .map(Encryption::from)
            .map(|e| e.into_inner())
            .map_or(Err(errors::ApiErrorResponse::InternalServerError), |e| {
                Ok(hex::encode(e.peek()))
            })?;
            let temp_card = api::CardDetail {
                card_number: CardNumber::from_str("4111111111111111")
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to form a temporary card number")?,
                card_exp_month: "12".to_string().into(),
                card_exp_year: "99".to_string().into(),
                card_holder_name: Some("Temporary Card".to_string().into()),
                nick_name: None,
            };
            let stored_resp = cards::call_to_card_hs(
                state,
                &temp_card,
                Some(&enc_str),
                &payout_attempt.customer_id,
                merchant_account,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
            let update_payout = storage::PayoutsUpdate::PayoutMethodIdUpdate {
                payout_method_id: Some(stored_resp.card_reference),
                last_modified_at: Some(common_utils::date_time::now()),
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_attempt.payout_id,
                update_payout,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in saved payout method")?;
            Ok(())
        }
    }
}

pub async fn get_or_create_customer_details(
    state: &AppState,
    customer_details: &CustomerDetails,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<Option<domain::Customer>> {
    let db: &dyn StorageInterface = &*state.store;
    // Create customer_id if not passed in request
    let customer_id =
        core_utils::get_or_generate_id("customer_id", &customer_details.customer_id, "cust")?;
    let merchant_id = &merchant_account.merchant_id;
    let key = key_store.key.get_inner().peek();

    match db
        .find_customer_optional_by_customer_id_merchant_id(&customer_id, merchant_id, key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
    {
        Some(customer) => Ok(Some(customer)),
        None => {
            let customer = domain::Customer {
                customer_id,
                merchant_id: merchant_id.to_string(),
                name: domain_types::encrypt_optional(customer_details.name.to_owned(), key)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
                email: domain_types::encrypt_optional(
                    customer_details.email.to_owned().map(|e| e.expose()),
                    key,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
                phone: domain_types::encrypt_optional(customer_details.phone.to_owned(), key)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
                description: None,
                phone_country_code: customer_details.phone_country_code.to_owned(),
                metadata: None,
                connector_customer: None,
                id: None,
                created_at: common_utils::date_time::now(),
                modified_at: common_utils::date_time::now(),
            };

            Ok(Some(
                db.insert_customer(customer, key_store)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
            ))
        }
    }
}

pub fn decide_payout_connector(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    request_straight_through: Option<api::PayoutStraightThroughAlgorithm>,
    routing_data: &mut storage::PayoutRoutingData,
) -> RouterResult<api::PayoutConnectorCallType> {
    if let Some(ref connector_name) = routing_data.routed_through {
        let connector_data = api::PayoutConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            api::GetToken::Connector,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in 'routed_through'")?;

        return Ok(api::PayoutConnectorCallType::Single(connector_data));
    }

    if let Some(routing_algorithm) = request_straight_through {
        let connector_name = match &routing_algorithm {
            api::PayoutStraightThroughAlgorithm::Single(conn) => conn.to_string(),
        };

        let connector_data = api::PayoutConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector_name,
            api::GetToken::Connector,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in routing algorithm")?;

        routing_data.routed_through = Some(connector_name);
        routing_data.algorithm = Some(routing_algorithm);
        return Ok(api::PayoutConnectorCallType::Single(connector_data));
    }

    if let Some(ref routing_algorithm) = routing_data.algorithm {
        let connector_name = match routing_algorithm {
            api::PayoutStraightThroughAlgorithm::Single(conn) => conn.to_string(),
        };

        let connector_data = api::PayoutConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector_name,
            api::GetToken::Connector,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in routing algorithm")?;

        routing_data.routed_through = Some(connector_name);
        return Ok(api::PayoutConnectorCallType::Single(connector_data));
    }

    let routing_algorithm = merchant_account
        .payout_routing_algorithm
        .clone()
        .get_required_value("PayoutRoutingAlgorithm")
        .change_context(errors::ApiErrorResponse::PreconditionFailed {
            message: "no routing algorithm has been configured".to_string(),
        })?
        .parse_value::<api::PayoutRoutingAlgorithm>("PayoutRoutingAlgorithm")
        .change_context(errors::ApiErrorResponse::InternalServerError) // Deserialization failed
        .attach_printable("Unable to deserialize merchant routing algorithm")?;

    let connector_name = match routing_algorithm {
        api::PayoutRoutingAlgorithm::Single(conn) => conn.to_string(),
    };

    let connector_data = api::PayoutConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Routing algorithm gave invalid connector")?;

    routing_data.routed_through = Some(connector_name);

    Ok(api::PayoutConnectorCallType::Single(connector_data))
}

pub async fn get_default_payout_connector(
    _state: &AppState,
    request_connector: Option<serde_json::Value>,
) -> CustomResult<api::PayoutConnectorChoice, errors::ApiErrorResponse> {
    Ok(request_connector.map_or(
        api::PayoutConnectorChoice::Decide,
        api::PayoutConnectorChoice::StraightThrough,
    ))
}

pub fn should_call_payout_connector_create_customer<'a>(
    state: &AppState,
    connector: &api::PayoutConnectorData,
    customer: &'a Option<domain::Customer>,
    connector_label: &str,
) -> (bool, Option<&'a str>) {
    // Check if create customer is required for the connector
    let connector_needs_customer = state
        .conf
        .connector_customer
        .payout_connector_list
        .contains(&connector.connector_name);

    if connector_needs_customer {
        let connector_customer_details = customer.as_ref().and_then(|customer| {
            get_connector_customer_details_if_present(customer, connector_label)
        });
        let should_call_connector = connector_customer_details.is_none();
        (should_call_connector, connector_customer_details)
    } else {
        (false, None)
    }
}

pub fn is_payout_initiated(status: api_enums::PayoutStatus) -> bool {
    matches!(
        status,
        api_enums::PayoutStatus::Pending | api_enums::PayoutStatus::RequiresFulfillment
    )
}

pub fn is_payout_terminal_state(status: api_enums::PayoutStatus) -> bool {
    !matches!(
        status,
        api_enums::PayoutStatus::Pending
            | api_enums::PayoutStatus::RequiresCreation
            | api_enums::PayoutStatus::RequiresFulfillment
            | api_enums::PayoutStatus::RequiresPayoutMethodData
    )
}

pub fn is_payout_err_state(status: api_enums::PayoutStatus) -> bool {
    matches!(
        status,
        api_enums::PayoutStatus::Cancelled
            | api_enums::PayoutStatus::Failed
            | api_enums::PayoutStatus::Ineligible
    )
}

pub fn is_eligible_for_local_payout_cancellation(status: api_enums::PayoutStatus) -> bool {
    matches!(
        status,
        api_enums::PayoutStatus::RequiresCreation
            | api_enums::PayoutStatus::RequiresPayoutMethodData,
    )
}
