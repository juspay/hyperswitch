use common_enums::enums;
use common_utils::pii::{Email, IpAddress};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        ResponseId, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{self},
    errors,
};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        AddressDetailsData, BrowserInformationData, CardData, PaymentsAuthorizeRequestData,
        PaymentsCancelRequestData, PaymentsCaptureRequestData, PaymentsSetupMandateRequestData,
        RefundsRequestData, RouterData as RouterDataUtils,
    },
};

#[derive(Debug, Serialize)]
pub struct HelcimRouterData<T> {
    pub amount: f64,
    pub router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for HelcimRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (&api::CurrencyUnit, enums::Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        let amount = crate::utils::get_amount_as_f64(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

pub fn check_currency(
    currency: enums::Currency,
) -> Result<enums::Currency, errors::ConnectorError> {
    if currency == enums::Currency::USD {
        Ok(currency)
    } else {
        Err(errors::ConnectorError::NotSupported {
            message: format!("currency {currency} is not supported for this merchant account"),
            connector: "Helcim",
        })?
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimVerifyRequest {
    currency: enums::Currency,
    ip_address: Secret<String, IpAddress>,
    card_data: HelcimCard,
    billing_address: HelcimBillingAddress,
    #[serde(skip_serializing_if = "Option::is_none")]
    ecommerce: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimPaymentsRequest {
    amount: f64,
    currency: enums::Currency,
    ip_address: Secret<String, IpAddress>,
    card_data: HelcimCard,
    invoice: HelcimInvoice,
    billing_address: HelcimBillingAddress,
    //The ecommerce field is an optional field in Connector Helcim.
    //Setting the ecommerce field to true activates the Helcim Fraud Defender.
    #[serde(skip_serializing_if = "Option::is_none")]
    ecommerce: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimBillingAddress {
    name: Secret<String>,
    street1: Secret<String>,
    postal_code: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    street2: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<Email>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimInvoice {
    invoice_number: String,
    line_items: Vec<HelcimLineItems>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimLineItems {
    description: String,
    quantity: u8,
    price: f64,
    total: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimCard {
    card_number: cards::CardNumber,
    card_expiry: Secret<String>,
    card_c_v_v: Secret<String>,
}

impl TryFrom<(&SetupMandateRouterData, &Card)> for HelcimVerifyRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: (&SetupMandateRouterData, &Card)) -> Result<Self, Self::Error> {
        let (item, req_card) = value;
        let card_data = HelcimCard {
            card_expiry: req_card
                .get_card_expiry_month_year_2_digit_with_delimiter("".to_string())?,
            card_number: req_card.card_number.clone(),
            card_c_v_v: req_card.card_cvc.clone(),
        };
        let req_address = item.get_billing_address()?.to_owned();

        let billing_address = HelcimBillingAddress {
            name: req_address.get_full_name()?,
            street1: req_address.get_line1()?.to_owned(),
            postal_code: req_address.get_zip()?.to_owned(),
            street2: req_address.line2,
            city: req_address.city,
            email: item.request.email.clone(),
        };
        let ip_address = item.request.get_browser_info()?.get_ip_address()?;
        let currency = check_currency(item.request.currency)?;
        Ok(Self {
            currency,
            ip_address,
            card_data,
            billing_address,
            ecommerce: None,
        })
    }
}

impl TryFrom<&SetupMandateRouterData> for HelcimVerifyRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => Self::try_from((item, &req_card)),
            PaymentMethodData::BankTransfer(_) => {
                Err(errors::ConnectorError::NotImplemented("Payment Method".to_string()).into())
            }
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    crate::utils::get_unimplemented_payment_method_error_message("Helcim"),
                ))?
            }
        }
    }
}

impl TryFrom<(&HelcimRouterData<&PaymentsAuthorizeRouterData>, &Card)> for HelcimPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&HelcimRouterData<&PaymentsAuthorizeRouterData>, &Card),
    ) -> Result<Self, Self::Error> {
        let (item, req_card) = value;
        let card_data = HelcimCard {
            card_expiry: req_card
                .get_card_expiry_month_year_2_digit_with_delimiter("".to_string())?,
            card_number: req_card.card_number.clone(),
            card_c_v_v: req_card.card_cvc.clone(),
        };
        let req_address = item
            .router_data
            .get_billing()?
            .to_owned()
            .address
            .ok_or_else(crate::utils::missing_field_err("billing.address"))?;

        let billing_address = HelcimBillingAddress {
            name: req_address.get_full_name()?,
            street1: req_address.get_line1()?.to_owned(),
            postal_code: req_address.get_zip()?.to_owned(),
            street2: req_address.line2,
            city: req_address.city,
            email: item.router_data.request.email.clone(),
        };

        let ip_address = item
            .router_data
            .request
            .get_browser_info()?
            .get_ip_address()?;
        let line_items = vec![
            (HelcimLineItems {
                description: item
                    .router_data
                    .description
                    .clone()
                    .unwrap_or("No Description".to_string()),
                // By default quantity is set to 1 and price and total is set to amount because these three fields are required to generate an invoice.
                quantity: 1,
                price: item.amount,
                total: item.amount,
            }),
        ];
        let invoice = HelcimInvoice {
            invoice_number: item.router_data.connector_request_reference_id.clone(),
            line_items,
        };
        let currency = check_currency(item.router_data.request.currency)?;
        Ok(Self {
            amount: item.amount,
            currency,
            ip_address,
            card_data,
            invoice,
            billing_address,
            ecommerce: None,
        })
    }
}

