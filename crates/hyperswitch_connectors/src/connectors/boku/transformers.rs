use std::fmt;

use common_enums::enums;
use common_utils::{request::Method, types::MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{self, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, AddressDetailsData, RouterData as _},
};

#[derive(Debug, Serialize)]
pub struct BokuRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for BokuRouterData<T> {
    fn from((amount, router_data): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BokuPaymentsRequest {
    BeginSingleCharge(SingleChargeData),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SingleChargeData {
    total_amount: MinorUnit,
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

impl TryFrom<&BokuRouterData<&types::PaymentsAuthorizeRouterData>> for BokuPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BokuRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Wallet(wallet_data) => Self::try_from((item, &wallet_data)),
            PaymentMethodData::Card(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("boku"),
                ))?
            }
        }
    }
}

impl
    TryFrom<(
        &BokuRouterData<&types::PaymentsAuthorizeRouterData>,
        &WalletData,
    )> for BokuPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: (
            &BokuRouterData<&types::PaymentsAuthorizeRouterData>,
            &WalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item_router_data, wallet_data) = value;
        let item = item_router_data.router_data;
        let address = item.get_billing_address()?;
        let country = address.get_country()?.to_string();
        let payment_method = get_wallet_type(wallet_data)?;
        let hosted = get_hosted_data(item);
        let auth_type = BokuAuthType::try_from(&item.connector_auth_type)?;
        let merchant_item_description = item.get_description()?;
        let payment_data = SingleChargeData {
            total_amount: item_router_data.amount,
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

fn get_wallet_type(wallet_data: &WalletData) -> Result<String, errors::ConnectorError> {
    match wallet_data {
        WalletData::DanaRedirect { .. } => Ok(BokuPaymentType::Dana.to_string()),
        WalletData::MomoRedirect { .. } => Ok(BokuPaymentType::Momo.to_string()),
        WalletData::GcashRedirect { .. } => Ok(BokuPaymentType::Gcash.to_string()),
        WalletData::GoPayRedirect { .. } => Ok(BokuPaymentType::GoPay.to_string()),
        WalletData::KakaoPayRedirect { .. } => Ok(BokuPaymentType::Kakaopay.to_string()),
        WalletData::AliPayQr(_)
        | WalletData::AliPayRedirect(_)
        | WalletData::AliPayHkRedirect(_)
        | WalletData::AmazonPayRedirect(_)
        | WalletData::ApplePay(_)
        | WalletData::ApplePayRedirect(_)
        | WalletData::ApplePayThirdPartySdk(_)
        | WalletData::GooglePay(_)
        | WalletData::GooglePayRedirect(_)
        | WalletData::GooglePayThirdPartySdk(_)
        | WalletData::MbWayRedirect(_)
        | WalletData::MobilePayRedirect(_)
        | WalletData::PaypalRedirect(_)
        | WalletData::PaypalSdk(_)
        | WalletData::Paze(_)
        | WalletData::SamsungPay(_)
        | WalletData::TwintRedirect {}
        | WalletData::VippsRedirect {}
        | WalletData::TouchNGoRedirect(_)
        | WalletData::WeChatPayRedirect(_)
        | WalletData::WeChatPayQr(_)
        | WalletData::CashappQr(_)
        | WalletData::SwishQr(_)
        | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("boku"),
        )),
    }
}

pub struct BokuAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) key_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BokuAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BokuResponse {
    BeginSingleChargeResponse(BokuPaymentsResponse),
    QueryChargeResponse(BokuPsyncResponse),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BokuPaymentsResponse {
    charge_status: String, // xml parse only string to fields
    charge_id: String,
    hosted: Option<HostedUrlResponse>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct HostedUrlResponse {
    redirect_url: Option<Url>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "query-charge-response")]
#[serde(rename_all = "kebab-case")]
pub struct BokuPsyncResponse {
    charges: ChargeResponseData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ChargeResponseData {
    charge: SingleChargeResponseData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SingleChargeResponseData {
    charge_status: String,
    charge_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, BokuResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BokuResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, transaction_id, redirection_data) = match item.response {
            BokuResponse::BeginSingleChargeResponse(response) => get_authorize_response(response),
            BokuResponse::QueryChargeResponse(response) => get_psync_response(response),
        }?;

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id),
                redirection_data: Box::new(redirection_data),
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
            .map(|url| RedirectForm::from((url, Method::Get)))),
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
    refund_amount: MinorUnit,
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

impl<F> TryFrom<&BokuRouterData<&RefundsRouterData<F>>> for BokuRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &BokuRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = BokuAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payment_data = Self {
            refund_amount: item.amount,
            merchant_id: auth_type.merchant_id,
            merchant_refund_id: Secret::new(item.router_data.request.refund_id.to_string()),
            merchant_request_id: Uuid::new_v4().to_string(),
            charge_id: item
                .router_data
                .request
                .connector_transaction_id
                .to_string(),
            reason_code: BokuRefundReasonCode::NonFulfillment.to_string(),
        };

        Ok(payment_data)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "query-refund-response")]
#[serde(rename_all = "kebab-case")]
pub struct BokuRsyncResponse {
    refunds: RefundResponseData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefundResponseData {
    refund: SingleRefundResponseData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SingleRefundResponseData {
    refund_status: String, // quick-xml only parse string as a field
    refund_id: String,
}

impl TryFrom<RefundsResponseRouterData<RSync, BokuRsyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, BokuRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
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

fn get_hosted_data(item: &types::PaymentsAuthorizeRouterData) -> Option<BokuHostedData> {
    item.request
        .router_return_url
        .clone()
        .map(|url| BokuHostedData { forward_url: url })
}
