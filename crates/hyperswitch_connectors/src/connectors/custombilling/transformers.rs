#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use crate::utils::ForeignTryFrom;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
const CUSTOM_BILLING_MCA_IDENTIFIER_FOR_MCA_FEATURE_METADATA: &str = "custombilling";

use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    revenue_recovery,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

//TODO: Fill the struct with respective fields
pub struct CustombillingRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for CustombillingRouterData<T> {
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
pub struct CustombillingPaymentsRequest {
    amount: StringMinorUnit,
    card: CustombillingCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CustombillingCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&CustombillingRouterData<&PaymentsAuthorizeRouterData>>
    for CustombillingPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CustombillingRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = CustombillingCard {
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
pub struct CustombillingAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CustombillingAuthType {
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
pub enum CustombillingPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<CustombillingPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: CustombillingPaymentStatus) -> Self {
        match item {
            CustombillingPaymentStatus::Succeeded => Self::Charged,
            CustombillingPaymentStatus::Failed => Self::Failure,
            CustombillingPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustombillingPaymentsResponse {
    status: CustombillingPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, CustombillingPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CustombillingPaymentsResponse, T, PaymentsResponseData>,
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
pub struct CustombillingRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&CustombillingRouterData<&RefundsRouterData<F>>> for CustombillingRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CustombillingRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
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
pub struct CustombillingErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl ForeignTryFrom<api_models::payments::RecoveryPaymentsCreate>
    for revenue_recovery::RevenueRecoveryAttemptData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: api_models::payments::RecoveryPaymentsCreate,
    ) -> Result<Self, Self::Error> {
        let amount = item.amount_details.order_amount().into();
        let currency = item.amount_details.currency();
        let merchant_reference_id = item.merchant_reference_id.clone();
        let connector_transaction_id = item
            .connector_transaction_id
            .clone()
            .map(common_utils::types::ConnectorTransactionId::TxnId);
        let error_code = item
            .error
            .as_ref()
            .map(|error_details| error_details.code.clone());
        let error_message = item
            .error
            .as_ref()
            .map(|error_details| error_details.message.clone());
        let payment_method_units = item.payment_method_units.clone();
        let primary_payment_method = item.payment_method_units.first().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_units[0]",
            },
        )?;
        let processor_payment_method_token = primary_payment_method.payment_processor_token.clone();
        let connector_customer_id = item.connector_customer_id.clone();
        let connector_account_reference_id =
            CUSTOM_BILLING_MCA_IDENTIFIER_FOR_MCA_FEATURE_METADATA.to_string();
        let transaction_created_at = item.transaction_created_at.clone();
        let status = item.status.clone();
        let payment_method_type = item.payment_method_type.clone();
        let payment_method_sub_type = item.payment_method_subtype.clone();
        let network_advice_code = item
            .error
            .as_ref()
            .and_then(|error| (error.network_advice_code.clone()));
        let network_decline_code = item
            .error
            .as_ref()
            .and_then(|error| (error.network_decline_code.clone()));
        let network_error_message = item
            .error
            .as_ref()
            .and_then(|error| (error.network_error_message.clone()));

        let retry_count = item.retry_count.clone();

        let invoice_next_billing_time = item.next_billing_date.clone();

        Ok(Self {
            amount,
            currency,
            merchant_reference_id,
            connector_transaction_id,
            error_code,
            error_message,
            processor_payment_method_token,
            connector_customer_id,
            connector_account_reference_id,
            transaction_created_at,
            status,
            payment_method_type,
            payment_method_sub_type,
            network_advice_code,
            network_decline_code,
            network_error_message,
            retry_count,
            invoice_next_billing_time,
            card_network: None,
            card_isin: None,
            // This field is none because it is specific to stripebilling.
            charge_id: None,
            payment_method_units,
        })
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl ForeignTryFrom<api_models::payments::RecoveryPaymentsCreate>
    for revenue_recovery::RevenueRecoveryInvoiceData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: api_models::payments::RecoveryPaymentsCreate,
    ) -> Result<Self, Self::Error> {
        let amount = item.amount_details.order_amount().into();
        let currency = item.amount_details.currency();
        let merchant_reference_id = item.merchant_reference_id.clone();
        let retry_count = item.retry_count.clone();
        let invoice_next_billing_time = item.next_billing_date.clone();
        let billing_address = item.billing.clone();
        Ok(Self {
            amount,
            currency,
            merchant_reference_id,
            billing_address,
            retry_count,
            next_billing_at: invoice_next_billing_time,
        })
    }
}