impl TryFrom<&HelcimRouterData<&PaymentsAuthorizeRouterData>> for HelcimPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &HelcimRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => Self::try_from((item, &req_card)),
            PaymentMethodData::BankTransfer(_) => {
                Err(errors::ConnectorError::NotImplemented("Payment Method".to_string()).into())
            }
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    crate::utils::get_unimplemented_payment_method_error_message("Helcim"),
                ))?
            }
        }
    }
}

// Auth Struct
pub struct HelcimAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for HelcimAuthType {
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
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HelcimPaymentStatus {
    Approved,
    Declined,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HelcimTransactionType {
    Purchase,
    PreAuth,
    Capture,
    Verify,
    Reverse,
}

impl From<HelcimPaymentsResponse> for enums::AttemptStatus {
    fn from(item: HelcimPaymentsResponse) -> Self {
        match item.transaction_type {
            HelcimTransactionType::Purchase | HelcimTransactionType::Verify => match item.status {
                HelcimPaymentStatus::Approved => Self::Charged,
                HelcimPaymentStatus::Declined => Self::Failure,
            },
            HelcimTransactionType::PreAuth => match item.status {
                HelcimPaymentStatus::Approved => Self::Authorized,
                HelcimPaymentStatus::Declined => Self::AuthorizationFailed,
            },
            HelcimTransactionType::Capture => match item.status {
                HelcimPaymentStatus::Approved => Self::Charged,
                HelcimPaymentStatus::Declined => Self::CaptureFailed,
            },
            HelcimTransactionType::Reverse => match item.status {
                HelcimPaymentStatus::Approved => Self::Voided,
                HelcimPaymentStatus::Declined => Self::VoidFailed,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimPaymentsResponse {
    status: HelcimPaymentStatus,
    transaction_id: u64,
    invoice_number: Option<String>,
    #[serde(rename = "type")]
    transaction_type: HelcimTransactionType,
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<F, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.to_string(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.invoice_number.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            status: enums::AttemptStatus::from(item.response),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HelcimMetaData {
    pub preauth_transaction_id: u64,
}

impl<F>
    TryFrom<
        ResponseRouterData<F, HelcimPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        //PreAuth Transaction ID is stored in connector metadata
        //Initially resource_id is stored as NoResponseID for manual capture
        //After Capture Transaction is completed it is updated to store the Capture ID
        let resource_id = if item.data.request.is_auto_capture()? {
            ResponseId::ConnectorTransactionId(item.response.transaction_id.to_string())
        } else {
            ResponseId::NoResponseId
        };
        let connector_metadata = if !item.data.request.is_auto_capture()? {
            Some(serde_json::json!(HelcimMetaData {
                preauth_transaction_id: item.response.transaction_id,
            }))
        } else {
            None
        };
        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: item.response.invoice_number.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            status: enums::AttemptStatus::from(item.response),
            ..item.data
        })
    }
}

// impl utils::MultipleCaptureSyncResponse for HelcimPaymentsResponse {
//     fn get_connector_capture_id(&self) -> String {
//         self.transaction_id.to_string()
//     }

//     fn get_capture_attempt_status(&self) -> diesel_models::enums::AttemptStatus {
//         enums::AttemptStatus::from(self.to_owned())
//     }

//     fn is_capture_response(&self) -> bool {
//         true
//     }

//     fn get_amount_captured(&self) -> Option<i64> {
//         Some(self.amount)
//     }
//     fn get_connector_reference_id(&self) -> Option<String> {
//         None
//     }
// }

impl<F>
    TryFrom<ResponseRouterData<F, HelcimPaymentsResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, HelcimPaymentsResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.data.request.sync_type {
            hyperswitch_domain_models::router_request_types::SyncRequestType::SinglePaymentSync => Ok(Self {
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.response.transaction_id.to_string(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: item.response.invoice_number.clone(),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                status: enums::AttemptStatus::from(item.response),
                ..item.data
            }),
            hyperswitch_domain_models::router_request_types::SyncRequestType::MultipleCaptureSync(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "manual multiple capture sync".to_string(),
                )
                .into())
                // let capture_sync_response_list =
                //     utils::construct_captures_response_hashmap(vec![item.response]);
                // Ok(Self {
                //     response: Ok(PaymentsResponseData::MultipleCaptureResponse {
                //         capture_sync_response_list,
                //     }),
                //     ..item.data
                // })
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimCaptureRequest {
    pre_auth_transaction_id: u64,
    amount: f64,
    ip_address: Secret<String, IpAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ecommerce: Option<bool>,
}

impl TryFrom<&HelcimRouterData<&PaymentsCaptureRouterData>> for HelcimCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &HelcimRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let ip_address = item
            .router_data
            .request
            .get_browser_info()?
            .get_ip_address()?;
        Ok(Self {
            pre_auth_transaction_id: item
                .router_data
                .request
                .connector_transaction_id
                .parse::<u64>()
                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
            amount: item.amount,
            ip_address,
            ecommerce: None,
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, HelcimPaymentsResponse, PaymentsCaptureData, PaymentsResponseData>,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.to_string(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.invoice_number.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            status: enums::AttemptStatus::from(item.response),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimVoidRequest {
    card_transaction_id: u64,
    ip_address: Secret<String, IpAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ecommerce: Option<bool>,
}

impl TryFrom<&PaymentsCancelRouterData> for HelcimVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let ip_address = item.request.get_browser_info()?.get_ip_address()?;
        Ok(Self {
            card_transaction_id: item
                .request
                .connector_transaction_id
                .parse::<u64>()
                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
            ip_address,
            ecommerce: None,
        })
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, HelcimPaymentsResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            HelcimPaymentsResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.to_string(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.invoice_number.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            status: enums::AttemptStatus::from(item.response),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelcimRefundRequest {
    amount: f64,
    original_transaction_id: u64,
    ip_address: Secret<String, IpAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ecommerce: Option<bool>,
}

impl<F> TryFrom<&HelcimRouterData<&RefundsRouterData<F>>> for HelcimRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &HelcimRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let original_transaction_id = item
            .router_data
            .request
            .connector_transaction_id
            .parse::<u64>()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let ip_address = item
            .router_data
            .request
            .get_browser_info()?
            .get_ip_address()?;
        Ok(Self {
            amount: item.amount,
            original_transaction_id,
            ip_address,
            ecommerce: None,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HelcimRefundTransactionType {
    Refund,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    status: HelcimPaymentStatus,
    transaction_id: u64,
    #[serde(rename = "type")]
    transaction_type: HelcimRefundTransactionType,
}

impl From<RefundResponse> for enums::RefundStatus {
    fn from(item: RefundResponse) -> Self {
        match item.transaction_type {
            HelcimRefundTransactionType::Refund => match item.status {
                HelcimPaymentStatus::Approved => Self::Success,
                HelcimPaymentStatus::Declined => Self::Failure,
            },
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response),
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
                connector_refund_id: item.response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, strum::Display, Deserialize, Serialize)]
#[serde(untagged)]
pub enum HelcimErrorTypes {
    StringType(String),
    JsonType(serde_json::Value),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HelcimPaymentsErrorResponse {
    pub errors: HelcimErrorTypes,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum HelcimErrorResponse {
    Payment(HelcimPaymentsErrorResponse),
    General(String),
}
