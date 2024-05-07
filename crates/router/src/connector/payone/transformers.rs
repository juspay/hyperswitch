#[cfg(feature = "payouts")]
use api_models::payouts::PayoutMethodData;
use cards::CardNumber;
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{get_unimplemented_payment_method_error_message, CardIssuer, CARD_REGEX},
    utils::OptionExt,
};

type Error = error_stack::Report<errors::ConnectorError>;

#[cfg(feature = "payouts")]
use crate::{
    connector::utils::RouterData,
    core::errors,
    logger,
    types::{self, storage::enums as storage_enums, transformers::ForeignFrom},
};

pub struct PayoneRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for PayoneRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub timestamp: Option<String>,
    pub errors: Option<Vec<SubError>>,
    pub status: Option<i32>,
    pub error: Option<String>,
    pub error_description: Option<String>,
    pub message: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubError {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
    pub field: Option<String>,
}

// Auth Struct
pub struct PayoneAuthType {
    pub(super) api_key: Secret<String>,
    #[allow(dead_code)]
    pub(super) merchant_account: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for PayoneAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayonePayoutFulfillRequest {
    amount_of_money: AmountOfMoney,
    card_payout_method_specific_input: CardPayoutMethodSpecificInput,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmountOfMoney {
    amount: i64,
    currency_code: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPayoutMethodSpecificInput {
    card: Card,
    payment_product_id: i32,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    card_holder_name: Secret<String>,
    card_number: CardNumber,
    expiry_date: Secret<String>,
}

impl Card {
    fn get_card_issuer(&self) -> Result<CardIssuer, Error> {
        for (k, v) in CARD_REGEX.iter() {
            let regex: Regex = v
                .clone()
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            if regex.is_match(self.card_number.clone().get_card_no().as_str()) {
                return Ok(*k);
            }
        }
        Err(error_stack::Report::new(
            errors::ConnectorError::NotImplemented("Card Type".into()),
        ))
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmnichannelPayoutSpecificInput {
    payment_id: String,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for PayonePayoutFulfillRequest {
    type Error = Error;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        match request.payout_type.to_owned() {
            storage_enums::PayoutType::Card => {
                let amount_of_money: AmountOfMoney = AmountOfMoney {
                    amount: item.request.amount,
                    currency_code: item.request.destination_currency.to_string(),
                };
                let card = Card::try_from(&item.get_payout_method_data()?)?;

                let card_payout_method_specific_input: CardPayoutMethodSpecificInput =
                    CardPayoutMethodSpecificInput {
                        #[allow(clippy::as_conversions)]
                        payment_product_id: Gateway::try_from(card.get_card_issuer()?)? as i32,
                        card,
                    };
                Ok(Self {
                    amount_of_money,
                    card_payout_method_specific_input,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Payone"),
            ))?,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum Gateway {
    Visa = 1,
    MasterCard = 3,
}

impl TryFrom<CardIssuer> for Gateway {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(issuer: CardIssuer) -> Result<Self, Self::Error> {
        logger::debug!("it is in PayoutCreateRequest Gateway debug");
        match issuer {
            CardIssuer::Master => Ok(Self::MasterCard),
            CardIssuer::Visa => Ok(Self::Visa),
            _ => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("payone"),
            )
            .into()),
        }
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<&PayoutMethodData> for Card {
    type Error = Error;
    fn try_from(item: &PayoutMethodData) -> Result<Self, Self::Error> {
        match item {
            PayoutMethodData::Card(card) => Ok(Self {
                card_number: card.card_number.clone(),
                card_holder_name: card
                    .card_holder_name
                    .clone()
                    .get_required_value("card_holder_name")
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "payout_method_data.card.holder_name",
                    })?,
                expiry_date: match card.get_expiry_date_as_mmyy() {
                    Ok(date) => {
                        logger::debug!("date date {}", date.clone().expose());
                        date
                    }
                    Err(_) => Err(errors::ConnectorError::MissingRequiredField {
                        field_name: "payout_method_data.card.expiry_date",
                    })?,
                },
            }),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payout_method_data.card",
            })?,
        }
    }
}
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayoneErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayoneStatus {
    Created,
    #[default]
    PendingApproval,
    Rejected,
    PayoutRequested,
    AccountCredited,
    RejectedCredit,
    Cancelled,
    Reversed,
}
#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoneTransferDetails {
    transfer_purpose: Option<String>,
    source_of_funds: Option<String>,
    transfer_purpose_sub_transfer_purpose: Option<String>,
}

#[allow(dead_code)]
#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayonePayoutFulfillResponse {
    id: String,
    payout_output: PayoutOutput,
    status: PayoneStatus,
}

#[allow(dead_code)]
#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutOutput {
    amount_of_money: AmountOfMoney,
}

#[cfg(feature = "payouts")]
impl ForeignFrom<PayoneStatus> for storage_enums::PayoutStatus {
    fn foreign_from(payone_status: PayoneStatus) -> Self {
        match payone_status {
            PayoneStatus::AccountCredited => Self::Success,
            PayoneStatus::RejectedCredit | PayoneStatus::Rejected => Self::Cancelled,
            PayoneStatus::Cancelled | PayoneStatus::Reversed => Self::Cancelled,
            PayoneStatus::Created
            | PayoneStatus::PendingApproval
            | PayoneStatus::PayoutRequested => Self::Pending,
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, PayonePayoutFulfillResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, PayonePayoutFulfillResponse>,
    ) -> Result<Self, Self::Error> {
        let response: PayonePayoutFulfillResponse = item.response;

        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(storage_enums::PayoutStatus::foreign_from(response.status)),
                connector_payout_id: "".to_string(),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
            }),
            ..item.data
        })
    }
}
