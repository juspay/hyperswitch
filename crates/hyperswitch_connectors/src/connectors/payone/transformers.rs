#[cfg(feature = "payouts")]
use api_models::payouts::PayoutMethodData;
#[cfg(feature = "payouts")]
use cards::CardNumber;
#[cfg(feature = "payouts")]
use common_enums::{PayoutStatus, PayoutType};
use common_utils::types::MinorUnit;
#[cfg(feature = "payouts")]
use common_utils::{ext_traits::OptionExt, transformers::ForeignFrom};
#[cfg(feature = "payouts")]
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::PoFulfill,
    types::{PayoutsResponseData, PayoutsRouterData},
};
use hyperswitch_interfaces::errors::ConnectorError;
use masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(feature = "payouts")]
use crate::utils::CardData as _;
use crate::utils::{
    get_unimplemented_payment_method_error_message, CardIssuer, ErrorCodeAndMessage,
};
#[cfg(feature = "payouts")]
use crate::{
    types::PayoutsResponseRouterData,
    utils::{PayoutsData, RouterData},
};
#[cfg(feature = "payouts")]
type Error = error_stack::Report<ConnectorError>;
use serde_repr::{Deserialize_repr, Serialize_repr};
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

impl From<SubError> for ErrorCodeAndMessage {
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

impl TryFrom<&ConnectorAuthType> for PayoneAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType)?,
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
impl TryFrom<PayoneRouterData<&PayoutsRouterData<PoFulfill>>> for PayonePayoutFulfillRequest {
    type Error = Error;
    fn try_from(
        item: PayoneRouterData<&PayoutsRouterData<PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        let request = item.router_data.request.to_owned();
        let payout_type = request.get_payout_type()?;
        match payout_type {
            PayoutType::Card => {
                let amount_of_money: AmountOfMoney = AmountOfMoney {
                    amount: item.amount,
                    currency_code: item.router_data.request.destination_currency.to_string(),
                };
                let card_payout_method_specific_input =
                    match item.router_data.get_payout_method_data()? {
                        PayoutMethodData::Card(card_data) => CardPayoutMethodSpecificInput {
                            card: Card {
                                card_number: card_data.card_number.clone(),
                                card_holder_name: card_data
                                    .card_holder_name
                                    .clone()
                                    .get_required_value("card_holder_name")
                                    .change_context(ConnectorError::MissingRequiredField {
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
                        PayoutMethodData::Bank(_)
                        | PayoutMethodData::Wallet(_)
                        | PayoutMethodData::BankRedirect(_)
                        | PayoutMethodData::Passthrough(_) => Err(ConnectorError::NotImplemented(
                            get_unimplemented_payment_method_error_message("Payone"),
                        ))?,
                    };
                Ok(Self {
                    amount_of_money,
                    card_payout_method_specific_input,
                })
            }
            PayoutType::Wallet | PayoutType::Bank | PayoutType::BankRedirect => {
                Err(ConnectorError::NotImplemented(
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
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(issuer: CardIssuer) -> Result<Self, Self::Error> {
        match issuer {
            CardIssuer::Master => Ok(Self::MasterCard),
            CardIssuer::Visa => Ok(Self::Visa),
            CardIssuer::AmericanExpress
            | CardIssuer::Maestro
            | CardIssuer::Discover
            | CardIssuer::DinersClub
            | CardIssuer::JCB
            | CardIssuer::CarteBlanche
            | CardIssuer::UnionPay
            | CardIssuer::CartesBancaires => Err(ConnectorError::NotImplemented(
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
impl ForeignFrom<PayoneStatus> for PayoutStatus {
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
impl<F> TryFrom<PayoutsResponseRouterData<F, PayonePayoutFulfillResponse>>
    for PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: PayoutsResponseRouterData<F, PayonePayoutFulfillResponse>,
    ) -> Result<Self, Self::Error> {
        let response: PayonePayoutFulfillResponse = item.response;

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::foreign_from(response.status)),
                connector_payout_id: Some(response.id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}
