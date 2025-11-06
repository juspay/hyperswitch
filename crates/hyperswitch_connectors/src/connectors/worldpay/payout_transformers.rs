use base64::Engine;
use common_enums::enums;
use common_utils::{consts::BASE64_ENGINE, pii, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::ConnectorAuthType, router_flow_types::payouts::PoFulfill,
    router_response_types::PayoutsResponseData, types,
};
use hyperswitch_interfaces::{api, errors};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use super::{payout_requests::*, payout_response::*};
use crate::{
    types::PayoutsResponseRouterData,
    utils::{self, CardData, RouterData as RouterDataTrait},
};

#[derive(Debug, Serialize)]
pub struct WorldpayPayoutRouterData<T> {
    amount: i64,
    router_data: T,
}
impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, MinorUnit, T)>
    for WorldpayPayoutRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, minor_amount, item): (
            &api::CurrencyUnit,
            enums::Currency,
            MinorUnit,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: minor_amount.get_amount_as_i64(),
            router_data: item,
        })
    }
}

pub struct WorldpayPayoutAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) entity_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for WorldpayPayoutAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => {
                let auth_key = format!("{}:{}", key1.peek(), api_key.peek());
                let auth_header = format!("Basic {}", BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: Secret::new(auth_header),
                    entity_id: api_secret.clone(),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpayPayoutConnectorMetadataObject {
    pub merchant_name: Option<Secret<String>>,
}

impl TryFrom<Option<&pii::SecretSerdeValue>> for WorldpayPayoutConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: Option<&pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.cloned())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

impl<F>
    TryFrom<(
        &WorldpayPayoutRouterData<&types::PayoutsRouterData<F>>,
        &Secret<String>,
    )> for WorldpayPayoutRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        req: (
            &WorldpayPayoutRouterData<&types::PayoutsRouterData<F>>,
            &Secret<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, entity_id) = req;

        let worldpay_connector_metadata_object: WorldpayPayoutConnectorMetadataObject =
            WorldpayPayoutConnectorMetadataObject::try_from(
                item.router_data.connector_meta_data.as_ref(),
            )?;

        let merchant_name = worldpay_connector_metadata_object.merchant_name.ok_or(
            errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata.merchant_name",
            },
        )?;

        Ok(Self {
            transaction_reference: item.router_data.connector_request_reference_id.clone(),
            merchant: Merchant {
                entity: entity_id.clone(),
            },
            instruction: PayoutInstruction {
                value: PayoutValue {
                    amount: item.amount,
                    currency: item.router_data.request.destination_currency,
                },
                narrative: InstructionNarrative {
                    line1: merchant_name,
                },
                payout_instrument: PayoutInstrument::try_from(
                    item.router_data.get_payout_method_data()?,
                )?,
            },
        })
    }
}

impl TryFrom<api_models::payouts::PayoutMethodData> for PayoutInstrument {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        payout_method_data: api_models::payouts::PayoutMethodData,
    ) -> Result<Self, Self::Error> {
        match payout_method_data {
            api_models::payouts::PayoutMethodData::Wallet(
                api_models::payouts::Wallet::ApplePayDecrypt(apple_pay_decrypted_data),
            ) => Ok(Self::ApplePayDecrypt(ApplePayDecrypt {
                payout_type: PayoutType::ApplePayDecrypt,
                dpan: apple_pay_decrypted_data.dpan.clone(),
                card_holder_name: apple_pay_decrypted_data.card_holder_name.clone(),
                card_expiry_date: PayoutExpiryDate {
                    month: apple_pay_decrypted_data.get_expiry_month_as_i8()?,
                    year: apple_pay_decrypted_data.get_expiry_year_as_4_digit_i32()?,
                },
            })),
            api_models::payouts::PayoutMethodData::Card(_)
            | api_models::payouts::PayoutMethodData::Bank(_)
            | api_models::payouts::PayoutMethodData::Wallet(_)
            | api_models::payouts::PayoutMethodData::BankRedirect(_)
            | api_models::payouts::PayoutMethodData::Passthrough(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "Selected Payout Method is not implemented for Worldpay".to_string(),
                )
                .into())
            }
        }
    }
}

impl From<PayoutOutcome> for enums::PayoutStatus {
    fn from(item: PayoutOutcome) -> Self {
        match item {
            PayoutOutcome::RequestReceived => Self::Initiated,
            PayoutOutcome::Error | PayoutOutcome::Refused => Self::Failed,
            PayoutOutcome::QueryRequired => Self::Pending,
        }
    }
}

impl TryFrom<PayoutsResponseRouterData<PoFulfill, WorldpayPayoutResponse>>
    for types::PayoutsRouterData<PoFulfill>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<PoFulfill, WorldpayPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(enums::PayoutStatus::from(item.response.outcome.clone())),
                connector_payout_id: None,
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
