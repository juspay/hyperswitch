use common_enums::enums;
use common_utils::{
    ext_traits::OptionExt,
    pii::{self, Email, SecretSerdeValue},
    types::MinorUnit,
};
use error_stack::ResultExt;
use masking::PeekInterface;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, ResponseId, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundsRouterData, ResponseRouterData,
    },
};
use hyperswitch_interfaces::{consts::NO_ERROR_CODE, errors, api};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData as TResponseRouterData},
    utils::CardData,
};
use hyperswitch_domain_models::router_response_types::RedirectForm;

pub struct ComputopRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for ComputopRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ComputopAuthType {
    pub merchant_id: Secret<String>,
    pub api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for ComputopAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match item {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret: _,
            } => Ok(Self {
                merchant_id: key1.to_owned(),
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ComputopPaymentsRequest {
    #[serde(rename = "MerchantID")]
    pub merchant_id: Secret<String>,
    #[serde(rename = "TransID")]
    pub trans_id: String,
    #[serde(rename = "Amount")]
    pub amount: MinorUnit,
    #[serde(rename = "Currency")]
    pub currency: enums::Currency,
    #[serde(rename = "URLSuccess")]
    pub url_success: String,
    #[serde(rename = "URLFailure")]
    pub url_failure: String,
    #[serde(rename = "URLNotify")]
    pub url_notify: String,
    #[serde(rename = "Response")]
    pub response: String,
    #[serde(rename = "MAC")]
    pub mac: String,
    #[serde(rename = "RefNr")]
    pub ref_nr: Option<String>,
    #[serde(rename = "OrderDesc")]
    pub order_desc: Option<String>,
    #[serde(rename = "ReqId")]
    pub req_id: Option<String>,
    #[serde(rename = "CCNr")]
    pub cc_nr: Option<Secret<String>>,
    #[serde(rename = "CCCVC")]
    pub cc_cvc: Option<Secret<String>>,
    #[serde(rename = "CCExpiry")]
    pub cc_expiry: Option<Secret<String>>,
    #[serde(rename = "CCBrand")]
    pub cc_brand: Option<String>,
}

impl TryFrom<&ComputopRouterData<&PaymentsAuthorizeRouterData>> for ComputopPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ComputopRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = ComputopAuthType::try_from(&item.router_data.connector_auth_type)?;
        
        let (cc_nr, cc_cvc, cc_expiry, cc_brand) = match &item.router_data.request.payment_method_data {
            PaymentMethodData::Card(card) => {
                let expiry = format!("{:02}{}", card.card_exp_month.peek(), card.card_exp_year.peek());
                (
                    Some(card.card_number.clone()),
                    card.card_cvc.clone(),
                    Some(Secret::new(expiry)),
                    Some("card".to_string()),
                )
            }
            _ => (None, None, None, None),
        };

        let return_url = item.router_data.request.router_return_url.clone().unwrap_or_default();
        
        Ok(Self {
            merchant_id: auth.merchant_id,
            trans_id: item.router_data.connector_request_reference_id.clone(),
            amount: item.amount,
            currency: item.router_data.request.currency,
            url_success: return_url.clone(),
            url_failure: return_url.clone(),
            url_notify: return_url,
            response: "encrypt".to_string(),
            mac: "".to_string(), // MAC will be computed separately
            ref_nr: Some(item.router_data.payment_id.clone()),
            order_desc: item.router_data.request.statement_descriptor.clone(),
            req_id: Some(item.router_data.attempt_id.clone()),
            cc_nr,
            cc_cvc,
            cc_expiry,
            cc_brand,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ComputopPaymentStatus {
    OK,
    AUTHORIZED,
    CAPTURED,
    CANCELLED,
    FAILED,
    PENDING,
}

impl From<ComputopPaymentStatus> for enums::AttemptStatus {
    fn from(item: ComputopPaymentStatus) -> Self {
        match item {
            ComputopPaymentStatus::OK | ComputopPaymentStatus::CAPTURED => Self::Charged,
            ComputopPaymentStatus::AUTHORIZED => Self::Authorized,
            ComputopPaymentStatus::PENDING => Self::Pending,
            ComputopPaymentStatus::FAILED => Self::Failure,
            ComputopPaymentStatus::CANCELLED => Self::Voided,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputopPaymentsResponse {
    #[serde(rename = "Status")]
    pub status: ComputopPaymentStatus,
    #[serde(rename = "Code")]
    pub code: Option<String>,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "TransID")]
    pub trans_id: String,
    #[serde(rename = "PayID")]
    pub pay_id: String,
    #[serde(rename = "XID")]
    pub xid: Option<String>,
    #[serde(rename = "MAC")]
    pub mac: String,
    #[serde(rename = "RedirectURL")]
    pub redirect_url: Option<String>,
    #[serde(rename = "Amount")]
    pub amount: Option<MinorUnit>,
    #[serde(rename = "Currency")]
    pub currency: Option<String>,
}

impl<F, T>
    TryFrom<TResponseRouterData<F, ComputopPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: TResponseRouterData<F, ComputopPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.status.clone());
        
        let error_response = if matches!(item.response.status, ComputopPaymentStatus::FAILED) {
            Some(ErrorResponse {
                code: item.response.code.clone().unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                message: item.response.description.clone().unwrap_or_else(|| "Payment failed".to_string()),
                reason: item.response.description.clone(),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.pay_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            })
        } else {
            None
        };

        let redirection_data = item.response.redirect_url.map(|url| {
            RedirectForm::from((url::Url::parse(&url).unwrap(), common_utils::request::Method::Get))
        });

        let payments_response = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.pay_id.clone()),
            redirection_data: Box::new(redirection_data),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(item.response.trans_id),
            incremental_authorization_allowed: None,
            charges: None,
        };

        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response), Err),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ComputopCaptureRequest {
    #[serde(rename = "MerchantID")]
    pub merchant_id: Secret<String>,
    #[serde(rename = "PayID")]
    pub pay_id: String,
    #[serde(rename = "Amount")]
    pub amount: Option<MinorUnit>,
    #[serde(rename = "MAC")]
    pub mac: String,
}

impl TryFrom<&ComputopRouterData<&PaymentsCaptureRouterData>> for ComputopCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ComputopRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = ComputopAuthType::try_from(&item.router_data.connector_auth_type)?;
        
