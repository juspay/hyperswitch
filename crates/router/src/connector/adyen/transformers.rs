use serde::{Deserialize, Serialize};

use crate::{
    consts,
    core::errors,
    pii::{PeekInterface, Secret},
    types::{self, api, storage::enums},
};

// Adyen Types Definition
// Payments Request and Response Types
#[derive(Default, Debug, Serialize, Deserialize)]
pub enum AdyenShopperInteraction {
    #[default]
    Ecommerce,
    #[serde(rename = "ContAuth")]
    ContinuedAuthentication,
    Moto,
    #[serde(rename = "POS")]
    Pos,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenRecurringModel {
    UnscheduledCardOnFile,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPaymentRequest {
    amount: Amount,
    merchant_account: String,
    payment_method: AdyenPaymentMethod,
    reference: String,
    return_url: String,
    shopper_interaction: AdyenShopperInteraction,
    #[serde(skip_serializing_if = "Option::is_none")]
    recurring_processing_model: Option<AdyenRecurringModel>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPaymentResponse {
    psp_reference: String,
    result_code: String,
    amount: Option<Amount>,
    merchant_reference: String,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    currency: String,
    value: i32,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPaymentMethod {
    #[serde(rename = "type")]
    payment_type: String,
    number: Option<Secret<String>>,
    expiry_month: Option<Secret<String>>,
    expiry_year: Option<Secret<String>>,
    cvc: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelRequest {
    merchant_account: String,
    original_reference: String,
    reference: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelResponse {
    merchant_account: String,
    psp_reference: String,
    response: String,
}

// Refunds Request and Response
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefundRequest {
    merchant_account: String,
    reference: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefundResponse {
    merchant_account: String,
    psp_reference: String,
    payment_psp_reference: String,
    reference: String,
    status: String,
}

pub struct AdyenAuthType {
    pub(super) api_key: String,
    pub(super) merchant_account: String,
}

impl TryFrom<&types::ConnectorAuthType> for AdyenAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
                merchant_account: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// Payment Request Transform
impl TryFrom<&types::PaymentsRouterData> for AdyenPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsRouterData) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let reference = item.payment_id.to_string();
        let amount = Amount {
            currency: item.currency.to_string(),
            value: item.amount,
        };
        let ccard = match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => Some(ccard),
            api::PaymentMethod::BankTransfer => None,
            api::PaymentMethod::PayLater(_) => None,
        };

        let shopper_interaction = match item.request.off_session {
            Some(true) => AdyenShopperInteraction::ContinuedAuthentication,
            _ => AdyenShopperInteraction::Ecommerce,
        };

        let recurring_processing_model = match item.request.setup_future_usage {
            Some(enums::FutureUsage::OffSession) => {
                Some(AdyenRecurringModel::UnscheduledCardOnFile)
            }
            _ => None,
        };

        let payment_method = AdyenPaymentMethod {
            payment_type: "scheme".to_string(),
            number: ccard.map(|x| x.card_number.peek().clone().into()), // FIXME: xxx: should also be secret?
            expiry_month: ccard.map(|x| x.card_exp_month.peek().clone().into()),
            expiry_year: ccard.map(|x| x.card_exp_year.peek().clone().into()),
            // TODO: CVV/CVC shouldn't be saved in our db
            // Will need to implement tokenization that allows us to make payments without cvv
            cvc: ccard.map(|x| x.card_cvc.peek().into()),
        };

        Ok(AdyenPaymentRequest {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference,
            return_url: "juspay.io".to_string(),
            shopper_interaction,
            recurring_processing_model,
        })
    }
}

impl TryFrom<&types::PaymentRouterCancelData> for AdyenCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentRouterCancelData) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(AdyenCancelRequest {
            merchant_account: auth_type.merchant_account,
            original_reference: item.request.connector_transaction_id.to_string(),
            reference: item.payment_id.to_string(),
        })
    }
}

impl TryFrom<types::PaymentsCancelResponseRouterData<AdyenCancelResponse>>
    for types::PaymentRouterCancelData
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::PaymentsCancelResponseRouterData<AdyenCancelResponse>,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.response.as_str() {
            "received" => enums::AttemptStatus::Voided,
            "processing" => enums::AttemptStatus::Pending,
            _ => enums::AttemptStatus::VoidFailed,
        };
        Ok(types::RouterData {
            status,
            response: Some(types::PaymentsResponseData {
                connector_transaction_id: item.response.psp_reference,
                redirection_data: None,
                redirect: false,
            }),
            ..item.data
        })
    }
}

