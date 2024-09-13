#[cfg(feature = "payouts")]
use api_models::payouts::PayoutMethodData;
#[cfg(feature = "payouts")]
use cards::CardNumber;
use common_utils::types::MinorUnit;
#[cfg(feature = "payouts")]
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::connector::{
    utils,
    utils::{get_unimplemented_payment_method_error_message, CardIssuer},
};

#[cfg(feature = "payouts")]
type Error = error_stack::Report<errors::ConnectorError>;

#[cfg(feature = "payouts")]
use crate::{
    connector::utils::{CardData, PayoutsData, RouterData},
    core::errors,
    types::{self, api, storage::enums as storage_enums, transformers::ForeignFrom},
    utils::OptionExt,
};
#[cfg(not(feature = "payouts"))]
use crate::{core::errors, types};

pub struct PayoneRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PayoneRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub errors: Option<Vec<SubError>>,
    pub error_id: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SubError {
    pub code: String,
    pub message: String,
    pub http_status_code: u16,
}

impl From<SubError> for utils::ErrorCodeAndMessage {
    fn from(error: SubError) -> Self {
        Self {
            error_code: error.code.to_string(),
            error_message: error.code.to_string(),
        }
    }
}
// Auth Struct
pub struct PayoneAuthType {
    pub(super) api_key: Secret<String>,
    pub merchant_account: Secret<String>,
    pub api_secret: Secret<String>,
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
    amount: MinorUnit,
    currency_code: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPayoutMethodSpecificInput {
    card: Card,
    payment_product_id: PaymentProductId,
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
impl TryFrom<PayoneRouterData<&types::PayoutsRouterData<api::PoFulfill>>>
    for PayonePayoutFulfillRequest
{
    type Error = Error;
    fn try_from(
        item: PayoneRouterData<&types::PayoutsRouterData<api::PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        let request = item.router_data.request.to_owned();
        let payout_type = request.get_payout_type()?;
        match payout_type {
            storage_enums::PayoutType::Card => {
                let amount_of_money: AmountOfMoney = AmountOfMoney {
                    amount: item.amount,
                    currency_code: item.router_data.request.destination_currency.to_string(),
                };
                let card_payout_method_specific_input = match item
                    .router_data
                    .get_payout_method_data()?
                {
                    PayoutMethodData::Card(card_data) => CardPayoutMethodSpecificInput {
                        card: Card {
                            card_number: card_data.card_number.clone(),
                            card_holder_name: card_data
                                .card_holder_name
                                .clone()
                                .get_required_value("card_holder_name")
                                .change_context(errors::ConnectorError::MissingRequiredField {
                                    field_name: "payout_method_data.card.holder_name",
                                })?,
                            expiry_date: card_data
                                .get_card_expiry_month_year_2_digit_with_delimiter(
                                    "".to_string(),
                                )?,
                        },
                        payment_product_id: PaymentProductId::try_from(
                            card_data.get_card_issuer()?,
                        )?,
                    },
                    PayoutMethodData::Bank(_) | PayoutMethodData::Wallet(_) => {
                        Err(errors::ConnectorError::NotImplemented(
                            get_unimplemented_payment_method_error_message("Payone"),
                        ))?
                    }
                };
                Ok(Self {
                    amount_of_money,
                    card_payout_method_specific_input,
                })
            }
            storage_enums::PayoutType::Wallet | storage_enums::PayoutType::Bank => {
                Err(errors::ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Payone"),
                ))?
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(i32)]
pub enum PaymentProductId {
    Visa = 1,
    MasterCard = 3,
}

impl TryFrom<CardIssuer> for PaymentProductId {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(issuer: CardIssuer) -> Result<Self, Self::Error> {
        match issuer {
            CardIssuer::Master => Ok(Self::MasterCard),
            CardIssuer::Visa => Ok(Self::Visa),
            CardIssuer::AmericanExpress
            | CardIssuer::Maestro
            | CardIssuer::Discover
            | CardIssuer::DinersClub
            | CardIssuer::JCB
            | CardIssuer::CarteBlanche => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("payone"),
            )
            .into()),
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
            PayoneStatus::RejectedCredit | PayoneStatus::Rejected => Self::Failed,
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
                connector_payout_id: Some(response.id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}
