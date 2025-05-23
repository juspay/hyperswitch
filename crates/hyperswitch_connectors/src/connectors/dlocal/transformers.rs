use common_enums::enums;
use common_utils::pii::Email;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::ConnectorAuthType,
    router_flow_types::{
        payments::{Authorize as AuthorizeFlow, Capture as CaptureFlow, PSync as PSyncFlow},
        refunds::{Execute as RefundExecuteFlow, RSync as RefundSyncFlow},
    },
    router_request_types::{self as router_req_types, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
// Duplicate imports of enums and serde removed by only having them once.
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::ResponseRouterData,
    // PaymentsSyncRequestData was already removed, ensure other utils are correctly used.
    utils::{
        AddressDetailsData, PaymentsAuthorizeRequestData, PaymentsCaptureRequestData,
        RefundsRequestData, RouterData as _,
    }, // Added PaymentsCaptureRequestData
};

pub struct DlocalRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T> From<(i64, T)> for DlocalRouterData<T> {
    fn from((amount, item): (i64, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct DlocalPayer {
    name: Secret<String>,
    email: Email,
    document: Option<Secret<String>>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DlocalCard {
    holder_name: Secret<String>,
    number: cards::CardNumber,
    cvv: Secret<String>,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    #[serde(rename = "capture")]
    auto_capture: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DlocalPaymentsRequest {
    amount: i64,
    currency: enums::Currency,
    country: String,
    payment_method_id: String,
    payment_method_flow: String,
    payer: DlocalPayer,
    card: DlocalCard,
    order_id: String,
    notification_url: String,
}

impl TryFrom<&DlocalRouterData<&PaymentsAuthorizeRouterData>> for DlocalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DlocalRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let card_details = match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(card) => card,
            _ => {
                return Err(errors::ConnectorError::NotImplemented(
                    "Payment method not supported by Dlocal".to_string(),
                )
                .into())
            }
        };

        let dlocal_card = DlocalCard {
            holder_name: card_details
                .card_holder_name
                .clone()
                .unwrap_or_else(|| Secret::new("Not Available".to_string())),
            number: card_details.card_number,
            cvv: card_details.card_cvc,
            expiration_month: card_details.card_exp_month,
            expiration_year: card_details.card_exp_year,
            auto_capture: item.router_data.request.capture_method
                == Some(enums::CaptureMethod::Automatic),
        };

        let billing_address = item.router_data.get_billing_address()?;
        let payer_name = billing_address
            .get_full_name()
            .unwrap_or_else(|_| Secret::new("Unknown".to_string()));
        // Changed to use item.router_data.request.email directly
        let payer_email = item.router_data.request.email.clone().ok_or_else(|| {
            errors::ConnectorError::MissingRequiredField {
                field_name: "email", // Changed from "email".to_string()
            }
        })?;

        let dlocal_payer = DlocalPayer {
            name: payer_name,
            email: payer_email,
            document: None,
        };

        let country_code = billing_address.get_country()?.to_string().to_uppercase();

        Ok(Self {
            amount: item.amount,
            currency: item.router_data.request.currency,
            country: country_code,
            payment_method_id: "CARD".to_string(),
            payment_method_flow: "DIRECT".to_string(),
            payer: dlocal_payer,
            card: dlocal_card,
            order_id: item.router_data.connector_request_reference_id.clone(),
            notification_url: item.router_data.request.get_webhook_url()?,
        })
    }
}

pub struct DlocalAuthType {
    pub x_login: Secret<String>,
    pub x_trans_key: Secret<String>,
    pub secret_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for DlocalAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                x_login: api_key.to_owned(),
                x_trans_key: key1.to_owned(),
                secret_key: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DlocalPaymentStatus {
    AUTHORIZED,
    PAID,
    REJECTED,
    CANCELLED,
    PENDING,
    #[default]
    #[serde(other)]
    Unknown,
}

impl From<DlocalPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: DlocalPaymentStatus) -> Self {
        match item {
            DlocalPaymentStatus::AUTHORIZED => Self::Authorized,
            DlocalPaymentStatus::PAID => Self::Charged,
            DlocalPaymentStatus::REJECTED => Self::Failure,
            DlocalPaymentStatus::CANCELLED => Self::Voided,
            DlocalPaymentStatus::PENDING => Self::Pending,
            DlocalPaymentStatus::Unknown => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalResponseCard {
    holder_name: Option<Secret<String>>,
    expiration_month: Option<Secret<String>>,
    expiration_year: Option<Secret<String>>,
    brand: Option<String>,
    last4: Option<String>,
    card_id: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalPaymentsResponse {
    id: String,
    status: DlocalPaymentStatus,
    status_code: Option<String>,
    status_detail: Option<String>,
    amount: Option<f64>, // Dlocal response amount is f64 major unit
    currency: Option<String>,
    order_id: Option<String>,
    card: Option<DlocalResponseCard>,
}

// TryFrom for PaymentsAuthorizeRouterData
impl
    TryFrom<
        ResponseRouterData<
            AuthorizeFlow,
            DlocalPaymentsResponse,
            router_req_types::PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            AuthorizeFlow,
            DlocalPaymentsResponse,
            router_req_types::PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // Self is PaymentsAuthorizeRouterData
            status: common_enums::AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.order_id.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// TryFrom for PaymentsSyncRouterData
impl
    TryFrom<
        ResponseRouterData<
            PSyncFlow,
            DlocalPaymentsResponse,
            router_req_types::PaymentsSyncData,
            PaymentsResponseData,
        >,
    > for PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            PSyncFlow,
            DlocalPaymentsResponse,
            router_req_types::PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // Self is PaymentsSyncRouterData
            status: common_enums::AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.order_id.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// DlocalCaptureRequest struct and its TryFrom implementation
#[derive(Debug, Serialize, PartialEq)]
pub struct DlocalCaptureRequest {
    authorization_id: String,
    amount: i64,
    currency: enums::Currency,
    order_id: String,
    notification_url: String, // Dlocal capture might also need a notification_url
}

impl TryFrom<&PaymentsCaptureRouterData> for DlocalCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            authorization_id: item.request.connector_transaction_id.clone(),
            amount: item.request.minor_amount_to_capture.get_amount_as_i64(),
            currency: item.request.currency,
            order_id: item.connector_request_reference_id.clone(),
            notification_url: item.request.get_webhook_url()?,
        })
    }
}

// TryFrom for PaymentsCaptureRouterData
impl
    TryFrom<
        ResponseRouterData<
            CaptureFlow,
            DlocalPaymentsResponse,
            router_req_types::PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            CaptureFlow,
            DlocalPaymentsResponse,
            router_req_types::PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // Self is PaymentsCaptureRouterData
            status: common_enums::AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.order_id.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// TryFrom for PaymentsCancelRouterData (Void Flow)
// Assuming Dlocal void/cancel returns a DlocalPaymentsResponse
impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Void,
            DlocalPaymentsResponse,
            router_req_types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    > for hyperswitch_domain_models::types::PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Void,
            DlocalPaymentsResponse,
            router_req_types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // Self is PaymentsCancelRouterData
            status: common_enums::AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.order_id.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct DlocalRefundRequest {
    payment_id: String,
    amount: i64,
    currency: enums::Currency,
    notification_url: String,
}

impl<F> TryFrom<&DlocalRouterData<&RefundsRouterData<F>>> for DlocalRefundRequest {
    // Removed RefundableFlow bound
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &DlocalRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_id: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount,
            currency: item.router_data.request.currency,
            notification_url: item.router_data.request.get_webhook_url()?,
        })
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum DlocalRefundStatus {
    SUCCESS,
    FAILED,
    #[default]
    #[serde(other)]
    PENDING,
}

