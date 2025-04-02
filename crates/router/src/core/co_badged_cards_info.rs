use common_enums::enums;
use common_utils::{errors::CustomResult, ext_traits::StringExt};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    co_badged_cards_info::{CoBadgedCardInfo, CoBadgedCardInfoResponse},
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
};
use num_traits::cast::ToPrimitive;

use super::payments::OperationSessionGetters;
use crate::{
    configs::settings, core::errors, routes::SessionState, services::logger, types::domain,
};

pub struct CoBadgedCardInfoList(Vec<CoBadgedCardInfo>);

impl CoBadgedCardInfoList {
    fn pad_card_number_to_19_digit(card_number: cards::CardNumber) -> String {
        let card_number = card_number.get_card_isin();

        format!("{:0<19}", card_number)
    }
    pub fn is_valid_length(&self) -> bool {
        if self.0.len() < 2 {
            logger::debug!("Invalid co-badged network list length");
            false
        } else {
            logger::debug!("Valid co-badged network list length");
            true
        }
    }

    pub fn filter_cards(self) -> Self {
        logger::debug!(
            "Filtering co-badged cards, Total cards before filtering: {}",
            self.0.len()
        );

        let filtered_cards: Vec<CoBadgedCardInfo> = self
            .0
            .into_iter()
            .filter(|card| {
                card.card_type == enums::CardType::Debit
                    && card.pan_or_token == enums::PanOrToken::Pan
                    && !card.prepaid
            })
            .collect();

        logger::debug!(
            "Filtering complete. Total cards after filtering: {}",
            filtered_cards.len()
        );

        Self(filtered_cards)
    }

    pub fn has_same_issuer(&self) -> CustomResult<bool, errors::ApiErrorResponse> {
        let first_element = self
            .0
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("The filtered co-badged card info list is empty")?;

        let first_issuer = &first_element.issuing_bank_name;
        let has_same_issuer = self
            .0
            .iter()
            .all(|card| &card.issuing_bank_name == first_issuer);
        Ok(has_same_issuer)
    }

    pub fn is_local_transaction(
        &self,
        acquirer_country: enums::CountryAlpha2,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        logger::debug!("Validating if the transaction is local or international");

        let first_element = self
            .0
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("The filtered co-badged card info list is empty")?;

        let issuer_country = first_element.country;
        Ok(acquirer_country == issuer_country)
    }

    pub fn extract_networks(&self) -> Vec<enums::CardNetwork> {
        self.0
            .iter()
            .map(|card| card.card_network.clone())
            .collect()
    }

    pub fn get_co_badged_cards_info_response(
        &self,
    ) -> CustomResult<CoBadgedCardInfoResponse, errors::ApiErrorResponse> {
        logger::debug!("Constructing co-badged card info response");

        let first_element = self
            .0
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("The filtered co-badged card info list is empty")?;

        Ok(CoBadgedCardInfoResponse {
            card_networks: self.extract_networks(),
            issuer_country: first_element.country,
            regulated: first_element.regulated,
            regulated_name: first_element.regulated_name.clone(),
        })
    }
}

// should we check for a case where at-least one network is international network
pub async fn get_co_badged_cards_info(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    card_number: cards::CardNumber,
    acquirer_country: enums::CountryAlpha2,
) -> CustomResult<Option<CoBadgedCardInfoResponse>, errors::ApiErrorResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &state.into();

    // pad the card number to 19 digits to match the co-badged card bin length
    let card_number_str = CoBadgedCardInfoList::pad_card_number_to_19_digit(card_number);

    let parsed_number: i64 = card_number_str
        .parse::<i64>()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Failed to convert card number to integer in co-badged cards info flow",
        )?;

    let co_badged_card_infos_record = db
        .find_co_badged_cards_info_by_card_bin(key_manager_state, key_store, parsed_number)
        .await;

    let filtered_co_badged_card_info_list_optional = match co_badged_card_infos_record {
        Err(error) => {
            if error.current_context().is_db_not_found() {
                logger::debug!("co-badged card info record not found {:?}", error);
                Ok(None)
            } else {
                Err(error)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("error while fetching co-badged card info record")
            }
        }
        Ok(co_badged_card_infos) => {
            logger::debug!("co-badged card info record retrieved successfully");
            let co_badged_card_infos_list = CoBadgedCardInfoList(co_badged_card_infos);

            let filtered_list_optional = co_badged_card_infos_list
                .is_valid_length()
                .then(|| co_badged_card_infos_list.filter_cards())
                .and_then(|filtered_co_badged_card_infos_list| {
                    filtered_co_badged_card_infos_list
                        .is_valid_length()
                        .then_some(filtered_co_badged_card_infos_list)
                });

            filtered_list_optional
                .and_then(|filtered_list| {
                    filtered_list
                        .is_local_transaction(acquirer_country)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Failed to check if the transaction is local or international",
                        )
                        .map(|is_local_transaction| is_local_transaction.then_some(filtered_list))
                        .transpose()
                })
                .transpose()
        }
    }?;

    let co_badged_cards_info_response = filtered_co_badged_card_info_list_optional
        .map(|filtered_co_badged_card_info_lis| {
            filtered_co_badged_card_info_lis.get_co_badged_cards_info_response()
        })
        .transpose()
        .attach_printable("Failed to construct co-badged card info response")?;

    Ok(co_badged_cards_info_response)
}

