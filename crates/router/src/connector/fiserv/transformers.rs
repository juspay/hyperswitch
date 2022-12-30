use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    core::errors,
    pii::{self, ExposeOptionInterface, Secret},
    types::{self,api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FiservPaymentsRequest {
    amount: Amount,
    source: Source,
    transaction_details: TransactionDetails,
    merchant_details: MerchantDetails,
    transaction_interaction: TransactionInteraction,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    source_type: String,
    card: CardData,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    card_number: Secret<String, pii::CardNumber>,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    // security_code: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Amount {
    total: i64,
    currency: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    capture_flag: bool
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantDetails {
    merchant_id: String,
    terminal_id: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInteraction {
    origin: String,
    eci_indicator: String,
    post_condition_code: String
}

/*
{"amount":{"total":12.04,"currency":"USD"},"source":{"sourceType":"PaymentCard","card":{"cardData":"4005550000000019","expirationMonth":"02","expirationYear":"2035"}},"transactionDetails":{"captureFlag":true},"merchantDetails":{"merchantId":"100008000003683","terminalId":"10000001"},"transactionInteraction":{"origin":"ECOM","eciIndicator":"CHANNEL_ENCRYPTED","posConditionCode":"CARD_NOT_PRESENT_ECOM"}}
*/
impl TryFrom<&types::PaymentsAuthorizeRouterData> for FiservPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
                let amount = Amount {
                    total: item.request.amount,
                    currency: item.request.currency.to_string(),
                };

                let card = CardData {
                    card_number: ccard.card_number.clone(),
                    expiration_month: ccard.card_exp_month.clone(),
                    expiration_year: ccard.card_exp_year.clone(),
                    // security_code: ccard.card_cvc,
                };
                let source = Source {
                    source_type: "PaymentCard".to_string(),
                    card,
                };
                let transaction_details = TransactionDetails {
                    capture_flag:  matches!(
                        item.request.capture_method,
                        Some(enums::CaptureMethod::Automatic) | None
                    ),
                };

                let merchant_details = MerchantDetails {
                    merchant_id: auth.merchant_account,
                    terminal_id: "10000001".to_string(),
                };

                let transaction_interaction = TransactionInteraction {
                    origin: "ECOM".to_string(),
                    eci_indicator: "CHANNEL_ENCRYPTED".to_string(),
                    post_condition_code: "CARD_NOT_PRESENT_ECOM".to_string(),
                };

                Ok(FiservPaymentsRequest { amount, source, transaction_details, merchant_details, transaction_interaction })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment Methods".to_string()))?
        }
    }
}

pub struct FiservAuthType {
    pub(super) api_key: String,
    pub(super) merchant_account: String,
    pub(super) api_secret: String,
}

impl TryFrom<&types::ConnectorAuthType> for FiservAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_string(),
                merchant_account: key1.to_string(),
                api_secret: api_secret.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub details: Vec<ErrorDetails>
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails{
    #[serde(rename = "type")]
    pub error_type: String,
    pub code: String,
    pub message: String,
}


#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FiservPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<FiservPaymentStatus> for enums::AttemptStatus {
    fn from(item: FiservPaymentStatus) -> Self {
        match item {
            FiservPaymentStatus::Succeeded => Self::Charged,
            FiservPaymentStatus::Failed => Self::Failure,
            FiservPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FiservPaymentsResponse {
    id: String,
    status: FiservPaymentStatus,
}

impl TryFrom<types::PaymentsResponseRouterData<FiservPaymentsResponse>> for types::PaymentsAuthorizeRouterData {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::PaymentsResponseRouterData<FiservPaymentsResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: item.response.status.into(),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct FiservRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for FiservRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
       todo!()
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

impl From<self::RefundStatus> for enums::RefundStatus {
    fn from(item: self::RefundStatus) -> Self {
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
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
         todo!()
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FiservErrorResponse {}