impl From<DlocalRefundStatus> for enums::RefundStatus {
    fn from(item: DlocalRefundStatus) -> Self {
        match item {
            DlocalRefundStatus::SUCCESS => Self::Success,
            DlocalRefundStatus::FAILED => Self::Failure,
            DlocalRefundStatus::PENDING => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DlocalRefundResponse {
    id: String,
    payment_id: Option<String>,
    amount: Option<f64>, // Dlocal response amount is f64 major unit
    amount_refunded: Option<f64>,
    currency: Option<String>,
    status: DlocalRefundStatus,
    status_code: Option<i64>,
    status_detail: Option<String>,
    created_date: Option<String>,
    order_id: Option<String>,
}

// TryFrom for RefundsRouterData<Execute>
impl
    TryFrom<
        ResponseRouterData<
            RefundExecuteFlow,
            DlocalRefundResponse,
            router_req_types::RefundsData,
            RefundsResponseData,
        >,
    > for RefundsRouterData<RefundExecuteFlow>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            RefundExecuteFlow,
            DlocalRefundResponse,
            router_req_types::RefundsData,
            RefundsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // Self is RefundsRouterData<Execute>
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

// TryFrom for RefundSyncRouterData (which is RouterData<RSync, RefundsData, RefundsResponseData>)
impl
    TryFrom<
        ResponseRouterData<
            RefundSyncFlow,
            DlocalRefundResponse,
            router_req_types::RefundsData,
            RefundsResponseData,
        >,
    > for RefundSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            RefundSyncFlow,
            DlocalRefundResponse,
            router_req_types::RefundsData,
            RefundsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // Self is RefundSyncRouterData
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DlocalErrorResponse {
    pub status_code: Option<String>,
    pub code: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
}
