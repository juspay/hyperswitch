use common_enums::enums;
use hyperswitch_domain_models::{
    payment_method_data::{CardNumber, PaymentMethodData}, // Path from initial user error list
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{payments::{Authorize, Capture, PSync}, refunds::RSync}, 
    router_request_types::{ResponseId, PaymentsCaptureData, PaymentsSyncData as PaymentsSyncRequestData}, // Renamed for clarity
    router_response_types::{
        PaymentsResponseData, RedirectForm, RedirectionData, RefundsResponseData, 
    },
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData, RefundsRouterData}, 
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret, ExposeInterface};
use std::marker::PhantomData;
use serde::{Deserialize, Serialize};
use error_stack::ResultExt;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData}, // RefundsResponseRouterData might be unused
    utils::PaymentsAuthorizeRequestData, // PaymentsAuthorizeRequestData might be unused
};

// SumUp specific request structs

#[derive(Debug, Serialize, PartialEq)]
pub struct SumUpCardDetails {
    pub number: CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvc: Secret<String>,
    pub holder_name: Option<Secret<String>>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SumUpCheckoutRequest {
    pub checkout_reference: String,
    pub amount: f64, // SumUp expects amount in base units
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>, // Required for 3DS
    pub merchant_code: String, 
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pay_to_email: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SumUpProcessCheckoutRequest {
    pub payment_type: String, 
    pub card: SumUpCardDetails,
}

pub struct SumupRouterData<'a, T> {
    pub amount: f64, 
    pub router_data: &'a T,
}

impl<'a, T> From<(f64, &'a T)> for SumupRouterData<'a, T> {
    fn from((amount, item): (f64, &'a T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

impl<'a> TryFrom<&SumupRouterData<'a, PaymentsAuthorizeRouterData>> for SumUpCheckoutRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SumupRouterData<'a, PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        let auth = SumupAuthType::try_from(&item.router_data.connector_auth_type)?;
        let merchant_code = auth.merchant_code.ok_or_else(|| {
            errors::ConnectorError::MissingRequiredField {
                field_name: "merchant_code",
            }
        })?;

        Ok(Self {
            checkout_reference: item.router_data.connector_request_reference_id.clone(),
            amount: item.amount, 
            currency: item.router_data.request.currency.to_string().to_uppercase(),
            customer_id: item.router_data.customer_id.clone().map(|c| c.get_string_repr().to_string()),
            description: None, // Provide None for optional field
            return_url: item.router_data.request.router_return_url.clone(),
            merchant_code,
            pay_to_email: item.router_data.request.email.as_ref().map(|e| e.expose().to_string()), // Ensure String
        })
    }
}

impl<'a> TryFrom<&SumupRouterData<'a, PaymentsAuthorizeRouterData>> for SumUpProcessCheckoutRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SumupRouterData<'a, PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(card) => {
                let sumup_card = SumUpCardDetails {
                    number: card.card_number,
                    expiry_month: card.card_exp_month,
                    expiry_year: card.card_exp_year,
                    cvc: card.card_cvc,
                    holder_name: card.card_holder_name,
                };
                Ok(Self {
                    payment_type: "card".to_string(),
                    card: sumup_card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not supported by SumUp for this operation".to_string(),
            )
            .into()),
        }
    }
}

pub struct SumupAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_code: Option<String>,
}

