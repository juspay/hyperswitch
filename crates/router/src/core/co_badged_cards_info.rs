use common_enums::enums;
use common_utils::{errors::CustomResult, ext_traits::StringExt};
use error_stack::ResultExt;
use hyperswitch_domain_models::co_badged_cards_info::{CoBadgedCardInfo, CoBadgedCardInfoResponse};

use crate::{configs::settings, core::errors, routes, services::logger, types::domain};

pub struct CoBadgedCardInfoList(Vec<CoBadgedCardInfo>);

impl CoBadgedCardInfoList {
    fn pad_card_number_to_16_digit(card_number: cards::CardNumber) -> String {
        let card_number = card_number.get_card_isin();
        format!("{:0>19}", card_number)
    }
    pub fn is_valid_length(&self) -> bool {
        if self.0.len() < 2 {
            logger::debug!("co-badged cards list length is less than 2");
            false
        } else {
            true
        }
    }

    pub fn filter_cards(self) -> Self {
        let filtered_cards = self
            .0
            .into_iter()
            .filter(|card| {
                !(card.card_type == enums::CardType::Debit
                    && card.pan_or_token == enums::PanOrToken::Pan
                    && !card.prepaid)
            })
            .collect();
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

    pub fn extract_networks(&self) -> Vec<enums::CardNetwork> {
        self.0
            .iter()
            .map(|card| card.card_network.clone())
            .collect()
    }

    pub fn is_regulated(&self) -> CustomResult<bool, errors::ApiErrorResponse> {
        let first_element = self
            .0
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("The filtered co-badged card info list is empty")?;
        Ok(first_element.regulated)
    }

    pub fn get_regulated_name(&self) -> CustomResult<Option<String>, errors::ApiErrorResponse> {
        let first_element = self
            .0
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("The filtered co-badged card info list is empty")?;
        Ok(first_element.regulated_name.clone())
    }

    pub fn get_co_badged_cards_info_response(
        &self,
    ) -> CustomResult<CoBadgedCardInfoResponse, errors::ApiErrorResponse> {
        Ok(CoBadgedCardInfoResponse {
            card_networks: self.extract_networks(),
            regulated: self.is_regulated()?,
            regulated_name: self.get_regulated_name()?,
        })
    }
}

pub async fn get_co_badged_cards_info(
    state: routes::SessionState,
    key_store: domain::MerchantKeyStore,
    card_number: cards::CardNumber,
) -> CustomResult<Option<CoBadgedCardInfoResponse>, errors::ApiErrorResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    // The card number is padded to 16 digits as the co-badged card bin info is stored as a 16 digit number
    let card_number_str = CoBadgedCardInfoList::pad_card_number_to_16_digit(card_number);

    let parsed_number: i64 = card_number_str
        .parse::<i64>()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Failed to convert card number to integer in co-badged cards info flow",
        )?;

    let co_badged_card_infos_record = db
        .find_co_badged_cards_info_by_card_bin(key_manager_state, &key_store, parsed_number)
        .await;

    let filtered_co_badged_card_info_list_optional = match co_badged_card_infos_record {
        Err(error) => {
            if error.current_context().is_db_not_found() {
                logger::debug!("co-badged card info record not found");
                Ok(None)
            } else {
                Err(error)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("error while fetching co-badged card info record")
            }
        }
        Ok(co_badged_card_infos) => {
            let co_badged_card_infos_list = CoBadgedCardInfoList(co_badged_card_infos);

            co_badged_card_infos_list
                .is_valid_length()
                .then(|| co_badged_card_infos_list.filter_cards())
                .and_then(|filtered_co_badged_card_infos_list| {
                    filtered_co_badged_card_infos_list
                        .is_valid_length()
                        .then_some(Ok(filtered_co_badged_card_infos_list))
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
    let fee_data = if *is_regulated {
        &debit_routing.interchange_fee.regulated
    } else {
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
                enums::RegulatedName::NonExemptWithFraud => {
                    logger::debug!("Regulated bank with non exemption for fraud");
                }
                enums::RegulatedName::ExemptFraud => {
                    logger::debug!("Regulated bank with exemption for fraud");
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
    co_badged_cards_info_optional: Option<CoBadgedCardInfoResponse>,
    state: routes::SessionState,
    amount: f64,
) -> CustomResult<Option<Vec<(enums::CardNetwork, f64)>>, errors::ApiErrorResponse> {
    let debit_routing_config = &state.conf.debit_routing_config;

    co_badged_cards_info_optional
        .map(|co_badged_cards_info| {
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
                    Ok(Some((network, total_fee)))
                })
                .collect::<CustomResult<Option<Vec<(enums::CardNetwork, f64)>>, errors::ApiErrorResponse>>()
        })
        .unwrap_or_else(|| Ok(None))
}

pub fn sort_networks_by_fee(
    network_fees: Vec<(enums::CardNetwork, f64)>,
) -> Vec<enums::CardNetwork> {
    let mut sorted_fees = network_fees;
    sorted_fees.sort_by(|(_network1, fee1), (_network2, fee2)| fee1.total_cmp(fee2));

    sorted_fees
        .into_iter()
        .map(|(network, _fee)| network)
        .collect()
}
