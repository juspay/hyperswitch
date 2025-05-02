use common_enums::enums;
use common_utils::{ext_traits::Encode, types::StringMinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{ResponseId, VaultRequestData},
    router_response_types::{PaymentsResponseData, RefundsResponseData, VaultResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData, VaultRouterData},
    vault::PaymentMethodVaultingData,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct VgsRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for VgsRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct VgsTokenRequestItem {
    value: String,
    classifiers: Vec<String>,
    format: String,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct VgsPaymentsRequest {
    data: Vec<VgsTokenRequestItem>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct VgsCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
}

impl<F> TryFrom<&VaultRouterData<F>> for VgsPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VaultRouterData<F>) -> Result<Self, Self::Error> {
        match item.request.payment_method_vaulting_data.clone() {
            Some(PaymentMethodVaultingData::Card(req_card)) => {
                let card = VgsCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc.unwrap(),
                };
                Ok(Self {
                    data: vec![VgsTokenRequestItem {
                        value: card.encode_to_string_of_json().unwrap(),
                        classifiers: vec!["data".to_string()],
                        format: "UUID".to_string(),
                    }],
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct VgsAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for VgsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: api_key.to_owned(),
                password: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VgsPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<VgsPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: VgsPaymentStatus) -> Self {
        match item {
            VgsPaymentStatus::Succeeded => Self::Charged,
            VgsPaymentStatus::Failed => Self::Failure,
            VgsPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VgsAliasItem {
    alias: String,
    format: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VgsTokenResponseItem {
    value: String,
    classifiers: Vec<String>,
    aliases: Vec<VgsAliasItem>,
    created_at: String,
    storage: String,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VgsPaymentsResponse {
    data: Vec<VgsTokenResponseItem>,
}

impl<F> TryFrom<ResponseRouterData<F, VgsPaymentsResponse, VaultRequestData, VaultResponseData>>
    for RouterData<F, VaultRequestData, VaultResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, VgsPaymentsResponse, VaultRequestData, VaultResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::Failure,
            response: Ok(VaultResponseData::VaultInsertResponse {
                connector_vault_id: item.response.data[0].aliases[0].alias.clone(),
                fingerprint_id: item.response.data[0].aliases[0].alias.clone(),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct VgsErrorItem {
    pub status: u16,
    pub code: String,
    pub detail: Option<String>,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct VgsErrorResponse {
    pub errors: Vec<VgsErrorItem>,
    pub trace_id: String,
}
