use crate::{core::errors, routes, services::logger, types::domain};
use common_enums::enums;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;

use hyperswitch_domain_models::co_badged_cards_info::CoBadgedCardInfo;

pub struct CoBadgedCardInfoList(Vec<CoBadgedCardInfo>);

impl CoBadgedCardInfoList {
    fn pad_card_number_to_16_digit(card_number: cards::CardNumber) -> String {
        let card_number = card_number.to_string();
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

    pub fn has_same_issuer(&self) -> bool {
        let first_issuer = &self.0[0].issuing_bank_name;
        self.0
            .iter()
            .all(|card| &card.issuing_bank_name == first_issuer)
    }

    pub fn extract_networks(&self) -> Vec<enums::CardNetwork> {
        self.0
            .iter()
            .map(|card| card.card_network.clone())
            .collect()
    }

    pub fn is_regulated(&self) -> bool {
        self.0[0].regulated
    }
}

pub async fn get_co_badged_cards_info(
    state: routes::SessionState,
    key_store: domain::MerchantKeyStore,
    card_number: cards::CardNumber,
) -> CustomResult<Option<(Vec<enums::CardNetwork>, bool)>, errors::ApiErrorResponse> {
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

    let co_badged_card_infos = match co_badged_card_infos_record {
        Err(error) => {
            if error.current_context().is_db_not_found() {
                Err(error).change_context(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "co-badged card info not found".to_string(),
                })?
            } else {
                return Err(error)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("error while fetching co-badged card info record");
            }
        }
        Ok(co_badged_card_infos) => co_badged_card_infos,
    };

    let co_badged_list = CoBadgedCardInfoList(co_badged_card_infos);

    let result = if co_badged_list.is_valid_length() {
        let filtered_co_badged_list = co_badged_list.filter_cards();

        if filtered_co_badged_list.is_valid_length() && filtered_co_badged_list.has_same_issuer() {
            Some((
                filtered_co_badged_list.extract_networks(),
                filtered_co_badged_list.is_regulated(),
            ))
        } else {
            None
        }
    } else {
        None
    };

    Ok(result)
}
