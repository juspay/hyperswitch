#[cfg(feature = "payouts")]
use api_models::payouts::{BankTransfer, PayoutMethodData};
#[cfg(feature = "payouts")]
use common_enums::PayoutStatus;
use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_response_types::PayoutsResponseData, types::PayoutsRouterData,
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(feature = "payouts")]
use crate::{
    types::PayoutsResponseRouterData,
    utils::{get_unimplemented_payment_method_error_message, RouterData as RouterDataTrait},
};

//TODO: Fill the struct with respective fields
pub struct GotymeSanlamRouterData<T> {
    pub amount: StringMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for GotymeSanlamRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub struct GotymeSanlamAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) profile_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for GotymeSanlamAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                profile_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GotymeSanlamErrorResponse {
    pub error_code: Option<String>,
    pub error_title: Option<String>,
    pub error_message: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GotymeSanlamPayoutTransferPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sa_id: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<GotymeSanlamBankNames>,
    pub amount: StringMajorUnit,
    pub idempotency_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GotymeSanlamPayoutTransferRequest {
    pub flow: GotymeSanlamPayoutFlow,
    pub payload: GotymeSanlamPayoutTransferPayload,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GotymeSanlamPayoutGetPayload {
    pub idempotency_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GotymeSanlamPayoutGetRequest {
    pub flow: GotymeSanlamPayoutFlow,
    pub payload: GotymeSanlamPayoutGetPayload,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GotymeSanlamPayoutFlow {
    PayoutCreate,
    PayoutSync,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<&GotymeSanlamRouterData<&PayoutsRouterData<F>>>
    for GotymeSanlamPayoutTransferRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &GotymeSanlamRouterData<&PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let payload =
            GotymeSanlamPayoutTransferPayload::try_from((item.router_data, item.amount.clone()))?;

        Ok(Self {
            flow: GotymeSanlamPayoutFlow::PayoutCreate,
            payload,
        })
    }
}

impl<F> TryFrom<(&PayoutsRouterData<F>, StringMajorUnit)> for GotymeSanlamPayoutTransferPayload {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (router_data, amount): (&PayoutsRouterData<F>, StringMajorUnit),
    ) -> Result<Self, Self::Error> {
        let idempotency_key = router_data.connector_request_reference_id.clone();

        match router_data.get_payout_method_data()? {
            PayoutMethodData::BankTransfer(BankTransfer::Payshap(payshap)) => {
                let bank_name = payshap
                    .bank_name
                    .as_ref()
                    .map(GotymeSanlamBankNames::try_from)
                    .transpose()?;

                Ok(Self {
                    account_name: payshap.account_holder_name.clone(),
                    sa_id: None,
                    account_number: Some(payshap.bank_account_number.clone()),
                    bank_name,
                    amount,
                    idempotency_key: idempotency_key.clone(),
                    description: router_data.description.clone(),
                })
            }
            PayoutMethodData::BankTransfer(BankTransfer::PayshapProxy(payshap_proxy)) => {
                let sa_id = payshap_proxy
                    .shap_id
                    .as_ref()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "payshap_proxy.shap_id".into(),
                    })?
                    .to_owned();

                Ok(Self {
                    account_name: None,
                    sa_id: Some(sa_id),
                    account_number: None,
                    bank_name: None,
                    amount,
                    idempotency_key,
                    description: router_data.description.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("GotymeSanlam"),
            ))?,
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<&PayoutsRouterData<F>> for GotymeSanlamPayoutGetRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            flow: GotymeSanlamPayoutFlow::PayoutSync,
            payload: GotymeSanlamPayoutGetPayload {
                idempotency_key: item.connector_request_reference_id.clone(),
            },
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GotymeSanlamBankNames {
    Absa,
}

impl TryFrom<&common_enums::BankNames> for GotymeSanlamBankNames {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(bank_name: &common_enums::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            common_enums::BankNames::Absa => Ok(Self::Absa),
            _ => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("GotymeSanlam"),
            ))?,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GotymeSanlamPayoutStatus {
    Pending,
    Successful,
    Failed,
    Reversed,
}

impl From<GotymeSanlamPayoutStatus> for PayoutStatus {
    fn from(status: GotymeSanlamPayoutStatus) -> Self {
        match status {
            GotymeSanlamPayoutStatus::Pending => Self::Initiated,
            GotymeSanlamPayoutStatus::Successful => Self::Success,
            GotymeSanlamPayoutStatus::Failed => Self::Failed,
            GotymeSanlamPayoutStatus::Reversed => Self::Reversed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GotymeSanlamPayoutResponse {
    pub id: Option<String>,
    pub idempotency_key: String,
    pub status: GotymeSanlamPayoutStatus,
    pub created_at: Option<String>,
    pub payment_processor_txn_id: Option<String>,
    pub reason: Option<GotymeSanlamPayoutReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GotymeSanlamPayoutReason {
    pub error_code: Option<String>,
    pub error_title: Option<String>,
    pub error_message: Option<String>,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, GotymeSanlamPayoutResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, GotymeSanlamPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        let status = PayoutStatus::from(item.response.status);
        let connector_payout_id = item.response.payment_processor_txn_id.clone();
        let response = match item.response.reason {
            Some(reason) => Ok(PayoutsResponseData {
                status: Some(status),
                connector_payout_id,
                error_code: reason.error_code,
                error_message: reason.error_message,
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                payout_connector_metadata: None,
                connector_eligibility_reference_id: None,
            }),
            None => Ok(PayoutsResponseData {
                status: Some(status),
                connector_payout_id,
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
                connector_eligibility_reference_id: None,
            }),
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}