        Ok(Self {
            merchant_id: auth.merchant_id,
            pay_id: item.router_data.request.connector_transaction_id.clone(),
            amount: Some(item.amount),
            mac: "".to_string(), // MAC will be computed separately
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ComputopCancelRequest {
    #[serde(rename = "MerchantID")]
    pub merchant_id: Secret<String>,
    #[serde(rename = "PayID")]
    pub pay_id: String,
    #[serde(rename = "MAC")]
    pub mac: String,
}

impl TryFrom<&ComputopRouterData<&PaymentsCancelRouterData>> for ComputopCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ComputopRouterData<&PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = ComputopAuthType::try_from(&item.router_data.connector_auth_type)?;
        
        Ok(Self {
            merchant_id: auth.merchant_id,
            pay_id: item.router_data.request.connector_transaction_id.clone(),
            mac: "".to_string(), // MAC will be computed separately
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ComputopSyncRequest {
    #[serde(rename = "MerchantID")]
    pub merchant_id: Secret<String>,
    #[serde(rename = "PayID")]
    pub pay_id: String,
    #[serde(rename = "MAC")]
    pub mac: String,
}

impl TryFrom<&ComputopRouterData<&PaymentsSyncRouterData>> for ComputopSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ComputopRouterData<&PaymentsSyncRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = ComputopAuthType::try_from(&item.router_data.connector_auth_type)?;
        
        let pay_id = item.router_data.request.connector_transaction_id.clone()
            .get_required_value("connector_transaction_id")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_transaction_id",
            })?;
        
        Ok(Self {
            merchant_id: auth.merchant_id,
            pay_id,
            mac: "".to_string(), // MAC will be computed separately
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ComputopRefundRequest {
    #[serde(rename = "MerchantID")]
    pub merchant_id: Secret<String>,
    #[serde(rename = "PayID")]
    pub pay_id: String,
    #[serde(rename = "Amount")]
    pub amount: MinorUnit,
    #[serde(rename = "RefNr")]
    pub ref_nr: String,
    #[serde(rename = "MAC")]
    pub mac: String,
}

impl<F> TryFrom<&ComputopRouterData<&RefundsRouterData<F>>> for ComputopRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ComputopRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let auth = ComputopAuthType::try_from(&item.router_data.connector_auth_type)?;
        
        Ok(Self {
            merchant_id: auth.merchant_id,
            pay_id: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount,
            ref_nr: item.router_data.request.refund_id.clone(),
            mac: "".to_string(), // MAC will be computed separately
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputopRefundResponse {
    #[serde(rename = "Status")]
    pub status: ComputopPaymentStatus,
    #[serde(rename = "Code")]
    pub code: Option<String>,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "RefundID")]
    pub refund_id: String,
    #[serde(rename = "PayID")]
    pub pay_id: String,
    #[serde(rename = "Amount")]
    pub amount: Option<MinorUnit>,
    #[serde(rename = "MAC")]
    pub mac: String,
}

impl<F> TryFrom<RefundsResponseRouterData<F, ComputopRefundResponse>>
    for RefundsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<F, ComputopRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.status {
            ComputopPaymentStatus::OK => enums::RefundStatus::Success,
            ComputopPaymentStatus::PENDING => enums::RefundStatus::Pending,
            ComputopPaymentStatus::FAILED => enums::RefundStatus::Failure,
            _ => enums::RefundStatus::Pending,
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComputopErrorResponse {
    #[serde(rename = "Code")]
    pub code: Option<String>,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Status")]
    pub status: Option<String>,
}

impl From<ComputopErrorResponse> for ErrorResponse {
    fn from(error_response: ComputopErrorResponse) -> Self {
        Self {
            status_code: 0,
            code: error_response.code.unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: error_response.description,
            reason: error_response.status,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
        }
    }
}

// MAC computation utility - this would need to be implemented based on Computop's MAC algorithm
pub fn compute_mac(data: &str, key: &str) -> String {
    // Placeholder - implement actual MAC computation according to Computop's specification
    use common_utils::crypto;
    let hmac = crypto::HmacSha256::new(key.as_bytes());
    hmac.sign(data.as_bytes()).to_lowercase()
}