pub fn calculate_interchange_fee(
    network: &enums::CardNetwork,
    is_regulated: &bool,
    regulated_name: Option<&String>,
    amount: f64,
    debit_routing: &settings::DebitRoutingConfig,
) -> CustomResult<f64, errors::ApiErrorResponse> {
    logger::debug!("Calculating interchange fee");
    let fee_data = if *is_regulated {
        logger::debug!("Regulated bank");
        &debit_routing.interchange_fee.regulated
    } else {
        logger::debug!("Non regulated bank");
        debit_routing
            .interchange_fee
            .non_regulated
            .0
            .get("merchant_category_code_0001")
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "interchange fee for merchant category code",
            })?
            .get(network)
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "interchange fee for non regulated",
            })
            .attach_printable(
                "Failed to fetch interchange fee for non regulated banks in debit routing",
            )?
    };

    let percentage = fee_data.percentage;

    let fixed_amount = fee_data.fixed_amount;

    let mut total_interchange_fee = (amount * percentage / 100.0) + fixed_amount;

    if *is_regulated {
        if let Some(regulated_name_string) = regulated_name {
            let regulated_name_enum: enums::RegulatedName = regulated_name_string
                .clone()
                .parse_enum("RegulatedName")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse regulated name")?;
            match regulated_name_enum {
                enums::RegulatedName::ExemptFraud => {
                    logger::debug!("Regulated bank with exemption for fraud");
                }
                enums::RegulatedName::NonExemptWithFraud => {
                    logger::debug!("Regulated bank with non exemption for fraud");
                    let fraud_check_fee = debit_routing.fraud_check_fee;

                    total_interchange_fee += fraud_check_fee
                }
            };
        }
    };

    Ok(total_interchange_fee)
}

pub fn calculate_network_fee(
    network: &enums::CardNetwork,
    amount: f64,
    debit_routing: &settings::DebitRoutingConfig,
) -> CustomResult<f64, errors::ApiErrorResponse> {
    logger::debug!("Calculating network fee");
    let fee_data = debit_routing
        .network_fee
        .get(network)
        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "interchange fee for non regulated",
        })
        .attach_printable(
            "Failed to fetch interchange fee for non regulated banks in debit routing",
        )?;
    let percentage = fee_data.percentage;
    let fixed_amount = fee_data.fixed_amount;
    let total_network_fee = (amount * percentage / 100.0) + fixed_amount;
    Ok(total_network_fee)
}

