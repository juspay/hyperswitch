#[cfg(feature = "payouts")]
use api_models::payouts::PayoutMethodData;
#[cfg(feature = "payouts")]
use cards::CardNumber;
#[cfg(feature = "payouts")]
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "payouts"))]
use crate::connector::utils;
use crate::connector::utils::{get_unimplemented_payment_method_error_message, CardIssuer};

#[cfg(feature = "payouts")]
type Error = error_stack::Report<errors::ConnectorError>;

#[cfg(feature = "payouts")]
use crate::{
    connector::utils::{self, CardData, RouterData},
    core::errors,
    types::{self, api, storage::enums as storage_enums, transformers::ForeignFrom},
    utils::OptionExt,
};
#[cfg(not(feature = "payouts"))]
use crate::{core::errors, types::{self,api,storage::enums as storage_enums}};

pub struct PayoneRouterData<T> {
    pub amount:String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &api::CurrencyUnit,
        storage_enums::Currency,
        i64,
        T,
    )> for PayoneRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &api::CurrencyUnit,
            storage_enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(_currency_unit, amount, _currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub errors: Vec<SubError>,
    pub error_id: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubError {
    pub code: String,
    pub message: String,
    pub http_status_code: u16,
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
        match auth_type {
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
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

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OmnichannelPayoutSpecificInput {
    payment_id: String,
}
#[cfg(feature = "payouts")]
pub struct CardAndCardIssuer(Card, CardIssuer);

#[cfg(feature = "payouts")]
impl TryFrom<PayoneRouterData<&types::PayoutsRouterData<api::PoFulfill>>>
    for PayonePayoutFulfillRequest
{
    type Error = Error;
    fn try_from(
        item: PayoneRouterData<&types::PayoutsRouterData<api::PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        let request = item.router_data.request.to_owned();
        match request.payout_type.to_owned() {
            storage_enums::PayoutType::Card => {
                let amount_of_money: AmountOfMoney = AmountOfMoney {
                    amount: item.amount.parse::<i64>().change_context(errors::ConnectorError::ParsingFailed)?,
                    currency_code: item.router_data.request.destination_currency.to_string(),
                };
                let card_issuer =
                    CardAndCardIssuer::try_from(&item.router_data.get_payout_method_data()?)?;
                let card = card_issuer.0;
                let card_issuer = card_issuer.1;

                let card_payout_method_specific_input: CardPayoutMethodSpecificInput =
                    CardPayoutMethodSpecificInput {
                        #[allow(clippy::as_conversions)]
                        payment_product_id: Gateway::try_from(card_issuer)? as i32,
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
impl TryFrom<&PayoutMethodData> for CardAndCardIssuer {
    type Error = Error;
    fn try_from(item: &PayoutMethodData) -> Result<Self, Self::Error> {
        match item {
            PayoutMethodData::Card(card) => Ok(Self(
                Card {
                    card_number: card.card_number.clone(),
                    card_holder_name: card
                        .card_holder_name
                        .clone()
                        .get_required_value("card_holder_name")
                        .change_context(errors::ConnectorError::MissingRequiredField {
                            field_name: "payout_method_data.card.holder_name",
                        })?,
                    expiry_date: match card.get_card_expiry_month_year_2_digit_with_delimiter("".to_string()) {
                        Ok(date) => date,
                        Err(_) => Err(errors::ConnectorError::MissingRequiredField {
                            field_name: "payout_method_data.card.expiry_date",
                        })?,
                    },
                },
                card.get_card_issuer()?,
            )),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payout_method_data.card",
            })?,
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayonePayoutFulfillResponse {
    id: String,
    payout_output: PayoutOutput,
    status: PayoneStatus,
}

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
            PayoneStatus::RejectedCredit | PayoneStatus::Rejected |
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
                connector_payout_id: response.id,
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
            }),
            ..item.data
        })
    }
}
