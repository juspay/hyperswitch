use common_enums::enums;
use common_utils::{errors::CustomResult, ext_traits::ByteSliceExt, types::StringMinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use common_utils::types::MinorUnit;
use time::PrimitiveDateTime;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::{
    router_flow_types::revenue_recovery::GetAdditionalRevenueRecoveryDetails,
    router_request_types::revenue_recovery::GetAdditionalRevenueRecoveryRequestData,
    router_response_types::revenue_recovery::GetAdditionalRevenueRecoveryResponseData,
    types::AdditionalRevenueRecoveryDetailsRouterData,
};
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use error_stack::ResultExt;
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use std::str::FromStr;

//TODO: Fill the struct with respective fields
pub struct RecurlyRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for RecurlyRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct RecurlyPaymentsRequest {
    amount: StringMinorUnit,
    card: RecurlyCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct RecurlyCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&RecurlyRouterData<&PaymentsAuthorizeRouterData>> for RecurlyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RecurlyRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = RecurlyCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct RecurlyAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for RecurlyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecurlyPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RecurlyPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: RecurlyPaymentStatus) -> Self {
        match item {
            RecurlyPaymentStatus::Succeeded => Self::Charged,
            RecurlyPaymentStatus::Failed => Self::Failure,
            RecurlyPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecurlyPaymentsResponse {
    status: RecurlyPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, RecurlyPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RecurlyPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct RecurlyRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&RecurlyRouterData<&RefundsRouterData<F>>> for RecurlyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RecurlyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecurlyErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecurlyWebhookBody {
    // transaction id
    pub uuid: String,
    pub event_type: RecurlyPaymentEventType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RecurlyPaymentEventType {
    #[serde(rename = "succeeded")]
    PaymentSucceeded,
    #[serde(rename = "failed")]
    PaymentFailed,
}

impl RecurlyWebhookBody {
    pub fn get_webhook_object_from_body(body: &[u8]) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("RecurlyWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RecurlyChargeStatus {
    Succeeded,
    Failed,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "snake_case")]
pub enum RecurlyFundingTypes {
    #[serde(rename = "credit")]
    Credit,
    #[serde(rename = "debit")]
    Debit,
    #[serde(rename = "prepaid")]
    Prepaid,
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "deferred_debit")]
    DeferredDebit,
    #[serde(rename = "charge")]
    Charge,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecurlyRecoveryDetailsData {
    pub amount: MinorUnit, 
    pub currency: common_enums::Currency,
    pub original_transaction_id: String,
    pub gateway_reference: String,
    pub status_code: String,
    pub status_message: String,
    pub account: Account, 
    pub invoice: Invoice, 
    pub payment_method: PaymentMethod, 
    pub payment_gateway: PaymentGateway,
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub collected_at: PrimitiveDateTime,
    pub status: RecurlyChargeStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentMethod {
    
    pub gateway_token: String, 
    pub funding_source:RecurlyFundingTypes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String, 
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentGateway {
    pub id: String,
    pub name: String,
}


#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    TryFrom<
        ResponseRouterData<
            GetAdditionalRevenueRecoveryDetails,
            RecurlyRecoveryDetailsData,
            GetAdditionalRevenueRecoveryRequestData,
            GetAdditionalRevenueRecoveryResponseData,
        >,
    > for AdditionalRevenueRecoveryDetailsRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            GetAdditionalRevenueRecoveryDetails,
            RecurlyRecoveryDetailsData,
            GetAdditionalRevenueRecoveryRequestData,
            GetAdditionalRevenueRecoveryResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let merchant_reference_id =
            common_utils::id_type::PaymentReferenceId::from_str(&item.response.invoice.id)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let connector_transaction_id = Some(common_utils::types::ConnectorTransactionId::from(
            item.response.original_transaction_id,
        ));
        
        Ok(Self {
            response: Ok(GetAdditionalRevenueRecoveryResponseData{
                status: item.response.status.into(),
                amount : item.response.amount,
                currency : item.response.currency,
                merchant_reference_id,
                connector_account_reference_id : "Recurly".to_string(),
                connector_transaction_id,
                error_code : Some(item.response.status_code),
                error_message : Some(item.response.status_message),
                processor_payment_method_token : item.response.payment_method.gateway_token,
                connector_customer_id : item.response.account.id,
                transaction_created_at : Some(item.response.collected_at),
                payment_method_sub_type: common_enums::PaymentMethodType::from(item.response.payment_method.funding_source),
                payment_method_type : common_enums::PaymentMethod::Card
            }),
            ..item.data
        })
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<RecurlyChargeStatus> for enums::AttemptStatus {
    fn from(status: RecurlyChargeStatus) -> Self {
        match status {
            RecurlyChargeStatus::Succeeded => Self::Charged,
            RecurlyChargeStatus::Failed => Self::Failure
        }
    }
}
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<RecurlyFundingTypes> for common_enums::PaymentMethodType {
    fn from(funding: RecurlyFundingTypes) -> Self {
        match funding {
            RecurlyFundingTypes::Credit|
            RecurlyFundingTypes::Charge => Self::Credit,
            RecurlyFundingTypes::Debit
            | RecurlyFundingTypes::Prepaid
            | RecurlyFundingTypes::DeferredDebit
            | RecurlyFundingTypes::Unknown => Self::Debit,
        }
    }
}
