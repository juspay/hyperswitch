use std::str::FromStr;

#[cfg(feature = "v2")]
use api_models::enums as api_enums;
#[cfg(feature = "v1")]
use api_models::enums;
use api_models::payment_methods as pm_api;
#[cfg(feature = "v1")]
use common_utils::{
    consts,
    crypto::Encryptable,
    ext_traits::{AsyncExt, ConfigExt},
    generate_id,
};
use common_utils::{errors::CustomResult, id_type};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, errors::api_error_response as errors, platform,
};
#[cfg(feature = "v1")]
use hyperswitch_domain_models::{ext_traits::OptionExt, payment_methods as domain_pm};
use masking::PeekInterface;
#[cfg(feature = "v1")]
use masking::Secret;
#[cfg(feature = "v1")]
use router_env::{instrument, logger, tracing};
#[cfg(feature = "v1")]
use serde_json::json;
use storage_impl::cards_info;

#[cfg(feature = "v1")]
use crate::{
    controller::create_encrypted_data,
    core::migration,
    helpers::{ForeignFrom, StorageErrorExt},
};
use crate::{controller::PaymentMethodsController, helpers::ForeignTryFrom, state};

#[cfg(feature = "v1")]
pub async fn migrate_payment_method(
    state: &state::PaymentMethodsState,
    req: pm_api::PaymentMethodMigrate,
    merchant_id: &id_type::MerchantId,
    platform: &platform::Platform,
    controller: &dyn PaymentMethodsController,
) -> CustomResult<ApplicationResponse<pm_api::PaymentMethodMigrateResponse>, errors::ApiErrorResponse>
{
    let mut req = req;
    let card_details = &req.card.get_required_value("card")?;

    let card_number_validation_result =
        cards::CardNumber::from_str(card_details.card_number.peek());

    let card_bin_details = populate_bin_details_for_masked_card(
        card_details,
        &*state.store,
        req.payment_method_type.as_ref(),
    )
    .await?;

    req.card = Some(api_models::payment_methods::MigrateCardDetail {
        card_issuing_country: card_bin_details.issuer_country.clone(),
        card_network: card_bin_details.card_network.clone(),
        card_issuer: card_bin_details.card_issuer.clone(),
        card_type: card_bin_details.card_type.clone(),
        ..card_details.clone()
    });

    if let Some(connector_mandate_details) = &req.connector_mandate_details {
        controller
            .validate_merchant_connector_ids_in_connector_mandate_details(
                platform.get_processor().get_key_store(),
                connector_mandate_details,
                merchant_id,
                card_bin_details.card_network.clone(),
            )
            .await?;
    };

    let should_require_connector_mandate_details = req.network_token.is_none();

    let mut migration_status = migration::RecordMigrationStatusBuilder::new();

    let resp = match card_number_validation_result {
        Ok(card_number) => {
            let payment_method_create_request =
                pm_api::PaymentMethodCreate::get_payment_method_create_from_payment_method_migrate(
                    card_number,
                    &req,
                );

            logger::debug!("Storing the card in locker and migrating the payment method");
            get_client_secret_or_add_payment_method_for_migration(
                state,
                payment_method_create_request,
                platform,
                &mut migration_status,
                controller,
            )
            .await?
        }
        Err(card_validation_error) => {
            logger::debug!("Card number to be migrated is invalid, skip saving in locker {card_validation_error}");
            skip_locker_call_and_migrate_payment_method(
                state,
                &req,
                merchant_id.to_owned(),
                platform,
                card_bin_details.clone(),
                should_require_connector_mandate_details,
                &mut migration_status,
                controller,
            )
            .await?
        }
    };
    let payment_method_response = match resp {
        ApplicationResponse::Json(response) => response,
        _ => Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch the payment method response")?,
    };

    let pm_id = payment_method_response.payment_method_id.clone();

    let network_token = req.network_token.clone();

    let network_token_migrated = match network_token {
        Some(nt_detail) => {
            logger::debug!("Network token migration");
            let network_token_requestor_ref_id = nt_detail.network_token_requestor_ref_id.clone();
            let network_token_data = &nt_detail.network_token_data;

            Some(
                controller
                    .save_network_token_and_update_payment_method(
                        &req,
                        platform.get_processor().get_key_store(),
                        network_token_data,
                        network_token_requestor_ref_id,
                        pm_id,
                    )
                    .await
                    .map_err(|err| logger::error!(?err, "Failed to save network token"))
                    .ok()
                    .unwrap_or_default(),
            )
        }
        None => {
            logger::debug!("Network token data is not available");
            None
        }
    };
    migration_status.network_token_migrated(network_token_migrated);
    let migrate_status = migration_status.build();

    Ok(ApplicationResponse::Json(
        pm_api::PaymentMethodMigrateResponse {
            payment_method_response,
            card_migrated: migrate_status.card_migrated,
            network_token_migrated: migrate_status.network_token_migrated,
            connector_mandate_details_migrated: migrate_status.connector_mandate_details_migrated,
            network_transaction_id_migrated: migrate_status.network_transaction_migrated,
        },
    ))
}

