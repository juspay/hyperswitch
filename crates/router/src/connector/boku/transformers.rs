use std::fmt;

use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    connector::utils::{self, AddressDetailsData, RouterData},
    core::errors,
    services::{self, RedirectForm},
    types::{self, api, storage::enums},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BokuPaymentsRequest {
    BeginSingleCharge(SingleChargeData),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SingleChargeData {
    total_amount: i64,
    currency: String,
    country: String,
    merchant_id: Secret<String>,
    merchant_transaction_id: Secret<String>,
    merchant_request_id: String,
    merchant_item_description: String,
    notification_url: Option<String>,
    payment_method: String,
    charge_type: String,
    hosted: Option<BokuHostedData>,
}

#[derive(Debug, Clone, Serialize)]
pub enum BokuPaymentType {
    Dana,
    Momo,
    Gcash,
    GoPay,
    Kakaopay,
}

impl fmt::Display for BokuPaymentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dana => write!(f, "Dana"),
            Self::Momo => write!(f, "Momo"),
            Self::Gcash => write!(f, "Gcash"),
            Self::GoPay => write!(f, "GoPay"),
            Self::Kakaopay => write!(f, "Kakaopay"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum BokuChargeType {
    Hosted,
}

impl fmt::Display for BokuChargeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hosted => write!(f, "hosted"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
struct BokuHostedData {
    forward_url: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BokuPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api_models::payments::PaymentMethodData::Wallet(wallet_data) => {
                Self::try_from((item, &wallet_data))
            }
            api_models::payments::PaymentMethodData::Card(_)
            | api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_)
            | api_models::payments::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("boku"),
                ))?
            }
        }
    }
}

impl TryFrom<(&types::PaymentsAuthorizeRouterData, &api::WalletData)> for BokuPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (&types::PaymentsAuthorizeRouterData, &api::WalletData),
    ) -> Result<Self, Self::Error> {
        let (item, wallet_data) = value;
        let address = item.get_billing_address()?;
        let country = address.get_country()?.to_string();
        let payment_method = get_wallet_type(wallet_data)?;
        let hosted = get_hosted_data(item);
        let auth_type = BokuAuthType::try_from(&item.connector_auth_type)?;
        let merchant_item_description = item.get_description()?;
        let payment_data = SingleChargeData {
            total_amount: item.request.amount,
            currency: item.request.currency.to_string(),
            country,
            merchant_id: auth_type.merchant_id,
            merchant_transaction_id: Secret::new(item.payment_id.to_string()),
            merchant_request_id: Uuid::new_v4().to_string(),
            merchant_item_description,
            notification_url: item.request.webhook_url.clone(),
            payment_method,
            charge_type: BokuChargeType::Hosted.to_string(),
            hosted,
        };

        Ok(Self::BeginSingleCharge(payment_data))
    }
}

fn get_wallet_type(wallet_data: &api::WalletData) -> Result<String, errors::ConnectorError> {
    match wallet_data {
        api_models::payments::WalletData::DanaRedirect { .. } => {
            Ok(BokuPaymentType::Dana.to_string())
        }
        api_models::payments::WalletData::MomoRedirect { .. } => {
            Ok(BokuPaymentType::Momo.to_string())
        }
        api_models::payments::WalletData::GcashRedirect { .. } => {
            Ok(BokuPaymentType::Gcash.to_string())
        }
        api_models::payments::WalletData::GoPayRedirect { .. } => {
            Ok(BokuPaymentType::GoPay.to_string())
        }
        api_models::payments::WalletData::KakaoPayRedirect { .. } => {
            Ok(BokuPaymentType::Kakaopay.to_string())
        }
        api_models::payments::WalletData::AliPayQr(_)
        | api_models::payments::WalletData::AliPayRedirect(_)
        | api_models::payments::WalletData::AliPayHkRedirect(_)
        | api_models::payments::WalletData::ApplePay(_)
        | api_models::payments::WalletData::ApplePayRedirect(_)
        | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
        | api_models::payments::WalletData::GooglePay(_)
        | api_models::payments::WalletData::GooglePayRedirect(_)
        | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
        | api_models::payments::WalletData::MbWayRedirect(_)
        | api_models::payments::WalletData::MobilePayRedirect(_)
        | api_models::payments::WalletData::PaypalRedirect(_)
        | api_models::payments::WalletData::PaypalSdk(_)
        | api_models::payments::WalletData::SamsungPay(_)
        | api_models::payments::WalletData::TwintRedirect {}
        | api_models::payments::WalletData::VippsRedirect {}
        | api_models::payments::WalletData::TouchNGoRedirect(_)
        | api_models::payments::WalletData::WeChatPayRedirect(_)
        | api_models::payments::WalletData::WeChatPayQr(_)
        | api_models::payments::WalletData::CashappQr(_)
        | api_models::payments::WalletData::SwishQr(_) => {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("boku"),
            ))
        }
    }
}

pub struct BokuAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) key_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for BokuAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                merchant_id: key1.to_owned(),
                key_id: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename = "query-charge-request")]
#[serde(rename_all = "kebab-case")]
pub struct BokuPsyncRequest {
    country: String,
    merchant_id: Secret<String>,
    merchant_transaction_id: Secret<String>,
}

