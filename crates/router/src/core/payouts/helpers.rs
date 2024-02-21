use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, StringExt, ValueExt},
};
use diesel_models::encryption::Encryption;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};

use crate::{
    core::{
        errors::{self, RouterResult},
        payment_methods::{
            cards, transformers,
            transformers::{StoreCardReq, StoreGenericReq, StoreLockerReq},
            vault,
        },
        payments::{customers::get_connector_customer_details_if_present, CustomerDetails},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        api::{self, enums as api_enums},
        domain::{
            self,
            types::{self as domain_types, AsyncLift},
        },
        storage,
        transformers::ForeignFrom,
    },
    utils::{self, OptionExt},
};

#[allow(clippy::too_many_arguments)]
pub async fn make_payout_method_data<'a>(
    state: &'a AppState,
    payout_method_data: Option<&api::PayoutMethodData>,
    payout_token: Option<&str>,
    customer_id: &str,
    merchant_id: &str,
    payout_id: &str,
    payout_type: Option<&api_enums::PayoutType>,
    merchant_key_store: &domain::MerchantKeyStore,
) -> RouterResult<Option<api::PayoutMethodData>> {
    let db = &*state.store;
    let certain_payout_type = payout_type.get_required_value("payout_type")?.to_owned();
    let hyperswitch_token = if let Some(payout_token) = payout_token {
        if payout_token.starts_with("temporary_token_") {
            Some(payout_token.to_string())
        } else {
            let key = format!(
                "pm_token_{}_{}_hyperswitch",
                payout_token,
                api_enums::PaymentMethod::foreign_from(certain_payout_type)
            );

            let redis_conn = state
                .store
                .get_redis_conn()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get redis connection")?;

            let hyperswitch_token = redis_conn
                .get_key::<Option<String>>(&key)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to fetch the token from redis")?
                .ok_or(error_stack::Report::new(
                    errors::ApiErrorResponse::UnprocessableEntity {
                        message: "Token is invalid or expired".to_owned(),
                    },
                ))?;
            let payment_token_data = hyperswitch_token
                .clone()
                .parse_struct("PaymentTokenData")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to deserialize hyperswitch token data")?;

            let payment_token = match payment_token_data {
                storage::PaymentTokenData::PermanentCard(storage::CardTokenData { token }) => {
                    Some(token)
                }
                storage::PaymentTokenData::TemporaryGeneric(storage::GenericTokenData {
                    token,
                }) => Some(token),
                _ => None,
            };
            payment_token.or(Some(payout_token.to_string()))
        }
    } else {
        None
    };

    match (payout_method_data.to_owned(), hyperswitch_token) {
        // Get operation
        (None, Some(payout_token)) => {
            if payout_token.starts_with("temporary_token_")
                || certain_payout_type == api_enums::PayoutType::Bank
            {
                let (pm, supplementary_data) = vault::Vault::get_payout_method_data_from_temporary_locker(
                    state,
                    &payout_token,
                    merchant_key_store,
                )
                .await
                .attach_printable(
                    "Payout method for given token not found or there was a problem fetching it",
                )?;
                utils::when(
                    supplementary_data
                        .customer_id
                        .ne(&Some(customer_id.to_owned())),
                    || {
                        Err(errors::ApiErrorResponse::PreconditionFailed { message: "customer associated with payout method and customer passed in payout are not same".into() })
                    },
                )?;
                Ok(pm)
            } else {
                let resp = cards::get_card_from_locker(
                    state,
                    customer_id,
                    merchant_id,
                    payout_token.as_ref(),
                )
                .await
                .attach_printable("Payout method [card] could not be fetched from HS locker")?;
                Ok(Some({
                    api::PayoutMethodData::Card(api::CardPayout {
                        card_number: resp.card_number,
                        expiry_month: resp.card_exp_month,
                        expiry_year: resp.card_exp_year,
                        card_holder_name: resp.name_on_card,
                    })
                }))
            }
        }

        // Create / Update operation
        (Some(payout_method), payout_token) => {
            let lookup_key = vault::Vault::store_payout_method_data_in_locker(
                state,
                payout_token.to_owned(),
                payout_method,
                Some(customer_id.to_owned()),
                merchant_key_store,
            )
            .await?;

            // Update payout_token in payout_attempt table
            if payout_token.is_none() {
                let payout_update = storage::PayoutAttemptUpdate::PayoutTokenUpdate {
                    payout_token: lookup_key,
                    last_modified_at: Some(common_utils::date_time::now()),
                };
                db.update_payout_attempt_by_merchant_id_payout_id(
                    merchant_id,
                    payout_id,
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
    let (locker_req, card_details, bank_details, wallet_details, payment_method_type) =
        match payout_method_data {
            api_models::payouts::PayoutMethodData::Card(card) => {
                let card_detail = api::CardDetail {
                    card_number: card.card_number.to_owned(),
                    card_holder_name: card.card_holder_name.to_owned(),
                    card_exp_month: card.expiry_month.to_owned(),
                    card_exp_year: card.expiry_year.to_owned(),
                    nick_name: None,
                    card_issuing_country: None,
                    card_network: None,
                    card_issuer: None,
                    card_type: None,
                };
                let payload = StoreLockerReq::LockerCard(StoreCardReq {
                    merchant_id: &merchant_account.merchant_id,
                    merchant_customer_id: payout_attempt.customer_id.to_owned(),
                    card: transformers::Card {
                        card_number: card.card_number.to_owned(),
                        name_on_card: card.card_holder_name.to_owned(),
                        card_exp_month: card.expiry_month.to_owned(),
                        card_exp_year: card.expiry_year.to_owned(),
                        card_brand: None,
                        card_isin: None,
                        nick_name: None,
                    },
                    requestor_card_reference: None,
                });
                (
                    payload,
                    Some(card_detail),
                    None,
                    None,
                    api_enums::PaymentMethodType::Debit,
                )
            }
            _ => {
                let key = key_store.key.get_inner().peek();
                let enc_data = async {
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
                let payload = StoreLockerReq::LockerGeneric(StoreGenericReq {
                    merchant_id: &merchant_account.merchant_id,
                    merchant_customer_id: payout_attempt.customer_id.to_owned(),
                    enc_data,
                });
                match payout_method_data {
                    api_models::payouts::PayoutMethodData::Bank(bank) => (
                        payload,
                        None,
                        Some(bank.to_owned()),
                        None,
                        api_enums::PaymentMethodType::foreign_from(bank.to_owned()),
                    ),
                    api_models::payouts::PayoutMethodData::Wallet(wallet) => (
                        payload,
                        None,
                        None,
                        Some(wallet.to_owned()),
                        api_enums::PaymentMethodType::foreign_from(wallet.to_owned()),
                    ),
                    api_models::payouts::PayoutMethodData::Card(_) => {
                        Err(errors::ApiErrorResponse::InternalServerError)?
                    }
                }
            }
        };
    // Store payout method in locker
    let stored_resp = cards::call_to_locker_hs(
        state,
        &locker_req,
        &payout_attempt.customer_id,
        api_enums::LockerChoice::HyperswitchCardVault,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    // Store card_reference in payouts table
    let db = &*state.store;
    let updated_payout = storage::PayoutsUpdate::PayoutMethodIdUpdate {
        payout_method_id: Some(stored_resp.card_reference.to_owned()),
        last_modified_at: Some(common_utils::date_time::now()),
    };
    db.update_payout_by_merchant_id_payout_id(
        &merchant_account.merchant_id,
        &payout_attempt.payout_id,
        updated_payout,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Error updating payouts in saved payout method")?;

    // fetch card info from db
    let card_isin = card_details
        .as_ref()
        .map(|c| c.card_number.clone().get_card_isin());

    let pm_data = card_isin
        .clone()
        .async_and_then(|card_isin| async move {
            db.get_card_info(&card_isin)
                .await
                .map_err(|error| services::logger::warn!(card_info_error=?error))
                .ok()
        })
        .await
        .flatten()
        .map(|card_info| {
            api::payment_methods::PaymentMethodsData::Card(
                api::payment_methods::CardDetailsPaymentMethod {
                    last4_digits: card_details
                        .as_ref()
                        .map(|c| c.card_number.clone().get_last4()),
                    issuer_country: card_info.card_issuing_country,
                    expiry_month: card_details.as_ref().map(|c| c.card_exp_month.clone()),
                    expiry_year: card_details.as_ref().map(|c| c.card_exp_year.clone()),
                    nick_name: card_details.as_ref().and_then(|c| c.nick_name.clone()),
                    card_holder_name: card_details
                        .as_ref()
                        .and_then(|c| c.card_holder_name.clone()),

                    card_isin: card_isin.clone(),
                    card_issuer: card_info.card_issuer,
                    card_network: card_info.card_network,
                    card_type: card_info.card_type,
                    saved_to_locker: true,
                },
            )
        })
        .unwrap_or_else(|| {
            api::payment_methods::PaymentMethodsData::Card(
                api::payment_methods::CardDetailsPaymentMethod {
                    last4_digits: card_details
                        .as_ref()
                        .map(|c| c.card_number.clone().get_last4()),
                    issuer_country: None,
                    expiry_month: card_details.as_ref().map(|c| c.card_exp_month.clone()),
                    expiry_year: card_details.as_ref().map(|c| c.card_exp_year.clone()),
                    nick_name: card_details.as_ref().and_then(|c| c.nick_name.clone()),
                    card_holder_name: card_details
                        .as_ref()
                        .and_then(|c| c.card_holder_name.clone()),

                    card_isin: card_isin.clone(),
                    card_issuer: None,
                    card_network: None,
                    card_type: None,
                    saved_to_locker: true,
                },
            )
        });

    let card_details_encrypted =
        cards::create_encrypted_payment_method_data(key_store, Some(pm_data)).await;

    // Insert in payment_method table
    let payment_method = api::PaymentMethodCreate {
        payment_method: api_enums::PaymentMethod::foreign_from(payout_method_data.to_owned()),
        payment_method_type: Some(payment_method_type),
        payment_method_issuer: None,
        payment_method_issuer_code: None,
        bank_transfer: bank_details,
        card: card_details,
        wallet: wallet_details,
        metadata: None,
        customer_id: Some(payout_attempt.customer_id.to_owned()),
        card_network: None,
    };

    let payment_method_id = common_utils::generate_id(crate::consts::ID_LENGTH, "pm");
    cards::create_payment_method(
        db,
        &payment_method,
        &payout_attempt.customer_id,
        &payment_method_id,
        Some(stored_resp.card_reference),
        &merchant_account.merchant_id,
        None,
        card_details_encrypted,
        key_store,
    )
    .await?;

    Ok(())
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
                address_id: None,
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