#[cfg(feature = "v2")]
pub async fn migrate_payment_method(
    _state: &state::PaymentMethodsState,
    _req: pm_api::PaymentMethodMigrate,
    _merchant_id: &id_type::MerchantId,
    _platform: &platform::Platform,
    _controller: &dyn PaymentMethodsController,
) -> CustomResult<ApplicationResponse<pm_api::PaymentMethodMigrateResponse>, errors::ApiErrorResponse>
{
    todo!()
}

#[cfg(feature = "v1")]
pub async fn populate_bin_details_for_masked_card(
    card_details: &api_models::payment_methods::MigrateCardDetail,
    db: &dyn state::PaymentMethodsStorageInterface,
    payment_method_type: Option<&enums::PaymentMethodType>,
) -> CustomResult<pm_api::CardDetailFromLocker, errors::ApiErrorResponse> {
    if let Some(
            // Cards
            enums::PaymentMethodType::Credit
            | enums::PaymentMethodType::Debit

            // Wallets
            | enums::PaymentMethodType::ApplePay
            | enums::PaymentMethodType::GooglePay,
        ) = payment_method_type {
        migration::validate_card_expiry(
            &card_details.card_exp_month,
            &card_details.card_exp_year,
        )?;
    }

    let card_number = card_details.card_number.clone();

    let (card_isin, _last4_digits) = get_card_bin_and_last4_digits_for_masked_card(
        card_number.peek(),
    )
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "Invalid masked card number".to_string(),
    })?;

    let card_bin_details = if card_details.card_issuer.is_some()
        && card_details.card_network.is_some()
        && card_details.card_type.is_some()
        && card_details.card_issuing_country.is_some()
    {
        pm_api::CardDetailFromLocker::foreign_try_from((card_details, None))?
    } else {
        let card_info = db
            .get_card_info(&card_isin)
            .await
            .map_err(|error| logger::error!(card_info_error=?error))
            .ok()
            .flatten();

        pm_api::CardDetailFromLocker::foreign_try_from((card_details, card_info))?
    };
    Ok(card_bin_details)
}

#[cfg(feature = "v1")]
impl
    ForeignTryFrom<(
        &api_models::payment_methods::MigrateCardDetail,
        Option<cards_info::CardInfo>,
    )> for pm_api::CardDetailFromLocker
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(
        (card_details, card_info): (
            &api_models::payment_methods::MigrateCardDetail,
            Option<cards_info::CardInfo>,
        ),
    ) -> Result<Self, Self::Error> {
        let (card_isin, last4_digits) =
            get_card_bin_and_last4_digits_for_masked_card(card_details.card_number.peek())
                .change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid masked card number".to_string(),
                })?;
        if let Some(card_bin_info) = card_info {
            Ok(Self {
                scheme: card_details
                    .card_network
                    .clone()
                    .or(card_bin_info.card_network.clone())
                    .map(|card_network| card_network.to_string()),
                last4_digits: Some(last4_digits.clone()),
                issuer_country: card_details
                    .card_issuing_country
                    .clone()
                    .or(card_bin_info.card_issuing_country),
                card_number: None,
                expiry_month: Some(card_details.card_exp_month.clone()),
                expiry_year: Some(card_details.card_exp_year.clone()),
                card_token: None,
                card_fingerprint: None,
                card_holder_name: card_details.card_holder_name.clone(),
                nick_name: card_details.nick_name.clone(),
                card_isin: Some(card_isin.clone()),
                card_issuer: card_details
                    .card_issuer
                    .clone()
                    .or(card_bin_info.card_issuer),
                card_network: card_details
                    .card_network
                    .clone()
                    .or(card_bin_info.card_network),
                card_type: card_details.card_type.clone().or(card_bin_info.card_type),
                saved_to_locker: false,
            })
        } else {
            Ok(Self {
                scheme: card_details
                    .card_network
                    .clone()
                    .map(|card_network| card_network.to_string()),
                last4_digits: Some(last4_digits.clone()),
                issuer_country: card_details.card_issuing_country.clone(),
                card_number: None,
                expiry_month: Some(card_details.card_exp_month.clone()),
                expiry_year: Some(card_details.card_exp_year.clone()),
                card_token: None,
                card_fingerprint: None,
                card_holder_name: card_details.card_holder_name.clone(),
                nick_name: card_details.nick_name.clone(),
                card_isin: Some(card_isin.clone()),
                card_issuer: card_details.card_issuer.clone(),
                card_network: card_details.card_network.clone(),
                card_type: card_details.card_type.clone(),
                saved_to_locker: false,
            })
        }
    }
}