impl TryFrom<&types::PaymentsSyncRouterData> for BokuPsyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let address = item.get_billing_address()?;
        let country = address.get_country()?.to_string();
        let auth_type = BokuAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            country,
            merchant_id: auth_type.merchant_id,
            merchant_transaction_id: Secret::new(item.payment_id.to_string()),
        })
    }
}

// Connector Meta Data
#[derive(Debug, Clone, Deserialize)]
pub struct BokuMetaData {
    pub(super) country: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BokuResponse {
    BeginSingleChargeResponse(BokuPaymentsResponse),
    QueryChargeResponse(BokuPsyncResponse),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BokuPaymentsResponse {
    charge_status: String, // xml parse only string to fields
    charge_id: String,
    hosted: Option<HostedUrlResponse>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HostedUrlResponse {
    redirect_url: Option<Url>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "query-charge-response")]
#[serde(rename_all = "kebab-case")]
pub struct BokuPsyncResponse {
    charges: ChargeResponseData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ChargeResponseData {
    charge: SingleChargeResponseData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SingleChargeResponseData {
    charge_status: String,
    charge_id: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, BokuResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, BokuResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, transaction_id, redirection_data) = match item.response {
            BokuResponse::BeginSingleChargeResponse(response) => get_authorize_response(response),
            BokuResponse::QueryChargeResponse(response) => get_psync_response(response),
        }?;

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(transaction_id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

fn get_response_status(status: String) -> enums::AttemptStatus {
    match status.as_str() {
        "Success" => enums::AttemptStatus::Charged,
        "Failure" => enums::AttemptStatus::Failure,
        _ => enums::AttemptStatus::Pending,
    }
}

fn get_authorize_response(
    response: BokuPaymentsResponse,
) -> Result<(enums::AttemptStatus, String, Option<RedirectForm>), errors::ConnectorError> {
    let status = get_response_status(response.charge_status);
    let redirection_data = match response.hosted {
        Some(hosted_value) => Ok(hosted_value
            .redirect_url
            .map(|url| services::RedirectForm::from((url, services::Method::Get)))),
        None => Err(errors::ConnectorError::MissingConnectorRedirectionPayload {
            field_name: "redirect_url",
        }),
    }?;

    Ok((status, response.charge_id, redirection_data))
}

fn get_psync_response(
    response: BokuPsyncResponse,
) -> Result<(enums::AttemptStatus, String, Option<RedirectForm>), errors::ConnectorError> {
    let status = get_response_status(response.charges.charge.charge_status);

    Ok((status, response.charges.charge.charge_id, None))
}

// REFUND :
#[derive(Debug, Clone, Serialize)]
#[serde(rename = "refund-charge-request")]
pub struct BokuRefundRequest {
    refund_amount: i64,
    merchant_id: Secret<String>,
    merchant_request_id: String,
    merchant_refund_id: Secret<String>,
    charge_id: String,
    reason_code: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum BokuRefundReasonCode {
    NonFulfillment,
}

impl fmt::Display for BokuRefundReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFulfillment => write!(f, "8"),
        }
    }
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BokuRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = BokuAuthType::try_from(&item.connector_auth_type)?;
        let payment_data = Self {
            refund_amount: item.request.refund_amount,
            merchant_id: auth_type.merchant_id,
            merchant_refund_id: Secret::new(item.request.refund_id.to_string()),
            merchant_request_id: Uuid::new_v4().to_string(),
            charge_id: item.request.connector_transaction_id.to_string(),
            reason_code: BokuRefundReasonCode::NonFulfillment.to_string(),
        };

        Ok(payment_data)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "refund-charge-response")]
pub struct RefundResponse {
    charge_id: String,
    refund_status: String,
}

fn get_refund_status(status: String) -> enums::RefundStatus {
    match status.as_str() {
        "Success" => enums::RefundStatus::Success,
        "Failure" => enums::RefundStatus::Failure,
        _ => enums::RefundStatus::Pending,
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.charge_id,
                refund_status: get_refund_status(item.response.refund_status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename = "query-refund-request")]
#[serde(rename_all = "kebab-case")]
pub struct BokuRsyncRequest {
    country: String,
    merchant_id: Secret<String>,
    merchant_transaction_id: Secret<String>,
}

impl TryFrom<&types::RefundSyncRouterData> for BokuRsyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let address = item.get_billing_address()?;
        let country = address.get_country()?.to_string();
        let auth_type = BokuAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            country,
            merchant_id: auth_type.merchant_id,
            merchant_transaction_id: Secret::new(item.payment_id.to_string()),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "query-refund-response")]
#[serde(rename_all = "kebab-case")]
pub struct BokuRsyncResponse {
    refunds: RefundResponseData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefundResponseData {
    refund: SingleRefundResponseData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SingleRefundResponseData {
    refund_status: String, // quick-xml only parse string as a field
    refund_id: String,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, BokuRsyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, BokuRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.refunds.refund.refund_id,
                refund_status: get_refund_status(item.response.refunds.refund.refund_status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct BokuErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BokuConnMetaData {
    country: String,
}

fn get_hosted_data(item: &types::PaymentsAuthorizeRouterData) -> Option<BokuHostedData> {
    item.return_url
        .clone()
        .map(|url| BokuHostedData { forward_url: url })
}