impl TryFrom<&ConnectorAuthType> for SumupAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_code: None, 
            }),
            ConnectorAuthType::BodyKey { api_key, key1 } => {
                Ok(Self {
                    api_key: api_key.to_owned(),
                    merchant_code: Some(key1.peek().clone()),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SumUpTransactionData {
    pub id: String,
    pub transaction_code: String,
    pub amount: f64,
    pub currency: String,
    pub timestamp: String,
    pub status: String, 
    pub payment_type: String,
    pub merchant_code: String,
    #[serde(default)]
    pub foreign_transaction_id: Option<String>,
    #[serde(default)]
    pub internal_transaction_id: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub card: Option<SumUpResponseCardDetails>,
    #[serde(default)]
    pub events: Option<Vec<SumUpTransactionEvent>>,
    #[serde(default)]
    pub next_step: Option<SumUpNextStep>,
    #[serde(default)]
    pub checkout_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SumUpResponseCardDetails {
    pub last_4_digits: String,
    pub card_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SumUpTransactionEvent {
    pub event: String,
    pub amount: Option<f64>,
    pub timestamp: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SumUpNextStep {
    #[serde(rename = "type")]
    pub step_type: String,
    pub method: String,
    pub href: String,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SumUp3dsRedirectResponse {
    pub next_step: SumUpNextStep,
    pub id: Option<String>, 
    pub transaction_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SumUpCheckoutResponse {
    pub id: String,
    pub status: String,
    pub amount: f64,
    pub currency: String,
    pub checkout_reference: String,
    pub merchant_code: String,
    pub date: String,
}

pub type SumUpProcessCheckoutResponse = SumUpTransactionData;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SumUpPaymentStatus {
    SUCCESSFUL,
    FAILED,
    PENDING,
    PAID,
    #[default]
    UNKNOWN,
}

impl From<SumUpPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: SumUpPaymentStatus) -> Self {
        match item {
            SumUpPaymentStatus::SUCCESSFUL | SumUpPaymentStatus::PAID => Self::Charged,
            SumUpPaymentStatus::FAILED => Self::Failure,
            SumUpPaymentStatus::PENDING => Self::Authorizing,
            SumUpPaymentStatus::UNKNOWN => Self::Pending,
        }
    }
}

impl TryFrom<ResponseRouterData<Authorize, SumUpTransactionData, PaymentsAuthorizeRouterData, PaymentsResponseData>>
    for RouterData<Authorize, PaymentsAuthorizeRouterData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<Authorize, SumUpTransactionData, PaymentsAuthorizeRouterData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let sumup_status_str = item.response.status.to_uppercase();
        let sumup_status: SumUpPaymentStatus = serde_json::from_value(serde_json::Value::String(sumup_status_str))
            .unwrap_or(SumUpPaymentStatus::UNKNOWN);

        let mut redirection_data = None;
        if let Some(next_step) = item.response.next_step.as_ref() {
            if next_step.step_type == "3ds_redirect" {
                 redirection_data = Some(RedirectionData {
                    return_url: item.data.request.request.router_return_url.clone().ok_or_else(|| errors::ConnectorError::MissingRequiredField { field_name: "router_return_url" })?,
                    redirect_form: hyperswitch_domain_models::router_response_types::RedirectForm::Form {
                        endpoint: next_step.href.clone(),
                        form_fields: next_step.parameters.clone().unwrap_or_default().as_object().map_or_else(Default::default, |map| { // Renamed parameters to form_fields
                            map.iter().map(|(k,v)| (k.clone(), v.as_str().unwrap_or_default().to_string())).collect()
                        }),
                    },
                    payment_method_data: None,
                    redirect_url: next_step.href.clone(),
                });
            }
        }

        Ok(Self {
            status: common_enums::AttemptStatus::from(sumup_status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: redirection_data.map(Box::new),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: item.response.internal_transaction_id.clone(),
                connector_response_reference_id: item.response.foreign_transaction_id.clone().or_else(|| item.response.checkout_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            amount_captured: if common_enums::AttemptStatus::from(sumup_status.clone()) == common_enums::AttemptStatus::Charged {
                let currency_enum = item.response.currency.parse::<enums::Currency>()
                    .change_context(errors::ConnectorError::InvalidDataFormat {
                        field_name: "currency"
                    })?;
                Some(crate::utils::to_currency_base_unit_asf64(item.response.amount, currency_enum)?) // Using crate::utils
            } else {
                None
            },
            ..item.data
        })
    }
}

impl TryFrom<ResponseRouterData<PSync, SumUpTransactionData, PaymentsSyncRouterData, PaymentsResponseData>>
    for RouterData<PSync, PaymentsSyncRouterData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<PSync, SumUpTransactionData, PaymentsSyncRouterData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let sumup_status_str = item.response.status.to_uppercase();
        let sumup_status: SumUpPaymentStatus = serde_json::from_value(serde_json::Value::String(sumup_status_str))
            .unwrap_or(SumUpPaymentStatus::UNKNOWN);

        let mut redirection_data = None;
        if let Some(next_step) = item.response.next_step.as_ref() {
            if next_step.step_type == "3ds_redirect" {
                 redirection_data = Some(RedirectionData {
                    return_url: item.data.request.router_return_url.clone().ok_or_else(|| errors::ConnectorError::MissingRequiredField { field_name: "router_return_url" })?, // Assuming PaymentsSyncData has router_return_url
                    redirect_form: hyperswitch_domain_models::router_response_types::RedirectForm::Form {
                        endpoint: next_step.href.clone(),
                        form_fields: next_step.parameters.clone().unwrap_or_default().as_object().map_or_else(Default::default, |map| { // Renamed parameters to form_fields
                            map.iter().map(|(k,v)| (k.clone(), v.as_str().unwrap_or_default().to_string())).collect()
                        }),
                    },
                    payment_method_data: None,
                    redirect_url: next_step.href.clone(),
                });
            }
        }
        
        let mut router_data = Self {
            flow: PhantomData,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: redirection_data.map(Box::new),
                mandate_reference: Box::new(None),
                connector_metadata: None, 
                network_txn_id: item.response.internal_transaction_id.clone(),
                connector_response_reference_id: item.response.foreign_transaction_id.clone().or_else(|| item.response.checkout_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        };

        router_data.status = common_enums::AttemptStatus::from(sumup_status.clone());
        router_data.amount_captured = if common_enums::AttemptStatus::from(sumup_status) == common_enums::AttemptStatus::Charged {
            let currency_enum = item.response.currency.parse::<enums::Currency>()
                .change_context(errors::ConnectorError::InvalidDataFormat {
                    field_name: "currency"
                })?;
            Some(crate::utils::to_currency_base_unit_asf64(item.response.amount, currency_enum)?) // Using crate::utils
        } else {
            None
        };
        Ok(router_data)
    }
}

impl TryFrom<ResponseRouterData<Capture, SumUpTransactionData, PaymentsCaptureRouterData, PaymentsResponseData>>
    for RouterData<Capture, PaymentsCaptureRouterData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<Capture, SumUpTransactionData, PaymentsCaptureRouterData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let sumup_status_str = item.response.status.to_uppercase();
        let sumup_status: SumUpPaymentStatus = serde_json::from_value(serde_json::Value::String(sumup_status_str))
            .unwrap_or(SumUpPaymentStatus::UNKNOWN);

        let mut router_data = Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None), 
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: item.response.internal_transaction_id.clone(),
                connector_response_reference_id: item.response.foreign_transaction_id.clone().or_else(|| item.response.checkout_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        };
        router_data.status = common_enums::AttemptStatus::from(sumup_status.clone());
        router_data.amount_captured = if router_data.status == common_enums::AttemptStatus::Charged {
             let currency_enum = item.response.currency.parse::<enums::Currency>()
                .change_context(errors::ConnectorError::InvalidDataFormat {
                    field_name: "currency"
                })?;
            Some(crate::utils::to_currency_base_unit_asf64(item.response.amount, currency_enum)?) // Using crate::utils
        } else {
            None
        };
        Ok(router_data)
    }
}

impl TryFrom<ResponseRouterData<Authorize, SumUp3dsRedirectResponse, PaymentsAuthorizeRouterData, PaymentsResponseData>>
    for RouterData<Authorize, PaymentsAuthorizeRouterData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<Authorize, SumUp3dsRedirectResponse, PaymentsAuthorizeRouterData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = RedirectionData {
            return_url: item.data.request.request.router_return_url.clone().ok_or_else(|| errors::ConnectorError::MissingRequiredField { field_name: "router_return_url" })?,
            redirect_form: hyperswitch_domain_models::router_response_types::RedirectForm::Form {
                endpoint: item.response.next_step.href.clone(),
                form_fields: item.response.next_step.parameters.clone().unwrap_or_default().as_object().map_or_else(Default::default, |map| { // Renamed parameters to form_fields
                    map.iter().map(|(k,v)| (k.clone(), v.as_str().unwrap_or_default().to_string())).collect()
                }),
            },
            payment_method_data: None,
            redirect_url: item.response.next_step.href.clone(),
        };

        Ok(Self {
            status: common_enums::AttemptStatus::AuthenticationPending, 
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone().or_else(|| item.response.id.clone()).unwrap_or_else(|| item.data.connector_request_reference_id.clone())
                ),
                redirection_data: Some(Box::new(redirection_data)),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.id.clone(), 
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Default)]
pub struct SumUpRefundRequest {
    pub amount: f64,
}

impl<'a, F> TryFrom<&SumupRouterData<'a, RefundsRouterData<F>>> for SumUpRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SumupRouterData<'a, RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
        })
    }
}

impl From<SumUpPaymentStatus> for enums::RefundStatus {
    fn from(item: SumUpPaymentStatus) -> Self {
        match item {
            SumUpPaymentStatus::SUCCESSFUL | SumUpPaymentStatus::PAID => Self::Success,
            SumUpPaymentStatus::FAILED => Self::Failure,
            SumUpPaymentStatus::PENDING => Self::Pending,
            SumUpPaymentStatus::UNKNOWN => Self::Pending,
        }
    }
}

impl TryFrom<ResponseRouterData<RSync, SumUpTransactionData, RefundsRouterData<RSync>, RefundsResponseData>>
    for RouterData<RSync, RefundsRouterData<RSync>, RefundsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<RSync, SumUpTransactionData, RefundsRouterData<RSync>, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let sumup_status_str = item.response.status.to_uppercase();
        let sumup_status: SumUpPaymentStatus = serde_json::from_value(serde_json::Value::String(sumup_status_str))
            .unwrap_or(SumUpPaymentStatus::UNKNOWN);

        let connector_refund_id = item.response.id.clone();
        
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id,
                refund_status: enums::RefundStatus::from(sumup_status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SumupErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