// Payment Response Transform
impl TryFrom<types::PaymentsResponseRouterData<AdyenPaymentResponse>>
    for types::PaymentsRouterData
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<AdyenPaymentResponse>,
    ) -> Result<Self, Self::Error> {
        let result = item.response.result_code;
        let status = match result.as_str() {
            "Authorised" => enums::AttemptStatus::Charged,
            // "Pending" => enums::AttemptStatus::Pending,
            "Refused" => enums::AttemptStatus::Failure,
            _ => enums::AttemptStatus::Pending,
        };
        let error = if item.response.refusal_reason.is_some()
            || item.response.refusal_reason_code.is_some()
        {
            Some(types::ErrorResponse {
                code: item
                    .response
                    .refusal_reason_code
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .refusal_reason
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: None,
            })
        } else {
            None
        };

        Ok(types::RouterData {
            status,
            // amount: amount,
            // amount_capturable: amount,
            response: Some(types::PaymentsResponseData {
                connector_transaction_id: item.response.psp_reference,
                redirection_data: None,
                redirect: false,
            }),
            error_response: error,
            ..item.data /*
                        client_secret: "fetch_client_secret_from_DB".to_string(),
                        created: "fetch_timestamp_from_DB".to_string(),
                        currency: currency,
                        customer: Some("fetch_customer_id_from_DB".to_string()),
                        ?? : item.statement_descriptor
                        ?? : item.statement_descriptor_suffix
                        ?? : item.metadata [orderId, txnId, txnUuid]
                        description: None,
                        */
        })
    }
}

/*
// This is a repeated code block from Stripe inegration. Can we avoid the repetition in every integration
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdyenPaymentStatus {
    Succeeded,
    Failed,
    Processing,
    RequiresCustomerAction,
    RequiresPaymentMethod,
    RequiresConfirmation,
}

// Default always be Processing
impl Default for AdyenPaymentStatus {
    fn default() -> Self {
        AdyenPaymentStatus::Processing
    }
}

impl From<AdyenPaymentStatus> for enums::Status {
    fn from(item: AdyenPaymentStatus) -> Self {
        match item {
            AdyenPaymentStatus::Succeeded => enums::Status::Charged,
            AdyenPaymentStatus::Failed => enums::Status::Failure,
            AdyenPaymentStatus::Processing
            | AdyenPaymentStatus::RequiresCustomerAction
            | AdyenPaymentStatus::RequiresPaymentMethod
            | AdyenPaymentStatus::RequiresConfirmation => enums::Status::Pending,
        }
    }
}
*/
// Refund Request Transform
impl<F> TryFrom<&types::RefundsRouterData<F>> for AdyenRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(AdyenRefundRequest {
            merchant_account: auth_type.merchant_account,
            reference: item.request.refund_id.clone(),
        })
    }
}

// Refund Response Transform
impl<F> TryFrom<types::RefundsResponseRouterData<F, AdyenRefundResponse>>
    for types::RefundsRouterData<F>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<F, AdyenRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.status.as_str() {
            // From the docs, the only value returned is "received", outcome of refund is available
            // through refund notification webhook
            "received" => enums::RefundStatus::Success,
            _ => enums::RefundStatus::Pending,
        };
        Ok(types::RouterData {
            response: Some(types::RefundsResponseData {
                connector_refund_id: item.response.reference,
                refund_status,
            }),
            error_response: None,
            ..item.data
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub status: i32,
    pub error_code: String,
    pub message: String,
    pub error_type: String,
    pub psp_reference: Option<String>,
}

// #[cfg(test)]
// mod test_adyen_transformers {
//     use super::*;

//     #[test]
//     fn verify_tranform_from_router_to_adyen_req() {
//         let router_req = PaymentsRequest {
//             amount: 0.0,
//             currency: "None".to_string(),
//             ..Default::default()
//         };
//         println!("{:#?}", &router_req);
//         let adyen_req = AdyenPaymentRequest::from(router_req);
//         println!("{:#?}", &adyen_req);
//         let adyen_req_json: String = serde_json::to_string(&adyen_req).unwrap();
//         println!("{}", adyen_req_json);
//         assert_eq!(true, true)
//     }
// }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenAdditionalDataWH {
    pub hmac_signature: String,
}

#[derive(Debug, Deserialize)]
pub struct AdyenAmountWH {
    pub value: i32,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenNotificationRequestItemWH {
    pub additional_data: AdyenAdditionalDataWH,
    pub amount: AdyenAmountWH,
    pub original_reference: Option<String>,
    pub psp_reference: String,
    pub event_code: String,
    pub merchant_account_code: String,
    pub merchant_reference: String,
    pub success: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdyenItemObjectWH {
    pub notification_request_item: AdyenNotificationRequestItemWH,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenIncomingWebhook {
    pub notification_items: Vec<AdyenItemObjectWH>,
}

impl From<AdyenNotificationRequestItemWH> for AdyenPaymentResponse {
    fn from(notif: AdyenNotificationRequestItemWH) -> Self {
        Self {
            psp_reference: notif.psp_reference,
            merchant_reference: notif.merchant_reference,
            result_code: String::from(match notif.success.as_str() {
                "true" => "Authorised",
                _ => "Refused",
            }),
            amount: Some(Amount {
                value: notif.amount.value,
                currency: notif.amount.currency,
            }),
            refusal_reason: None,
            refusal_reason_code: None,
        }
    }
}