pub fn calculate_total_fees_per_network(
    state: &SessionState,
    co_badged_cards_info: CoBadgedCardInfoResponse,
    amount: f64,
) -> CustomResult<Option<Vec<(enums::CardNetwork, f64)>>, errors::ApiErrorResponse> {
    logger::debug!("Calculating total fees per network");
    let debit_routing_config = &state.conf.debit_routing_config;

    co_badged_cards_info
        .card_networks
        .into_iter()
        .map(|network| {
            let interchange_fee = calculate_interchange_fee(
                &network,
                &co_badged_cards_info.regulated,
                co_badged_cards_info.regulated_name.as_ref(),
                amount,
                debit_routing_config,
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to calculate debit routing interchange_fee")?;

            let network_fee = calculate_network_fee(&network, amount, debit_routing_config)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to calculate debit routing network_fee")?;

            let total_fee = interchange_fee + network_fee;
            logger::debug!(
                "Total fee for network {} is {}",
                network.to_string(),
                total_fee
            );
            Ok(Some((network, total_fee)))
        })
        .collect::<CustomResult<Option<Vec<(enums::CardNetwork, f64)>>, errors::ApiErrorResponse>>()
}

pub fn sort_networks_by_fee(
    network_fees: Vec<(enums::CardNetwork, f64)>,
) -> Vec<enums::CardNetwork> {
    logger::debug!("Sorting networks by fee");
    let mut sorted_fees = network_fees;
    sorted_fees.sort_by(|(_network1, fee1), (_network2, fee2)| fee1.total_cmp(fee2));

    sorted_fees
        .into_iter()
        .map(|(network, _fee)| network)
        .collect()
}

pub fn request_validation(
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    debit_routing_config: &settings::DebitRoutingConfig,
) -> bool {
    logger::debug!("Validating request for debit routing");
    let is_currency_supported = payment_intent.currency.map(|currency| {
        debit_routing_config
            .supported_currencies
            .contains(&currency)
    });

    payment_intent.setup_future_usage != Some(enums::FutureUsage::OffSession)
        && payment_intent.amount.get_amount_as_i64() > 0
        && is_currency_supported == Some(true)
        && payment_attempt.authentication_type != Some(enums::AuthenticationType::ThreeDs)
        && payment_attempt.payment_method == Some(enums::PaymentMethod::Card)
        && payment_attempt.payment_method_type == Some(enums::PaymentMethodType::Debit)
}

pub async fn get_sorted_co_badged_networks_by_fee<F: Clone, D: OperationSessionGetters<F>>(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_data: &D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<Vec<enums::CardNetwork>> {
    logger::debug!("Fetching sorted card networks based on their respective network fees");

    // for saved card payments we need to check the supported network in the payment method info

    let payment_method_data_optional = payment_data.get_payment_method_data();
    let payment_intent = payment_data.get_payment_intent();

    if let Some(hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card)) =
        payment_method_data_optional
    {
        let co_badged_card_info =
            get_co_badged_cards_info(state, key_store, card.card_number.clone(), acquirer_country)
                .await
                .map_err(|error| {
                    logger::warn!(?error, "Failed to calculate total fees per network");
                })
                .ok()
                .flatten();

        if let Some(card_info) = co_badged_card_info {
            let amount_f64_optional = payment_intent.amount.get_amount_as_i64().to_f64();

            // Calculate total fees per network within this scope
            if let Some(amount_f64) = amount_f64_optional {
                let cost_calculated_network =
                    calculate_total_fees_per_network(state, card_info, amount_f64)
                        .map_err(|error| {
                            logger::warn!(?error, "Failed to calculate total fees per network");
                        })
                        .ok()
                        .flatten();

                if let Some(networks) = cost_calculated_network {
                    return Some(sort_networks_by_fee(networks));
                }
            }
        }
    }
    None
}

pub async fn check_for_debit_routing_connector_in_profile<
    F: Clone,
    D: OperationSessionGetters<F>,
>(
    state: &SessionState,
    business_profile_id: &common_utils::id_type::ProfileId,
    payment_data: &D,
) -> CustomResult<bool, errors::ApiErrorResponse> {
    logger::debug!("Checking for debit routing connector in profile");
    let debit_routing_supported_connectors =
        state.conf.debit_routing_config.supported_connectors.clone();

    let transaction_data = super::routing::PaymentsDslInput::new(
        payment_data.get_setup_mandate(),
        payment_data.get_payment_attempt(),
        payment_data.get_payment_intent(),
        payment_data.get_payment_method_data(),
        payment_data.get_address(),
        payment_data.get_recurring_details(),
        payment_data.get_currency(),
    );

    let fallback_config_optional = super::routing::helpers::get_merchant_default_config(
        &*state.clone().store,
        business_profile_id.get_string_repr(),
        &enums::TransactionType::from(&super::routing::TransactionData::Payment(transaction_data)),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .map_err(|error| {
        logger::warn!(?error, "Failed to fetch default connector for a profile");
    })
    .ok();

    let is_debit_routable_connector_present = fallback_config_optional
        .map(|fallback_config| {
            fallback_config.iter().any(|fallback_config_connector| {
                debit_routing_supported_connectors.contains(&api_models::enums::Connector::from(
                    fallback_config_connector.connector,
                ))
            })
        })
        .unwrap_or(false);

    Ok(is_debit_routable_connector_present)
}