#[cfg(feature = "v2")]
impl
    ForeignTryFrom<(
        &api_models::payment_methods::MigrateCardDetail,
        Option<cards_info::CardInfo>,
    )> for pm_api::CardDetailFromLocker
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(
        (card_details, card_info): (
            &api_models::payment_methods::MigrateCardDetail,
            Option<cards_info::CardInfo>,
        ),
    ) -> Result<Self, Self::Error> {
        let (card_isin, last4_digits) =
            get_card_bin_and_last4_digits_for_masked_card(card_details.card_number.peek())
                .change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Invalid masked card number".to_string(),
                })?;
        if let Some(card_bin_info) = card_info {
            Ok(Self {
                last4_digits: Some(last4_digits.clone()),
                issuer_country: card_details
                    .card_issuing_country
                    .as_ref()
                    .map(|c| api_enums::CountryAlpha2::from_str(c))
                    .transpose()
                    .ok()
                    .flatten()
                    .or(card_bin_info
                        .card_issuing_country
                        .as_ref()
                        .map(|c| api_enums::CountryAlpha2::from_str(c))
                        .transpose()
                        .ok()
                        .flatten()),
                card_number: None,
                expiry_month: Some(card_details.card_exp_month.clone()),
                expiry_year: Some(card_details.card_exp_year.clone()),
                card_fingerprint: None,
                card_holder_name: card_details.card_holder_name.clone(),
                nick_name: card_details.nick_name.clone(),
                card_isin: Some(card_isin.clone()),
                card_issuer: card_details
                    .card_issuer
                    .clone()
                    .or(card_bin_info.card_issuer),
                card_network: card_details
                    .card_network
                    .clone()
                    .or(card_bin_info.card_network),
                card_type: card_details.card_type.clone().or(card_bin_info.card_type),
                saved_to_locker: false,
            })
        } else {
            Ok(Self {
                last4_digits: Some(last4_digits.clone()),
                issuer_country: card_details
                    .card_issuing_country
                    .as_ref()
                    .map(|c| api_enums::CountryAlpha2::from_str(c))
                    .transpose()
                    .ok()
                    .flatten(),
                card_number: None,
                expiry_month: Some(card_details.card_exp_month.clone()),
                expiry_year: Some(card_details.card_exp_year.clone()),
                card_fingerprint: None,
                card_holder_name: card_details.card_holder_name.clone(),
                nick_name: card_details.nick_name.clone(),
                card_isin: Some(card_isin.clone()),
                card_issuer: card_details.card_issuer.clone(),
                card_network: card_details.card_network.clone(),
                card_type: card_details.card_type.clone(),
                saved_to_locker: false,
            })
        }
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn get_client_secret_or_add_payment_method_for_migration(
    state: &state::PaymentMethodsState,
    req: pm_api::PaymentMethodCreate,
    platform: &platform::Platform,
    migration_status: &mut migration::RecordMigrationStatusBuilder,
    controller: &dyn PaymentMethodsController,
) -> CustomResult<ApplicationResponse<pm_api::PaymentMethodResponse>, errors::ApiErrorResponse> {
    let merchant_id = platform.get_processor().get_account().get_id();
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;

    #[cfg(not(feature = "payouts"))]
    let condition = req.card.is_some();
    #[cfg(feature = "payouts")]
    let condition = req.card.is_some() || req.bank_transfer.is_some() || req.wallet.is_some();
    let key_manager_state = &state.into();

    let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
        .billing
        .clone()
        .async_map(|billing| {
            create_encrypted_data(
                key_manager_state,
                platform.get_processor().get_key_store(),
                billing,
            )
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?;

    let connector_mandate_details = req
        .connector_mandate_details
        .clone()
        .map(serde_json::to_value)
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    if condition {
        Box::pin(save_migration_payment_method(
            req,
            migration_status,
            controller,
        ))
        .await
    } else {
        let payment_method_id = generate_id(consts::ID_LENGTH, "pm");

        let res = controller
            .create_payment_method(
                &req,
                &customer_id,
                payment_method_id.as_str(),
                None,
                merchant_id,
                None,
                None,
                None,
                connector_mandate_details.clone(),
                Some(enums::PaymentMethodStatus::AwaitingData),
                None,
                payment_method_billing_address,
                None,
                None,
                None,
                None,
                Default::default(),
            )
            .await?;
        migration_status.connector_mandate_details_migrated(
            connector_mandate_details
                .clone()
                .and_then(|val| (val != json!({})).then_some(true))
                .or_else(|| {
                    req.connector_mandate_details
                        .clone()
                        .and_then(|val| (!val.0.is_empty()).then_some(false))
                }),
        );
        //card is not migrated in this case
        migration_status.card_migrated(false);

        if res.status == enums::PaymentMethodStatus::AwaitingData {
            controller
                .add_payment_method_status_update_task(
                    &res,
                    enums::PaymentMethodStatus::AwaitingData,
                    enums::PaymentMethodStatus::Inactive,
                    merchant_id,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to add payment method status update task in process tracker",
                )?;
        }

        Ok(ApplicationResponse::Json(
            pm_api::PaymentMethodResponse::foreign_from((None, res)),
        ))
    }
}
#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn skip_locker_call_and_migrate_payment_method(
    state: &state::PaymentMethodsState,
    req: &pm_api::PaymentMethodMigrate,
    merchant_id: id_type::MerchantId,
    platform: &platform::Platform,
    card: pm_api::CardDetailFromLocker,
    should_require_connector_mandate_details: bool,
    migration_status: &mut migration::RecordMigrationStatusBuilder,
    controller: &dyn PaymentMethodsController,
) -> CustomResult<ApplicationResponse<pm_api::PaymentMethodResponse>, errors::ApiErrorResponse> {
    let db = &*state.store;
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;

    // In this case, since we do not have valid card details, recurring payments can only be done through connector mandate details.
    //if network token data is present, then connector mandate details are not mandatory

    let connector_mandate_details = if should_require_connector_mandate_details {
        let connector_mandate_details_req = req
            .connector_mandate_details
            .clone()
            .and_then(|c| c.payments)
            .clone()
            .get_required_value("connector mandate details")?;

        Some(
            serde_json::to_value(&connector_mandate_details_req)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse connector mandate details")?,
        )
    } else {
        req.connector_mandate_details
            .clone()
            .and_then(|c| c.payments)
            .map(|mandate_details_req| {
                serde_json::to_value(&mandate_details_req)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse connector mandate details")
            })
            .transpose()?
    };
    let key_manager_state = &state.into();
    let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
        .billing
        .clone()
        .async_map(|billing| {
            create_encrypted_data(
                key_manager_state,
                platform.get_processor().get_key_store(),
                billing,
            )
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?;

    let customer = db
        .find_customer_by_customer_id_merchant_id(
            &customer_id,
            &merchant_id,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let payment_method_card_details = pm_api::PaymentMethodsData::Card(
        pm_api::CardDetailsPaymentMethod::from((card.clone(), None)),
    );

    let payment_method_data_encrypted: Option<Encryptable<Secret<serde_json::Value>>> = Some(
        create_encrypted_data(
            &state.into(),
            platform.get_processor().get_key_store(),
            payment_method_card_details,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method card details")?,
    );

    let payment_method_metadata: Option<serde_json::Value> =
        req.metadata.as_ref().map(|data| data.peek()).cloned();

    let network_transaction_id = req.network_transaction_id.clone();

    let payment_method_id = generate_id(consts::ID_LENGTH, "pm");

    let current_time = common_utils::date_time::now();

    let response = db
        .insert_payment_method(
            platform.get_processor().get_key_store(),
            domain_pm::PaymentMethod {
                customer_id: customer_id.to_owned(),
                merchant_id: merchant_id.to_owned(),
                payment_method_id: payment_method_id.to_string(),
                locker_id: None,
                payment_method: req.payment_method,
                payment_method_type: req.payment_method_type,
                payment_method_issuer: req.payment_method_issuer.clone(),
                scheme: req.card_network.clone().or(card.scheme.clone()),
                metadata: payment_method_metadata.map(Secret::new),
                payment_method_data: payment_method_data_encrypted,
                connector_mandate_details: connector_mandate_details.clone(),
                customer_acceptance: None,
                client_secret: None,
                status: enums::PaymentMethodStatus::Active,
                network_transaction_id: network_transaction_id.clone(),
                payment_method_issuer_code: None,
                accepted_currency: None,
                token: None,
                cardholder_name: None,
                issuer_name: None,
                issuer_country: None,
                payer_country: None,
                is_stored: None,
                swift_code: None,
                direct_debit_token: None,
                created_at: current_time,
                last_modified: current_time,
                last_used_at: current_time,
                payment_method_billing_address,
                updated_by: None,
                version: common_types::consts::API_VERSION,
                network_token_requestor_reference_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                vault_source_details: Default::default(),
                created_by: None,
                last_modified_by: None,
            },
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in db")?;

    logger::debug!("Payment method inserted in db");

    migration_status.network_transaction_id_migrated(
        network_transaction_id.and_then(|val| (!val.is_empty_after_trim()).then_some(true)),
    );

    migration_status.connector_mandate_details_migrated(
        connector_mandate_details
            .clone()
            .and_then(|val| if val == json!({}) { None } else { Some(true) })
            .or_else(|| {
                req.connector_mandate_details.clone().and_then(|val| {
                    val.payments
                        .and_then(|payin_val| (!payin_val.0.is_empty()).then_some(false))
                })
            }),
    );

    if customer.default_payment_method_id.is_none() && req.payment_method.is_some() {
        let _ = controller
            .set_default_payment_method(&merchant_id, &customer_id, payment_method_id.to_owned())
            .await
            .map_err(|error| logger::error!(?error, "Failed to set the payment method as default"));
    }
    Ok(ApplicationResponse::Json(
        pm_api::PaymentMethodResponse::foreign_from((Some(card), response)),
    ))
}

// need to discuss regarding the migration APIs for v2
#[cfg(feature = "v2")]
pub async fn skip_locker_call_and_migrate_payment_method(
    _state: state::PaymentMethodsState,
    _req: &pm_api::PaymentMethodMigrate,
    _merchant_id: id_type::MerchantId,
    _platform: &platform::Platform,
    _card: pm_api::CardDetailFromLocker,
) -> CustomResult<ApplicationResponse<pm_api::PaymentMethodResponse>, errors::ApiErrorResponse> {
    todo!()
}
pub fn get_card_bin_and_last4_digits_for_masked_card(
    masked_card_number: &str,
) -> Result<(String, String), cards::CardNumberValidationErr> {
    let last4_digits = masked_card_number
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();

    let card_isin = masked_card_number.chars().take(6).collect::<String>();

    cards::validate::validate_card_number_chars(&card_isin)
        .and_then(|_| cards::validate::validate_card_number_chars(&last4_digits))?;

    Ok((card_isin, last4_digits))
}
#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn save_migration_payment_method(
    req: pm_api::PaymentMethodCreate,
    migration_status: &mut migration::RecordMigrationStatusBuilder,
    controller: &dyn PaymentMethodsController,
) -> CustomResult<ApplicationResponse<pm_api::PaymentMethodResponse>, errors::ApiErrorResponse> {
    let connector_mandate_details = req
        .connector_mandate_details
        .clone()
        .map(serde_json::to_value)
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let network_transaction_id = req.network_transaction_id.clone();

    let res = controller.add_payment_method(&req).await?;

    migration_status.card_migrated(true);
    migration_status.network_transaction_id_migrated(
        network_transaction_id.and_then(|val| (!val.is_empty_after_trim()).then_some(true)),
    );

    migration_status.connector_mandate_details_migrated(
        connector_mandate_details
            .and_then(|val| if val == json!({}) { None } else { Some(true) })
            .or_else(|| {
                req.connector_mandate_details
                    .and_then(|val| (!val.0.is_empty()).then_some(false))
            }),
    );
    Ok(res)
}
